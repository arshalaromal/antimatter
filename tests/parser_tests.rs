// Parser tests

use antimatter::ast::*;
use antimatter::error::CompilerResult;
use antimatter::lexer::*;
use antimatter::parser::Parser;

/// Parses a full program from source. Panics with the compiler's error
/// message if lexing or parsing fails
fn parse(src: &str) -> Program {
    let tokens = tokenize(src).unwrap_or_else(|e| panic!("{}", e));
    let mut parser = Parser::new(tokens);
    parser.parse().unwrap_or_else(|e| panic!("{}", e))
}

/// Parses a full program and returns the Result, for tests that
/// specifically want to check a lex or parse failure.
fn try_parse(src: &str) -> CompilerResult<Program> {
    let tokens = tokenize(src)?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

/// Parses a single expression by wrapping it in a throwaway function body.
/// eg: parse_expr_src("1 + 2 * 3")
fn parse_expr_src(src: &str) -> Expr {
    let wrapped = format!("fn _test() {{ {}; }}", src);
    let prog = parse(&wrapped);
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::Expression(e) => e.clone(),
            other => panic!("Expected expression statement, got {:?}", other),
        },
        _ => panic!("Expected function"),
    }
}

/// Parses a type by embedding it in a variable declaration's annotation.
/// eg: parse_type_src("[Int]")
fn parse_type_src(ty_src: &str) -> Type {
    let wrapped = format!("fn _t() {{ var x: {}; }}", ty_src);
    let prog = parse(&wrapped);
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::VarDecl { type_annotation, .. } => {
                type_annotation.clone().expect("Expected a type annotation")
            }
            other => panic!("Expected VarDecl, got {:?}", other),
        },
        _ => panic!("Expected function"),
    }
}

// PROGRAM STRUCTURE
#[test]
fn test_empty_program() {
    let prog = parse("");
    assert_eq!(prog.items.len(), 0);
}

#[test]
fn test_empty_function() {
    let prog = parse("fn main() { }");
    match &prog.items[0] {
        Item::Function(f) => {
            assert_eq!(f.name, "main");
            assert_eq!(f.parameters.len(), 0);
            assert!(f.return_type.is_none());
            assert_eq!(f.body.len(), 0);
        }
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_function_with_return_type() {
    let prog = parse("fn add(a: Int, b: Int) -> Int { return a; }");
    match &prog.items[0] {
        Item::Function(f) => {
            assert_eq!(f.parameters.len(), 2);
            assert_eq!(f.parameters[0].name, "a");
            assert_eq!(f.parameters[1].name, "b");
            assert_eq!(f.return_type, Some(Type::Basic("Int".to_string())));
        }
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_pub_function() {
    let prog = parse("pub fn do() { }");
    match &prog.items[0] {
        Item::Function(f) => assert!(f.is_pub),
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_directive_on_function() {
    let prog = parse("@deprecated pub fn old() { }");
    match &prog.items[0] {
        Item::Function(f) => {
            assert_eq!(f.directives, vec!["deprecated".to_string()]);
            assert!(f.is_pub);
        }
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_multiple_directives_order() {
    let prog = parse("@deprecated @experimental pub fn risky() { }");
    match &prog.items[0] {
        Item::Function(f) => {
            assert_eq!(f.directives, vec!["deprecated".to_string(), "experimental".to_string()]);
        }
        _ => panic!("Expected function"),
    }
}

// VARIABLES
#[test]
fn test_var_decl_with_type_and_value() {
    let prog = parse("fn _t() { var x: Int = 42; }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::VarDecl { is_const, name, type_annotation, initializer } => {
                assert!(!is_const);
                assert_eq!(name, "x");
                assert_eq!(*type_annotation, Some(Type::Basic("Int".to_string())));
                assert_eq!(*initializer, Some(Expr::Literal(Literal::Int(42))));
            }
            _ => panic!("Expected VarDecl"),
        },
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_const_decl() {
    let prog = parse("fn _t() { const PI = 3.14; }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::VarDecl { is_const, .. } => assert!(is_const),
            _ => panic!("Expected VarDecl"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_var_decl_type_only_no_initializer() {
    let prog = parse("fn _t() { var x: Int; }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::VarDecl { initializer, .. } => assert!(initializer.is_none()),
            _ => panic!("Expected VarDecl"),
        },
        _ => panic!(),
    }
}

// EXPRESSIONS - PRECEDENCE & ASSOCIATIVITY
#[test]
fn test_precedence_mul_over_add() {
    // 2 + 3 * 4  must parse as  2 + (3 * 4)
    let expr = parse_expr_src("2 + 3 * 4");
    assert_eq!(expr, Expr::Binary {
        left: Box::new(Expr::Literal(Literal::Int(2))),
        operator: Token::Plus,
        right: Box::new(Expr::Binary {
            left: Box::new(Expr::Literal(Literal::Int(3))),
            operator: Token::Star,
            right: Box::new(Expr::Literal(Literal::Int(4))),
        }),
    });
}

#[test]
fn test_left_associativity_subtraction() {
    // 5 - 3 - 1  must parse as  (5 - 3) - 1,  Not  5 - (3 - 1)
    let expr = parse_expr_src("5 - 3 - 1");
    assert_eq!(expr, Expr::Binary {
        left: Box::new(Expr::Binary {
            left: Box::new(Expr::Literal(Literal::Int(5))),
            operator: Token::Minus,
            right: Box::new(Expr::Literal(Literal::Int(3))),
        }),
        operator: Token::Minus,
        right: Box::new(Expr::Literal(Literal::Int(1))),
    });
}

#[test]
fn test_left_associativity_division() {
    // 8 / 4 / 2  must be  (8 / 4) / 2,  not  8 / (4 / 2)
    let expr = parse_expr_src("8 / 4 / 2");
    match expr {
        Expr::Binary { left, operator: Token::Slash, .. } => match *left {
            Expr::Binary { operator: Token::Slash, .. } => {}
            _ => panic!("Division must be left-associative"),
        },
        _ => panic!("Expected binary"),
    }
}

#[test]
fn test_unary_minus() {
    let expr = parse_expr_src("-13");
    assert_eq!(expr, Expr::Unary {
        operator: Token::Minus,
        right: Box::new(Expr::Literal(Literal::Int(13))),
    });
}

#[test]
fn test_unary_not() {
    let expr = parse_expr_src("!active");
    assert_eq!(expr, Expr::Unary {
        operator: Token::Not,
        right: Box::new(Expr::Identifier("active".to_string())),
    });
}

#[test]
fn test_unary_bitnot() {
    let expr = parse_expr_src("~mask");
    assert_eq!(expr, Expr::Unary {
        operator: Token::BitNot,
        right: Box::new(Expr::Identifier("mask".to_string())),
    });
}

#[test]
fn test_grouping_overrides_precedence() {
    let expr = parse_expr_src("(2 + 3) * 4");
    match expr {
        Expr::Binary { operator: Token::Star, left, .. } => match *left {
            Expr::Binary { operator: Token::Plus, .. } => {}
            _ => panic!("Expected addition inside grouping"),
        },
        _ => panic!("Expected multiplication at top"),
    }
}

#[test]
fn test_and_over_or_precedence() {
    // a || b && c  must be  a || (b && c)
    let expr = parse_expr_src("a || b && c");
    match expr {
        Expr::Binary { operator: Token::Or, right, .. } => match *right {
            Expr::Binary { operator: Token::And, .. } => {}
            _ => panic!("Expected && nested under ||"),
        },
        _ => panic!("Expected || at top"),
    }
}

#[test]
fn test_bitand_over_bitor_precedence() {
    // a | b & c  must be  a | (b & c)
    let expr = parse_expr_src("a | b & c");
    match expr {
        Expr::Binary { operator: Token::BitOr, right, .. } => match *right {
            Expr::Binary { operator: Token::BitAnd, .. } => {}
            _ => panic!("Expected & nested under |"),
        },
        _ => panic!("Expected | at top"),
    }
}

#[test]
fn test_bitxor_between_bitor_and_bitand() {
    // a | b ^ c & d  must be  a | (b ^ (c & d))
    let expr = parse_expr_src("a | b ^ c & d");
    match expr {
        Expr::Binary { operator: Token::BitOr, right, .. } => match *right {
            Expr::Binary { operator: Token::BitXor, right: xor_right, .. } => match *xor_right {
                Expr::Binary { operator: Token::BitAnd, .. } => {}
                _ => panic!("Expected & nested under ^"),
            },
            _ => panic!("Expected ^ nested under |"),
        },
        _ => panic!("Expected | at top"),
    }
}

#[test]
fn test_term_over_shift_precedence() {
    // a << 1 + 2  must be  a << (1 + 2)
    let expr = parse_expr_src("a << 1 + 2");
    match expr {
        Expr::Binary { operator: Token::Shl, right, .. } => match *right {
            Expr::Binary { operator: Token::Plus, .. } => {}
            _ => panic!("Expected + nested under <<"),
        },
        _ => panic!("Expected << at top"),
    }
}

#[test]
fn test_comparison_over_equality_precedence() {
    // a == b < c  must be  a == (b < c)
    let expr = parse_expr_src("a == b < c");
    match expr {
        Expr::Binary { operator: Token::EqEq, right, .. } => match *right {
            Expr::Binary { operator: Token::Lt, .. } => {}
            _ => panic!("Expected < nested under =="),
        },
        _ => panic!("Expected == at top"),
    }
}

#[test]
fn test_comparison_left_associative() {
    // a < b < c  parses as  (a < b) < c
    // This is purely a parser associativity check.
    let expr = parse_expr_src("a < b < c");
    match expr {
        Expr::Binary { operator: Token::Lt, left, .. } => match *left {
            Expr::Binary { operator: Token::Lt, .. } => {}
            _ => panic!("Expected left-nested comparison"),
        },
        _ => panic!("Expected comparison at top"),
    }
}

// CONTROL FLOW
#[test]
fn test_if_else() {
    let prog = parse("fn _t() { if x > 0 { return 1; } else { return 0; } }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::If { else_branch, .. } => assert!(else_branch.is_some()),
            _ => panic!("Expected If"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_if_no_else() {
    let prog = parse("fn _t() { if x > 0 { return 1; } }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::If { else_branch, .. } => assert!(else_branch.is_none()),
            _ => panic!("Expected If"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_else_if_chain() {
    let prog = parse("fn _t() { if a { } else if b { } else { } }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::If { else_branch: Some(eb), .. } => match **eb {
                Stmt::If { ref else_branch, .. } => assert!(else_branch.is_some()),
                _ => panic!("Expected nested If for 'else if'"),
            },
            _ => panic!("Expected If with else branch"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_while_loop() {
    let prog = parse("fn _t() { while active { break; } }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::While { .. } => {}
            _ => panic!("Expected While"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_for_loop() {
    let prog = parse("fn _t() { for item in list { continue; } }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::For { item_name, .. } => assert_eq!(item_name, "item"),
            _ => panic!("Expected For"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_braceless_if() {
    let prog = parse("fn _t() { if active return true; }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::If { then_branch, .. } => match **then_branch {
                Stmt::Return(_) => {}
                _ => panic!("Expected braceless Return as then-branch"),
            },
            _ => panic!("Expected If"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_if_condition_not_confused_with_struct_init() {
    let prog = parse("fn _t() { if active { return true; } }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::If { condition, .. } => {
                assert_eq!(*condition, Expr::Identifier("active".to_string()));
            }
            _ => panic!("Expected If"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_while_condition_not_confused_with_struct_init() {
    let prog = parse("fn _t() { while running { break; } }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::While { condition, .. } => {
                assert_eq!(*condition, Expr::Identifier("running".to_string()));
            }
            _ => panic!("Expected While"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_for_iterator_not_confused_with_struct_init() {
    let prog = parse("fn _t() { for x in items { continue; } }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::For { iterator, .. } => {
                assert_eq!(*iterator, Expr::Identifier("items".to_string()));
            }
            _ => panic!("Expected For"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_switch_target_not_confused_with_struct_init() {
    let expr = parse_expr_src("switch status { default -> 0 }");
    match expr {
        Expr::Switch { target, .. } => {
            assert_eq!(*target, Expr::Identifier("status".to_string()));
        }
        _ => panic!("Expected Switch"),
    }
}

#[test]
fn test_struct_init_allowed_inside_parens_in_condition() {
    // Parens re-enable struct init even inside a condition context.
    let prog = parse(r#"fn _t() { if (User { active: true }).active { return true; } }"#);
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::If { condition, .. } => match condition {
                Expr::MemberAccess { object, .. } => match **object {
                    Expr::StructInit { .. } => {}
                    _ => panic!("Expected StructInit inside parens"),
                },
                _ => panic!("Expected MemberAccess"),
            },
            _ => panic!("Expected If"),
        },
        _ => panic!(),
    }
}

// STRUCTS, ENUMS, IMPL, TYPE ALIASES
#[test]
fn test_struct_definition() {
    let prog = parse("struct User { name: String, age: Int, }");
    match &prog.items[0] {
        Item::Struct { name, fields, .. } => {
            assert_eq!(name, "User");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "name");
            assert_eq!(fields[1].name, "age");
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn test_enum_definition() {
    let prog = parse("enum Status { Active, Pending, Closed, }");
    match &prog.items[0] {
        Item::Enum { name, values, .. } => {
            assert_eq!(name, "Status");
            assert_eq!(values, &vec!["Active".to_string(), "Pending".to_string(), "Closed".to_string()]);
        }
        _ => panic!("Expected Enum"),
    }
}

#[test]
fn test_impl_block_single_method() {
    let prog = parse("impl User { pub fn get_name() -> String { return self.name; } }");
    match &prog.items[0] {
        Item::Impl { target, methods } => {
            assert_eq!(target, "User");
            assert_eq!(methods.len(), 1);
            assert_eq!(methods[0].name, "get_name");
            assert!(methods[0].is_pub);
        }
        _ => panic!("Expected Impl"),
    }
}

#[test]
fn test_impl_block_multiple_methods() {
    let prog = parse("impl User { fn a() { } fn b() { } }");
    match &prog.items[0] {
        Item::Impl { methods, .. } => assert_eq!(methods.len(), 2),
        _ => panic!("Expected Impl"),
    }
}

#[test]
fn test_type_alias() {
    let prog = parse("type Port = Int;");
    match &prog.items[0] {
        Item::TypeAlias { name, target_type, .. } => {
            assert_eq!(name, "Port");
            assert_eq!(*target_type, Type::Basic("Int".to_string()));
        }
        _ => panic!("Expected TypeAlias"),
    }
}

#[test]
fn test_pub_type_alias() {
    let prog = parse("pub type Port = Int;");
    match &prog.items[0] {
        Item::TypeAlias { is_pub, .. } => assert!(is_pub),
        _ => panic!("Expected TypeAlias"),
    }
}

// USE STATEMENTS
#[test]
fn test_use_single_item() {
    let prog = parse("use net::http;");
    match &prog.items[0] {
        Item::Use { path, items } => {
            assert_eq!(path, &vec!["net".to_string()]);
            assert_eq!(items, &vec!["http".to_string()]);
        }
        _ => panic!("Expected Use"),
    }
}

#[test]
fn test_use_multi_import() {
    let prog = parse("use net::http::{Request, Response};");
    match &prog.items[0] {
        Item::Use { path, items } => {
            assert_eq!(path, &vec!["net".to_string(), "http".to_string()]);
            assert_eq!(items, &vec!["Request".to_string(), "Response".to_string()]);
        }
        _ => panic!("Expected Use"),
    }
}

#[test]
fn test_use_multi_import_trailing_comma() {
    let prog = parse("use a::{b, c,};");
    match &prog.items[0] {
        Item::Use { items, .. } => assert_eq!(items.len(), 2),
        _ => panic!("Expected Use"),
    }
}

// TYPE PARSER

#[test]
fn test_basic_type() {
    assert_eq!(parse_type_src("Int"), Type::Basic("Int".to_string()));
}

#[test]
fn test_list_type() {
    assert_eq!(parse_type_src("[Int]"), Type::List(Box::new(Type::Basic("Int".to_string()))));
}

#[test]
fn test_nested_list_type() {
    assert_eq!(
        parse_type_src("[[Int]]"),
        Type::List(Box::new(Type::List(Box::new(Type::Basic("Int".to_string())))))
    );
}

#[test]
fn test_map_type() {
    assert_eq!(
        parse_type_src("{String: Int}"),
        Type::Map {
            key: Box::new(Type::Basic("String".to_string())),
            value: Box::new(Type::Basic("Int".to_string())),
        }
    );
}

#[test]
fn test_function_type() {
    assert_eq!(
        parse_type_src("fn(Int, String) -> Bool"),
        Type::Function {
            parameters: vec![Type::Basic("Int".to_string()), Type::Basic("String".to_string())],
            return_type: Box::new(Type::Basic("Bool".to_string())),
        }
    );
}

#[test]
fn test_failable_type() {
    assert_eq!(
        parse_type_src("!Int"),
        Type::Failable(Box::new(Type::Basic("Int".to_string())))
    );
}

#[test]
fn test_failable_list_type() {
    assert_eq!(
        parse_type_src("![Int]"),
        Type::Failable(Box::new(Type::List(Box::new(Type::Basic("Int".to_string())))))
    );
}

// DEFAULT PARAMETER VALUES

#[test]
fn test_default_parameter_value() {
    let prog = parse(r#"fn greet(name: String = "World") { }"#);
    match &prog.items[0] {
        Item::Function(f) => {
            assert_eq!(f.parameters[0].name, "name");
            assert_eq!(
                f.parameters[0].default_value,
                Some(Expr::Literal(Literal::String("World".to_string())))
            );
        }
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_mixed_required_and_default_parameters() {
    let prog = parse("fn connect(host: String, port: Int = 8080) { }");
    match &prog.items[0] {
        Item::Function(f) => {
            assert!(f.parameters[0].default_value.is_none());
            assert!(f.parameters[1].default_value.is_some());
        }
        _ => panic!("Expected function"),
    }
}

// ASSIGNMENT STATEMENTS
#[test]
fn test_simple_assignment_stmt() {
    let prog = parse("fn _t() { x = 5; }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::Assignment { target, operator, value } => {
                assert_eq!(*target, AssignTarget::Identifier("x".to_string()));
                assert_eq!(*operator, Token::Assign);
                assert_eq!(*value, Expr::Literal(Literal::Int(5)));
            }
            _ => panic!("Expected Assignment"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_compound_plus_assignment() {
    let prog = parse("fn _t() { x += 1; }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::Assignment { operator, .. } => assert_eq!(*operator, Token::PlusEq),
            _ => panic!("Expected Assignment"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_compound_bitwise_assignment() {
    let prog = parse("fn _t() { flags &= mask; }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::Assignment { operator, .. } => assert_eq!(*operator, Token::BitAndEq),
            _ => panic!("Expected Assignment"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_assignment_to_member_access() {
    let prog = parse(r#"fn _t() { user.name = "Stark"; }"#);
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::Assignment { target, .. } => match target {
                AssignTarget::MemberAccess { property, .. } => assert_eq!(property, "name"),
                _ => panic!("Expected MemberAccess target"),
            },
            _ => panic!("Expected Assignment"),
        },
        _ => panic!(),
    }
}

#[test]
fn test_assignment_to_index() {
    let prog = parse("fn _t() { list[0] = 9; }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::Assignment { target, .. } => match target {
                AssignTarget::Index { .. } => {}
                _ => panic!("Expected Index target"),
            },
            _ => panic!("Expected Assignment"),
        },
        _ => panic!(),
    }
}

// DEFER
#[test]
fn test_defer_statement() {
    let prog = parse("fn _t() { defer file.close(); }");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::Defer(inner) => match **inner {
                Stmt::Expression(Expr::Call { .. }) => {}
                _ => panic!("Expected deferred call expression"),
            },
            _ => panic!("Expected Defer"),
        },
        _ => panic!(),
    }
}

// STRUCT INIT EXPRESSIONS
#[test]
fn test_struct_init_expression() {
    let expr = parse_expr_src(r#"User { name: "John", age: 522 }"#);
    match expr {
        Expr::StructInit { name, fields } => {
            assert_eq!(*name, Expr::Identifier("User".to_string()));
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].0, "name");
            assert_eq!(fields[1].0, "age");
        }
        _ => panic!("Expected StructInit"),
    }
}

#[test]
fn test_struct_init_trailing_comma() {
    let expr = parse_expr_src("Point { x: 1, y: 2, }");
    match expr {
        Expr::StructInit { fields, .. } => assert_eq!(fields.len(), 2),
        _ => panic!("Expected StructInit"),
    }
}

#[test]
fn test_struct_init_nested() {
    let expr = parse_expr_src("Outer { inner: Inner { value: 5 } }");
    match expr {
        Expr::StructInit { fields, .. } => match &fields[0].1 {
            Expr::StructInit { .. } => {}
            _ => panic!("Expected nested StructInit"),
        },
        _ => panic!("Expected StructInit"),
    }
}

// ERROR HANDLING SYNTAX

#[test]
fn test_or_block_no_binding() {
    let expr = parse_expr_src("divide(a, b) or { yield 0; }");
    match expr {
        Expr::OrFallback { error_binding: None, .. } => {}
        _ => panic!("Expected OrFallback with no binding"),
    }
}

#[test]
fn test_or_block_with_binding() {
    let expr = parse_expr_src("divide(a, b) or (e) { yield 0; }");
    match expr {
        Expr::OrFallback { error_binding: Some(name), .. } => assert_eq!(name, "e"),
        _ => panic!("Expected OrFallback with binding 'e'"),
    }
}

#[test]
fn test_propagate_error() {
    let expr = parse_expr_src("divide(a, b)?");
    match expr {
        Expr::PropagateError(_) => {}
        _ => panic!("Expected PropagateError"),
    }
}

#[test]
fn test_error_primitive() {
    let expr = parse_expr_src(r#"Error(#NotFound, "Item missing")"#);
    match expr {
        Expr::Error { code, message } => {
            assert_eq!(code, "NotFound");
            assert_eq!(*message, Expr::Literal(Literal::String("Item missing".to_string())));
        }
        _ => panic!("Expected Error Type"),
    }
}

#[test]
fn test_error_returned_from_failable_function() {
    let prog = parse(r#"fn f() -> !Int { return Error(#Bad, "bad"); }"#);
    match &prog.items[0] {
        Item::Function(f) => {
            assert_eq!(f.return_type, Some(Type::Failable(Box::new(Type::Basic("Int".to_string())))));
            match &f.body[0] {
                Stmt::Return(Some(Expr::Error { code, .. })) => assert_eq!(code, "Bad"),
                _ => panic!("Expected Return(Error(...))"),
            }
        }
        _ => panic!("Expected function"),
    }
}

// COLLECTIONS
#[test]
fn test_list_literal() {
    let expr = parse_expr_src("[1, 2, 3]");
    match expr {
        Expr::List(items) => assert_eq!(items.len(), 3),
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_list_trailing_comma() {
    let expr = parse_expr_src("[1, 2, 3,]");
    match expr {
        Expr::List(items) => assert_eq!(items.len(), 3),
        _ => panic!("Expected List with trailing comma to work"),
    }
}

#[test]
fn test_map_literal() {
    let expr = parse_expr_src(r#"{"key": 42}"#);
    match expr {
        Expr::Map(pairs) => assert_eq!(pairs.len(), 1),
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_block_not_confused_with_map() {
    // A block starting with a statement keyword must NOT be parsed as a map.
    let expr = parse_expr_src("{ var x = 5; yield x; }");
    match expr {
        Expr::Block(_) => {}
        _ => panic!("Expected Block, not Map"),
    }
}

#[test]
fn test_block_with_semicolon_not_confused_with_map() {
    let expr = parse_expr_src("{ foo(); }");
    match expr {
        Expr::Block(_) => {}
        _ => panic!("Expected Block, not Map"),
    }
}

// SWITCH
#[test]
fn test_switch_expression() {
    let expr = parse_expr_src("switch status { Active -> 1, default -> 0 }");
    match expr {
        Expr::Switch { arms, .. } => assert_eq!(arms.len(), 1),
        _ => panic!("Expected Switch"),
    }
}

// CALLS, MEMBER ACCESS, INDEXING — INCLUDING CHAINED POSTFIX
#[test]
fn test_function_call() {
    let expr = parse_expr_src("foo(1, 2)");
    match expr {
        Expr::Call { arguments, .. } => assert_eq!(arguments.len(), 2),
        _ => panic!("Expected Call"),
    }
}

#[test]
fn test_named_argument() {
    let expr = parse_expr_src(r#"connect(host="localhost", port=8080)"#);
    match expr {
        Expr::Call { arguments, .. } => {
            assert_eq!(arguments[0].0, Some("host".to_string()));
            assert_eq!(arguments[1].0, Some("port".to_string()));
        }
        _ => panic!("Expected Call with named args"),
    }
}

#[test]
fn test_member_access_chain() {
    let expr = parse_expr_src("user.profile.name");
    match expr {
        Expr::MemberAccess { property, object } => {
            assert_eq!(property, "name");
            match *object {
                Expr::MemberAccess { property, .. } => assert_eq!(property, "profile"),
                _ => panic!("Expected nested MemberAccess"),
            }
        }
        _ => panic!("Expected MemberAccess"),
    }
}

#[test]
fn test_index_access() {
    let expr = parse_expr_src("list[0]");
    match expr {
        Expr::Index { .. } => {}
        _ => panic!("Expected Index"),
    }
}

#[test]
fn test_chained_call() {
    // make_adder(5)(3)
    let expr = parse_expr_src("make_adder(5)(3)");
    match expr {
        Expr::Call { callee, .. } => match *callee {
            Expr::Call { .. } => {}
            _ => panic!("Expected nested Call as callee"),
        },
        _ => panic!("Expected outer Call"),
    }
}

#[test]
fn test_chained_index() {
    // grid[0][1]
    let expr = parse_expr_src("grid[0][1]");
    match expr {
        Expr::Index { list, .. } => match *list {
            Expr::Index { .. } => {}
            _ => panic!("Expected nested Index"),
        },
        _ => panic!("Expected outer Index"),
    }
}

#[test]
fn test_propagate_then_member_access() {
    // divide(a, b)?.code
    let expr = parse_expr_src("divide(a, b)?.code");
    match expr {
        Expr::MemberAccess { object, property } => {
            assert_eq!(property, "code");
            match *object {
                Expr::PropagateError(_) => {}
                _ => panic!("Expected PropagateError before member access"),
            }
        }
        _ => panic!("Expected MemberAccess"),
    }
}

// STRING INTERPOLATION — PARSER LEVEL
#[test]
fn test_interpolated_string_to_ast() {
    let expr = parse_expr_src(r#""Hello ${name}!""#);
    match expr {
        Expr::InterpolatedString(segments) => {
            assert_eq!(segments.len(), 3);
            assert_eq!(segments[0], StringSegment::Literal("Hello ".to_string()));
            assert_eq!(segments[1], StringSegment::Expr(Box::new(Expr::Identifier("name".to_string()))));
            assert_eq!(segments[2], StringSegment::Literal("!".to_string()));
        }
        _ => panic!("Expected InterpolatedString"),
    }
}

#[test]
fn test_interpolated_string_with_member_access() {
    let expr = parse_expr_src(r#""User: ${user.name}""#);
    match expr {
        Expr::InterpolatedString(segments) => match &segments[1] {
            StringSegment::Expr(e) => match **e {
                Expr::MemberAccess { ref property, .. } => assert_eq!(property, "name"),
                _ => panic!("Expected MemberAccess inside interpolation"),
            },
            _ => panic!("Expected Expr segment"),
        },
        _ => panic!("Expected InterpolatedString"),
    }
}

#[test]
fn test_plain_string_is_not_interpolated() {
    let expr = parse_expr_src(r#""no interpolation here""#);
    assert_eq!(expr, Expr::Literal(Literal::String("no interpolation here".to_string())));
}

// ERROR
#[test]
fn test_missing_semicolon_is_an_error() {
    let result = try_parse("fn _t() { var x = 5 }");
    assert!(result.is_err());
}

#[test]
fn test_missing_closing_brace_is_an_error() {
    let result = try_parse("fn _t() { var x = 5;");
    assert!(result.is_err());
}

#[test]
fn test_unexpected_token_at_top_level_is_an_error() {
    let result = try_parse("123");
    assert!(result.is_err());
}

#[test]
fn test_missing_default_arm_in_switch_is_an_error() {
    let result = try_parse("fn _t() { switch x { Active -> 1 }; }");
    assert!(result.is_err());
}

#[test]
fn test_or_with_two_bindings_is_an_error() {
    let result = try_parse("fn _t() { divide(a, b) or (x, y) { yield 0; }; }");
    assert!(result.is_err());
}

#[test]
fn test_invalid_assignment_target_is_an_error() {
    // You cannot assign to a literal.
    let result = try_parse("fn _t() { 5 = 10; }");
    assert!(result.is_err());
}

#[test]
fn test_invalid_named_argument_target_is_an_error() {
    // Left side of '=' in a call must be a plain identifier.
    let result = try_parse("fn _t() { foo(1 + 2 = 3); }");
    assert!(result.is_err());
}

#[test]
fn test_error_message_reports_correct_line() {
    let src = "fn _t() {\n    var x = 5\n}";
    let result = try_parse(src);
    match result {
        Err(e) => assert_eq!(e.line, 3),
        Ok(_) => panic!("Expected a parse error"),
    }
}

#[test]
fn test_error_display_format() {
    let result = try_parse("123");
    match result {
        Err(e) => {
            let formatted = format!("{}", e);
            assert!(formatted.starts_with(&format!("{}:{}:", e.line, e.column)));
            assert!(formatted.contains("error:"));
        }
        Ok(_) => panic!("Expected a parse error"),
    }
}

#[test]
fn test_parser_does_not_panic_on_empty_input_after_partial_program() {
    let result = try_parse("fn broken(");
    assert!(result.is_err());
}

#[test]
fn test_lexer_error_propagates_through_try_parse() {
    let result = try_parse("fn _t() { var x = $; }");
    assert!(result.is_err());
}
