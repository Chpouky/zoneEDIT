//! Classification of a cfg line (no regex, no allocation).
//!
//! Recognized forms:
//! - `Name : struct.begin {attributes}` (or `[0] : struct.begin`, or
//!   `struct.begin` alone) — the prefix must be empty or end with `:`,
//!   otherwise it is a value containing the word;
//! - `struct.end`;
//! - `Key = value // comment` (the comment is excluded from the value);
//! - `// comment`, empty line.

/// Portion of a line, in bytes relative to the start of the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Start index (byte, relative to the start of the line).
    pub start: u32,
    /// Index de fin (exclusif).
    pub end: u32,
}

impl Span {
    /// Designated text in `line`.
    pub fn get(self, line: &str) -> &str {
        &line[self.start as usize..self.end as usize]
    }

    /// The span designates no character.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Nature d'une ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// Ligne vide (ou ne contenant que des espaces).
    Blank,
    /// Commentaire (`// …`).
    Comment,
    /// Structure opening: name (`[?]` if empty on the caller side) and
    /// raw attributes (`{bskipref}`, `{refurl=…;refkey=…}`), empty if absent.
    Begin {
        /// Nom du bloc (`[0]`, `Foo`, …) ou `[?]` si absent.
        name: Span,
        /// Attributs bruts entre accolades, vides si absents.
        attrs: Span,
    },
    /// Fermeture de structure.
    End,
    /// `key = value`; the value can be empty.
    KeyValue {
        /// Key (before the first `=`).
        key: Span,
        /// Value (after the `=`, comment excluded); can be empty.
        value: Span,
    },
    /// Unrecognized line (kept as-is).
    Other,
}

const BEGIN: &str = "struct.begin";
const END: &str = "struct.end";

/// Position of a sub-string `sub` (from `line`) in `line`.
fn span_of(line: &str, sub: &str) -> Span {
    let start = sub.as_ptr() as usize - line.as_ptr() as usize;
    Span {
        start: start as u32,
        end: (start + sub.len()) as u32,
    }
}

/// Classe une ligne (sans fin de ligne).
pub fn lex_line(line: &str) -> LineKind {
    let t = line.trim();
    if t.is_empty() {
        return LineKind::Blank;
    }
    if t.starts_with("//") {
        return LineKind::Comment;
    }
    if let Some(pos) = t.find(BEGIN) {
        let prefix = t[..pos].trim();
        if prefix.is_empty() || prefix.ends_with(':') {
            let name = prefix.trim_end_matches(':').trim_end();
            let attrs = t[pos + BEGIN.len()..].trim();
            return LineKind::Begin {
                name: span_of(line, name),
                attrs: span_of(line, attrs),
            };
        }
    }
    if t.starts_with(END) {
        return LineKind::End;
    }
    if let Some((k, v)) = t.split_once('=') {
        let v = match v.find("//") {
            Some(c) => &v[..c],
            None => v,
        };
        return LineKind::KeyValue {
            key: span_of(line, k.trim()),
            value: span_of(line, v.trim()),
        };
    }
    LineKind::Other
}

/// Attributs d'un `struct.begin` : `{refurl=../X.cfg;refkey=[0]}` →
/// `[("refurl", "../X.cfg"), ("refkey", "[0]")]` ; `{bskipref}` →
/// `[("bskipref", "")]`.
pub fn parse_attrs(attrs: &str) -> Vec<(&str, &str)> {
    let inner = attrs
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();
    if inner.is_empty() {
        return Vec::new();
    }
    inner
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| match s.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => (s, ""),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(line: &str) -> (&str, &str) {
        match lex_line(line) {
            LineKind::KeyValue { key, value } => (key.get(line), value.get(line)),
            other => panic!("{line:?} → {other:?}"),
        }
    }

    #[test]
    fn lexes_all_forms() {
        assert_eq!(lex_line("   "), LineKind::Blank);
        assert_eq!(lex_line("  // x = 1 struct.begin"), LineKind::Comment);
        assert_eq!(lex_line("\t\tstruct.end"), LineKind::End);
        assert_eq!(lex_line("Foo : struct"), LineKind::Other);

        let l = "  [0] : struct.begin {bskipref}";
        let LineKind::Begin { name, attrs } = lex_line(l) else {
            panic!()
        };
        assert_eq!((name.get(l), attrs.get(l)), ("[0]", "{bskipref}"));

        let l = "struct.begin";
        let LineKind::Begin { name, attrs } = lex_line(l) else {
            panic!()
        };
        assert!(name.is_empty() && attrs.is_empty());

        assert_eq!(kv("   Weight = 5"), ("Weight", "5"));
        assert_eq!(kv("Note = a = b"), ("Note", "a = b"));
        assert_eq!(kv("Weight =   "), ("Weight", ""));
        assert_eq!(kv("Weight = 7 // was 5"), ("Weight", "7"));
        // `struct.begin` dans une valeur : pas une ouverture.
        assert_eq!(
            kv("Note = foo struct.begin bar"),
            ("Note", "foo struct.begin bar")
        );
    }

    #[test]
    fn parses_attributes() {
        assert_eq!(
            parse_attrs("{refurl=../ItemGeneratorPrototypes.cfg;refkey=[0]}"),
            vec![
                ("refurl", "../ItemGeneratorPrototypes.cfg"),
                ("refkey", "[0]")
            ]
        );
        assert_eq!(parse_attrs("{bskipref}"), vec![("bskipref", "")]);
        assert!(parse_attrs("").is_empty());
    }
}
