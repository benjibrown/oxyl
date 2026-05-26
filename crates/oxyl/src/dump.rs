// colourisation for dump tokens output 
//
// the diagnostic renderer in the diagnostics crate manages all of 
// its own color. token dump output is a whole diff beast lol, no severity
// or anything, just tokenkind and span.
// own palette - seperate to the rest of the program and still follows
// plain or ansi like diagnostics does 
//
// control seqs (cmds) are magenta 
// group markers {} are yellow 
// $ is cyan 
// &, #, ^, _, ~ are blue 
// % comments are green
// spaces or pars are dim (like a grey basically)
// characters are plain / default
// byte spans are dim 
// header line is bold

use oxyl_diagnostics::Style;
use oxyl_lexer::{Token, TokenKind};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[1;34m";
const MAGENTA: &str = "\x1b[1;35m";
const CYAN: &str = "\x1b[1;36m";

/// Wrap `s` in `code` ... RESET when style is ansi - if not then pass thru
fn paint(style: Style, code: &str, s: &str) -> String {
    match style {
        Style::Plain => s.to_owned(),
        Style::Ansi => format!("{code}{s}{RESET}"),
    }
}

/// SGR code for a given token kind
fn kind_code(kind: &TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::ControlSeq(_) => Some(MAGENTA),
        TokenKind::BeginGroup
        | TokenKind::EndGroup => Some(YELLOW),
        TokenKind::MathShift => Some(CYAN),
        TokenKind::AlignTab
        | TokenKind::Parameter 
        | TokenKind::Superscript
        | TokenKind::Subscript
        | TokenKind::Tilde => Some(BLUE),
        TokenKind::Comment(_) => Some(GREEN),
        TokenKind::Space
        | TokenKind::ParagraphBreak => Some(DIM),
        TokenKind::Char(_) => None,
    }
}

/// Print the token dump for `--dump-tokens`. Honours `style`: when
/// `Plain`, output contains zero ansi - for scenarios where
/// output is being piped etc.
pub fn dump_tokens(tokens: &[Token], style: Style) {
    let header = format!("=== tokens ({}) ===", tokens.len());
    println!("{}", paint(style, BOLD, &header));

    for tok in tokens {
        // spans aren't really necessary unless ur looking
        // for a specific span range so dim all 
        // of those. >=6 digit offset - so wide files are 
        // nice and readable!
        let span = format!("{:>6}..{:<6}", tok.span.start, tok.span.end);
        let span = paint(style, DIM, &span);

        let kind_text = tok.kind.to_string();
        let kind = match kind_code(&tok.kind) {
            Some(code) => paint(style, code, &kind_text),
            None => kind_text,
        };

        println!("  {span}  {kind}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyl_lexer::Span;

    #[test]
    fn plain_passes_through_kind_text() {
        assert_eq!(paint(Style::Plain, MAGENTA, "\\foo"), "\\foo");
    }

    #[test]
    fn ansi_wraps_in_escape_codes() {
        let out = paint(Style::Ansi, MAGENTA, "\\foo");
        assert!(out.starts_with("\x1b["));
        assert!(out.ends_with(RESET));
        assert!(out.contains("\\foo"));
    }

    #[test]
    fn every_kind_has_colour() {
        // char has no colour so kind_code returns None for it
        // other than that though, all other token kinds 
        // have colour codes assigned to them !
        assert!(kind_code(&TokenKind::ControlSeq("x".into())).is_some());

        assert!(kind_code(&TokenKind::BeginGroup).is_some());
        assert!(kind_code(&TokenKind::EndGroup).is_some());

        assert!(kind_code(&TokenKind::MathShift).is_some());
        
        assert!(kind_code(&TokenKind::AlignTab).is_some());
        assert!(kind_code(&TokenKind::Parameter).is_some());
        assert!(kind_code(&TokenKind::Superscript).is_some());
        assert!(kind_code(&TokenKind::Subscript).is_some());
        assert!(kind_code(&TokenKind::Tilde).is_some());

        assert!(kind_code(&TokenKind::Comment("x".into())).is_some());
        
        assert!(kind_code(&TokenKind::Space).is_some());
        assert!(kind_code(&TokenKind::ParagraphBreak).is_some());
        // char has no colour code so check is none
        assert!(kind_code(&TokenKind::Char('a')).is_none());
    }
}
