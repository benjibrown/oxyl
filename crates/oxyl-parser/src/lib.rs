// oxyl-parser
// 
// Converts a token stream from oxyl-lexer into an AST.
//
// Currently will only handle plain text runs 
// and paragrah breaks. Everything else will be left as a Char token for now 
// and collected into text. 
// TODO - peel off commands, groups, environments, math (pefired this todo lol as ik i wont do it)

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
// Parser 
// --- 

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }
    
    /// Parse the token stream into a [`Document`].
    pub fn parse(mut self) -> Document {
        let body = self.parse_nodes();
        Document { body }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(tok)
    }

    /// Parse nodes until we run out of tokens (or hit a stop condition that
    /// inner callers will add later).
    fn parse_nodes(&mut self) -> Vec<Node> {
        let mut nodes: Vec<Node> = Vec::new();

        while let Some(tok) = self.peek() {
            match &tok.kind {
                // Two ore more newlines in a row come through as a Space that 
                // contains `\n`. The lexer collapses all whitespace, so we 
                // detect paragraph breaks by checking for the Space token that 
                // immediately follows another Space or is at the start. But for
                // now, rely on a simpler signal - the lexer already emits a s 
                // single Space for all runs, so we just collect text and 
                // treat Space as a space character inside text runs.
                // long ahh comment - putting here otherwise ill forget what i wrote
                // Will add paragraph breaks later on when we 
                // give the lexer a dedicated ParagraphBreak token.
                TokenKind::Space => {
                    let span = tok.span;
                    self.bump();
                    // Append a space to the previous Text node if there is 
                    // one, otherwise start a new one.
                    match nodes.last_mut() {
                        Some(Node::Text(s, existing_span)) => {
                            s.push(' ');
                            *existing_span = existing_span.merge(span);
                        }
                        _ => nodes.push(Node::Text(" ".into(), span)),
                    }
                }
                
                TokenKind::Char(c) => {
                    let c = *c;
                    let span = tok.span;
                    self.bump();
                    match nodes.last_mut() {
                        Some(Node::Text(s, existing_span)) => {
                            s.push(c);
                            *existing_span = existing_span.merge(span);
                        }
                        _ => nodes.push(Node::Text(c.to_string(), span)),
                    }
                }

                // Everything else is left unhandled for now so skip it.
                _ => {
                    self.bump();
                }
            }
        }

        nodes
    }
}



// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use oxyl_lexer::Lexer;

    fn parse(src: &str) -> Document {
        let tokens = Lexer::new(src).tokenise().tokens;
        Parser::new(tokens).parse()
    }
    
    #[test]
    fn empty_source_gives_empty_body() {
        let doc = parse("");
        assert!(doc.body.is_empty());
    }

    #[test]
    fn plain_text_becomes_single_text_node() {
        let doc = parse("hello");
        assert_eq!(doc.body.len(), 1);
        assert!(matches!(&doc.body[0], Node::Text(s, _) if s == "hello"));
    }
    
    #[test]
    fn spaces_merged_into_text() {
        let doc = parse("hi there");
        assert_eq!(doc.body.len(), 1);
        assert!(matches!(&doc.body[0], Node::Text(s, _) if s == "hi there"));
    }

    #[test]
    fn text_node_span_covers_full_run() {
        let doc = parse("abc");
        if let Node::Text(_, span) = &doc.body[0] {
            assert_eq!(span.start, 0);
            assert_eq!(span.end, 3);
        } else {
            panic!("expected Text node");
        }
    }
}
