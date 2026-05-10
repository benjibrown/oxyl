// oxyl-diagnostics 
// 
// Shared error and warning types used across all oxyl crates.


/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning, 
    Note,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note => write!(f, "note"),
        }
    }
}

/// A byte range used to point at source text in a diagnostic
///
/// Every diagnostic has a severity, a short code (e.g. "E001"), and a message.
/// Below - mirrors `Span` in oxyl-lexer but lives here so that the diagnostics 
/// stay independent of the lexer crate. Will keep the two types in sync 
/// manually for now; will unify when the crate graph is refactored,
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagSpan {
    pub start: usize,
    pub end: usize,
}

impl DiagSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl std::fmt::Display for DiagSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A single compiler diagnostic with an optional source location.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Short alphanumeric code, e.g. "E001".
    pub code: &'static str,
    pub message: String,
    /// Byte range in the source file, if known.
    pub span: Option<DiagSpan>,
    /// A short extract of the source shown below the message, if provided.
    pub source_hint: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self { 
            severity: Severity::Error, 
            code, 
            message: message.into(),
            span: None,
            source_hint: None,
        }
    }

    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self { 
            severity: Severity::Warning,
            code,
            message: message.into(),
            span: None,
            source_hint: None,
        }
    }

    pub fn with_span(mut self, span: DiagSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_source_hint(mut self, hint: impl Into<String>) -> Self {
        self.source_hint = Some(hint.into());
        self 
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]: {}", self.severity, self.code, self.message)?;
        if let Some(span) = &self.span {
            write!(f, " (at {span})")?;
        }
        if let Some(hint) = &self.source_hint {
            write!(f, "\n  | {hint}")?;
        }
        Ok(())
    }
}

/// An error produced during lexing.
///
/// Stored inside [`LexResult`] so the caller can handle all the errors after 
/// tokenisation rather than stopping at the first problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    /// A lone backslash at the very end of the file with nothing after it.
    UnexpectedEndAfterBackslash { pos: usize },
    /// A UTF-8 character outside the ASCII range was encountered. Full 
    /// Unicode support is planned; for now we record the byte position.
    NonAsciiChar { pos: usize, ch: char },
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnexpectedEndAfterBackslash { pos } => {
                write!(f, "unexpected end of input after '\\' at byte {pos}")
            }
            LexError::NonAsciiChar { pos, ch } => {
                write!(f, "non-ASCII character '{ch}' at byte {pos} (Unicode support coming soon!)")
            }
        }
    }
}

impl From<LexError> for Diagnostic {
    fn from(e: LexError) -> Self {
        let span = match e {
            LexError::UnexpectedEndAfterBackslash { pos } => DiagSpan::new(pos, pos + 1),
            LexError::NonAsciiChar {pos, .. } => DiagSpan::new(pos, pos + 1),
        };
        Diagnostic::error("E010", e.to_string()).with_span(span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_no_span() {
        let d = Diagnostic::error("E001", "undefined control sequence");
        assert_eq!(d.to_string(), "error [E001]: undefined control sequence");
    }

    #[test]
    fn error_display_with_span() {
        let d = Diagnostic::error("E001", "bad input")
            .with_span(DiagSpan::new(4, 9));
        assert!(d.to_string().contains("at 4..9"));
    }

    #[test]
    fn error_display_with_hint() {
        let d = Diagnostic::error("E001", "bad input")
            .with_span(DiagSpan::new(0,3))
            .with_source_hint("abc");
        assert!(d.to_string().contains("| abc"));
    }

    #[test]
    fn warning_severity() {
        let d = Diagnostic::warning("W001", "overfull hbox");
        assert_eq!(d.severity, Severity::Warning);
    }

    #[test]
    fn lex_error_into_diagnostic() {
        let e = LexError::UnexpectedEndAfterBackslash { pos: 42 };
        let d: Diagnostic = e.into();
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.code, "E010");
    }

    #[test]
    fn lex_error_display_() {
        let e = LexError::NonAsciiChar { pos: 5, ch: 'è'};
        assert!(e.to_string().contains("non-ASCII"));
    }

    #[test]
    fn lex_error_into_diagnostic_carries_span() {
        let e = LexError::UnexpectedEndAfterBackslash { pos: 7 };
        let d: Diagnostic =  e.into();
        assert!(d.span.is_some());
    }
}
