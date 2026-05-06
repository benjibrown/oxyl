// oxyl-lexer 

use oxyl_diagnostics::LexError;

/// A half-open byte range `[start, end]` within a source file.
///
/// Every token carries one of these so errors can point at the 
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

    /// Merge two spans into one convering both. `self` should come before
    /// `other` in the source.
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
    /// A control sequence such as `\frac` or `\begin`. Stores the name 
    /// without the leading backslash.
    ControlSeq(String),

    /// `{`
    BeginGroup,

    /// `}`
    EndGroup,

    /// `$` - math mode switch.
    MathShift,

    /// `&` - column seperator in tables and alignments.
    AlignTab,

    /// `#` - parameter character in macro definitions.
    Parameter,

    /// `^` - superscript.
    Superscript,

    /// `_` - subscript.
    Subscript,

    /// `~` - non-breaking space (active character in plain LaTeX).
    Tilde,

    /// A `%` line comment. Stores the comment body, not the `%` or newline.
    Comment(String),

    /// One or more spaces, tabs, or newlines (collapsed into a single token).
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

// --- 
// LexResult
// ---- 

/// The result of tokenising a source file.
///
/// We collect errors rather than stopping at the first one so the CLI can 
/// report everything in a single pass.
#[derive(Debug)] 
pub struct LexResult {
    pub tokens: Vec<Token>,
    /// Any non-fatal errors encountered. The token stream is still usable
    /// even when this is non-empty.
    pub errors: Vec<LexError>,
}

impl LexResult {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
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
        Self { src, pos: 0 }
    }

    /// Tokenise the source string and return all tokens.
    pub fn tokenise(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
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

        // Line comments: consume to end of line 
        if c == '%' {
            self.bump(); // consume '%'
            let body = self.take_while(|ch| ch != '\n').to_owned();
            // Consume the newline itself so it does not become a Space token.
            if self.peek() == Some('\n') {
                self.bump();
            }
            return Some(Token::new(
                TokenKind::Comment(body),
                Span::new(start, self.pos),
            ));
        }

        // Spaces: collapse runs.
        if c == ' ' || c == '\t' || c == '\n' {
            self.take_while(|ch| ch == ' ' || ch == '\t' || ch == '\n');
            return Some(Token::new(TokenKind::Space, Span::new(start, self.pos)));
        }

        // Control sequences.
        if c == '\\' {
            self.bump();
            if self.peek().map_or(false, |ch| ch.is_ascii_alphabetic()) {
                let name_start = self.pos;
                self.take_while(|ch| ch.is_ascii_alphabetic());
                let name = self.src[name_start..self.pos].to_owned();
                // TeX skips spaces after a control word.
                self.take_while(|ch| ch == ' ' || ch == '\t');
                return Some(Token::new(
                    TokenKind::ControlSeq(name),
                    Span::new(start, self.pos),
                ));
            }
            // Backslash followed by a non-letter: treat as a plain char for now.
            let sym = self.bump().unwrap_or('\\');
            return Some(Token::new(TokenKind::Char(sym), Span::new(start, self.pos)));
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
        Some(Token::new(kind, span))
    }
}




// Tests 
#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src).tokenise().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn span_len() {
        assert_eq!(Span::new(0, 5).len(), 5);
    }
    
    #[test]
    fn span_is_empty() {
        assert!(Span::new(2, 2).is_empty());
        assert!(!Span::new(2, 3).is_empty());
    }
    
    #[test]
    fn empty_input() {
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
        // TeX eats spaces after a control word. 
        let ks = kinds("\\foo bar");
        assert_eq!(ks[0], TokenKind::ControlSeq("foo".into()));
        assert_eq!(ks[1], TokenKind::Char('b'));
    }

    #[test]
    fn groups() {
        assert_eq!(kinds("{}"), vec![TokenKind::BeginGroup, TokenKind::EndGroup]);
    }

    #[test]
    fn space_collapse() {
        assert_eq!(kinds("a  b"), vec![
            TokenKind::Char('a'),
            TokenKind::Space,
            TokenKind::Char('b'),
        ]);
    }

    #[test]
    fn special_chars() {
        assert_eq!(kinds("$"), vec![TokenKind::MathShift]);
        assert_eq!(kinds("&"), vec![TokenKind::AlignTab]);
        assert_eq!(kinds("#"), vec![TokenKind::Parameter]);
        assert_eq!(kinds("^"), vec![TokenKind::Superscript]);
        assert_eq!(kinds("_"), vec![TokenKind::Subscript]);
        assert_eq!(kinds("~"), vec![TokenKind::Tilde]);
    }

    #[test]
    fn comment_to_end_of_line() {
        let ks = kinds("a% this is ignored\nb");
        assert_eq!(ks[0], TokenKind::Char('a'));
        assert_eq!(ks[1], TokenKind::Comment(" this is ignored".into()));
        assert_eq!(ks[2], TokenKind::Char('b'));
    }

    #[test]
    fn comment_at_end_of_input() {
        let ks = kinds("% no newline");
        assert_eq!(ks, vec![TokenKind::Comment(" no newline".into())]);
    }

    #[test]
    fn span_merge() {
        let a = Span::new(0, 5);
        let b = Span::new(8, 12);
        assert_eq!(a.merge(b), Span::new(0,12));
    }

    #[test]
    fn span_display() {
        assert_eq!(Span::new(3, 7).to_string(), "3..7");
    }

    #[test]
    fn token_display() {
        let t = Token::new(TokenKind::ControlSeq("frac".into()), Span::new(0, 5));
        assert_eq!(t.to_string(), "\\frac @ 0..5");
    }
}
