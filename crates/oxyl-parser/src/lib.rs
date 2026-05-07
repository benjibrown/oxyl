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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_node_span() {
        let span = Span::new(0, 5);
        let node = Node::Text("hello".into(), span);
        assert_eq!(node.span(), span);
    }

    #[test]
    fn group_node_span() {
        let span = Span::new(2, 9);
        let node = Node::Group(vec![], span);
        assert_eq!(node.span(), span);
    }
}
