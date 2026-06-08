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
//
//
// AST palette is pretty similar tbh
//
// variant names - cyan 
// arg discrims - magenta 
// strings (embedded) - green 
// byte spans - dim / grey 
// header - bold

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
fn kind_code(kind: &TokenKind<'_>) -> Option<&'static str> {
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
pub fn dump_tokens(tokens: &[Token<'src>], style: Style) {
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
//
// prints the ast dump for the dump ast flag
// each node renders on its own line 
// container nodes recurse too which is so awesome
pub fn dump_ast(nodes: &[Node<'src>], style: Style) {
    let header = format!("=== AST ({} top-level node(s)) ===", nodes.len());
    println!("{}", paint(style, BOLD, &header));
    for node in nodes {
        print_node(node, 1, style);
    }
}

fn print_node(node: &Node<'_>, depth: usize, style: Style) {
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
        }
        Node::Group(children, _) => {
            let variant = paint(style, CYAN, "Group");
            println!("{indent}{variant} {span}");
            for child in children {
                print_node(child, depth + 1, style);
            }
        }
        Node::Math(children, _) => {
            let variant = paint(style, CYAN, "Math");
            println!("{indent}{variant} {span}");
            for child in children {
                print_node(child, depth + 1, style);
            }
        }
        Node::DisplayMath(children, _) => {
            let variant = paint(style, CYAN, "DisplayMath");
            println!("{indent}{variant} {span}");
            for child in children {
                print_node(child, depth + 1, style);
            }
        }
        Node::Comment(body, _) => {
            let variant = paint(style, CYAN, "Comment");
            let body = paint(style, GREEN, &format!("{body:?}"));
            println!("{indent}{variant} {body} {span}");
        }
        Node::AlignTab(_) => {
            let variant = paint(style, CYAN, "AlignTab");
            println!("{indent}{variant} {span}");
        }
        Node::Tilde(_) => {
            let variant = paint(style, CYAN, "Tilde");
            println!("{indent}{variant} {span}");
        }
        Node::Environment { name, args, body, .. } => {
            let variant = paint(style, CYAN, "Environment");
            let name = paint(style, GREEN, &format!("{name:?}"));
            println!("{indent}{variant} {name} {span}");
            for arg in args {
                print_arg(arg, depth + 1, style);
            }
            for child in body {
                print_node(child, depth + 1, style);
            }
        }
    }
}

fn print_arg(arg: &Arg<'_>, depth: usize, style: Style) {
    let indent = "  ".repeat(depth);
    match arg {
        Arg::Mandatory(children) => {
            let label = paint(style, MAGENTA, "Mandatory");
            println!("{indent}{label}");
            for child in children {
                print_node(child, depth + 1, style);
            }
        }
        Arg::Optional(children) => {
            let label = paint(style, MAGENTA, "Optional");
            println!("{indent}{label}");
            for child in children {
                print_node(child, depth + 1, style);
            }
        }
    }
}

fn format_span(span: oxyl_lexer::Span) -> String {
    format!("@ {}..{}", span.start, span.end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyl_lexer::Span;

    fn tok(kind: TokenKind<'static>) -> Token<'static> {
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


    // ast dump tests

    use oxyl_parser::{Arg, Node};

    fn s() -> oxyl_lexer::Span { oxyl_lexer::Span::new(0, 0) }

    #[test]
    fn dump_ast_doesnt_panic_empty() {
        dump_ast(&[], Style::Plain);
        dump_ast(&[], Style::Ansi);
    }

    #[test]
    fn ast_handles_nested_tree() {
        // a command with a mandatory arg 
        // that itself contains a text node. dumper should
        // go thu recursively, not stop at the first depth
        let nodes = vec![
            Node::Command {
                name: "section".into(),
                args: vec![Arg::Mandatory(vec![Node::Text("Intro".to_string(), s())])],
                span: s(),
            },
            Node::ParagraphBreak(s()),
            Node::Environment {
                name: "tabular".into(),
                args: vec![Arg::Mandatory(vec![Node::Text("cc".to_string(), s())])],
                body: vec![
                    Node::Text("a".into(), s()),
                    Node::AlignTab(s()),
                    Node::Text("b".into(), s()),
                ],
                span: s(),
            },
        ];
        dump_ast(&nodes, Style::Plain);
        dump_ast(&nodes, Style::Ansi);
    }

    #[test]
    fn dump_ast_every_node() {
        // construct each variant atleast once - if a new var is added
        // and not here, print_node will fail to compile 
        // so will have to update this lol
        let nodes = vec![
            Node::Text("t".into(), s()),
            Node::ParagraphBreak(s()),
            Node::Command{ name: "x".into(), args: vec![], span: s() },
            Node::Group(vec![], s()),
            Node::Math(vec![], s()),
            Node::DisplayMath(vec![], s()),
            Node::Comment("c".into(), s()),
            Node::AlignTab(s()),
            Node::Tilde(s()),
            Node::Environment{ name: "e".into(), args: vec![], body: vec![], span: s() },
        ];
        dump_ast(&nodes, Style::Plain);
    }
}
