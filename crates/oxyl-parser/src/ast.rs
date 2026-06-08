// AST types produced by the parser.
//
// Document is the top level container Node is the union
// of every thing the parser knows how to recognise - rn 
// the parser is very very dumb lol

use std::borrow::Cow;

use oxyl_lexer::Span;

#[derive(Debug, Clone)]
pub struct Document<'src> {
    pub body: Vec<Node<'src>>,
}

#[derive(Debug, Clone)]
pub enum Node<'src> {
    Text(Cow<'src, str>, Span),
    ParagraphBreak(Span),
Command { name: Cow<'src, str>, args: Vec<Arg<'src>>, span: Span },
    Group(Vec<Node<'src>>, Span),
    /// Inline match: `$ ... $`. The span covers both `$` delimiters.
    Math(Vec<Node<'src>>, Span),
    /// Display math: `\[ ... \]`. The span covers both delimiters.
    DisplayMath(Vec<Node<'src>>, Span),
    /// A `% ...` line comment. THe string is the body without the leading 
    /// `%` and without the trailing newline - the span covers the whole 
    /// run, including both. Comments in AST since they can actually affect produced PDF.
    Comment(Cow<'src, str>, Span),
    /// A `&` column separator inside `tabular`/`array`/`align` and other environments.
    AlignTab(Span),
    /// A `~` - a non-breaking space. Acts like a regular space for layout
    /// but forbids a line break at this point.
    Tilde(Span),
    /// `\begin{name} ... \end{name}`. `args` is everything after the 
    /// environment name (optionals and additional mandatory groups). `body`
    /// holds the parsed children; the span also covers the entire construct.
    Environment {
        name: Cow<'src, str>,
        args: Vec<Arg<'src>>,
        body: Vec<Node<'src>>,
        span: Span,
    },
}

impl<'src> Node<'src> {
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
pub enum Arg<'src> {
    Mandatory(Vec<Node<'src>>),
    Optional(Vec<Node<'src>>),
}
