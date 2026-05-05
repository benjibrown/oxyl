// oxyl-parser
// 
// AST node types for oxyl. 
// TODO - add actual parsing logic :)

use oxyl_lexer::Span;

/// The root of a parsed LaTeX document.
#[derive(Debug, Clone)]
pub struct Document {
    /// Everything before `\begin{document}`.
    pub preamble: Vec<Node>,
    /// The body of the document.
    pub body: Vec<Node>
}

/// A single node in the LaTeX AST.
///
/// More variants will be added as the parser is built out.
#[derive(Debug, Clone)]
pub enum Node {
    /// A run of plain text characters
    Text(String, Span),

    /// A paragraph break (blank line in the source)
    ParagraphBreak(Span),

    /// A LaTeX command and its arguments, e.g. `\textbf{hello}`.
    Command {
        name: String ,
        args: Vec<Arg>,
        body: Vec<Node>,
        span: Span,
    },

    /// A `\begin{name}...\end{name}` environment
    Environment {
        name: String,
        args: Vec<Arg>,
        body: Vec<Node>,
        span: Span,
    },

    /// A braced group `{...}` that is not a command argument.
    Group(Vec<Node>, Span),
}

impl Node {
    /// The source span of this node.
    pub fn span(&self) -> Span {
        match self {
            Node::Text(_,s) => *s,
            Node::ParagraphBreak(s) => *s,
            Node::Command { span, .. } => *span,
            Node::Environment { span, .. } => *span,
            Node::Group(_, s)
        }
    }
}

/// A single argument to a command or environment 
#[derive(Debug, Clone)]
pub enum Arg {
    /// A mandatory argument in braces `{...}`.
    Mandatory(Vec<Node>),
    /// An optional argument in brackets `[...]`
    Optional(Vec<Node>).
}

#[cfg(tests)]
mod tests {
    use super::*;
}
