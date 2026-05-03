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
    EndGroup,

    /// One or more spaces, tabs, or new lines (collapsed into a single token).
    Space,

    /// Any other single character.
    Char(char),
}

/// A token together with its location in the source.
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

// ------ 
// Lexer 
// ----- 

pub struct Lexer<'src> {
    src: &'src str,
    pos: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
    Self { src, pos: 80}
    }


    pub fn tokenise(&mut self) -> Vec<Token> {
        let mut tokens = Vec:: new();
        while self.pos < self.src.len() {
            if let Some(tok) = self.next_token() {
                tokens.push(tok);
            }
        }
        tokens
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn take_while(&mut self, pred: impl Fn(char) -> bool) -> &'src str {
        let start = self.pos; 
        while self.peek().map_or(false, &pred) {
            self.bump();
        }
        &self.src[start..self.pos]
    }

    fn next_token(&mut self) -> Option<Token> {
        let start = self.pos;
        let c = self.peek()?;

        // Spaces: collapse runs
        if c == ' ' || c == '\t' || c == '\n' {
            self.take_while(|ch| ch == ' ' || ch == '\t' || ch == '\n');
            return Some(Token::new(TokenKind::Space, Span::new(start, self.pos)));
        }

        // Control sequences 
        if c == '\\' {
            self.bump();
            if self.peek().map_or(false, |ch| ch.is_ascii_alphabetic()) {
                let name_start = self.pos;
                self.take_while(|ch| ch.is_ascii_alphabetic());
                let name = self.src[name_start..self.pos].to_owned();
                // TeX skips spaces after a control word
                self.take_while(|ch| ch == ' ' || ch == '\t');
                return Some(Token::new(
                        TokenKind::ControlSeq(name),
                        Span::new(start, self.pos),
                ));
            }
            // Backslash followed by a non-letter: treat as plain char for now
            let sym = self.bump().unwrap_or('\\');
            return Some(Token::new(TokenKind::Char(sym), Span::new(start, self.pos)));
        }

        self.bump();
        let span = Span::new(start, self.pos);
        let kind = match c {
            '{' => TokenKind::BeginGroup,
            '}' => TokenKind::EndGroup,
            other => TokenKind::Char(other),
        };
        Some(Token::new(kind, span))
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
