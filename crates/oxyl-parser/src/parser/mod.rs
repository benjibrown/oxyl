// The parser turns a flat token stream into a tree of Nodes 
// TODO - formalise all below and put in docs 
// - commands greedily pick up [] (optional args) and also {} 
// which are mandatory args - until it finds a token that isnt any of these 
// - a pair of $ tokens wraps a math node 
// a \[...\] pair wraps a display math node. inline and display math 
// children are parsed like ordinary text
// TODO - atoms + scripts + operators !!
// \begin \end {name} produce an env node whose body is parsed 
// recusrively so nested envs work. the first mandatory arg 
// after \begin is treated as the name, everything else 
// stays in args 
// comments are preserved as comment nodes for any source-fidelity 
// tools to utilise :)
// active specials ie & and ~ are align tab and tilde nodes, they have no 
// children of their ownn - downstream passes that care about 
// tabular layour or whatever can read them off the 
// node sequence directly
// every error carries a diag span pointing at the token 
// that triggered the error so that the cli can render src 
// contex directly from span !!!!!!

use oxyl_diagnostics::Diagnostic;
use oxyl_lexer::{Span, Token, TokenKind};

use crate::ast::{Arg, Document, Node};

mod helpers;
use helpers::{diag_span, find_env_name, is_display_math_close, is_end_control_seq};

#[cfg(test)]
mod tests;


/// Returned by [`Parser::parse`]. The document is always produced; errors 
/// are collected alongside it so the caller sees everything at once.
#[derive(Debug)]
pub struct ParseResult {
    pub document: Document,
    pub errors: Vec<Diagnostic>,
}


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
                
                TokenKind::Comment(body) => {
                    nodes.push(Node::Comment(body, tok.span));
                }
               
                // begin{name} opens an environment.
                TokenKind::ControlSeq(ref name) if name == "begin" => {
                    let env = self.parse_environment(tok.span);
                    nodes.push(env);
                }

                // A bare \end outside an environment is a stray closer. :)
                TokenKind::ControlSeq(ref name) if name == "end" => {
                    self.errors.push(
                        Diagnostic::error("E043", "stray '\\end' (no matching '\\begin')")
                            .with_span(diag_span(tok.span)),
                    );
                    // Eat its name arg so we don't cause a slippery slope of errors lol.
                    let _ = self.parse_args();
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

                TokenKind::AlignTab => nodes.push(Node::AlignTab(tok.span)),
                TokenKind::Tilde => nodes.push(Node::Tilde(tok.span)),

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

    /// Parse `\begin{name} body \end{name}`. The opening `\begin` token has
    /// already been consumed; `begin_span` is its span.
    fn parse_environment(&mut self, begin_span: Span) -> Node {
        let mut args = self.parse_args();

        // First mandatory arg is the environment name. Without one we
        // record the error and fall back to a plain cmd so the AST 
        // still contains atleast something useful
        let (name_idx, env_name) = match find_env_name(&args) {
            Some(x) => x,
            None => {
                self.errors.push(
                    Diagnostic::error("E040", "'\\begin' missing environment name")
                        .with_span(diag_span(begin_span)),
                );
                return Node::Command {
                    name: "begin".to_owned(),
                    args,
                    span: begin_span,
                };
            }
        };
        args.remove(name_idx);

        let body = self.parse_nodes(is_end_control_seq);

        // Try consume the matching \end
        let close_span = if matches!(self.peek_kind(), Some(TokenKind::ControlSeq(s)) if s == "end") {
            let end_tok = self.bump().unwrap();
            let end_args = self.parse_args();
            let close_name = find_env_name(&end_args).map(|(_, n)| n);

            if close_name.as_deref() != Some(env_name.as_str()) {
                self.errors.push(
                    Diagnostic::error("E042", format!(
                        "'\\end{{{}}}' does not match '\\begin{{{}}}'",
                        close_name.as_deref().unwrap_or(""), env_name,
                    ))
                    .with_span(diag_span(end_tok.span))
                    .with_note(format!("the matching '\\begin' opened the '{env_name}' environment")),
                );
            }

            // Stretch the span to the last argument of \end (if any)
            end_args.last()
                .and_then(|a| match a {
                    Arg::Mandatory(c) | Arg::Optional(c) => c.last().map(|n| n.span()),
                })
                .map(|s| end_tok.span.merge(s))
                .unwrap_or(end_tok.span)
        } else {
            self.errors.push(
                Diagnostic::error("E041", format!("unclosed '\\begin{{{}}}'", env_name))
                    .with_span(diag_span(begin_span)),
            );
            body.last().map(|n| n.span()).unwrap_or(begin_span)
        };

        Node::Environment {
            name: env_name, 
            args,
            body,
            span: begin_span.merge(close_span),
        }
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

