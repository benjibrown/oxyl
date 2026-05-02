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
}
