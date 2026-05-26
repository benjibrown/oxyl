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
use oxyl_parser::{Arg, Node};

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

// ast dump stuff 

pub fn dump_ast(nodes: &[Node], style: Style) {
    let header = format!("=== AST ({} top-level node(s)) ===", nodes.len());
    println!("{}", paint(style, BOLD, &header));
    for node in nodes {
        print_node(node, 1, style);
    }
}

fn print_node(node: &Node, depth: usize, style: Style) {
    let indent = "  ".repeat(depth);
    let span = paint(style, DIM, &format_span(node.span()));

    match node {
        Node::Text(text, _) => {
            let variant = paint(style, CYAN, "Text");
            let text = paint(style, GREEN, &format!("{text:?}"));
            println!("{indent}{variant} {text} {span}");
        }
        Node::ParagraphBreak(_) => {
            let variant = paint(style, CYAN, "ParagraphBreak");
            println!("{indent}{variant} {span}");
        }
        Node::Command{ name, args, .. } => {
            let variant = paint(style, CYAN, "Command");
            let name = paint(style, GREEN, &format!("\"\\{name}\""));
            println!("{indent}{variant} {name} {span}");
            for arg in args{
                print_arg(arg, depth + 1, style);
            }
        },
        _ => todo!()
    }
}

fn print_arg(arg: &Arg, depth: usize, style: Style) {
    let indent = "  ".repeat(depth);
    match arg {
        Arg::Mandatory(children) => {
            let label = paint(style, MAGENTA, "Mandatory");
            println!("{indent}{label}");
            for child in children {
                print_node(child, depth + 1, style);
            }
        },
    _ => todo!()
    }
}

fn format_span(span: oxyl_lexer::Span) -> String {
    format!("@ {}..{}", span.start, span.end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyl_lexer::Span;

    fn tok(kind: TokenKind) -> Token {
        Token::new(kind, Span::new(0, 1))
    }

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

    #[test]
    fn group_kinds_share_colour() {
        // matching pairs should look the same
        assert_eq!(kind_code(&TokenKind::BeginGroup), kind_code(&TokenKind::EndGroup));
    }

    #[test]
    fn structural_kinds_share_colour() {
        let blue = kind_code(&TokenKind::AlignTab);

        assert_eq!(blue, kind_code(&TokenKind::Parameter));
        assert_eq!(blue, kind_code(&TokenKind::Superscript));
        assert_eq!(blue, kind_code(&TokenKind::Subscript));
        assert_eq!(blue, kind_code(&TokenKind::Tilde));
    }

    #[test]
    fn dump_tokens_doesnt_panic_on_empty() {
        dump_tokens(&[], Style::Plain);
        dump_tokens(&[], Style::Ansi);
    }

    #[test]
    fn dump_tokens_handles_mixed_input() {
        // a mix of kinds shouldnt cause panic and 
        // should write something to stdout.
        // super peak test
        let toks = vec![
            tok(TokenKind::ControlSeq("foo".into())),
            tok(TokenKind::BeginGroup),
            tok(TokenKind::Char('a')),
            tok(TokenKind::EndGroup),
            tok(TokenKind::Comment(" hi".into())),
        ];
        dump_tokens(&toks, Style::Plain);
        dump_tokens(&toks, Style::Ansi);
    }
}
