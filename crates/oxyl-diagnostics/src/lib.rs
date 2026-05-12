// oxyl-diagnostics 
// 
// Shared Severity, Diagnostic and lex error types used across the other crates.
// Kept small so its at bottom of dep. graph.
//
//
// The source helper maps byte spans to 1 based line/col and lets 
// the Diagnostic::Render produce a caret-style listing the CLI shows.
// Display keeps workinf without a source for callers that dont have one too 
// (tests, lib users etc).

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

// --- 
// Source - byte offset to line/col map 
//


/// A view of source text with a precomputed line index.
///
/// Construct once per file; the line table is built upfront so that 
/// repeated `line_col` lookups (one per diagnostic) aren't super slow
/// I think O(log lines) - will fact check this later.
pub struct Source <'a> {
    text: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> Source <'a> {
    pub fn new(text: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { text, line_starts }
    }
    
    /// Convert a byte offset into a 1-based `(line, column)`.
    ///
    /// Columns are byte-counted; the lexer rejects non-ASCII, so this 
    /// matches what a user would expect from any ASCII editor.
    pub fn line_col(&self, byte: usize) -> (usize, usize) {
        let line_idx = match self.line_starts.binary_search(&byte) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx];
        let col = byte.min(self.text.len()) - line_start;
        (line_idx + 1, col + 1)
    }
    
    /// Return the text of a 1-based line, without the trailing newline.
    pub fn line_text(&self, line: usize) -> &str {
        let idx = line.saturating_sub(1).min(self.line_starts.len() - 1);
        let start = self.line_starts[idx];
        let end = self.line_starts.get(idx + 1)
            .map(|&n| n.saturating_sub(1))
            .unwrap_or(self.text.len());
        &self.text[start..end.min(self.text.len())]
    }
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

    pub fn render(&self, source: &Source) -> String {
        let span = match self.span {
            Some(s) => s,
            None => return self.to_string(),
        };

        let (line, col) = source.line_col(span.start);
        let line_text = source.line_text(line);
        let gutter_w = line.to_string().len();
        let pad = " ".repeat(col.saturating_sub(1));
        // Clamp the caret length so it never overflows the displayed line.
        let visible_room = line_text.len().saturating_sub(col.saturating_sub(1));
        let caret_len = (span.end - span.start).max(1).min(visible_room.max(1));
        let carets = "^".repeat(caret_len);
        let blank_gutter = " ".repeat(gutter_w);

        format!(
            "{sev} [{code}]: {msg}\n\
            {blank} --> line {line}:{col}\n\
            {blank} |\n\
            {line:>w$} | {line_text}\n\
            {blank} | {pad}{carets}",
            sev = self.severity,
            code = self.code,
            msg = self.message,
            blank = blank_gutter,
            line = line, 
            col = col,
            w = gutter_w,
            line_text = line_text,
            pad = pad,
            carets = carets,
        )
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
