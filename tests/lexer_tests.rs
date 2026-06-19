use logos::Logos;
use antimatter::lexer::Token;

fn assert_tokens(source: &str, expected: &[Token]) {
    let mut lexer = Token::lexer(source);

    for (i, expected_token) in expected.iter().enumerate() {
        let next_token = lexer.next();
        assert_eq!(
            next_token,
            Some(Ok(expected_token.clone())),
            "\nToken mismatch at index {} for source: '{}'\nExpected: {:?}\nGot: {:?}",
            i, source, expected_token, next_token
        );
    }

    let final_token = lexer.next();
    assert_eq!(
        final_token, None,
        "\nExpected EOF, but found extra token: {:?} in source: '{}'",
        final_token, source
    );
}

#[test]
fn test_keywords() {
    let code = "var const fn return if else while for in switch default defer pub struct impl enum true false break continue type use self";

    assert_tokens(code, &[
        Token::Var, Token::Const, Token::Fn, Token::Return,
        Token::If, Token::Else, Token::While, Token::For,
        Token::In, Token::Switch, Token::Default, Token::Defer,
        Token::Pub, Token::Struct, Token::Impl, Token::Enum,
        Token::True, Token::False, Token::Break, Token::Continue,
        Token::Type, Token::Use, Token::SelfKw,
    ]);
}

#[test]
fn test_punctuation() {
    let code = "@ -> :: . : ; , ( ) { } [ ]";

    assert_tokens(code, &[
        Token::At, Token::Arrow, Token::DoubleColon, Token::Dot,
        Token::Colon, Token::SemiColon, Token::Comma,
        Token::LParen, Token::RParen, Token::LBrace, Token::RBrace,
        Token::LBracket, Token::RBracket
    ]);
}

#[test]
fn test_math_and_logic_operators() {
    let code = "= == ! != < > <= >= + - * / % && ||";

    assert_tokens(code, &[
        Token::Assign, Token::EqEq, Token::Not, Token::NotEq,
        Token::Lt, Token::Gt, Token::LtEq, Token::GtEq,
        Token::Plus, Token::Minus, Token::Star, Token::Slash,
        Token::Percent, Token::And, Token::Or
    ]);
}

#[test]
fn test_bitwise_and_compound_assignments() {
    let code = "& | ^ ~ << >> += -= *= /= &= |= ^= <<= >>=";

    assert_tokens(code, &[
        Token::BitAnd, Token::BitOr, Token::BitXor, Token::BitNot,
        Token::Shl, Token::Shr, Token::PlusEq, Token::MinusEq,
        Token::StarEq, Token::SlashEq, Token::BitAndEq, Token::BitOrEq,
        Token::BitXorEq, Token::ShlEq, Token::ShrEq
    ]);
}

#[test]
fn test_literals_and_identifiers() {
    let code = "1024 3.14159 my_variable _hiddenVal";

    assert_tokens(code, &[
        Token::IntLit(1024),
        Token::FloatLit(3.14159),
        Token::Ident("my_variable".to_string()),
        Token::Ident("_hiddenVal".to_string()),
    ]);
}

#[test]
fn test_strings() {
    let code = r#"
        "Hello ${name}"
        `Raw string \n no escapes`
    "#;

    assert_tokens(code, &[
        Token::StringLit("\"Hello ${name}\"".to_string()),
        Token::RawString("Raw string \\n no escapes".to_string()),
    ]);
}

#[test]
fn test_comments_are_ignored() {
    let code = r#"
        var x = 10; // This is a single line comment
        /* This is a
           multi-line comment */
        var y = 20;
    "#;

    assert_tokens(code, &[
        Token::Var, Token::Ident("x".to_string()), Token::Assign, Token::IntLit(10), Token::SemiColon,
        Token::Var, Token::Ident("y".to_string()), Token::Assign, Token::IntLit(20), Token::SemiColon,
    ]);
}