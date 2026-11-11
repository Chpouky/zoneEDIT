//! cfg document in memory: intact source text + index of the lexed lines.
//!
//! The text is kept as-is (line endings included): the rendering
//! only replaces the bytes of the modified values, everything else
//! (whitespaces, comments, CRLF/LF even mixed, BOM, end of file) is
//! restored byte for byte.

use super::lexer::{lex_line, LineKind, Span};

const BOM: &str = "\u{feff}";

/// Une ligne du document.
#[derive(Debug, Clone, Copy)]
pub struct Line {
    /// Start byte in the text (without BOM).
    pub start: u32,
    /// Octet de fin, fin de ligne exclue (`\r\n` ou `\n`).
    pub end: u32,
    /// Nature of the line (lexed).
    pub kind: LineKind,
}

/// Erreur de lecture d'un document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocError {
    /// Le fichier n'est pas de l'UTF-8 valide.
    InvalidUtf8,
}

/// Remplacement de la valeur d'une ligne.
#[derive(Debug, Clone)]
pub struct ValueEdit<'a> {
    /// Ligne 1-based.
    pub line: usize,
    /// Nouvelle valeur (sans commentaire, sans `=`).
    pub value: &'a str,
}

/// Rendering error: the targeted line has no `key = value` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoValueOnLine(
    /// 1-based line concerned.
    pub usize,
);

/// cfg document: intact source text + index of the lexed lines.
#[derive(Debug, Clone)]
pub struct Document {
    /// Text without BOM, line endings included.
    text: String,
    /// The file started with a UTF-8 BOM.
    bom: bool,
    lines: Vec<Line>,
    crlf_lines: usize,
    lf_lines: usize,
    begins: usize,
    ends: usize,
}

impl Document {
    /// Decodes (strict UTF-8, optional BOM) and indexes a file.
    pub fn parse(bytes: &[u8]) -> Result<Self, DocError> {
        let text = std::str::from_utf8(bytes).map_err(|_| DocError::InvalidUtf8)?;
        Ok(Self::from_text(text))
    }

    /// Indexes already-decoded text (optional BOM).
    pub fn from_text(text: &str) -> Self {
        let (bom, body) = match text.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let mut doc = Document {
            text: body.to_string(),
            bom,
            lines: Vec::with_capacity(body.len() / 24),
            crlf_lines: 0,
            lf_lines: 0,
            begins: 0,
            ends: 0,
        };
        doc.index();
        doc
    }

    fn index(&mut self) {
        let bytes = self.text.as_bytes();
        let mut start = 0usize;
        while start < bytes.len() {
            let (end, next) = match memchr_nl(&bytes[start..]) {
                Some(i) => {
                    let nl = start + i;
                    if nl > start && bytes[nl - 1] == b'\r' {
                        self.crlf_lines += 1;
                        (nl - 1, nl + 1)
                    } else {
                        self.lf_lines += 1;
                        (nl, nl + 1)
                    }
                }
                None => (bytes.len(), bytes.len()),
            };
            let kind = lex_line(&self.text[start..end]);
            match kind {
                LineKind::Begin { .. } => self.begins += 1,
                LineKind::End => self.ends += 1,
                _ => {}
            }
            self.lines.push(Line {
                start: start as u32,
                end: end as u32,
                kind,
            });
            start = next;
        }
    }

    /// Indexed lines (0-based).
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Nombre de lignes (fichier vide → 0).
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Texte d'une ligne (index 0-based), sans fin de ligne.
    pub fn line_text(&self, idx: usize) -> &str {
        let l = self.lines[idx];
        &self.text[l.start as usize..l.end as usize]
    }

    /// Texte d'un span d'une ligne (index 0-based).
    pub fn span_text(&self, idx: usize, span: Span) -> &str {
        span.get(self.line_text(idx))
    }

    /// The file started with a UTF-8 BOM.
    pub fn has_bom(&self) -> bool {
        self.bom
    }

    /// The file mixes CRLF and LF (information: the rendering preserves it).
    pub fn mixed_eol(&self) -> bool {
        self.crlf_lines > 0 && self.lf_lines > 0
    }

    /// Nombre de `struct.begin` / `struct.end`.
    pub fn struct_counts(&self) -> (usize, usize) {
        (self.begins, self.ends)
    }

    /// The `struct.begin` and `struct.end` are balanced.
    pub fn is_balanced(&self) -> bool {
        self.begins == self.ends
    }

    /// Renders the document with the values replaced. Without modification,
    /// the result is byte-for-byte identical to the original file.
    pub fn render(&self, edits: &[ValueEdit<'_>]) -> Result<String, NoValueOnLine> {
        let mut sorted: Vec<&ValueEdit<'_>> = edits.iter().collect();
        sorted.sort_by_key(|e| e.line);
        let extra: usize = sorted.iter().map(|e| e.value.len()).sum();
        let mut out = String::with_capacity(self.text.len() + extra + BOM.len());
        if self.bom {
            out.push_str(BOM);
        }
        let mut pos = 0usize;
        for e in sorted {
            let line = e
                .line
                .checked_sub(1)
                .and_then(|i| self.lines.get(i))
                .ok_or(NoValueOnLine(e.line))?;
            let LineKind::KeyValue { value, .. } = line.kind else {
                return Err(NoValueOnLine(e.line));
            };
            let vs = line.start as usize + value.start as usize;
            let ve = line.start as usize + value.end as usize;
            if vs < pos {
                return Err(NoValueOnLine(e.line)); // same line edited twice
            }
            out.push_str(&self.text[pos..vs]);
            out.push_str(e.value);
            pos = ve;
        }
        out.push_str(&self.text[pos..]);
        Ok(out)
    }
}

/// Position du prochain `\n`.
fn memchr_nl(b: &[u8]) -> Option<usize> {
    b.iter().position(|&c| c == b'\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_is_byte_exact_for_all_formats() {
        for src in [
            "A : struct.begin\r\n   W = 1\r\nstruct.end\r\n",
            "A : struct.begin\n   W = 1\nstruct.end",
            "\u{feff}A : struct.begin\n   W = 1\r\nstruct.end\n\n",
            "",
            "\n\n",
        ] {
            let d = Document::from_text(src);
            assert_eq!(d.render(&[]).unwrap(), src, "{src:?}");
        }
    }

    #[test]
    fn replaces_only_the_value() {
        let src = "\u{feff}A : struct.begin\r\n\tWeight   =  5 // old\r\n\tMax = 2\nstruct.end";
        let d = Document::from_text(src);
        assert!(d.has_bom() && d.mixed_eol() && d.is_balanced());
        let out = d
            .render(&[
                ValueEdit {
                    line: 3,
                    value: "9",
                },
                ValueEdit {
                    line: 2,
                    value: "12",
                },
            ])
            .unwrap();
        assert_eq!(
            out,
            "\u{feff}A : struct.begin\r\n\tWeight   =  12 // old\r\n\tMax = 9\nstruct.end"
        );
        assert_eq!(
            d.render(&[ValueEdit {
                line: 1,
                value: "x"
            }]),
            Err(NoValueOnLine(1))
        );
        assert_eq!(
            d.render(&[ValueEdit {
                line: 99,
                value: "x"
            }]),
            Err(NoValueOnLine(99))
        );
    }

    #[test]
    fn counts_structs_like_the_parser() {
        let d = Document::from_text(
            "// struct.begin\nA : struct.begin\nN = x struct.begin\n[0] : struct.begin {b}\nstruct.end\n",
        );
        assert_eq!(d.struct_counts(), (2, 1));
        assert!(!d.is_balanced());
    }
}
