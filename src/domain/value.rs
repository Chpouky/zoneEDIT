//! Numeric values of the cfg format: reading, checking, conversion to %.

use std::fmt;

use crate::config::{FieldSpec, ValueType};

/// Parses a cfg value into an `f64` (tolerates the `f` suffix: `0.f`,
/// `1.0`, `20`, and the percent suffix used by a few weapon fields:
/// `ChanceBleedingPerShot = 10%` → `10.0`).
pub fn parse_value(text: &str) -> Option<f64> {
    let t = text.trim();
    let t = t.strip_suffix('%').unwrap_or(t).trim_end_matches('f');
    t.parse::<f64>().ok()
}

/// Reason for the refusal of a value.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueError {
    /// Non-numeric text.
    NotANumber(String),
    /// Non-finite number (NaN, infinity).
    NotFinite(String),
    /// Value below the minimum bound.
    BelowMin {
        /// Entered text.
        text: String,
        /// Minimum bound of the schema.
        min: f64,
    },
    /// Value above the maximum bound.
    AboveMax {
        /// Entered text.
        text: String,
        /// Maximum bound of the schema.
        max: f64,
    },
    /// Value outside the interval [min, max].
    OutOfRange {
        /// Entered text.
        text: String,
        /// Minimum bound of the schema.
        min: f64,
        /// Maximum bound of the schema.
        max: f64,
    },
    /// Integer expected, decimal provided.
    NotInteger(String),
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotANumber(t) => write!(f, "‘{t}’ is not a valid number"),
            Self::NotFinite(t) => write!(f, "‘{t}’ is not a finite number"),
            Self::BelowMin { text, min } => write!(f, "‘{text}’ must be ≥ {min}"),
            Self::AboveMax { text, max } => write!(f, "‘{text}’ must be ≤ {max}"),
            Self::OutOfRange { text, min, max } => {
                write!(f, "‘{text}’ must be between {min} and {max}")
            }
            Self::NotInteger(t) => write!(f, "‘{t}’ must be an integer"),
        }
    }
}

/// Checks a cfg value (stored text) against the schema of the field.
pub fn check(spec: &FieldSpec, text: &str) -> Result<f64, ValueError> {
    let v = parse_value(text).ok_or_else(|| ValueError::NotANumber(text.to_string()))?;
    if !v.is_finite() {
        return Err(ValueError::NotFinite(text.to_string()));
    }
    let below = spec.min.is_some_and(|m| v < m);
    let above = spec.max.is_some_and(|m| v > m);
    if below || above {
        return Err(match (spec.min, spec.max) {
            (Some(min), Some(max)) => ValueError::OutOfRange {
                text: text.to_string(),
                min,
                max,
            },
            (Some(min), None) => ValueError::BelowMin {
                text: text.to_string(),
                min,
            },
            (None, Some(max)) => ValueError::AboveMax {
                text: text.to_string(),
                max,
            },
            (None, None) => unreachable!("bounds missing"),
        });
    }
    if spec.value_type == ValueType::Int && v.fract() != 0.0 {
        return Err(ValueError::NotInteger(text.to_string()));
    }
    Ok(v)
}

/// Rounding to 6 decimal places (avoids f64 artifacts: `0.29 * 100`).
fn round6(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

/// Cfg value in [0, 1] → display as percentage (`0.1f` → `10%`).
/// Non-numeric text: returned as-is.
pub fn percent_display(text: &str) -> String {
    let Some(p) = parse_value(text) else {
        return text.to_string();
    };
    let pct = round6(p * 100.0);
    if (pct - pct.round()).abs() < 5e-7 {
        format!("{}%", pct.round() as i64)
    } else {
        format!("{pct}%")
    }
}

/// Reads a percentage input (`25` or `25%`).
pub fn parse_percent(text: &str) -> Option<f64> {
    text.trim().trim_end_matches('%').trim().parse::<f64>().ok()
}

/// Percentage → cfg text in vanilla format (`25` → `0.25f`, `100` → `1.f`).
pub fn percent_to_cfg(pct: f64) -> String {
    let v = round6(pct / 100.0);
    let mut s = format!("{v:.6}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
    }
    if !s.contains('.') {
        s.push('.');
    }
    s.push('f');
    s
}

/// User input → cfg text, according to the display unit of the field.
/// Error if the input is not a valid number for the schema.
pub fn input_to_cfg(spec: &FieldSpec, input: &str) -> Result<String, ValueError> {
    let input = input.trim();
    let text = if spec.is_percent() {
        let pct = parse_percent(input).ok_or_else(|| ValueError::NotANumber(input.into()))?;
        let (min, max) = (
            spec.min.unwrap_or(0.0) * 100.0,
            spec.max.unwrap_or(1.0) * 100.0,
        );
        if !(min..=max).contains(&pct) {
            return Err(ValueError::OutOfRange {
                text: format!("{input}%"),
                min,
                max,
            });
        }
        percent_to_cfg(pct)
    } else {
        input.to_string()
    };
    check(spec, &text)?;
    Ok(text)
}

/// Cfg value → text displayed in the UI (percentage if needed).
pub fn display(spec: &FieldSpec, text: &str) -> String {
    if spec.is_percent() {
        percent_display(text)
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::field::FieldKind;

    /// Raw float spec (anomaly `Radius`).
    fn fspec() -> FieldSpec {
        FieldSpec {
            kind: FieldKind::Radius,
            short: "Radius".into(),
            long: "Radius".into(),
            value_type: ValueType::Float,
            min: None,
            max: None,
            display: Default::default(),
            scope: Default::default(),
        }
    }

    /// Raw int spec (counts).
    fn ispec() -> FieldSpec {
        FieldSpec {
            kind: FieldKind::MinCount,
            short: "Min".into(),
            long: "Min".into(),
            value_type: ValueType::Int,
            min: Some(0.0),
            max: Some(1000.0),
            display: Default::default(),
            scope: Default::default(),
        }
    }

    /// Spawn-chance spec, bounds [0, 1], raw display.
    fn pspec() -> FieldSpec {
        FieldSpec {
            kind: FieldKind::Chance,
            short: "Chance".into(),
            long: "Chance".into(),
            value_type: ValueType::Float,
            min: Some(0.0),
            max: Some(1.0),
            display: Default::default(),
            scope: Default::default(),
        }
    }

    #[test]
    fn parse_value_tolerates_suffix() {
        assert_eq!(parse_value("160.f"), Some(160.0));
        assert_eq!(parse_value("0.25f"), Some(0.25));
        assert_eq!(parse_value("12"), Some(12.0));
        assert_eq!(parse_value("10%"), Some(10.0));
        assert_eq!(parse_value("2.5%"), Some(2.5));
        assert_eq!(parse_value("abc"), None);
    }

    #[test]
    fn input_to_cfg_keeps_text_as_is() {
        let s = fspec();
        // The stored text is exactly the entered text (no normalization).
        assert_eq!(input_to_cfg(&s, "200").unwrap(), "200");
        assert_eq!(input_to_cfg(&s, "200.f").unwrap(), "200.f");
        assert_eq!(input_to_cfg(&s, "0.25").unwrap(), "0.25");
        assert_eq!(input_to_cfg(&s, " 160.f ").unwrap(), "160.f");
        assert!(input_to_cfg(&s, "abc").is_err());
        assert!(input_to_cfg(&s, "NaN").is_err());
    }

    #[test]
    fn input_to_cfg_int_stays_bare() {
        let s = ispec();
        assert_eq!(input_to_cfg(&s, "7").unwrap(), "7");
        assert!(input_to_cfg(&s, "7.5").is_err(), "decimal in an int field");
        assert!(input_to_cfg(&s, "-1").is_err(), "below min");
        assert!(input_to_cfg(&s, "1001").is_err(), "above max");
    }

    #[test]
    fn input_to_cfg_chance_validates_bounds() {
        let s = pspec();
        assert_eq!(input_to_cfg(&s, "0.25f").unwrap(), "0.25f");
        assert_eq!(input_to_cfg(&s, "0").unwrap(), "0");
        assert!(input_to_cfg(&s, "150").is_err(), "above max");
        assert!(input_to_cfg(&s, "-0.1f").is_err(), "below min");
    }
}
