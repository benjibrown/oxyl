// oxyl-lexer 

use oxyl_diagnostics::LexError;

/// A half-open byte range `[start, end]` within a source file.
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

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// The kind of a single lexical token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    ControlSeq(String),
    BeginGroup,
    EndGroup,
    MathShift,
    AlignTab,
    Parameter,
    Superscript,
    Subscript,
    Tilde,
    Comment(String),
    /// A blank line (two or more consecutive newlines). Signals a new 
    /// paragraph - the parser does not need to count newlines itself.
    ParagraphBreak,
    Space,
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

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::ControlSeq(name) => write!(f, "\\{name}"),
            TokenKind::BeginGroup => write!(f, "{{"),
            TokenKind::EndGroup => write!(f, "}}"),
            TokenKind::MathShift => write!(f, "$"),
            TokenKind::AlignTab => write!(f, "&"),
            TokenKind::Parameter => write!(f, "#"),
            TokenKind::Superscript => write!(f, "^"),
            TokenKind::Subscript => write!(f, "_"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::Space => write!(f, "<space>"),
            TokenKind::ParagraphBreak => write!(f, "<par>"),
            TokenKind::Comment(body) => write!(f, "%{body}"),
            TokenKind::Char(c) => write!(f, "{c}"),
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @ {}", self.kind, self.span)
    }
}

/// The result of tokenising a source file.
#[derive(Debug)] 
pub struct LexResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
}

impl LexResult {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

// ----
// Lexer 
// ----

pub struct Lexer<'src> {
    src: &'src str,
    pos: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Self { src, pos: 0 }
    }

    pub fn tokenise(&mut self) -> LexResult {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        while self.pos < self.src.len() {
            match self.next_token() {
                Ok(Some(tok)) => tokens.push(tok),
                Ok(None) => {}
                Err(e) => errors.push(e),
            }
        }
        LexResult { tokens, errors }
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

    fn next_token(&mut self) -> Result<Option<Token>, LexError> {
        let start = self.pos;
        let c = match self.peek() {
            Some(c) => c,
            None => return Ok(None),
        };

        if !c.is_ascii() {
            self.bump();
            return Err(LexError::NonAsciiChar { pos: start, ch: c });
        }

        // Line comments.
        if c == '%' {
            self.bump();
            let body = self.take_while(|ch| ch != '\n').to_owned();
            if self.peek() == Some('\n') {
                self.bump();
            }
            return Ok(Some(Token::new(
                TokenKind::Comment(body),
                Span::new(start, self.pos),
            )));
        }
        
        // Newlines: detect paragraph breaks (blank lines) vs ordinary space.
        if c == '\n' {
            self.bump();
            // Skips any spaces/tabs on the next line.
            self.take_while(|ch| ch == ' ' || ch == '\t');
            if self.peek() == Some('\n') {
                // Blank line - consume remaining blank-line whitespace.
                self.take_while(|ch| ch == '\n' || ch == ' ' || ch == '\t');
                return Ok(Some(Token::new(
                            TokenKind::ParagraphBreak,
                            Span::new(start, self.pos),
                )));
            }
            // Single newline is just whitespace. 
            return Ok(Some(Token::new(TokenKind::Space, Span::new(start, self.pos))));
        }

        // Horizontal whitespace.
        if c == ' ' || c == '\t' {
            self.take_while(|ch| ch == ' ' || ch == '\t');
                return Ok(Some(Token::new(TokenKind::Space, Span::new(start, self.pos))));
        }

        // Control sequences.
        if c == '\\' {
            self.bump();
            match self.peek() {
                None => { return Err(LexError::UnexpectedEndAfterBackslash { pos: start }); }
                Some(next) if next.is_ascii_alphabetic() => {
                    let name_start = self.pos;
                    self.take_while(|ch| ch.is_ascii_alphabetic());
                    let name = self.src[name_start..self.pos].to_owned();
                    self.take_while(|ch| ch == ' ' || ch == '\t');
                    return Ok(Some(Token::new(
                        TokenKind::ControlSeq(name),
                        Span::new(start, self.pos),
                    )));
                }
                Some(sym) => {
                    self.bump();
                    return Ok(Some(Token::new(
                        TokenKind::Char(sym),
                        Span::new(start, self.pos),
                    )));
                }
            }
        }

        self.bump();
        let span = Span::new(start, self.pos);
        let kind = match c {
            '{' => TokenKind::BeginGroup,
            '}' => TokenKind::EndGroup,
            '$' => TokenKind::MathShift,
            '&' => TokenKind::AlignTab,
            '#' => TokenKind::Parameter,
            '^' => TokenKind::Superscript,
            '_' => TokenKind::Subscript,
            '~' => TokenKind::Tilde,
            other => TokenKind::Char(other),
        };
        Ok(Some(Token::new(kind, span)))
    }
}

// --- 
// Tests 
// ---

#[cfg(test)]
mod tests {
    use super::*;
    
    fn lex(src: &str) -> LexResult {
        Lexer::new(src).tokenise()
    }

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn empty() {
        assert_eq!(kinds(""), vec![]);
    }

    #[test]
    fn plain_chars() {
        assert_eq!(kinds("ab"), vec![TokenKind::Char('a'), TokenKind::Char('b')]);
    }

    #[test]
    fn control_seq() {
        assert_eq!(kinds("\\hello"), vec![TokenKind::ControlSeq("hello".into())]);
    }

    #[test]
    fn control_seq_skips_trailing_space() {
        let ks = kinds("\\foo bar");
        assert_eq!(ks[0], TokenKind::ControlSeq("foo".into()));
        assert_eq!(ks[1], TokenKind::Char('b'));
    }

    #[test]
    fn paragraph_break_detected() {
        assert!(kinds("a\n\nb").contains(&TokenKind::ParagraphBreak));
    }

    #[test]
    fn single_newline_is_space() {
        assert_eq!(kinds("a\nb"), vec![
            TokenKind::Char('a'),
            TokenKind::Space,
            TokenKind::Char('b'),
        ]);
    }

    #[test]
    fn comment() {
        let ks = kinds("a% hi\nb");
        assert_eq!(ks[1], TokenKind::Comment(" hi".into()));
    }

    #[test]
    fn special_chars() {
        assert_eq!(kinds("$^_~&#"), vec![
            TokenKind::MathShift, TokenKind::Superscript, TokenKind::Subscript,
            TokenKind::Tilde, TokenKind::AlignTab, TokenKind::Parameter,
        ]);
    }

    #[test]
    fn non_ascii_error() {
        assert!(lex("\\").has_errors());
    }
}
