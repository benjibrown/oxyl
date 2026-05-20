/// A view of source text with a precomputed line index.
///
/// Construct once per file; the line table is built upfront so that 
/// repeated `line_col` lookups (one per diagnostic) aren't super slow
/// I think O(log lines) - will fact check this later.
/// An optional `name` (typically the file path) can be included 
/// and rendered in diagnostics as `--> name:line:col`.
pub struct Source<'a> {
    text: &'a str,
    name: Option<String>,
    line_starts: Vec<usize>,
}

impl<'a> Source<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { text, name: None, line_starts }
    }

    /// Build a `Source` whose rendered diagnostics include `name` (usually
    /// a file path) in the location header.
    pub fn with_name(text: &'a str, name: impl Into<String>) -> Self {
        let mut s = Self::new(text);
        s.name = Some(name.into());
        s
    }

    /// The display name attached to this source, if any.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    
    /// Convert a byte offset into a 1-based `(line, column)`.
    ///
    /// Columns are byte-counted; the lexer rejects non-ASCII, so this 
    /// matches what a user would expect from any ASCII editor.
    pub fn line_col(&self, byte: usize) -> (usize, usize) {
        let line_idx = match self.line_starts.binary_search(&byte) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx];
        let col = byte.min(self.text.len()) - line_start;
        (line_idx + 1, col + 1)
    }
    
    /// Return the text of a 1-based line, without the trailing newline.
    pub fn line_text(&self, line: usize) -> &str {
        let idx = line.saturating_sub(1).min(self.line_starts.len() - 1);
        let start = self.line_starts[idx];
        let end = self.line_starts.get(idx + 1)
            .map(|&n| n.saturating_sub(1))
            .unwrap_or(self.text.len());
        &self.text[start..end.min(self.text.len())]
    }
}
