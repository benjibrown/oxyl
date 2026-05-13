// oxyl-parser
// 
// Builds a Document of Nodes from the lexer's token stream.
//
// - Commands greedily pick up [...] and {...} until the next token 
// is neither of those.
// - A pair of $ tokens wraps a math node.
// - A \[...\] pair wraps a display math node. Inline and display math children 
// are parsed with the same machinery as ordinairy text;
// will do atoms and scripts later - TODO
//  - Every error carries a DiagSpan poiting at the token that triggered it 
//  (the unmatched bracket or dollar sign) so the cli can render source 
//  context without having to extract it from the message string :D


use oxyl_diagnostics::{DiagSpan, Diagnostic};
use oxyl_lexer::{Span, Token, TokenKind};

fn diag_span(s: Span) -> DiagSpan {
    DiagSpan::new(s.start, s.end)
}

/// Stop predicate for `parse_nodes` when scanning the body of `\[ ... \]`.
fn is_display_math_close(k: &TokenKind) -> bool {
    matches!(k, TokenKind::ControlSeq(s) if s == "]")
}

// --- 
// AST Types 
//

/// The root of a parsed LaTeX document.
///
/// For now we do not distinguish preamble from body - everything lands in 
/// `body`. Will add that split when handling for `\begin{document}` is done.
#[derive(Debug, Clone)]
pub struct Document {
    pub body: Vec<Node>,
}

/// A single node in the LaTeX AST.
#[derive(Debug, Clone)]
pub enum Node {
    /// A run of plain text characters
    Text(String, Span),

    /// A blank line in the source - signals a paragraph break.
    ParagraphBreak(Span),

    /// A LaTeX command and its arguments, e.g. `\textbf{hello}`.
    Command {
        /// Name without the leading backslash, e.g. `"textbf"`.
        name: String ,
        args: Vec<Arg>,
        span: Span,
    },

    /// A braced group `{...}`.
    Group(Vec<Node>, Span),
    
    /// Inline match: `$ ... $`. The span covers both `$` delimiters.
    Math(Vec<Node>, Span),

    /// Display math: `\[ ... \]`. The span covers both delimiters.
    DisplayMath(Vec<Node>, Span),
}

impl Node {
    pub fn span(&self) -> Span {
        match self {
            Node::Text(_, s) => *s,
            Node::ParagraphBreak(s) => *s,
            Node::Command { span, .. } => *span,
            Node::Group(_, s) => *s,
            Node::Math(_, s) => *s,
            Node::DisplayMath(_, s) => *s,
        }
    }
}

/// A single argument to a command or environment 
#[derive(Debug, Clone)]
pub enum Arg {
    Mandatory(Vec<Node>),
    Optional(Vec<Node>),
}

// --- 
// Parser Result 
// --- 

/// Returned by [`Parser::parse`]. The document is always produced; errors 
/// are collected alongside it so the caller sees everything at once.
#[derive(Debug)]
pub struct ParseResult {
    pub document: Document,
    pub errors: Vec<Diagnostic>,
}

// --- 
// Parser 
// --- 

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, errors: Vec::new() }
    }
    
    /// Parse the token stream.
    pub fn parse(mut self) -> ParseResult {
        let body = self.parse_nodes(|_| false);
        ParseResult { document: Document { body }, errors: self.errors }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|t| &t.kind)
    }

    fn bump(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    /// Parse a run of nodes until the token stream is exhausted or 
    /// `stop` returns true for the next token's kind. The stopping token is 
    /// left unconsumed so it can be examined and bumped by the caller !
    ///
    /// `stop` is used by the group parser to halt at `}` - it is a function pointer 
    /// rather than an `impl Fn` so the recursive calls don't blow up the parser.
    fn parse_nodes(&mut self, stop: fn(&TokenKind) -> bool) -> Vec<Node> {
        let mut nodes: Vec<Node> = Vec::new();
        
        loop {
            match self.peek() {
                None => break,
                Some(tok) if stop(&tok.kind) => break,
                _ => {}
            }

            let tok = self.bump().unwrap();

            match tok.kind {
                TokenKind::Char(c) => self.push_char(&mut nodes, c, tok.span),
                TokenKind::Space => self.push_char(&mut nodes, ' ', tok.span),

                TokenKind::ParagraphBreak => {
                    nodes.push(Node::ParagraphBreak(tok.span));
                }

                // `\[` opens display math. 
                TokenKind::ControlSeq(ref name) if name == "[" => {
                    let open_span = tok.span;
                    let children = self.parse_nodes(is_display_math_close);
                    if matches!(self.peek_kind(), Some(TokenKind::ControlSeq(s)) if s == "]") {
                        let close = self.bump().unwrap();
                        nodes.push(Node::DisplayMath(children, open_span.merge(close.span)));
                    } else {
                        self.errors.push(
                            Diagnostic::error("E031", "unclosed '\\[' (display math)")
                            .with_span(diag_span(open_span)),
                        );
                        nodes.push(Node::DisplayMath(children, open_span));
                    }
                }

                // A bare `\]` outside display math is a stray closer.
                TokenKind::ControlSeq(ref name) if name == "]" => {
                    self.errors.push(
                        Diagnostic::error("E032", "stray '\\]' (no matching '\\[')")
                        .with_span(diag_span(tok.span)),
                    );
                }

                TokenKind::ControlSeq(name) => {
                    let cmd_span = tok.span; 
                    let args = self.parse_args();
                    // Extend the span to cover the last argument. 
                    let full_span = args.last()
                        .and_then(|a| match a {
                            Arg::Mandatory(children) => children.last().map(|n| n.span()),
                            Arg::Optional(children) => children.last().map(|n| n.span()), 
                        })
                        .map(|s| cmd_span.merge(s))
                        .unwrap_or(cmd_span);
                    nodes.push(Node::Command { name, args, span: full_span });
                }

                TokenKind::BeginGroup => {
                    let open_span = tok.span;
                    let children = self.parse_nodes(|k| matches!(k, TokenKind::EndGroup));
                    if self.peek_kind() == Some(&TokenKind::EndGroup) {
                        let close = self.bump().unwrap();
                        nodes.push(Node::Group(children, open_span.merge(close.span)));
                    } else {
                        // Unclosed group - record the error, keep what we parsed.
                        self.errors.push(
                            Diagnostic::error("E020", "unclosed '{'")
                                .with_span(diag_span(open_span)),
                        );
                        nodes.push(Node::Group(children, open_span));
                    }
                }
                
                TokenKind::MathShift => {
                    let open_span = tok.span;
                    let children = self.parse_nodes(|k| matches!(k, TokenKind::MathShift));
                    if self.peek_kind() == Some(&TokenKind::MathShift) {
                        let close = self.bump().unwrap();
                        nodes.push(Node::Math(children, open_span.merge(close.span)));
                    } else {
                        self.errors.push(
                            Diagnostic::error("E030", "unclosed '$' (math mode)")
                                .with_span(diag_span(open_span)),
                        );
                        nodes.push(Node::Math(children, open_span));
                    }
                }
                // Everything else is left unhandled for now so skip it.
                _ => {}
            }
        }

        nodes
    }
    /// Consume all immediately following `[...] and `{ ... }` groups as args.
    ///
    /// TeX commands pick up their arguments greedily; we skip spaces between
    /// the command name and each argument to match TeX's behaviour. The loop
    /// stops at the first token that is neither `[` nor `{`.
    fn parse_args(&mut self) -> Vec<Arg> {
        let mut args = Vec::new();
        
        loop {
            // Skip spaces between the command and its next argument.
            if self.peek_kind() == Some(&TokenKind::Space) {
                self.bump();
            }

            match self.peek_kind() {
                Some(&TokenKind::BeginGroup) => args.push(self.parse_mandatory_arg()),
                Some(&TokenKind::Char('[')) => args.push(self.parse_optional_arg()),
                _ => break,
            }
        }
        args
    }    

    fn parse_mandatory_arg(&mut self) -> Arg {
        // Consume the opening brace, remembering its span for diagnostics.
        let open_span = self.bump().unwrap().span;
        let children = self.parse_nodes(|k| matches!(k, TokenKind::EndGroup));
        if self.peek_kind() == Some(&TokenKind::EndGroup) {
            self.bump();
        } else {
            self.errors.push(
                Diagnostic::error("E021","unclosed mandatory argument")
                    .with_span(diag_span(open_span)),
            );
        }
        Arg::Mandatory(children)
    }

    fn parse_optional_arg(&mut self) -> Arg {
        // Consume the opening `[`, remembering its span for diagnostics.
        let open_span = self.bump().unwrap().span;
        let children = self.parse_nodes(|k| matches!(k, TokenKind::Char(']')));
        if self.peek_kind() == Some(&TokenKind::Char(']')) {
            self.bump();
        } else {
            self.errors.push(
                Diagnostic::error("E022","unclosed optional argument")
                    .with_span(diag_span(open_span)),
            );
        }
        Arg::Optional(children)
    }
    
    /// Append a character to the last `Text` node, or start a new one.
    fn push_char(&self, nodes: &mut Vec<Node>, c: char, span: Span) {
        match nodes.last_mut() {
            Some(Node::Text(s, existing)) => {
                s.push(c);
                *existing = existing.merge(span);
            }
            _ => nodes.push(Node::Text(c.to_string(), span)),
        }
    }
}



// Tests

#[cfg(test)]
mod tests {
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
}
