// AST types produced by the parser.
//
// Document is the top level container Node is the union
// of every thing the parser knows how to recognise - rn 
// the parser is very very dumb lol

use oxyl_lexer::{Span};

#[derive(Debug, Clone)]
pub struct Document {
    pub body: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Text(String, Span),
    ParagraphBreak(Span),
    Command { name: String , args: Vec<Arg>, span: Span, },
    Group(Vec<Node>, Span),
    /// Inline match: `$ ... $`. The span covers both `$` delimiters.
    Math(Vec<Node>, Span),
    /// Display math: `\[ ... \]`. The span covers both delimiters.
    DisplayMath(Vec<Node>, Span),
    /// A `% ...` line comment. THe string is the body without the leading 
    /// `%` and without the trailing newline - the span covers the whole 
    /// run, including both. Comments in AST since they can actually affect produced PDF.
    Comment(String, Span),
    /// A `&` column separator inside `tabular`/`array`/`align` and other environments.
    AlignTab(Span),
    /// A `~` - a non-breaking space. Acts like a regular space for layout
    /// but forbids a line break at this point.
    Tilde(Span),
    /// `\begin{name} ... \end{name}`. `args` is everything after the 
    /// environment name (optionals and additional mandatory groups). `body`
    /// holds the parsed children; the span also covers the entire construct.
    Environment {
        name: String,
        args: Vec<Arg>,
        body: Vec<Node>,
        span: Span,
    },
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
            Node::Comment(_, s) => *s,
            Node::AlignTab(s) => *s,
            Node::Tilde(s) => *s,
            Node::Environment{ span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Arg {
    Mandatory(Vec<Node>),
    Optional(Vec<Node>),
}
