use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]

// Skip standard whitespace
#[logos(skip r"[ \t\n\f\r]+")]

// Skip single-line comments: // followed by anything until a newline
#[regex(r"//[^\n]*", logos::skip)]

// Skip multi-line comments: /* followed by anything until */
#[regex(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/", logos::skip)]


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

    #[regex(r#""(?:[^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    StringLit(String),

    // --- LITERALS & IDENTIFIERS ---
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().unwrap())]
    FloatLit(f64),

    #[regex("[0-9]+", |lex| lex.slice().parse::<i64>().unwrap())]
    IntLit(i64),

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),
}