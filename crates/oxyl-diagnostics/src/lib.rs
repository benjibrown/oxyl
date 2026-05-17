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
/// An optional `name` (typically the file path) can be included 
/// and rendered in diagnostics as `--> name:line:col`.
pub struct Source<'a> {
    text: &'a str,
    name: Option<String>,
    line_starts: Vec<usize>,
}

impl<'a> Source<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { text, name: None, line_starts }
    }

    /// Build a `Source` whose rendered diagnostics include `name` (usually
    /// a file path) in the location header.
    pub fn with_name(text: &'a str, name: impl Into<String>) -> Self {
        let mut s = Self::new(text);
        s.name = Some(name.into());
        s
    }

    /// The display name attached to this source, if any.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
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

        // --> file:line:col if the source carries a name, else line:col.
        let location = match source.name() {
            Some(name) => format!("{name}:{line}:{col}"),
            None => format!("line {line}:{col}"),
        };

        format!(
            "{sev} [{code}]: {msg}\n\
             {blank} --> {location}\n\
             {blank} |\n\
             {line:>w$} | {line_text}\n\
             {blank} | {pad}{carets}",
            sev = self.severity,
            code = self.code,
            msg = self.message,
            blank = blank_gutter,
            location = location,
            line = line, 
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
    fn lex_error_into_diagnostic_carries_span() {
        let e = LexError::UnexpectedEndAfterBackslash { pos: 7 };
        let d: Diagnostic =  e.into();
        assert!(d.span.is_some());
    }

    #[test]
    fn source_line_col_first_line() {
        let s = Source::new("hello\nworld\n");
        assert_eq!(s.line_col(0), (1, 1));
        assert_eq!(s.line_col(4), (1,5));
    }

    #[test]
    fn source_line_col_subsequent_lines() {
        let s = Source::new("hello\nworld\n!");
        assert_eq!(s.line_col(6), (2,1)); // w
        assert_eq!(s.line_col(10), (2, 5)); // d 
        assert_eq!(s.line_col(12), (3,1)); // !
    }

    #[test]
    fn source_line_text() {
        let s = Source::new("hello\nworld\n!");
        assert_eq!(s.line_text(1), "hello");
        assert_eq!(s.line_text(2), "world");
        assert_eq!(s.line_text(3), "!");
    }

    #[test]
    fn render_include_caret_and_line_number() {
        let src = Source::new("foo {bar\n");
        let d = Diagnostic::error("E020", "unclosed '{'")
            .with_span(DiagSpan::new(4, 5));
        let out = d.render(&src);
        assert!(out.contains("line 1:5"));
        assert!(out.contains("foo {bar"));
        assert!(out.contains("^"));
    }

    #[test]
    fn render_falls_back_when_no_span() {
        let src = Source::new("anything");
        let d = Diagnostic::error("E001", "no location");
        // Without a span, render should match plain Display.
        assert_eq!(d.render(&src), d.to_string());
    }

    #[test]
    fn render_uses_source_name() {
        let src = Source::with_name("foo {bar\n", "main.tex");
        let d = Diagnostic::error("E020", "unclosed '{'")
            .with_span(DiagSpan::new(4, 5));
        let out = d.render(&src);
        assert!(out.contains("main.tex:1:5"), "got: {out}");
    }

    #[test]
    fn render_drops_name_prefix_when_unnamed() {
        let src = Source::new("foo {bar\n");
        let d = Diagnostic::error("E020", "unclosed '{'")
            .with_span(DiagSpan::new(4, 5));
        let out = d.render(&src);
        assert!(out.contains("line 1:5"));
        assert!(!out.contains("foo {bar:"), "name should not leak");
    }
}
