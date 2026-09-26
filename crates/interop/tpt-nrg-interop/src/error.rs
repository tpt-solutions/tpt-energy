//! Error types for `tpt-nrg-interop`.

use thiserror::Error;

use tpt_nrg_core::CoreError;

/// Result alias for `tpt-nrg-interop`.
pub type InteropResult<T> = Result<T, InteropError>;

/// Machine-readable classification of an interop failure.
///
/// This mirrors the `ErrorKind` discriminant proposed in
/// [RFC 0006](https://github.com/tpt-solutions/tpt-energy/blob/master/rfcs/0006-unify-error-handling.md)
/// so that the CLI and the language bindings can branch on a stable string
/// rather than on message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteropErrorKind {
    /// The input parsed, but a required record or field was absent.
    MissingRecord,
    /// A field was present but could not be interpreted.
    BadValue,
    /// The input was not valid JSON, YAML, CSV, or XML.
    Parse,
    /// The input parsed but described a structurally invalid system.
    InvalidSystem,
    /// The requested format or dialect is not implemented.
    Unsupported,
    /// A file could not be read or written.
    Io,
}

impl InteropErrorKind {
    /// Stable lowercase slug, used by the CLI and by language bindings.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingRecord => "missing_record",
            Self::BadValue => "bad_value",
            Self::Parse => "parse",
            Self::InvalidSystem => "invalid_system",
            Self::Unsupported => "unsupported",
            Self::Io => "io",
        }
    }
}

impl std::fmt::Display for InteropErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised while converting between TPT Energy and external formats.
#[derive(Debug, Error)]
pub enum InteropError {
    /// A required block, record, or field was missing.
    #[error("{format}: missing {what}")]
    Missing {
        /// Source format slug (`"matpower"`, `"psse"`, `"cim"`, `"csv"`, ...).
        format: &'static str,
        /// Human-readable description of what was missing.
        what: String,
    },

    /// A field could not be interpreted as the expected type.
    #[error("{format}: bad value for {field}: {detail}")]
    BadValue {
        /// Source format slug.
        format: &'static str,
        /// Field or record label.
        field: String,
        /// Why parsing failed.
        detail: String,
    },

    /// The input could not be parsed at all.
    #[error("{format}: parse error: {detail}")]
    Parse {
        /// Source format slug.
        format: &'static str,
        /// Parser diagnostic.
        detail: String,
    },

    /// The parsed system failed structural validation.
    #[error("invalid system: {0}")]
    InvalidSystem(#[from] CoreError),

    /// The requested format or dialect is not implemented.
    #[error("unsupported {what}")]
    Unsupported {
        /// What was requested.
        what: String,
    },

    /// An I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl InteropError {
    /// Build a [`InteropError::Missing`].
    pub fn missing(format: &'static str, what: impl Into<String>) -> Self {
        Self::Missing {
            format,
            what: what.into(),
        }
    }

    /// Build a [`InteropError::BadValue`].
    pub fn bad_value(
        format: &'static str,
        field: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self::BadValue {
            format,
            field: field.into(),
            detail: detail.into(),
        }
    }

    /// Build a [`InteropError::Parse`].
    pub fn parse(format: &'static str, detail: impl Into<String>) -> Self {
        Self::Parse {
            format,
            detail: detail.into(),
        }
    }

    /// Build a [`InteropError::Unsupported`].
    pub fn unsupported(what: impl Into<String>) -> Self {
        Self::Unsupported { what: what.into() }
    }

    /// Classification of this error for programmatic handling.
    #[must_use]
    pub fn kind(&self) -> InteropErrorKind {
        match self {
            Self::Missing { .. } => InteropErrorKind::MissingRecord,
            Self::BadValue { .. } => InteropErrorKind::BadValue,
            Self::Parse { .. } => InteropErrorKind::Parse,
            Self::InvalidSystem(_) => InteropErrorKind::InvalidSystem,
            Self::Unsupported { .. } => InteropErrorKind::Unsupported,
            Self::Io(_) => InteropErrorKind::Io,
        }
    }
}

/// Convert a parsed numeric field to a non-negative integer identifier.
///
/// Bus, branch, and equipment identifiers are integers in every supported
/// format, but arrive as `f64` because MATPOWER, PSS/E, and CIM are all
/// whitespace- or field-delimited numeric tables. Values are rounded and
/// clamped into `0..=2^53`, which is exact in `f64` and far above any real
/// identifier, so the casts below are lossless in practice.
///
/// Returns `0` for a missing, negative, or non-finite value, which every
/// caller treats as "absent".
#[must_use]
pub fn field_index(value: f64) -> usize {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    // Clamp before rounding so a huge but finite value cannot saturate.
    let clamped = value.min(9.007_199_254_740_992e15);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        clamped.round() as usize
    }
}

/// Format a number the way a fixed-column engineering file expects: bare
/// integers without a decimal point, otherwise the value as-is.
///
/// Values are clamped into the exactly-representable `i64` range so the cast
/// cannot saturate.
#[must_use]
pub fn format_number(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return String::new();
    }
    if value.fract() == 0.0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let integral = value.clamp(-9.223_372_036_854_776e18, 9.223_372_036_854_776e18) as i64;
        return integral.to_string();
    }
    format!("{value:.decimals$}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_have_stable_slugs() {
        assert_eq!(InteropErrorKind::MissingRecord.as_str(), "missing_record");
        assert_eq!(InteropErrorKind::BadValue.as_str(), "bad_value");
        assert_eq!(InteropErrorKind::Parse.as_str(), "parse");
        assert_eq!(InteropErrorKind::InvalidSystem.as_str(), "invalid_system");
        assert_eq!(InteropErrorKind::Unsupported.as_str(), "unsupported");
        assert_eq!(InteropErrorKind::Io.as_str(), "io");
    }

    #[test]
    fn error_kind_matches_variant() {
        assert_eq!(
            InteropError::missing("matpower", "mpc.bus").kind(),
            InteropErrorKind::MissingRecord
        );
        assert_eq!(
            InteropError::bad_value("psse", "bus 1 type", "not an integer").kind(),
            InteropErrorKind::BadValue
        );
        assert_eq!(
            InteropError::parse("cim", "unclosed tag").kind(),
            InteropErrorKind::Parse
        );
        assert_eq!(
            InteropError::unsupported("psse v29").kind(),
            InteropErrorKind::Unsupported
        );
    }

    #[test]
    fn display_includes_context() {
        let e = InteropError::missing("matpower", "mpc.branch");
        assert!(e.to_string().contains("matpower"));
        assert!(e.to_string().contains("mpc.branch"));
    }
}
