use crate::error::{CompilerError, ErrorKind};
use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f\r]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
#[logos(skip(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/", allow_greedy = true))]
pub enum Token {
    // --- KEYWORDS ---
    #[token("var")]
    Var,
    #[token("const")]
    Const,
    #[token("fn")]
    Fn,
    #[token("return")]
    Return,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("switch")]
    Switch,
    #[token("default")]
    Default,
    #[token("defer")]
    Defer,
    #[token("pub")]
    Pub,
    #[token("struct")]
    Struct,
    #[token("impl")]
    Impl,
    #[token("enum")]
    Enum,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("type")]
    Type,
    #[token("use")]
    Use,
    #[token("self")]
    SelfKw,
    #[token("yield")]
    Yield,
    #[token("or")]
    OrKw,


    // --- PUNCTUATION & DIRECTIVES ---
    #[token("@")]
    At,
    #[token("->")]
    Arrow,
    #[token("::")]
    DoubleColon,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token(";")]
    SemiColon,
    #[token(",")]
    Comma,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("?")]
    Question,
    #[token("#")]
    Hash,

    // --- MATH & LOGIC OPERATORS ---
    #[token("=")]
    Assign,
    #[token("==")]
    EqEq,
    #[token("!")]
    Not,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("&&")]
    And,
    #[token("||")]
    Or,

    // --- BITWISE & COMPOUND ASSIGNMENT ---
    #[token("&")]
    BitAnd,
    #[token("|")]
    BitOr,
    #[token("^")]
    BitXor,
    #[token("~")]
    BitNot,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("&=")]
    BitAndEq,
    #[token("|=")]
    BitOrEq,
    #[token("^=")]
    BitXorEq,
    #[token("<<=")]
    ShlEq,
    #[token(">>=")]
    ShrEq,

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

/// Used for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

/// A struct that pairs the  Token with its exact position in the source code.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

/// Executes the lexer over the raw source code.
/// Returns a vector of `SpannedTokens`, or terminates with Error.
pub fn tokenize(source: &str) -> Vec<SpannedToken> {
    let mut lex = Token::lexer(source);
    let mut tokens = Vec::new();

    while let Some(result) = lex.next() {
        let byte_span = lex.span();

        // Convert byte span to Line & Column
        let line = source[..byte_span.start]
            .chars()
            .filter(|&c| c == '\n')
            .count()
            + 1;
        let last_newline = source[..byte_span.start]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let column = byte_span.start - last_newline + 1;

        match result {
            Ok(token) => {
                tokens.push(SpannedToken {
                    token,
                    span: Span { line, column },
                });
            }
            Err(_) => {
                let bad_text = lex.slice().to_string();
                CompilerError::new(
                    ErrorKind::Lexer,
                    line,
                    column,
                    format!("Unrecognized token '{}'", bad_text),
                )
                .throw(); // This internally calls process::exit(1)
            }
        }
    }

    tokens
}
