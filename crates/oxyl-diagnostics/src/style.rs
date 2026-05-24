// ANSI styling for rendered diagnostics
//
// set style to plain to write nothing (zero ansi). set style to ansi 
// to wrap bits of output in sgr escape seqs.
//
// colours follow rustc conventions - bold red for errors, yellow for 
// warnings, cyan for notes and last but by no means least, bold blue for 
// gutter elements ( --> | etc)

use crate::Severity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Plain,
    Ansi,
}

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[1;33m";
const CYAN: &str = "\x1b[1;36m";
const BLUE: &str = "\x1b[1;34m";

impl Style {
    /// Wrap `s` in the colour used for the given severity
    pub fn severity(self, sev: Severity, s: &str) -> String {
        match self {
            Style::Plain => s.to_owned(),
            Style::Ansi => format!("{}{s}{RESET}", sev_color(sev)),
        }
    }

    /// Wrap `s` in bold (used for diagnostic code and also the message)!!
    pub fn bold(self, s: &str) -> String {
        match self {
            Style::Plain => s.to_owned(),
            Style::Ansi => format!("{BOLD}{s}{RESET}"),
        }
    }

    /// Wrap `s` in the gutter colour (used for `-->` and `|` and line nums)
    pub fn gutter(self, s: &str) -> String {
        match self {
            Style::Plain => s.to_owned(),
            Style::Ansi => format!("{BLUE}{s}{RESET}"),
        }
    }

    /// Wrap `s` in the caret colour (same as severity so just call that).
    pub fn caret(self, sev: Severity, s: &str) -> String { // no it is not "carrot" bro
        self.severity(sev, s) 
    }
}

fn sev_color(sev: Severity) -> &'static str {
    match sev {
        Severity::Error => RED,
        Severity::Warning => YELLOW,
        Severity::Note => CYAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_style_passes_through() {
        assert_eq!(Style::Plain.severity(Severity::Error, "error"), "error");
        assert_eq!(Style::Plain.bold("hi"), "hi");
        assert_eq!(Style::Plain.gutter("-->"), "-->");
    }

    #[test]
    fn ansi_style_wraps_with_escape_codes() {
        let out = Style::Ansi.severity(Severity::Error, "error");
        assert!(out.starts_with("\x1b["));
        assert!(out.ends_with("\x1b[0m"));
        assert!(out.contains("error"));
    }

    #[test]
    fn ansi_severity_uses_distinct_colors() {
        let e = Style::Ansi.severity(Severity::Error, "x");
        let w = Style::Ansi.severity(Severity::Warning, "x");
        let n = Style::Ansi.severity(Severity::Note, "x");
        assert_ne!(e, w);
        assert_ne!(w, n);
        assert_ne!(e, n);
    }
}
