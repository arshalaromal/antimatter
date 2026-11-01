/// Position in the source code (1-indexed line and column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    pub fn dummy() -> Self {
        Self { line: 0, column: 0 }
    }
}
