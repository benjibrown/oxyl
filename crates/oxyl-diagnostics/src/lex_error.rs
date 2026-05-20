/// An error produced during lexing.
///
/// Stored inside [`LexResult`] so the caller can handle all the errors after 
/// tokenisation rather than stopping at the first problem.

use crate::{DiagSpan, Diagnostic};

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
