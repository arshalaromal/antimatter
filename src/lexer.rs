use crate::error::{CompilerError, CompilerResult};
use logos::Logos;

/// One segment of an interpolated string.
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Literal(String),    // plain text
    ExprSource(String), // raw expr inside ${}
}

/// Splits "Hello ${name}, age ${age}!" into parts.
/// Handles nested braces: "${obj.method({})}".
fn split_interpolated(s: &str) -> Vec<StringPart> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Detect ${
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            if !literal.is_empty() {
                parts.push(StringPart::Literal(literal.clone()));
                literal.clear();
            }
            i += 2; // skip ${
            let mut depth = 1;
            let mut expr_src = String::new();
            while i < chars.len() {
                match chars[i] {
                    '{' => {
                        depth += 1;
                        expr_src.push('{');
                    }
                    '}' => {
                        depth -= 1;
                        if depth == 0 { break; }
                        expr_src.push('}');
                    }
                    c => { expr_src.push(c); }
                }
                i += 1;
            }
            parts.push(StringPart::ExprSource(expr_src.trim().to_string()));
            i += 1; // skip closing }
        } else if chars[i] == '\\' && i + 1 < chars.len() {
            // Preserve escape sequences
            literal.push('\\');
            literal.push(chars[i + 1]);
            i += 2;
        } else {
            literal.push(chars[i]);
            i += 1;
        }
    }

    if !literal.is_empty() {
        parts.push(StringPart::Literal(literal));
    }
    parts
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f\r]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
#[logos(skip(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/", allow_greedy = true))]
pub enum Token {
    //KEYWORDS
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
    #[token("Error")]
    Error,

    //PUNCTUATION & DIRECTIVES
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

    //MATH and LOGIC OPERATORS
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

    //BITWISE and COMPOUND ASSIGNMENT
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

    //STRINGS
    #[regex(r"`[^`]*`", |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    RawString(String),

    /// Plain string: no ${} inside.
    #[regex(r#""(?:[^"\\$]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    StringLit(String),

    /// Interpolated string: contains ${expr} segments.
    /// The lexer splits it then the parser reparses each expr segment.
    #[regex(r#""(?:[^"\\]|\\.)*\$\{(?:[^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        let inner = &s[1..s.len()-1];
        Some(split_interpolated(inner))
    })]
    InterpolatedStringLit(Vec<StringPart>),

    //LITERALS and IDENTIFIERS
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    FloatLit(f64),

    #[regex("[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntLit(i64),

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),
}

/// Used for error reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

/// Pairs a Token with its exact position in the source code.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

/// Runs the lexer over raw source code and returns a token stream.
///
/// Returns `Err(CompilerError)` on the first unrecognized character instead
/// of exiting the process. The caller decides what to do with a lex failure.
pub fn tokenize(source: &str) -> CompilerResult<Vec<SpannedToken>> {
    let mut lex = Token::lexer(source);
    let mut tokens = Vec::new();

    while let Some(result) = lex.next() {
        let byte_span = lex.span();

        // Convert byte span to line & column.
        let line = source[..byte_span.start].chars().filter(|&c| c == '\n').count() + 1;
        let last_newline = source[..byte_span.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let column = byte_span.start - last_newline + 1;

        match result {
            Ok(token) => {
                tokens.push(SpannedToken { token, span: Span { line, column } });
            }
            Err(_) => {
                let bad_text = lex.slice().to_string();
                return Err(CompilerError::new(
                    line,
                    column,
                    format!("Unrecognized token '{}'", bad_text),
                ));
            }
        }
    }

    Ok(tokens)
}