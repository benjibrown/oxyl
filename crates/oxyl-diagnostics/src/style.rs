// TODO - WIRE UP ANSI Into MAIN CLI PROPERLY and do tests

// ANSI styling for rendered diagnostics
//
// essentially just uses ansi and allat lol 

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
const BLUE: &str = "\x1b[1:34m";

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
