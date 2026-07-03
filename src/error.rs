use std::fmt;

/// A single Error produced anywhere in the compiler.
///
/// Plain data. It does not print itself and it does not exit the process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl CompilerError {
    pub fn new(line: usize, column: usize, message: String) -> Self {
        Self { line, column, message }
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: error: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for CompilerError {}

/// The result type every compiler stage returns.
pub type CompilerResult<T> = Result<T, CompilerError>;