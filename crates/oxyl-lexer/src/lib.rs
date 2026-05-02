// oxyl-lexer 

/// A half-open byte range `[start, end]` within a source file.
///
/// Every token will carry one of these so errors can point at the 
/// exact bytes that caused the problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// The kind of a single lexical token.
///
/// This is a first pass - will add more variants later on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// A control sequence such as `\frac` or `\begin`. Stores the name 
    /// without the leading backslash.
    ControlSeq(String),

    /// `{`
    BeginGroup,

    /// `}`
    Endgroup,

    /// One or more spaces or tabs (collapsed into a single token).
    Space,

    /// Any other single character.
    Char(char),
}

/// A token together with uits location in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_len() {
        assert_eq!(Span::new(0, 5).len(), 5);
    }
    
    #[test]
    fn span_zero_len() {
        assert_eq!(Span::new(3, 3).len(), 0);
    }

    #[test]
    fn span_is_empty() {
        assert!(Span::new(2, 2).is_empty());
        assert!(!Span::new(2, 3).is_empty());
    }

    #[test]
    fn token_stores_span() {
        let t = Token::new(TokenKind::BeginGroup, Span::new(4, 5));
        assert_eq!(t.span.len(), 1);
    }
}
