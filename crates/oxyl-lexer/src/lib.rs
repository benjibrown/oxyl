// oxyl-lexer 

use std::borrow::Cow;

use oxyl_diagnostics::LexError;

/// A half-open byte range `[start, end]` within a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[inline]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    #[inline]
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
pub enum TokenKind<'src> {
    ControlSeq(Cow<'src, str>),
    BeginGroup,
    EndGroup,
    MathShift,
    AlignTab,
    Parameter,
    Superscript,
    Subscript,
    Tilde,
    Comment(Cow<'src, str>),
    /// A blank line (two or more consecutive newlines). Signals a new 
    /// paragraph - the parser does not need to count newlines itself.
    ParagraphBreak,
    Space,
    Char(char),
}

/// A token together with its location in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'src> {
    pub kind: TokenKind<'src>,
    pub span: Span,
}

impl<'src> Token<'src> {
    #[inline]
    pub fn new(kind: TokenKind<'src>, span: Span) -> Self {
        Self { kind, span }
    }
}

impl std::fmt::Display for TokenKind<'_> {
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

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @ {}", self.kind, self.span)
    }
}

/// The result of tokenising a source file.
#[derive(Debug)] 
pub struct LexResult<'src> {
    pub tokens: Vec<Token<'src>>,
    pub errors: Vec<LexError>,
}

impl LexResult<'_> {
    #[inline]
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
    #[inline]
    pub fn new(src: &'src str) -> Self {
        Self { src, pos: 0 }
    }

    pub fn tokenise(&mut self) -> LexResult<'src> {
        // heurstic - avg token is 4 source bytes (one char per
        // ascii letter, plus the occassional control word lol)
        // over allocating is technically cheaper than 
        // under allocating any costs.
        let mut tokens = Vec::with_capacity(self.src.len() / 4 + 8);
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

    #[inline]
    fn peek(&self) -> Option<char> {
        let bytes = self.src.as_bytes();
        let b = *bytes.get(self.pos)?;
        if b < 0x80 {
            Some(b as char) 
        } else {
            self.src[self.pos..].chars().next()
        }
    }

    #[inline]
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

    fn next_token(&mut self) -> Result<Option<Token<'src>>, LexError> {
        let start = self.pos;
        let c = match self.peek() {
            Some(c) => c,
            None => return Ok(None),
        };

        if !c.is_ascii() {
            self.bump();
            return Err(LexError::NonAsciiChar { pos: start, ch: c });
        }

        // line comments
        if c == '%' {
            self.bump();
            let body_start = self.pos;
            self.take_while(|ch| ch != '\n');
            let body = Cow::Borrowed(&self.src[body_start..self.pos]);
            if self.peek() == Some('\n') {
                self.bump();
            }
            return Ok(Some(Token::new(
                TokenKind::Comment(body),
                Span::new(start, self.pos),
            )));
        }
        
        // newline detect paragraph breaks and allat
        if c == '\n' {
            self.bump();
            // Skips any spaces/tabs on the next line.
            self.take_while(|ch| ch == ' ' || ch == '\t');
            if self.peek() == Some('\n') {
                // blank line - consume remaining blank line whitespace.
                self.take_while(|ch| ch == '\n' || ch == ' ' || ch == '\t');
                return Ok(Some(Token::new(
                    TokenKind::ParagraphBreak,
                    Span::new(start, self.pos),
                )));
            }
            //single newline is just whitespace
            return Ok(Some(Token::new(TokenKind::Space, Span::new(start, self.pos))));
        }

        // horizontal whitespace
        if c == ' ' || c == '\t' {
            self.take_while(|ch| ch == ' ' || ch == '\t');
            return Ok(Some(Token::new(TokenKind::Space, Span::new(start, self.pos))));
        }

        // control sequences
        //
        // TeX has two ways of doing this - a control word like \foo is a \ + a run of 
        // letters, and eats trailing spaces; a control symbol like \$, \\, \[..\]
        // is exactly one letter (following the \) and does not eat spaces.
        // Both share the ControlSeq token kind tho.
        if c == '\\' {
            self.bump();
            match self.peek() {
                None => return Err(LexError::UnexpectedEndAfterBackslash { pos: start }),
                Some(next) if next.is_ascii_alphabetic() => {
                    let name_start = self.pos;
                    self.take_while(|ch| ch.is_ascii_alphabetic());
                    let name = Cow::Borrowed(&self.src[name_start..self.pos]);
                    self.take_while(|ch| ch == ' ' || ch == '\t');
                    return Ok(Some(Token::new(
                        TokenKind::ControlSeq(name),
                        Span::new(start, self.pos),
                    )));
                }
                Some(_sym) => {
                    // control symbolss 
                    // borrow the one byte slice from src so no need to allocate a one char string
                    // :)
                    let sym_start = self.pos;
                    self.bump();
                    let name = Cow::Borrowed(&self.src[sym_start..self.pos]);
                    return Ok(Some(Token::new(
                        TokenKind::ControlSeq(name),
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
    
    fn lex(src: &str) -> LexResult<'_> {
        Lexer::new(src).tokenise()
    }

    fn kinds(src: &str) -> Vec<TokenKind<'_>> {
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
        assert!(lex("é").has_errors());
    }

    #[test]
    fn lone_backslash_error() {
        assert!(lex("\\").has_errors());
    }

    #[test]
    fn control_symbol_emits_control_seq() {
        assert_eq!(kinds("\\$"), vec![TokenKind::ControlSeq("$".into())]);
        assert_eq!(kinds("\\\\"), vec![TokenKind::ControlSeq("\\".into())]);
        assert_eq!(kinds("\\#"), vec![TokenKind::ControlSeq("#".into())]);
        assert_eq!(kinds("\\["), vec![TokenKind::ControlSeq("[".into())]);
    }

    #[test]
    fn control_symbol_does_not_skip_trailing_space() {
        assert_eq!(kinds("\\$ x"), vec![
            TokenKind::ControlSeq("$".into()),
            TokenKind::Space,
            TokenKind::Char('x'),
        ]);
    }

    // TODO - TESTS FOR COW STUFF
}
