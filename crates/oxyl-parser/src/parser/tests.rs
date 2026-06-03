// parser tests - seperate file so i dont clutter everything
// thats important

use super::*;
use oxyl_lexer::Lexer;

fn parse(src: &str) -> ParseResult {
    let tokens = Lexer::new(src).tokenise().tokens;
    Parser::new(tokens).parse()
}

fn first_command(src: &str) -> (String, Vec<Arg>) {
    let r = parse(src);
    for node in &r.document.body {
        if let Node::Command { name, args, .. } = node {
            return (name.clone(), args.clone());
        }
    }
    panic!("no command found in: {src}");
}

#[test]
fn command_no_args() {
    let (name, args) = first_command("\\LaTeX");
    assert_eq!(name, "LaTeX");
    assert!(args.is_empty());
}

#[test]
fn command_one_mandatory_arg() {
    let (name, args) = first_command("\\textbf{hello}");
    assert_eq!(name, "textbf");
    assert_eq!(args.len(), 1);
    assert!(matches!(&args[0], Arg::Mandatory(children)
        if matches!(&children[0], Node::Text(s, _) if s == "hello")));
}

#[test]
fn command_two_mandatory_args() {
    let (name, args) = first_command("\\frac{a}{b}");
    assert_eq!(name, "frac");
    assert_eq!(args.len(), 2);
}

#[test]
fn unclosed_arg_produces_error() {
    let r = parse("\\cmd{oops");
    assert!(!r.errors.is_empty());
}

#[test]
fn paragraph_break_still_works() {
    let r = parse("line one\n\nline two");
    let has_par = r.document.body.iter().any(|n| matches!(n, Node::ParagraphBreak(_)));
    assert!(has_par);
}

#[test]
fn nested_command_in_arg() {
    let r = parse("\\outer{\\inner{x}}");
    assert!(r.errors.is_empty());
    if let Node::Command { args, .. } = &r.document.body[0] {
        if let Arg::Mandatory(inner) = &args[0] {
            assert!(matches!(&inner[0], Node::Command { name, .. } if name == "inner"));
        } else { panic!("expected mandatory arg"); }
    } else { panic!("expected command"); }
}

#[test]
fn command_with_optional_arg() {
    let (name, args) = first_command("\\sqrt[3]{27}");
    assert_eq!(name, "sqrt");
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0], Arg::Optional(children)
        if matches!(&children[0], Node::Text(s, _) if s == "3")));
    assert!(matches!(&args[1], Arg::Mandatory(children)
        if matches!(&children[0], Node::Text(s, _) if s == "27")));
}

#[test]
fn command_with_only_optional_arg() {
    let (name, args) = first_command("\\foo[opt]");
    assert_eq!(name, "foo");
    assert_eq!(args.len(), 1);
    assert!(matches!(&args[0], Arg::Optional(_)));
}

#[test]
fn optional_then_two_mandatory() {
    // two diff types of option + ordering 
    let (_, args) = first_command("\\section[short]{long}{extra}");
    assert_eq!(args.len(), 3);
    assert!(matches!(&args[0], Arg::Optional(_)));
    assert!(matches!(&args[1], Arg::Mandatory(_)));
    assert!(matches!(&args[2], Arg::Mandatory(_)));
}

#[test]
fn unclosed_optional_arg_produces_error() {
    let r = parse("\\cmd[oops");
    assert!(!r.errors.is_empty());
}

#[test]
fn bracket_outside_command_is_text() {
    // A `'[` not directly after a control sequence is just ordinary text.
    let r = parse("hello [world]");
    assert!(r.errors.is_empty());
    assert!(matches!(&r.document.body[0], Node::Text(s, _) if s == "hello [world]"));
}

#[test]
fn inline_math_simple() {
    let r = parse("$x+1$");
    assert!(r.errors.is_empty());
    assert_eq!(r.document.body.len(), 1);
    assert!(matches!(&r.document.body[0], Node::Math(children, _)
        if matches!(&children[0], Node::Text(s, _) if s == "x+1")));
}

#[test]
fn inline_math_with_command() {
    let r = parse("$\\alpha + \\beta$");
    assert!(r.errors.is_empty());
    if let Node::Math(children, _) = &r.document.body[0] {
        let names: Vec<_> = children.iter().filter_map(|n| match n {
            Node::Command { name, .. } => Some(name.as_str()),
            _ => None, 
        }).collect();
        assert_eq!(names, vec!["alpha", "beta"]);
    } else {
        panic!("expected math node");
    }
}

#[test]
fn unclosed_math_produces_error() {
    let r = parse("text $oops");
    assert!(!r.errors.is_empty());
}

#[test]
fn parser_errors_carry_spans() {
    // Every parser error must point at the offending opener so the CLI 
    // can render the location from the diagnostic span instead of
    // picking it ouf the message text.
    let cases = [
        "\\cmd{oops", // E021
        "\\cmd[oops", // E022
        "{", // E020
        "$oops", // E030
    ];
    for src in cases {
        let r = parse(src);
        assert!(!r.errors.is_empty(), "expected error for {src:?}");
        for e in &r.errors {
            assert!(e.span.is_some(), "error for {src:?} has no span: {e:?}");
        }
    }
}

#[test]
fn math_after_text() {
    let r = parse("hello $x$");
    assert!(r.errors.is_empty());
    assert_eq!(r.document.body.len(), 2);
    assert!(matches!(&r.document.body[0], Node::Text(s, _) if s == "hello "));
    assert!(matches!(&r.document.body[1], Node::Math(_, _)));
}

#[test]
fn display_math_simple() {
    let r = parse("\\[x+1\\]");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.document.body.len(), 1);
    assert!(matches!(&r.document.body[0], Node::DisplayMath(children, _)
        if matches!(&children[0], Node::Text(s, _) if s == "x+1")));
}

#[test]
fn display_math_with_command() {
    let r = parse("\\[ \\sum_{i=0}^n i \\]");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(matches!(&r.document.body[0], Node::DisplayMath(_, _)));
}

#[test]
fn unclosed_display_math_produces_error() {
    let r = parse("\\[ a + b");
    assert!(r.errors.iter().any(|e| e.code == "E031"));
}

#[test]
fn stray_close_display_math_produces_error() {
    let r = parse("oops \\] more");
    assert!(r.errors.iter().any(|e| e.code == "E032"));
}

#[test]
fn comment_preserved() {
    let r = parse("% hello\nworld");
    assert!(r.errors.is_empty());
    assert!(matches!(&r.document.body[0], Node::Comment(s, _) if s == " hello"));
    assert!(matches!(&r.document.body[1], Node::Text(s, _) if s == "world"));
}

#[test]
fn comment_inside_command_arg() {
    let r = parse("\\textbf{foo % drop?\nbar}");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    if let Node::Command { args, .. } = &r.document.body[0] {
        if let Arg::Mandatory(children) = &args[0] {
            assert!(children.iter().any(|n| matches!(n, Node::Comment(_, _))));
        } else { panic!("expected mandatory arg"); }
    } else { panic!("expected command"); }
}

#[test]
fn environment_simple() {
    let r = parse("\\begin{quote}hello\\end{quote}");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    if let Node::Environment { name, args, body, .. } = &r.document.body[0] {
        assert_eq!(name, "quote");
        assert!(args.is_empty());
        assert!(matches!(&body[0], Node::Text(s, _) if s == "hello"));
    } else {
        panic!("expected environment, got {:?}", r.document.body[0]);
    }
}

#[test]
fn environment_with_starred_name() {
    let r = parse("\\begin{equation*}x = 1\\end{equation*}");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(matches!(&r.document.body[0], Node::Environment { name, .. } if name == "equation*"));
}

#[test]
fn environment_with_extra_args() {
    // \begin{tabular}{cc} keeps {cc} as env arg, not as the name.
    let r = parse("\\begin{tabular}{cc}A & B\\end{tabular}");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    if let Node::Environment { name, args, .. } = &r.document.body[0] {
        assert_eq!(name, "tabular");
        assert_eq!(args.len(), 1);
        assert!(matches!(&args[0], Arg::Mandatory(_)));
    } else { panic!("expected environment"); }
}

#[test]
fn nested_environments() {
    let r = parse("\\begin{outer}\\begin{inner}x\\end{inner}\\end{outer}");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    if let Node::Environment { name, body, .. } = &r.document.body[0] {
        assert_eq!(name, "outer");
        assert!(matches!(&body[0], Node::Environment {name, .. } if name == "inner"));
    } else { panic!("expected outer environment"); }
}

#[test]
fn mismatched_end_produces_error() {
    let r = parse("\\begin{a}x\\end{b}");
    assert!(r.errors.iter().any(|e| e.code == "E042"));
}

#[test]
fn unclosed_begin_produces_error() {
    let r = parse("\\begin{a}body");
    assert!(r.errors.iter().any(|e| e.code == "E041"));
}

#[test]
fn stray_end_produces_error() {
    let r = parse("\\end{a}");
    assert!(r.errors.iter().any(|e| e.code == "E043"));
}

#[test]
fn begin_without_name_produces_error() {
    let r = parse("\\begin foo");
    assert!(r.errors.iter().any(|e| e.code == "E040"));
}

#[test]
fn align_tab_becomes_node() {
    let r = parse("a & b");
    assert!(r.errors.is_empty());
    let kinds: Vec<_> = r.document.body.iter().map(|n| match n {
        Node::Text(s, _) => format!("T({s})"),
        Node::AlignTab(_) => "&".to_owned(),
        other => format!("{other:?}"),
    }).collect();
    assert_eq!(kinds, vec!["T(a )", "&", "T( b)"]);
}

#[test]
fn tilde_becomes_node() {
    let r = parse("oxyl.~isthebest");
    assert!(r.errors.is_empty());
    // Order should be oxyl. (text), tilde, isthebest (text)
    assert!(matches!(&r.document.body[1], Node::Tilde(_)));
}

#[test]
fn align_tab_inside_tabular_body() {
    let r = parse("\\begin{tabular}{cc}A & B\\end{tabular}");
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    if let Node::Environment { body, .. } = &r.document.body[0] {
        assert!(body.iter().any(|n| matches!(n, Node::AlignTab(_))));
    } else { panic!("expected environment"); }
}
