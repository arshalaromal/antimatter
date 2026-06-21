use logos::Logos;
use crate::error::{CompilerError, ErrorKind};

#[derive(Logos, Debug, PartialEq, Clone)]

// Skip standard whitespace
#[logos(skip r"[ \t\n\f\r]+")]

// Skip single-line comments (explicitly allowing greedy matching)
#[logos(skip(r"//[^\n]*", allow_greedy = true))]

// Skip multi-line comments (explicitly allowing greedy matching)
#[logos(skip(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/", allow_greedy = true))]

//This Token enum contains all tokens for lexer to output.
pub enum Token {
    // --- KEYWORDS ---
    #[token("var")] Var,
    #[token("const")] Const,
    #[token("fn")] Fn,
    #[token("return")] Return,
    #[token("if")] If,
    #[token("else")] Else,
    #[token("while")] While,
    #[token("for")] For,
    #[token("in")] In,
    #[token("switch")] Switch,
    #[token("default")] Default,
    #[token("defer")] Defer,
    #[token("pub")] Pub,
    #[token("struct")] Struct,
    #[token("impl")] Impl,
    #[token("enum")] Enum,
    #[token("true")] True,
    #[token("false")] False,
    #[token("break")] Break,
    #[token("continue")] Continue,
    #[token("type")] Type,
    #[token("use")] Use,
    #[token("self")] SelfKw,
    #[token("yield")] Yield,

    // --- PUNCTUATION & DIRECTIVES ---
    #[token("@")] At,
    #[token("->")] Arrow,
    #[token("::")] DoubleColon,
    #[token(".")] Dot,
    #[token(":")] Colon,
    #[token(";")] SemiColon,
    #[token(",")] Comma,
    #[token("(")] LParen,
    #[token(")")] RParen,
    #[token("{")] LBrace,
    #[token("}")] RBrace,
    #[token("[")] LBracket,
    #[token("]")] RBracket,

    // --- MATH & LOGIC OPERATORS ---
    #[token("=")] Assign,
    #[token("==")] EqEq,
    #[token("!")] Not,
    #[token("!=")] NotEq,
    #[token("<")] Lt,
    #[token(">")] Gt,
    #[token("<=")] LtEq,
    #[token(">=")] GtEq,
    #[token("+")] Plus,
    #[token("-")] Minus,
    #[token("*")] Star,
    #[token("/")] Slash,
    #[token("%")] Percent,
    #[token("&&")] And,
    #[token("||")] Or,

    // --- BITWISE & COMPOUND ASSIGNMENT ---
    #[token("&")] BitAnd,
    #[token("|")] BitOr,
    #[token("^")] BitXor,
    #[token("~")] BitNot,
    #[token("<<")] Shl,
    #[token(">>")] Shr,
    #[token("+=")] PlusEq,
    #[token("-=")] MinusEq,
    #[token("*=")] StarEq,
    #[token("/=")] SlashEq,
    #[token("&=")] BitAndEq,
    #[token("|=")] BitOrEq,
    #[token("^=")] BitXorEq,
    #[token("<<=")] ShlEq,
    #[token(">>=")] ShrEq,

    // --- STRINGS ---
    #[regex(r"`[^`]*`", |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    RawString(String),


    #[regex(r#""(?:[^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    StringLit(String),

    // --- LITERALS & IDENTIFIERS ---

    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    FloatLit(f64),

    #[regex("[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntLit(i64),

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),
}


/// Executes the lexer over the raw source code.
/// Returns a vector of tokens, or forcefully terminates with Error.
pub fn tokenize(source: &str) -> Vec<Token> {
    // Initialize the Logos lexer
    let mut lex = Token::lexer(source);
    let mut tokens = Vec::new();

    // Loop through every token in the text
    while let Some(result) = lex.next() {
        match result {
            Ok(token) => tokens.push(token),
            Err(_) => {
                let span = lex.span();

                // Count how many newlines (\n) happened before this error
                let line = source[..span.start].chars().filter(|&c| c == '\n').count() + 1;

                // Find the distance from the last newline to get the column
                let last_newline = source[..span.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let column = span.start - last_newline + 1;

                let bad_text = lex.slice().to_string();

                CompilerError::new(
                    ErrorKind::Lexer,
                    line,
                    column,
                    format!("Unrecognized token '{}'", bad_text),
                ).throw();
            }
        }
    }

    tokens
}