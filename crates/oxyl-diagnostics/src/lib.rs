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

/// A single compiler diagnostic.
///
/// Every diagnostic has a severity, a short code (e.g. "E001"), and a message.
/// Source locations will be added once the lexer and parser carry span info
/// through to error sites.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Short alphanumeric code, e.g. "E001".
    pub code: &'static str,
    pub message: String,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Error, code, message: message.into() }
    }

    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Warning, code, message: message.into() }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]: {}", self.severity, self.code, self.message)
    }
}

/// An error produced during lexing.
///
/// Stored inside [LexResult`] so the caller can handle all the erros after 
/// tokenisation rather than stopping at the first problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    /// A lone backslash at the very end of the file with nothing after it.
    UnexpectedEndAfterBackslash { pos: usize },
    /// A UTF-8 character outside the ASCII range was encountered. Full 
    /// Unicode support is planned; for now we record the byte position.
    NonAsciiChar { pos: usize, ch: char}
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
        Diagnostic::error("E010", e.to_string())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let d = Diagnostic::error("E001", "undefined control sequence");
        assert_eq!(d.to_string(), "error [E001]: undefined control sequence");
    }

    #[test]
    fn warning_severity() {
        let d = Diagnostic::warning("W001", "overfull hbox");
        assert_eq!(d.severity, Severity::Warning);
    }
}
