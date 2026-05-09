// oxyl-parser
// Converts a token stream from oxyl-lexer into an AST.
// Currently will only handle plain text runs 
// and paragrah breaks. Everything else will be left as a Char token for now 
// and collected into text. 
// Currently working on support for mandatory brace argument parsing 
// After a \command the parser greedily consumes any immediately following 
// { ... } groups. 
// This should cover most common patterns like \frac{a}{b} etc.


use oxyl_diagnostics::Diagnostic;
use oxyl_lexer::{Span, Token, TokenKind};

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
}

impl Node {
    pub fn span(&self) -> Span {
        match self {
            Node::Text(_,s) => *s,
            Node::ParagraphBreak(s) => *s,
            Node::Command { span, .. } => *span,
            Node::Group(_, s) => *s,
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
//

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
        let body = self.parse_nodes(None);
        ParseResult {
            document: Document { body },
            errors: self.errors,
        }
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

    /// Parse nodes until the token stream ends or `stop` matches. 
    ///
    /// `stop` is used by the group parser to halt at `}`.
    fn parse_nodes(&mut self, stop: Option<&TokenKind>) -> Vec<Node> {
        let mut nodes: Vec<Node> = Vec::new();
        
        loop {
            match self.peek() {
                None => break,
                Some(tok) if stop.map_or(false, |s| &tok.kind == s) => break,
                _ => {}
            }

            let tok = self.bump().unwrap();

            match tok.kind {
                TokenKind::Char(c) => self.push_char(&mut nodes, c, tok.span),
                TokenKind::Space => self.push_char(&mut nodes, ' ', tok.span),

                TokenKind::ParagraphBreak => nodes.push(Node::ParagraphBreak(tok.span)),

                TokenKind::ControlSeq(name) => {
                    let cmd_span = tok.span; 
                    let args = self.parse_mandatory_args();
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
                    let children = self.parse_nodes(Some(&TokenKind::EndGroup));
                    if self.peek_kind() == Some(&TokenKind::EndGroup) {
                        let close = self.bump().unwrap();
                        nodes.push(Node::Group(children, open_span.merge(close.span)));
                    } else {
                        // Unclosed group - record the error, keep what we parsed.
                        self.errors.push(Diagnostic::error(
                                "E020",
                                format!("unclosed '{{' at {open_span}"),
                        ));
                        nodes.push(Node::Group(children, open_span));
                    }
                }

                // Everything else is left unhandled for now so skip it.
                _ => {}
            }
        }

        nodes
    }
    /// Consume all immediately following `{ ... }` groups as mandatory args.
    ///
    /// TeX commands pick up their arguments greedily; we skip spaces between
    /// the command name and the first argument to match TeX's behaviour.
    fn parse_mandatory_args(&mut self) -> Vec<Arg> {
        let mut args = Vec::new();
        
        loop {
            // Skip spaces between the command and its arguments.
            if self.peek_kind() == Some(&TokenKind::Space) {
                self.bump();
            }

            if self.peek_kind() != Some(&TokenKind::BeginGroup) {
                break;
            }
            
            // Consume the opening brace.
            self.bump();
            let children = self.parse_nodes(Some(&TokenKind::EndGroup));
            if self.peek_kind() == Some(&TokenKind::EndGroup) {
                self.bump();
            } else {
                self.errors.push(Diagnostic::error(
                        "E021",
                        "unclosed mandatory argument",
                ));
            }
            args.push(Arg::Mandatory(children));

            // Only keep consuming args if the very next non-space token is 
            // also a `{`. Most LaTeX commands take a fixed number of args but 
            // I haven't gotten round to tracking that yet.
        }

        args
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
        panic!("no command found in: {src}")
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
        assert_eq!(name, "texbf");
        assert_eq!(args.len(), 1);
        assert!(matches!(&args[0], Arg::Mandatory(children)
                if matches!(&children[0], Node::Text(s, _) if s == "hello")));
    }
    
    #[test]
    fn empty_source() {
        let r = parse("");
        assert!(r.document.body.is_empty());
        assert!(r.errors.is_empty());
    }

    #[test]
    fn plain_text_node() {
        let r = parse("hello world");
        assert_eq!(r.document.body.len(), 1);
        assert!(matches!(&r.document.body[0], Node::Text(s, _) if s == "hello world"));
    }
    
    #[test]
    fn paragraph_break_node() {
        let r = parse("first\n\nsecond");
        let kinds: Vec<&str> = r.document.body.iter().map(|n| match n {
            Node::Text(..) => "text",
            Node::ParagraphBreak(..) => "par",
            Node::Command { .. } => "cmd",
            Node::Group(..) => "group",
        }).collect();
        assert!(kinds.contains(&"par"), "expected a paragraph break node");
    }

    #[test]
    fn bare_command_node() {
        let r = parse("\\LaTeX");
        assert!(matches!(&r.document.body[0], Node::Command { name, .. } if name == "LaTeX"));
    }

    #[test]
    fn brace_group_node() {
        let r = parse("{hello}");
        assert!(matches!(&r.document.body[0], Node::Group(..)));
        assert!(r.errors.is_empty());
    }

    #[test]
    fn unclosed_group_produces_error() {
        let r = parse("{oops");
        assert!(!r.errors.is_empty());
        assert_eq!(r.errors[0].code, "E020" );
    }
}
