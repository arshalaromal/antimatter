#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    Lexer,
    Parser,
    TypeCheck,
}

/// The single, top-level Error struct that handles everything.
#[derive(Debug, Clone)]
pub struct CompilerError {
    pub kind: ErrorKind,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl CompilerError {
    pub fn new(kind: ErrorKind, line: usize, column: usize, message: String) -> Self {
        Self { kind, line, column, message }
    }

    /// Formats the error into a human-readable string and forcefully terminates the compiler.
    pub fn throw(&self) {
        let error_type = match self.kind {
            ErrorKind::Lexer => "Syntax Error",
            ErrorKind::Parser => "Parsing Error",
            ErrorKind::TypeCheck => "Type Error",
        };

        eprintln!(
            "[{}] Line {}, Col {}: {}",
            error_type, self.line, self.column, self.message
        );

        std::process::exit(1);
    }
}