// Lexer tests

use antimatter::error::CompilerError;
use antimatter::lexer::{tokenize, StringPart, Token};

/// Lexes source and returns the tokens, dropping span info. Panics with the compiler's error message on bad input which is fine for tests
/// that expect valid syntax.
fn lex(src: &str) -> Vec<Token> {
    tokenize(src)
        .unwrap_or_else(|e| panic!("{}", e))
        .into_iter()
        .map(|st| st.token)
        .collect()
}

/// Lexes source and returns the raw Result for tests that specifically
/// want to check a lex failure.
fn try_lex(src: &str) -> Result<Vec<Token>, CompilerError> {
    tokenize(src).map(|tokens| tokens.into_iter().map(|st| st.token).collect())
}

//KEYWORDS
#[test]
fn test_keywords() {
    assert_eq!(
        lex(
            "var const fn return if else while for in pub struct impl enum type use self yield or break continue defer switch default Error true false"
        ),
        vec![
            Token::Var,
            Token::Const,
            Token::Fn,
            Token::Return,
            Token::If,
            Token::Else,
            Token::While,
            Token::For,
            Token::In,
            Token::Pub,
            Token::Struct,
            Token::Impl,
            Token::Enum,
            Token::Type,
            Token::Use,
            Token::SelfKw,
            Token::Yield,
            Token::OrKw,
            Token::Break,
            Token::Continue,
            Token::Defer,
            Token::Switch,
            Token::Default,
            Token::Error,
            Token::True,
            Token::False
        ]
    );
}
//OPERATORS
#[test]
fn test_arithmetic_operators() {
    assert_eq!(
        lex("+ - * / %"),
        vec![
            Token::Plus,
            Token::Minus,
            Token::Star,
            Token::Slash,
            Token::Percent
        ]
    );
}

#[test]
fn test_comparison_operators() {
    assert_eq!(
        lex("== != < > <= >="),
        vec![
            Token::EqEq,
            Token::NotEq,
            Token::Lt,
            Token::Gt,
            Token::LtEq,
            Token::GtEq
        ]
    );
}

#[test]
fn test_compound_assignment() {
    assert_eq!(
        lex("+= -= *= /= &= |= ^= <<= >>="),
        vec![
            Token::PlusEq,
            Token::MinusEq,
            Token::StarEq,
            Token::SlashEq,
            Token::BitAndEq,
            Token::BitOrEq,
            Token::BitXorEq,
            Token::ShlEq,
            Token::ShrEq,
        ]
    );
}

#[test]
fn test_bitwise_operators() {
    assert_eq!(
        lex("& | ^ ~ << >>"),
        vec![
            Token::BitAnd,
            Token::BitOr,
            Token::BitXor,
            Token::BitNot,
            Token::Shl,
            Token::Shr
        ]
    );
}

#[test]
fn test_logical_operators() {
    assert_eq!(lex("&& ||"), vec![Token::And, Token::Or]);
}

#[test]
fn test_multichar_operators_not_split() {
    assert_eq!(lex(">="), vec![Token::GtEq]);
    assert_eq!(lex("<="), vec![Token::LtEq]);
    assert_eq!(lex("=="), vec![Token::EqEq]);
    assert_eq!(lex("&&"), vec![Token::And]);
    assert_eq!(lex("<<="), vec![Token::ShlEq]);
}

#[test]
fn test_adjacent_single_char_operators_stay_separate() {
    assert_eq!(lex("! ="), vec![Token::Not, Token::Assign]);
    assert_eq!(lex("!="), vec![Token::NotEq]);
}

//PUNCTUATION
#[test]
fn test_punctuation() {
    assert_eq!(
        lex("@ -> :: . : ; , ( ) { } [ ] ? #"),
        vec![
            Token::At,
            Token::Arrow,
            Token::DoubleColon,
            Token::Dot,
            Token::Colon,
            Token::SemiColon,
            Token::Comma,
            Token::LParen,
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
            Token::LBracket,
            Token::RBracket,
            Token::Question,
            Token::Hash,
        ]
    );
}

//NUMERIC LITERALS
#[test]
fn test_integer_literal() {
    assert_eq!(lex("42"), vec![Token::IntLit(42)]);
    assert_eq!(lex("0"), vec![Token::IntLit(0)]);
}

#[test]
fn test_float_literal() {
    assert_eq!(lex("3.14"), vec![Token::FloatLit(3.14)]);
    assert_eq!(lex("0.5"), vec![Token::FloatLit(0.5)]);
}

#[test]
fn test_chained_dots_after_number() {
    assert_eq!(
        lex("1.2.3"),
        vec![Token::FloatLit(1.2), Token::Dot, Token::IntLit(3)]
    );
}

#[test]
fn test_integer_then_dot_access() {
    // Plain integer followed by member access must NOT be swallowed into a float.
    assert_eq!(
        lex("5.toString"),
        vec![
            Token::IntLit(5),
            Token::Dot,
            Token::Ident("toString".to_string())
        ]
    );
}

//STRING LITERALS
#[test]
fn test_string_literal() {
    assert_eq!(
        lex(r#""hello""#),
        vec![Token::StringLit("hello".to_string())]
    );
    assert_eq!(
        lex(r#""hello world""#),
        vec![Token::StringLit("hello world".to_string())]
    );
}

#[test]
fn test_raw_string() {
    assert_eq!(
        lex(r#"`raw \n string`"#),
        vec![Token::RawString(r"raw \n string".to_string())]
    );
}

#[test]
fn test_empty_string() {
    assert_eq!(lex(r#""""#), vec![Token::StringLit("".to_string())]);
}

#[test]
fn test_string_with_escaped_quote() {
    assert_eq!(
        lex(r#""he said \"hi\"""#),
        vec![Token::StringLit(r#"he said \"hi\""#.to_string())]
    );
}

#[test]
fn test_escaped_dollar_is_not_interpolation() {
    assert_eq!(
        lex(r#""price: \$5""#),
        vec![Token::StringLit(r"price: \$5".to_string())]
    );
}

//IDENTIFIERS
#[test]
fn test_identifiers() {
    assert_eq!(
        lex("foo bar _private camelCase snake_case STARK"),
        vec![
            Token::Ident("foo".to_string()),
            Token::Ident("bar".to_string()),
            Token::Ident("_private".to_string()),
            Token::Ident("camelCase".to_string()),
            Token::Ident("snake_case".to_string()),
            Token::Ident("STARK".to_string()),
        ]
    );
}

#[test]
fn test_identifier_with_numbers() {
    assert_eq!(
        lex("foo123 x1"),
        vec![
            Token::Ident("foo123".to_string()),
            Token::Ident("x1".to_string())
        ]
    );
}

//COMMENTS
#[test]
fn test_line_comment_skipped() {
    assert_eq!(
        lex("var x // this is a comment"),
        vec![Token::Var, Token::Ident("x".to_string())]
    );
}

#[test]
fn test_block_comment_skipped() {
    assert_eq!(
        lex("var /* ignore this */ x"),
        vec![Token::Var, Token::Ident("x".to_string())]
    );
}

#[test]
fn test_comment_only() {
    assert_eq!(lex("// nothing here"), vec![]);
}

#[test]
fn test_block_comments_do_not_nest() {
    // the block-comment regex is non-nesting C-style. It ends at the FIRST
    // '*/' it finds, regardless of any '/*' seen along the way. In
    // "/* a /* b */ c */" the comment is "/* a /* b */" (closed at the
    // first */), leaving " c */" to be lexed as real tokens: Ident("c"),
    // then Star and Slash from the leftover "*/".
    assert_eq!(
        lex("/* a /* b */ c */"),
        vec![Token::Ident("c".to_string()), Token::Star, Token::Slash]
    );
}

//SPANS (line/column tracking)
#[test]
fn test_span_first_token() {
    let tokens = tokenize("var x = 5;").unwrap();
    assert_eq!(tokens[0].span.line, 1);
    assert_eq!(tokens[0].span.column, 1);
}

#[test]
fn test_span_multiline() {
    let tokens = tokenize("var x = 5;\nvar y = 10;").unwrap();
    assert_eq!(tokens[5].span.line, 2);
    assert_eq!(tokens[5].span.column, 1);
}

#[test]
fn test_span_column_after_tab() {
    let tokens = tokenize("\tx").unwrap();
    assert_eq!(tokens[0].span.column, 2);
}

#[test]
fn test_span_survives_crlf_line_endings() {
    let tokens = tokenize("var x = 5;\r\nvar y = 10;").unwrap();
    assert_eq!(tokens[5].span.line, 2);
    assert_eq!(tokens[5].span.column, 1);
}

// STRING INTERPOLATION
#[test]
fn test_interpolated_string_simple() {
    let tokens = lex(r#""Hello ${name}""#);
    match &tokens[0] {
        Token::InterpolatedStringLit(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], StringPart::Literal("Hello ".to_string()));
            assert_eq!(parts[1], StringPart::ExprSource("name".to_string()));
        }
        other => panic!("Expected InterpolatedStringLit, got {:?}", other),
    }
}

#[test]
fn test_interpolated_string_multiple() {
    let tokens = lex(r#""${a} and ${b}""#);
    match &tokens[0] {
        Token::InterpolatedStringLit(parts) => {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], StringPart::ExprSource("a".to_string()));
            assert_eq!(parts[1], StringPart::Literal(" and ".to_string()));
            assert_eq!(parts[2], StringPart::ExprSource("b".to_string()));
        }
        other => panic!("Expected InterpolatedStringLit, got {:?}", other),
    }
}

#[test]
fn test_plain_string_no_dollar() {
    assert_eq!(
        lex(r#""no interpolation""#),
        vec![Token::StringLit("no interpolation".to_string())]
    );
}

#[test]
fn test_interpolation_at_very_start() {
    let tokens = lex(r#""${x}!""#);
    match &tokens[0] {
        Token::InterpolatedStringLit(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], StringPart::ExprSource("x".to_string()));
            assert_eq!(parts[1], StringPart::Literal("!".to_string()));
        }
        other => panic!("Expected InterpolatedStringLit, got {:?}", other),
    }
}

#[test]
fn test_interpolation_at_very_end() {
    let tokens = lex(r#""start${x}""#);
    match &tokens[0] {
        Token::InterpolatedStringLit(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], StringPart::Literal("start".to_string()));
            assert_eq!(parts[1], StringPart::ExprSource("x".to_string()));
        }
        other => panic!("Expected InterpolatedStringLit, got {:?}", other),
    }
}

#[test]
fn test_interpolation_empty_expression() {
    let tokens = lex(r#""${}""#);
    match &tokens[0] {
        Token::InterpolatedStringLit(parts) => {
            assert_eq!(parts, &vec![StringPart::ExprSource("".to_string())]);
        }
        other => panic!("Expected InterpolatedStringLit, got {:?}", other),
    }
}

#[test]
fn test_interpolation_with_method_call() {
    let tokens = lex(r#""Hi ${user.get_name()}""#);
    match &tokens[0] {
        Token::InterpolatedStringLit(parts) => {
            assert_eq!(
                parts[1],
                StringPart::ExprSource("user.get_name()".to_string())
            );
        }
        other => panic!("Expected InterpolatedStringLit, got {:?}", other),
    }
}

#[test]
fn test_interpolation_with_nested_braces() {
    let tokens = lex(r#""${obj.method({})}""#);
    match &tokens[0] {
        Token::InterpolatedStringLit(parts) => {
            assert_eq!(
                parts,
                &vec![StringPart::ExprSource("obj.method({})".to_string())]
            );
        }
        other => panic!("Expected InterpolatedStringLit, got {:?}", other),
    }
}

//ERROR
#[test]
fn test_unrecognized_character_is_an_error() {
    let result = try_lex("var x = $;");
    assert!(result.is_err());
}

#[test]
fn test_unrecognized_character_error_has_correct_position() {
    let result = tokenize("var x = $;");
    match result {
        Err(e) => {
            assert_eq!(e.line, 1);
            assert_eq!(e.column, 9);
        }
        Ok(_) => panic!("Expected a lex error"),
    }
}

#[test]
fn test_unrecognized_character_error_message() {
    let result = tokenize("$");
    match result {
        Err(e) => {
            let formatted = format!("{}", e);
            assert!(formatted.contains("Unrecognized token"));
            assert!(formatted.contains('$'));
        }
        Ok(_) => panic!("Expected a lex error"),
    }
}

#[test]
fn test_unrecognized_character_on_second_line_reports_correct_line() {
    let result = tokenize("var x = 1;\nvar y = $;");
    match result {
        Err(e) => assert_eq!(e.line, 2),
        Ok(_) => panic!("Expected a lex error"),
    }
}

#[test]
fn test_valid_tokens_before_error_do_not_panic() {
    let result = try_lex("var a = 1; var b = 2; var c = $;");
    assert!(result.is_err());
}

//REAL CODE
#[test]
fn test_real_var_decl() {
    assert_eq!(
        lex("var x: Int = 42;"),
        vec![
            Token::Var,
            Token::Ident("x".to_string()),
            Token::Colon,
            Token::Ident("Int".to_string()),
            Token::Assign,
            Token::IntLit(42),
            Token::SemiColon,
        ]
    );
}

#[test]
fn test_real_function_signature() {
    assert_eq!(
        lex("fn add(a: Int, b: Int) -> Int"),
        vec![
            Token::Fn,
            Token::Ident("add".to_string()),
            Token::LParen,
            Token::Ident("a".to_string()),
            Token::Colon,
            Token::Ident("Int".to_string()),
            Token::Comma,
            Token::Ident("b".to_string()),
            Token::Colon,
            Token::Ident("Int".to_string()),
            Token::RParen,
            Token::Arrow,
            Token::Ident("Int".to_string()),
        ]
    );
}

#[test]
fn test_real_error_primitive() {
    assert_eq!(
        lex(r#"Error(#NotFound, "missing")"#),
        vec![
            Token::Error,
            Token::LParen,
            Token::Hash,
            Token::Ident("NotFound".to_string()),
            Token::Comma,
            Token::StringLit("missing".to_string()),
            Token::RParen,
        ]
    );
}

#[test]
fn test_interpolated_string_multiple_segments() {
    let tokens = lex(r#""${a} and ${b}""#);
    match &tokens[0] {
        Token::InterpolatedStringLit(parts) => {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], StringPart::ExprSource("a".to_string()));
            assert_eq!(parts[1], StringPart::Literal(" and ".to_string()));
            assert_eq!(parts[2], StringPart::ExprSource("b".to_string()));
        }
        _ => panic!("Expected InterpolatedStringLit"),
    }
}

#[test]
fn test_interpolated_string_three_segments() {
    let tokens = lex(r#""${x} + ${y} = ${z}""#);
    match &tokens[0] {
        Token::InterpolatedStringLit(parts) => {
            assert_eq!(parts.len(), 5);
            assert_eq!(parts[0], StringPart::ExprSource("x".to_string()));
            assert_eq!(parts[1], StringPart::Literal(" + ".to_string()));
            assert_eq!(parts[2], StringPart::ExprSource("y".to_string()));
            assert_eq!(parts[3], StringPart::Literal(" = ".to_string()));
            assert_eq!(parts[4], StringPart::ExprSource("z".to_string()));
        }
        _ => panic!("Expected InterpolatedStringLit"),
    }
}

#[test]
fn test_unclosed_interpolation_does_not_panic() {
    let result = tokenize(r#""${unclosed""#);
    match result {
        Ok(tokens) => {
            assert!(tokens.len() >= 1);
        }
        Err(_) => {}
    }
}

#[test]
fn test_integer_literal_overflow_is_clear_error() {
    let result = tokenize("99999999999999999999999");
    match result {
        Err(e) => {
            // must mention the literal itself and the max value
            assert!(
                e.message.contains("too large"),
                "Expected 'too large' in message, got: {}",
                e.message
            );
            assert!(
                e.message.contains("9223372036854775807"),
                "Expected max Int value in message, got: {}",
                e.message
            );
        }
        Ok(_) => panic!("Expected error for overflowing integer literal"),
    }
}

#[test]
fn test_max_i64_literal_is_valid() {
    let result = tokenize("9223372036854775807");
    assert!(result.is_ok());
    match &result.unwrap()[0].token {
        Token::IntLit(n) => assert_eq!(*n, i64::MAX),
        _ => panic!("Expected IntLit"),
    }
}
