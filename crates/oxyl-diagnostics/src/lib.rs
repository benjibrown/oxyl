// oxyl-diagnostics 
// 
// Shared severity and diagnostic and lex error types used across 
// the other oxyl crates. sits at bottom of the dep graph :)
//
// source (byte offset to line/col mapping) in source, conversions
// done in LexError and its Into<Diagnostic> conversion is done too.
// core diag/diagpspan and severity stuff is chilling here too - which 
// produces the error output with that awesome caret (fyi a caret is ^)

mod source; 
mod lex_error;
mod style;

pub use source::Source;
pub use lex_error::LexError;
pub use style::Style;

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
    /// Free-form follow-up lines following common formatting / conventions 
    /// cos why the hell not (`=note = ...`). Preserves order.
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self { 
            severity: Severity::Error,
            code,
            message: message.into(),
            span: None,
            source_hint: None,
            notes: Vec::new(),
        }
    }

    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self { 
            severity: Severity::Warning,
            code,
            message: message.into(),
            span: None,
            source_hint: None,
            notes: Vec::new(),
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
   
    /// Attach a follow-up note. Notes appear after the caret line 
    /// when using styled render output - printed in the note colour 
    /// for extra awesomeness. Multiple notes pushed will stack in the order 
    /// they are inserted.
    pub fn with_note(mut self, msg: impl Into<String>) -> Self {
        self.notes.push(msg.into());
        self
    }

    /// Render the diagnostic with a source listing and a caret under 
    /// the span that actually caused it. If the diagnostic has no span,
    /// falls back to `Display` representation. Output has no escape codes 
    /// (not styled) - used if writing out to a file or pipe :D
    pub fn render(&self, source: &Source) -> String {
        self.render_styled(source, Style::Plain)
    }

    /// 
    pub fn render_styled(&self, source: &Source, style: Style) -> String {
        let span = match self.span {
            Some(s) => s,
            None => return self.to_string(),
        };

        let (line, col) = source.line_col(span.start);
        let line_text = source.line_text(line);
        let gutter_w = line.to_string().len();
        let pad = " ".repeat(col.saturating_sub(1));
        // clamp the caret length so it never overflows the displayed line.
        let visible_room = line_text.len().saturating_sub(col.saturating_sub(1));
        let caret_len = (span.end - span.start).max(1).min(visible_room.max(1));
        let carets_raw = "^".repeat(caret_len);
        let blank_gutter = " ".repeat(gutter_w);

        // --> file:line:col if the source carries a name, else line:col
        let location = match source.name() {
            Some(name) => format!("{name}:{line}:{col}"),
            None => format!("line {line}:{col}"),
        };

        // I knew this would be needed thanks to dennis lol but basically
        // the width of line in the gutter needs to be set before we paint it,
        // because when its wrapped in escape codes, the byte length wont match 
        // the visible width :(
        let line_num_padded = format!("{line:>w$}", line = line, w = gutter_w);
        let sev_word = style.severity(self.severity, &self.severity.to_string());
        let code_word = style.bold(&format!("[{}]", self.code));
        let msg_word = style.bold(&self.message);
        let arrow = style.gutter("-->");
        let bar = style.gutter("|");
        let line_num = style.gutter(&line_num_padded);
        let carets = style.caret(self.severity, &carets_raw);

        let mut out = format!(
            "{sev_word} {code_word}: {msg_word}\n\
             {blank} {arrow} {location}\n\
             {blank} {bar}\n\
             {line_num} {bar} {line_text}\n\
             {blank} {bar} {pad}{carets}",
            blank = blank_gutter,
        );
        
        // rustc style because i like to follow conventions and i am not 
        // re-inventing the wheel fr 
        // the = should line up with the | above since it also sits in the 
        // gutter - msg stays plain so it remains readable.
        for note in &self.notes {
            let eq = style.gutter("=");
            let note_word = style.severity(Severity::Note, "note:");
            out.push('\n');
            out.push_str(&format!("{blank_gutter} {eq} {note_word} {note}"))
        }
        out
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
        for note in &self.notes {
            write!(f, "\n  = note: {note}")?;
        }
        Ok(())
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

    #[test]
    fn render_plain_has_no_escape_codes() {
        let src = Source::new("foo {bar\n");
        let d = Diagnostic::error("E020", "unclosed '{'")
            .with_span(DiagSpan::new(4, 5));
        let plain = d.render(&src);
        assert!(!plain.contains('\x1b'), "plain render should not contain ESC: {plain:?}");
    }

    #[test]
    fn render_ansi_paints_severity_and_carets() {
        let src = Source::new("foo {bar\n");
        let d = Diagnostic::error("E020", "unclosed '{'")
            .with_span(DiagSpan::new(4, 5));
        let ansi = d.render_styled(&src, Style::Ansi);

        // the bare error word should in theory be wrapped in an 
        // SGR sequence and so should the caret.
        // loc. should still be in the plain text in the output !!
        assert!(ansi.contains('\x1b'), "ansi render should contain ESC");
        assert!(ansi.contains("error"));
        assert!(ansi.contains("line 1:5"));
        assert!(ansi.contains('^'));
    }

    #[test]
    fn render_warning_uses_yellow_not_red() {
        let src = Source::new("foo\n");
        let err = Diagnostic::error("E001", "x")
            .with_span(DiagSpan::new(0, 1))
            .render_styled(&src, Style::Ansi);
        let warn = Diagnostic::warning("W001", "x")
            .with_span(DiagSpan::new(0, 1))
            .render_styled(&src, Style::Ansi);
        assert_ne!(err, warn, "error and warning should pick different colours");
    }

    #[test]
    fn render_includes_notes_after_caret() {
        let src = Source::new("foo {bar\n");
        let d = Diagnostic::error("E020", "unclosed '{'")
            .with_span(DiagSpan::new(4, 5))
            .with_note("braces must be balanced")
            .with_note("did you forget a '}'?");
        let out = d.render(&src);
        assert!(out.contains("= note: braces must be balanced"), "got: {out}");
        assert!(out.contains("= note: did you forget a '}'"));
        // notes always come after the caret line
        let caret_idx = out.find('^').unwrap();
        let note_idx = out.find("braces").unwrap();
        assert!(note_idx > caret_idx, "notes must follow the caret");
    }

    #[test]
    fn ansi_render_paints_note_word() {
        let src = Source::new("x\n");
        let d = Diagnostic::error("E001", "boom")
            .with_span(DiagSpan::new(0, 1))
            .with_note("a follow-up");
        let ansi = d.render_styled(&src, Style::Ansi);
        let plain = d.render_styled(&src, Style::Plain);
        assert!(ansi.contains("a follow-up"));
        assert!(plain.contains("a follow-up"));
        // the worde "note" should be wrapped in sgr if ansi on
        // but not at all in plain mode
        assert!(ansi.contains("\x1b[1;36mnote:\x1b[0m"));
        assert!(!plain.contains('\x1b'));
    }

    #[test]
    fn display_renders_notes_when_no_source_available() {
        // the display path is the fallback for callers w/out a source
        // so notes should still render anyway
        let d = Diagnostic::error("E001", "x").with_note("hello");
        assert!(d.to_string().contains("= note: hello"));
    }
}
