// Parser tests

use antimatter::ast::*;
use antimatter::error::CompilerResult;
use antimatter::lexer::*;
use antimatter::parser::Parser;

/// Parses a full program from source. Panics with the compiler's error message
/// if lexing or parsing fails.
fn parse(src: &str) -> Program {
    let tokens = tokenize(src).unwrap_or_else(|e| panic!("Lex error: {}", e));
    let mut parser = Parser::new(tokens);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse error: {}\nSource:\n{}", e, src))
}

/// Parses a full program and returns the Result, used for testing syntax rejection.
fn try_parse(src: &str) -> CompilerResult<Program> {
    let tokens = tokenize(src)?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

/// Parses a single top-level Item from source.
fn parse_single_item(src: &str) -> Item {
    let prog = parse(src);
    assert_eq!(
        prog.items.len(),
        1,
        "Expected exactly 1 item for source: {}",
        src
    );
    prog.items.into_iter().next().unwrap()
}

/// Parses a single statement by wrapping it in a throwaway function body.
fn parse_stmt_src(stmt_src: &str) -> Stmt {
    let wrapped = format!("fn _test() {{ {} }}", stmt_src);
    let prog = parse(&wrapped);
    match &prog.items[0] {
        Item::Function(f) => {
            assert_eq!(
                f.body.len(),
                1,
                "Expected exactly 1 statement in body for: {}",
                stmt_src
            );
            f.body[0].clone()
        }
        _ => panic!("Expected function wrapper"),
    }
}

/// Parses a single expression by wrapping it in a throwaway function body.
fn parse_expr_src(expr_src: &str) -> Expr {
    let wrapped = format!("fn _test() {{ {}; }}", expr_src);
    let prog = parse(&wrapped);
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::Expression(e) => e.clone(),
            other => panic!("Expected expression statement, got {:?}", other),
        },
        _ => panic!("Expected function wrapper"),
    }
}

/// Parses a type expression by embedding it in a variable declaration.
fn parse_type_src(ty_src: &str) -> Type {
    let wrapped = format!("fn _test() {{ var x: {}; }}", ty_src);
    let prog = parse(&wrapped);
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0] {
            Stmt::VarDecl {
                type_annotation, ..
            } => type_annotation
                .clone()
                .expect("Expected type annotation in wrapper"),
            other => panic!("Expected VarDecl, got {:?}", other),
        },
        _ => panic!("Expected function wrapper"),
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
    assert_eq!(
        parse_single_item("fn main() { }"),
        Item::Function(FunctionDecl {
            directives: vec![],
            is_pub: false,
            name: "main".to_string(),
            parameters: vec![],
            return_type: None,
            body: vec![],
        })
    );
}

#[test]
fn test_pub_function() {
    assert_eq!(
        parse_single_item("pub fn run() { }"),
        Item::Function(FunctionDecl {
            directives: vec![],
            is_pub: true,
            name: "run".to_string(),
            parameters: vec![],
            return_type: None,
            body: vec![],
        })
    );
}

#[test]
fn test_function_with_parameters_and_return_type() {
    assert_eq!(
        parse_single_item("fn add(a: Int, b: Int) -> Int { return a; }"),
        Item::Function(FunctionDecl {
            directives: vec![],
            is_pub: false,
            name: "add".to_string(),
            parameters: vec![
                Parameter {
                    name: "a".to_string(),
                    type_name: Type::Basic("Int".to_string()),
                    default_value: None,
                },
                Parameter {
                    name: "b".to_string(),
                    type_name: Type::Basic("Int".to_string()),
                    default_value: None,
                },
            ],
            return_type: Some(Type::Basic("Int".to_string())),
            body: vec![Stmt::Return(Some(Expr::Identifier("a".to_string())))],
        })
    );
}

#[test]
fn test_function_parameters_trailing_comma() {
    let item = parse_single_item("fn test(x: Int, y: Float,) { }");
    match item {
        Item::Function(f) => {
            assert_eq!(f.parameters.len(), 2);
            assert_eq!(f.parameters[0].name, "x");
            assert_eq!(f.parameters[1].name, "y");
        }
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_function_parameter_default_values() {
    assert_eq!(
        parse_single_item(r#"fn greet(name: String = "World") { }"#),
        Item::Function(FunctionDecl {
            directives: vec![],
            is_pub: false,
            name: "greet".to_string(),
            parameters: vec![Parameter {
                name: "name".to_string(),
                type_name: Type::Basic("String".to_string()),
                default_value: Some(Expr::Literal(Literal::String("World".to_string()))),
            }],
            return_type: None,
            body: vec![],
        })
    );
}

#[test]
fn test_function_mixed_required_and_default_parameters() {
    let item = parse_single_item("fn connect(host: String, port: Int = 8080) { }");
    match item {
        Item::Function(f) => {
            assert!(f.parameters[0].default_value.is_none());
            assert_eq!(
                f.parameters[1].default_value,
                Some(Expr::Literal(Literal::Int(8080)))
            );
        }
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_function_single_directive() {
    let item = parse_single_item("@deprecated pub fn old() { }");
    match item {
        Item::Function(f) => {
            assert_eq!(f.directives, vec!["deprecated".to_string()]);
            assert!(f.is_pub);
        }
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_function_multiple_directives_order() {
    let item = parse_single_item("@deprecated @experimental pub fn risky() { }");
    match item {
        Item::Function(f) => {
            assert_eq!(
                f.directives,
                vec!["deprecated".to_string(), "experimental".to_string()]
            );
            assert!(f.is_pub);
        }
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_extern_function_no_body() {
    assert_eq!(
        parse_single_item("@extern fn sin(x: Float) -> Float;"),
        Item::Function(FunctionDecl {
            directives: vec!["extern".to_string()],
            is_pub: false,
            name: "sin".to_string(),
            parameters: vec![Parameter {
                name: "x".to_string(),
                type_name: Type::Basic("Float".to_string()),
                default_value: None,
            }],
            return_type: Some(Type::Basic("Float".to_string())),
            body: vec![],
        })
    );
}

#[test]
fn test_extern_pub_function() {
    let item = parse_single_item("@extern pub fn malloc(size: Int) -> Int;");
    match item {
        Item::Function(f) => {
            assert!(f.directives.contains(&"extern".to_string()));
            assert!(f.is_pub);
            assert!(f.body.is_empty());
        }
        _ => panic!("Expected extern function"),
    }
}

#[test]
fn test_extern_function_no_params() {
    let item = parse_single_item("@extern fn get_time() -> Int;");
    match item {
        Item::Function(f) => {
            assert_eq!(f.parameters.len(), 0);
            assert!(f.body.is_empty());
        }
        _ => panic!("Expected extern function"),
    }
}

#[test]
fn test_extern_function_multiple_params() {
    let item = parse_single_item("@extern fn memcpy(dst: Int, src: Int, n: Int) -> Int;");
    match item {
        Item::Function(f) => {
            assert_eq!(f.parameters.len(), 3);
            assert!(f.body.is_empty());
        }
        _ => panic!("Expected extern function"),
    }
}

// Rejections on function syntax
#[test]
fn test_extern_function_with_body_is_rejected() {
    // @extern requires ';' and rejects a '{' body
    let result = try_parse("@extern fn sin(x: Float) -> Float { return x; }");
    assert!(result.is_err());
}

#[test]
fn test_bodyless_function_without_extern_is_rejected() {
    // Normal fn requires a '{' body
    let result = try_parse("fn missing_body(x: Int) -> Int;");
    assert!(result.is_err());
}

#[test]
fn test_pub_before_directive_is_rejected() {
    // Modifiers rule: directives come first, then 'pub'
    let result = try_parse("pub @deprecated fn bad_order() { }");
    assert!(result.is_err());
}

// STRUCTS, ENUMS, IMPL, TYPE ALIASES

#[test]
fn test_struct_definition() {
    assert_eq!(
        parse_single_item("struct User { name: String, age: Int, }"),
        Item::Struct {
            directives: vec![],
            is_pub: false,
            name: "User".to_string(),
            fields: vec![
                StructField {
                    directives: vec![],
                    is_pub: false,
                    name: "name".to_string(),
                    type_name: Type::Basic("String".to_string()),
                },
                StructField {
                    directives: vec![],
                    is_pub: false,
                    name: "age".to_string(),
                    type_name: Type::Basic("Int".to_string()),
                },
            ],
        }
    );
}

#[test]
fn test_struct_with_pub_and_weak_fields() {
    assert_eq!(
        parse_single_item("pub struct Node { pub name: String, @weak parent: Node }"),
        Item::Struct {
            directives: vec![],
            is_pub: true,
            name: "Node".to_string(),
            fields: vec![
                StructField {
                    directives: vec![],
                    is_pub: true,
                    name: "name".to_string(),
                    type_name: Type::Basic("String".to_string()),
                },
                StructField {
                    directives: vec!["weak".to_string()],
                    is_pub: false,
                    name: "parent".to_string(),
                    type_name: Type::Basic("Node".to_string()),
                },
            ],
        }
    );
}

#[test]
fn test_empty_struct() {
    assert_eq!(
        parse_single_item("struct Empty { }"),
        Item::Struct {
            directives: vec![],
            is_pub: false,
            name: "Empty".to_string(),
            fields: vec![],
        }
    );
}

#[test]
fn test_enum_definition() {
    assert_eq!(
        parse_single_item("enum Status { Active, Pending, Closed, }"),
        Item::Enum {
            directives: vec![],
            is_pub: false,
            name: "Status".to_string(),
            values: vec![
                "Active".to_string(),
                "Pending".to_string(),
                "Closed".to_string()
            ],
        }
    );
}

#[test]
fn test_pub_enum_without_trailing_comma() {
    assert_eq!(
        parse_single_item("pub enum Direction { North, South, East, West }"),
        Item::Enum {
            directives: vec![],
            is_pub: true,
            name: "Direction".to_string(),
            values: vec![
                "North".to_string(),
                "South".to_string(),
                "East".to_string(),
                "West".to_string()
            ],
        }
    );
}

#[test]
fn test_empty_enum() {
    assert_eq!(
        parse_single_item("enum Empty { }"),
        Item::Enum {
            directives: vec![],
            is_pub: false,
            name: "Empty".to_string(),
            values: vec![],
        }
    );
}

#[test]
fn test_impl_block_single_method() {
    assert_eq!(
        parse_single_item("impl User { pub fn get_name() -> String { return self.name; } }"),
        Item::Impl {
            target: "User".to_string(),
            methods: vec![FunctionDecl {
                directives: vec![],
                is_pub: true,
                name: "get_name".to_string(),
                parameters: vec![],
                return_type: Some(Type::Basic("String".to_string())),
                body: vec![Stmt::Return(Some(Expr::MemberAccess {
                    object: Box::new(Expr::SelfKw),
                    property: "name".to_string(),
                }))],
            }],
        }
    );
}

#[test]
fn test_impl_block_multiple_methods() {
    let item = parse_single_item("impl User { fn a() { } fn b() { } }");
    match item {
        Item::Impl { target, methods } => {
            assert_eq!(target, "User");
            assert_eq!(methods.len(), 2);
            assert_eq!(methods[0].name, "a");
            assert_eq!(methods[1].name, "b");
        }
        _ => panic!("Expected Impl"),
    }
}

#[test]
fn test_pub_impl_is_rejected() {
    // 'impl' cannot take 'pub' or directives
    let result = try_parse("pub impl User { fn a() {} }");
    assert!(result.is_err());
}

#[test]
fn test_non_fn_inside_impl_is_rejected() {
    let result = try_parse("impl User { var x = 5; }");
    assert!(result.is_err());
}

#[test]
fn test_type_alias() {
    assert_eq!(
        parse_single_item("type Port = Int;"),
        Item::TypeAlias {
            directives: vec![],
            is_pub: false,
            name: "Port".to_string(),
            target_type: Type::Basic("Int".to_string()),
        }
    );
}

#[test]
fn test_pub_type_alias_with_composite_type() {
    assert_eq!(
        parse_single_item("pub type LookupTable = {String: [Int]};"),
        Item::TypeAlias {
            directives: vec![],
            is_pub: true,
            name: "LookupTable".to_string(),
            target_type: Type::Map {
                key: Box::new(Type::Basic("String".to_string())),
                value: Box::new(Type::List(Box::new(Type::Basic("Int".to_string())))),
            },
        }
    );
}

// USE STATEMENTS

#[test]
fn test_use_single_stdlib_module() {
    // "use io;" -> path: [], items: ["io"]
    assert_eq!(
        parse_single_item("use io;"),
        Item::Use {
            path: vec![],
            items: vec!["io".to_string()],
        }
    );
}

#[test]
fn test_use_single_path_item() {
    // "use net::http;" -> path: ["net"], items: ["http"]
    assert_eq!(
        parse_single_item("use net::http;"),
        Item::Use {
            path: vec!["net".to_string()],
            items: vec!["http".to_string()],
        }
    );
}

#[test]
fn test_use_multi_import() {
    // "use net::http::{Request, Response};" -> path: ["net", "http"], items: ["Request", "Response"]
    assert_eq!(
        parse_single_item("use net::http::{Request, Response};"),
        Item::Use {
            path: vec!["net".to_string(), "http".to_string()],
            items: vec!["Request".to_string(), "Response".to_string()],
        }
    );
}

#[test]
fn test_use_multi_import_trailing_comma() {
    assert_eq!(
        parse_single_item("use a::{b, c,};"),
        Item::Use {
            path: vec!["a".to_string()],
            items: vec!["b".to_string(), "c".to_string()],
        }
    );
}

// TYPE PARSER

#[test]
fn test_basic_types() {
    assert_eq!(parse_type_src("Int"), Type::Basic("Int".to_string()));
    assert_eq!(parse_type_src("Float"), Type::Basic("Float".to_string()));
    assert_eq!(parse_type_src("Bool"), Type::Basic("Bool".to_string()));
    assert_eq!(parse_type_src("String"), Type::Basic("String".to_string()));
    assert_eq!(
        parse_type_src("CustomType"),
        Type::Basic("CustomType".to_string())
    );
}

#[test]
fn test_dedicated_void_and_ecode_types() {
    assert_eq!(parse_type_src("Void"), Type::Void);
    assert_eq!(parse_type_src("ECode"), Type::ECode);
}

#[test]
fn test_list_type() {
    assert_eq!(
        parse_type_src("[Int]"),
        Type::List(Box::new(Type::Basic("Int".to_string())))
    );
}

#[test]
fn test_nested_list_type() {
    assert_eq!(
        parse_type_src("[[Int]]"),
        Type::List(Box::new(Type::List(Box::new(Type::Basic(
            "Int".to_string()
        )))))
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
            parameters: vec![
                Type::Basic("Int".to_string()),
                Type::Basic("String".to_string()),
            ],
            return_type: Box::new(Type::Basic("Bool".to_string())),
        }
    );
}

#[test]
fn test_function_type_void_return() {
    assert_eq!(
        parse_type_src("fn() -> Void"),
        Type::Function {
            parameters: vec![],
            return_type: Box::new(Type::Void),
        }
    );
}

#[test]
fn test_failable_types() {
    assert_eq!(
        parse_type_src("!Int"),
        Type::Failable(Box::new(Type::Basic("Int".to_string())))
    );
    assert_eq!(
        parse_type_src("![Int]"),
        Type::Failable(Box::new(Type::List(Box::new(Type::Basic(
            "Int".to_string()
        )))))
    );
    assert_eq!(
        parse_type_src("!{String: Int}"),
        Type::Failable(Box::new(Type::Map {
            key: Box::new(Type::Basic("String".to_string())),
            value: Box::new(Type::Basic("Int".to_string())),
        }))
    );
}

#[test]
fn test_double_failable_is_rejected() {
    assert!(try_parse("fn _t() { var x: !!Int; }").is_err());
    assert!(try_parse("fn _t() { var x: !![Int]; }").is_err());
    assert!(try_parse("fn f() -> !!Int { }").is_err());
}

#[test]
fn test_function_type_without_return_arrow_is_rejected() {
    // Function types require '-> ReturnType'
    assert!(try_parse("fn _t() { var f: fn(Int); }").is_err());
}

// VARIABLES

#[test]
fn test_var_decl_with_type_and_value() {
    assert_eq!(
        parse_stmt_src("var x: Int = 42;"),
        Stmt::VarDecl {
            is_const: false,
            name: "x".to_string(),
            type_annotation: Some(Type::Basic("Int".to_string())),
            initializer: Some(Expr::Literal(Literal::Int(42))),
        }
    );
}

#[test]
fn test_var_decl_type_inferred() {
    assert_eq!(
        parse_stmt_src("var y = 5;"),
        Stmt::VarDecl {
            is_const: false,
            name: "y".to_string(),
            type_annotation: None,
            initializer: Some(Expr::Literal(Literal::Int(5))),
        }
    );
}

#[test]
fn test_var_decl_type_only_no_initializer() {
    assert_eq!(
        parse_stmt_src("var z: Int;"),
        Stmt::VarDecl {
            is_const: false,
            name: "z".to_string(),
            type_annotation: Some(Type::Basic("Int".to_string())),
            initializer: None,
        }
    );
}

#[test]
fn test_var_decl_bare_no_type_no_initializer() {
    // Parses in grammar; semantic checker later catches/rejects it
    assert_eq!(
        parse_stmt_src("var bare;"),
        Stmt::VarDecl {
            is_const: false,
            name: "bare".to_string(),
            type_annotation: None,
            initializer: None,
        }
    );
}

#[test]
fn test_const_decl() {
    assert_eq!(
        parse_stmt_src("const PI = 3.14;"),
        Stmt::VarDecl {
            is_const: true,
            name: "PI".to_string(),
            type_annotation: None,
            initializer: Some(Expr::Literal(Literal::Float(3.14))),
        }
    );
}

// ASSIGNMENT STATEMENTS
#[test]
fn test_assignment_to_identifier() {
    assert_eq!(
        parse_stmt_src("x = 5;"),
        Stmt::Assignment {
            target: AssignTarget::Identifier("x".to_string()),
            operator: Token::Assign,
            value: Expr::Literal(Literal::Int(5)),
        }
    );
}

#[test]
fn test_assignment_to_member_access() {
    assert_eq!(
        parse_stmt_src(r#"user.name = "Stark";"#),
        Stmt::Assignment {
            target: AssignTarget::MemberAccess {
                object: Box::new(Expr::Identifier("user".to_string())),
                property: "name".to_string(),
            },
            operator: Token::Assign,
            value: Expr::Literal(Literal::String("Stark".to_string())),
        }
    );
}

#[test]
fn test_assignment_to_nested_member_access() {
    assert_eq!(
        parse_stmt_src("a.b.c = 10;"),
        Stmt::Assignment {
            target: AssignTarget::MemberAccess {
                object: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Identifier("a".to_string())),
                    property: "b".to_string(),
                }),
                property: "c".to_string(),
            },
            operator: Token::Assign,
            value: Expr::Literal(Literal::Int(10)),
        }
    );
}

#[test]
fn test_assignment_to_index() {
    assert_eq!(
        parse_stmt_src("list[0] = 9;"),
        Stmt::Assignment {
            target: AssignTarget::Index {
                list: Box::new(Expr::Identifier("list".to_string())),
                index: Box::new(Expr::Literal(Literal::Int(0))),
            },
            operator: Token::Assign,
            value: Expr::Literal(Literal::Int(9)),
        }
    );
}

#[test]
fn test_compound_assignments() {
    assert_eq!(
        parse_stmt_src("x += 1;"),
        Stmt::Assignment {
            target: AssignTarget::Identifier("x".to_string()),
            operator: Token::PlusEq,
            value: Expr::Literal(Literal::Int(1)),
        }
    );
    assert_eq!(
        parse_stmt_src("flags &= mask;"),
        Stmt::Assignment {
            target: AssignTarget::Identifier("flags".to_string()),
            operator: Token::BitAndEq,
            value: Expr::Identifier("mask".to_string()),
        }
    );
    assert_eq!(
        parse_stmt_src("val <<= 2;"),
        Stmt::Assignment {
            target: AssignTarget::Identifier("val".to_string()),
            operator: Token::ShlEq,
            value: Expr::Literal(Literal::Int(2)),
        }
    );
}

#[test]
fn test_invalid_assignment_target_is_rejected() {
    assert!(try_parse("fn _t() { 5 = 10; }").is_err());
    assert!(try_parse("fn _t() { (a + b) = 10; }").is_err());
}

// CONTROL FLOW

#[test]
fn test_if_no_else() {
    assert_eq!(
        parse_stmt_src("if x > 0 { return 1; }"),
        Stmt::If {
            condition: Expr::Binary {
                left: Box::new(Expr::Identifier("x".to_string())),
                operator: Token::Gt,
                right: Box::new(Expr::Literal(Literal::Int(0))),
            },
            then_branch: Box::new(Stmt::Expression(Expr::Block(vec![Stmt::Return(Some(
                Expr::Literal(Literal::Int(1))
            ))]))),
            else_branch: None,
        }
    );
}

#[test]
fn test_if_else() {
    let stmt = parse_stmt_src("if x > 0 { return 1; } else { return 0; }");
    match stmt {
        Stmt::If { else_branch, .. } => assert!(else_branch.is_some()),
        _ => panic!("Expected If"),
    }
}

#[test]
fn test_else_if_chain() {
    let stmt = parse_stmt_src("if a { } else if b { } else { }");
    match stmt {
        Stmt::If { else_branch, .. } => match *else_branch.expect("Expected else branch") {
            Stmt::If {
                else_branch: inner_else,
                ..
            } => {
                assert!(inner_else.is_some());
            }
            _ => panic!("Expected nested If for 'else if'"),
        },
        _ => panic!("Expected If"),
    }
}

#[test]
fn test_braceless_if() {
    assert_eq!(
        parse_stmt_src("if active return true;"),
        Stmt::If {
            condition: Expr::Identifier("active".to_string()),
            then_branch: Box::new(Stmt::Return(Some(Expr::Literal(Literal::Bool(true))))),
            else_branch: None,
        }
    );
}

#[test]
fn test_while_loop() {
    assert_eq!(
        parse_stmt_src("while active { break; }"),
        Stmt::While {
            condition: Expr::Identifier("active".to_string()),
            body: Box::new(Stmt::Expression(Expr::Block(vec![Stmt::Break]))),
        }
    );
}

#[test]
fn test_braceless_while() {
    assert_eq!(
        parse_stmt_src("while active break;"),
        Stmt::While {
            condition: Expr::Identifier("active".to_string()),
            body: Box::new(Stmt::Break),
        }
    );
}

#[test]
fn test_for_loop() {
    assert_eq!(
        parse_stmt_src("for item in list { continue; }"),
        Stmt::For {
            item_name: "item".to_string(),
            iterator: Expr::Identifier("list".to_string()),
            body: Box::new(Stmt::Expression(Expr::Block(vec![Stmt::Continue]))),
        }
    );
}

#[test]
fn test_for_loop_with_discard_variable() {
    assert_eq!(
        parse_stmt_src("for _ in list { continue; }"),
        Stmt::For {
            item_name: "_".to_string(),
            iterator: Expr::Identifier("list".to_string()),
            body: Box::new(Stmt::Expression(Expr::Block(vec![Stmt::Continue]))),
        }
    );
}

#[test]
fn test_condition_not_confused_with_struct_init() {
    // Heads of if/while/for/switch parse expressions with allow_struct = false
    assert_eq!(
        parse_stmt_src("if active { return true; }"),
        Stmt::If {
            condition: Expr::Identifier("active".to_string()),
            then_branch: Box::new(Stmt::Expression(Expr::Block(vec![Stmt::Return(Some(
                Expr::Literal(Literal::Bool(true))
            ))]))),
            else_branch: None,
        }
    );

    assert_eq!(
        parse_stmt_src("while running { break; }"),
        Stmt::While {
            condition: Expr::Identifier("running".to_string()),
            body: Box::new(Stmt::Expression(Expr::Block(vec![Stmt::Break]))),
        }
    );

    assert_eq!(
        parse_stmt_src("for x in items { continue; }"),
        Stmt::For {
            item_name: "x".to_string(),
            iterator: Expr::Identifier("items".to_string()),
            body: Box::new(Stmt::Expression(Expr::Block(vec![Stmt::Continue]))),
        }
    );
}

#[test]
fn test_struct_init_allowed_inside_parens_in_condition() {
    // Parentheses re-enable struct init in conditional contexts
    let stmt = parse_stmt_src(r#"if (User { active: true }).active { return true; }"#);
    match stmt {
        Stmt::If { condition, .. } => match condition {
            Expr::MemberAccess { object, .. } => match *object {
                Expr::StructInit { name, fields } => {
                    assert_eq!(name, "User");
                    assert_eq!(fields.len(), 1);
                }
                _ => panic!("Expected StructInit inside parens"),
            },
            _ => panic!("Expected MemberAccess"),
        },
        _ => panic!("Expected If"),
    }
}

#[test]
fn test_break_and_continue_statements() {
    assert_eq!(parse_stmt_src("break;"), Stmt::Break);
    assert_eq!(parse_stmt_src("continue;"), Stmt::Continue);
}

#[test]
fn test_return_statements() {
    assert_eq!(parse_stmt_src("return;"), Stmt::Return(None));
    assert_eq!(
        parse_stmt_src("return 42;"),
        Stmt::Return(Some(Expr::Literal(Literal::Int(42))))
    );
}

#[test]
fn test_yield_statement() {
    assert_eq!(
        parse_stmt_src("yield x * 2;"),
        Stmt::Yield(Expr::Binary {
            left: Box::new(Expr::Identifier("x".to_string())),
            operator: Token::Star,
            right: Box::new(Expr::Literal(Literal::Int(2))),
        })
    );
}

#[test]
fn test_defer_statement() {
    assert_eq!(
        parse_stmt_src("defer file.close();"),
        Stmt::Defer(Box::new(Stmt::Expression(Expr::Call {
            callee: Box::new(Expr::MemberAccess {
                object: Box::new(Expr::Identifier("file".to_string())),
                property: "close".to_string(),
            }),
            arguments: vec![],
        })))
    );
}

// LITERALS & IDENTIFIERS

#[test]
fn test_literals() {
    assert_eq!(parse_expr_src("42"), Expr::Literal(Literal::Int(42)));
    assert_eq!(parse_expr_src("3.14"), Expr::Literal(Literal::Float(3.14)));
    assert_eq!(
        parse_expr_src(r#""hello""#),
        Expr::Literal(Literal::String("hello".to_string()))
    );
    assert_eq!(
        parse_expr_src("`raw string`"),
        Expr::Literal(Literal::String("raw string".to_string()))
    );
    assert_eq!(parse_expr_src("true"), Expr::Literal(Literal::Bool(true)));
    assert_eq!(parse_expr_src("false"), Expr::Literal(Literal::Bool(false)));
}

#[test]
fn test_identifiers_and_self() {
    assert_eq!(
        parse_expr_src("my_var"),
        Expr::Identifier("my_var".to_string())
    );
    assert_eq!(parse_expr_src("_"), Expr::Identifier("_".to_string()));
    assert_eq!(parse_expr_src("self"), Expr::SelfKw);
}

#[test]
fn test_ecode_literal_standalone() {
    assert_eq!(
        parse_expr_src("#NotFound"),
        Expr::ECodeLit("NotFound".to_string())
    );
}

#[test]
fn test_ecode_literal_in_comparison() {
    assert_eq!(
        parse_expr_src("e.code == #NotFound"),
        Expr::Binary {
            left: Box::new(Expr::MemberAccess {
                object: Box::new(Expr::Identifier("e".to_string())),
                property: "code".to_string(),
            }),
            operator: Token::EqEq,
            right: Box::new(Expr::ECodeLit("NotFound".to_string())),
        }
    );
}

#[test]
fn test_enum_variant_standalone() {
    assert_eq!(
        parse_expr_src("Status::Active"),
        Expr::EnumVariant {
            enum_name: "Status".to_string(),
            variant: "Active".to_string(),
        }
    );
}

#[test]
fn test_enum_variant_in_comparison() {
    assert_eq!(
        parse_expr_src("Status::Active == Status::Pending"),
        Expr::Binary {
            left: Box::new(Expr::EnumVariant {
                enum_name: "Status".to_string(),
                variant: "Active".to_string(),
            }),
            operator: Token::EqEq,
            right: Box::new(Expr::EnumVariant {
                enum_name: "Status".to_string(),
                variant: "Pending".to_string(),
            }),
        }
    );
}

#[test]
fn test_double_colon_left_must_be_ident() {
    assert!(try_parse("fn _t() { var x = 5::Active; }").is_err());
    assert!(try_parse("fn _t() { var x = get_status()::Active; }").is_err());
}

// EXPRESSIONS - PRECEDENCE & ASSOCIATIVITY

#[test]
fn test_unary_operators() {
    assert_eq!(
        parse_expr_src("-13"),
        Expr::Unary {
            operator: Token::Minus,
            right: Box::new(Expr::Literal(Literal::Int(13))),
        }
    );
    assert_eq!(
        parse_expr_src("!active"),
        Expr::Unary {
            operator: Token::Not,
            right: Box::new(Expr::Identifier("active".to_string())),
        }
    );
    assert_eq!(
        parse_expr_src("~mask"),
        Expr::Unary {
            operator: Token::BitNot,
            right: Box::new(Expr::Identifier("mask".to_string())),
        }
    );
}

#[test]
fn test_grouping_overrides_precedence() {
    assert_eq!(
        parse_expr_src("(2 + 3) * 4"),
        Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Literal::Int(2))),
                operator: Token::Plus,
                right: Box::new(Expr::Literal(Literal::Int(3))),
            }),
            operator: Token::Star,
            right: Box::new(Expr::Literal(Literal::Int(4))),
        }
    );
}

#[test]
fn test_precedence_factor_over_term() {
    // 2 + 3 * 4 -> 2 + (3 * 4)
    assert_eq!(
        parse_expr_src("2 + 3 * 4"),
        Expr::Binary {
            left: Box::new(Expr::Literal(Literal::Int(2))),
            operator: Token::Plus,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Literal::Int(3))),
                operator: Token::Star,
                right: Box::new(Expr::Literal(Literal::Int(4))),
            }),
        }
    );

    // 10 - 8 / 2 -> 10 - (8 / 2)
    assert_eq!(
        parse_expr_src("10 - 8 / 2"),
        Expr::Binary {
            left: Box::new(Expr::Literal(Literal::Int(10))),
            operator: Token::Minus,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Literal::Int(8))),
                operator: Token::Slash,
                right: Box::new(Expr::Literal(Literal::Int(2))),
            }),
        }
    );

    // 5 + 7 % 3 -> 5 + (7 % 3)
    assert_eq!(
        parse_expr_src("5 + 7 % 3"),
        Expr::Binary {
            left: Box::new(Expr::Literal(Literal::Int(5))),
            operator: Token::Plus,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Literal::Int(7))),
                operator: Token::Percent,
                right: Box::new(Expr::Literal(Literal::Int(3))),
            }),
        }
    );
}

#[test]
fn test_precedence_term_over_shift() {
    // a << 1 + 2 -> a << (1 + 2)
    assert_eq!(
        parse_expr_src("a << 1 + 2"),
        Expr::Binary {
            left: Box::new(Expr::Identifier("a".to_string())),
            operator: Token::Shl,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Literal::Int(1))),
                operator: Token::Plus,
                right: Box::new(Expr::Literal(Literal::Int(2))),
            }),
        }
    );
}

#[test]
fn test_precedence_shift_over_comparison() {
    // a < b << 1 -> a < (b << 1)
    assert_eq!(
        parse_expr_src("a < b << 1"),
        Expr::Binary {
            left: Box::new(Expr::Identifier("a".to_string())),
            operator: Token::Lt,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier("b".to_string())),
                operator: Token::Shl,
                right: Box::new(Expr::Literal(Literal::Int(1))),
            }),
        }
    );
}

#[test]
fn test_precedence_comparison_over_equality() {
    // a == b < c -> a == (b < c)
    assert_eq!(
        parse_expr_src("a == b < c"),
        Expr::Binary {
            left: Box::new(Expr::Identifier("a".to_string())),
            operator: Token::EqEq,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier("b".to_string())),
                operator: Token::Lt,
                right: Box::new(Expr::Identifier("c".to_string())),
            }),
        }
    );
}

#[test]
fn test_precedence_equality_over_bitand() {
    // a & b == c -> a & (b == c)
    assert_eq!(
        parse_expr_src("a & b == c"),
        Expr::Binary {
            left: Box::new(Expr::Identifier("a".to_string())),
            operator: Token::BitAnd,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier("b".to_string())),
                operator: Token::EqEq,
                right: Box::new(Expr::Identifier("c".to_string())),
            }),
        }
    );
}

#[test]
fn test_precedence_bitand_over_bitxor() {
    // a ^ b & c -> a ^ (b & c)
    assert_eq!(
        parse_expr_src("a ^ b & c"),
        Expr::Binary {
            left: Box::new(Expr::Identifier("a".to_string())),
            operator: Token::BitXor,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier("b".to_string())),
                operator: Token::BitAnd,
                right: Box::new(Expr::Identifier("c".to_string())),
            }),
        }
    );
}

#[test]
fn test_precedence_bitxor_over_bitor() {
    // a | b ^ c -> a | (b ^ c)
    assert_eq!(
        parse_expr_src("a | b ^ c"),
        Expr::Binary {
            left: Box::new(Expr::Identifier("a".to_string())),
            operator: Token::BitOr,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier("b".to_string())),
                operator: Token::BitXor,
                right: Box::new(Expr::Identifier("c".to_string())),
            }),
        }
    );
}

#[test]
fn test_precedence_bitor_over_and() {
    // a && b | c -> a && (b | c)
    assert_eq!(
        parse_expr_src("a && b | c"),
        Expr::Binary {
            left: Box::new(Expr::Identifier("a".to_string())),
            operator: Token::And,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier("b".to_string())),
                operator: Token::BitOr,
                right: Box::new(Expr::Identifier("c".to_string())),
            }),
        }
    );
}

#[test]
fn test_precedence_and_over_or() {
    // a || b && c -> a || (b && c)
    assert_eq!(
        parse_expr_src("a || b && c"),
        Expr::Binary {
            left: Box::new(Expr::Identifier("a".to_string())),
            operator: Token::Or,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier("b".to_string())),
                operator: Token::And,
                right: Box::new(Expr::Identifier("c".to_string())),
            }),
        }
    );
}

#[test]
fn test_left_associativity_arithmetic() {
    // 5 - 3 - 1 -> (5 - 3) - 1
    assert_eq!(
        parse_expr_src("5 - 3 - 1"),
        Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Literal::Int(5))),
                operator: Token::Minus,
                right: Box::new(Expr::Literal(Literal::Int(3))),
            }),
            operator: Token::Minus,
            right: Box::new(Expr::Literal(Literal::Int(1))),
        }
    );

    // 8 / 4 / 2 -> (8 / 4) / 2
    assert_eq!(
        parse_expr_src("8 / 4 / 2"),
        Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Literal::Int(8))),
                operator: Token::Slash,
                right: Box::new(Expr::Literal(Literal::Int(4))),
            }),
            operator: Token::Slash,
            right: Box::new(Expr::Literal(Literal::Int(2))),
        }
    );
}

#[test]
fn test_left_associativity_comparisons() {
    // a < b < c -> (a < b) < c
    assert_eq!(
        parse_expr_src("a < b < c"),
        Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier("a".to_string())),
                operator: Token::Lt,
                right: Box::new(Expr::Identifier("b".to_string())),
            }),
            operator: Token::Lt,
            right: Box::new(Expr::Identifier("c".to_string())),
        }
    );
}

// CALLS, MEMBER ACCESS, INDEXING - INCLUDING CHAINED POSTFIX

#[test]
fn test_function_call_positional() {
    assert_eq!(
        parse_expr_src("foo(1, 2)"),
        Expr::Call {
            callee: Box::new(Expr::Identifier("foo".to_string())),
            arguments: vec![
                (None, Expr::Literal(Literal::Int(1))),
                (None, Expr::Literal(Literal::Int(2))),
            ],
        }
    );
}

#[test]
fn test_function_call_named_arguments() {
    assert_eq!(
        parse_expr_src(r#"connect(host="localhost", port=8080)"#),
        Expr::Call {
            callee: Box::new(Expr::Identifier("connect".to_string())),
            arguments: vec![
                (
                    Some("host".to_string()),
                    Expr::Literal(Literal::String("localhost".to_string()))
                ),
                (Some("port".to_string()), Expr::Literal(Literal::Int(8080))),
            ],
        }
    );
}

#[test]
fn test_function_call_mixed_arguments() {
    assert_eq!(
        parse_expr_src(r#"connect("localhost", port=8080)"#),
        Expr::Call {
            callee: Box::new(Expr::Identifier("connect".to_string())),
            arguments: vec![
                (None, Expr::Literal(Literal::String("localhost".to_string()))),
                (Some("port".to_string()), Expr::Literal(Literal::Int(8080))),
            ],
        }
    );
}

#[test]
fn test_function_call_trailing_comma() {
    assert_eq!(
        parse_expr_src("foo(1, 2,)"),
        Expr::Call {
            callee: Box::new(Expr::Identifier("foo".to_string())),
            arguments: vec![
                (None, Expr::Literal(Literal::Int(1))),
                (None, Expr::Literal(Literal::Int(2))),
            ],
        }
    );
}

#[test]
fn test_positional_after_named_argument_is_rejected() {
    // Positional cannot follow a named argument
    assert!(try_parse(r#"fn _t() { connect(host="localhost", 8080); }"#).is_err());
}

#[test]
fn test_invalid_named_argument_target_is_rejected() {
    // Left side of '=' in call argument must be a plain identifier
    assert!(try_parse("fn _t() { foo(1 + 2 = 3); }").is_err());
}

#[test]
fn test_member_access_chain() {
    assert_eq!(
        parse_expr_src("user.profile.name"),
        Expr::MemberAccess {
            object: Box::new(Expr::MemberAccess {
                object: Box::new(Expr::Identifier("user".to_string())),
                property: "profile".to_string(),
            }),
            property: "name".to_string(),
        }
    );
}

#[test]
fn test_index_access_chain() {
    assert_eq!(
        parse_expr_src("grid[0][1]"),
        Expr::Index {
            list: Box::new(Expr::Index {
                list: Box::new(Expr::Identifier("grid".to_string())),
                index: Box::new(Expr::Literal(Literal::Int(0))),
            }),
            index: Box::new(Expr::Literal(Literal::Int(1))),
        }
    );
}

#[test]
fn test_chained_call() {
    assert_eq!(
        parse_expr_src("make_adder(5)(3)"),
        Expr::Call {
            callee: Box::new(Expr::Call {
                callee: Box::new(Expr::Identifier("make_adder".to_string())),
                arguments: vec![(None, Expr::Literal(Literal::Int(5)))],
            }),
            arguments: vec![(None, Expr::Literal(Literal::Int(3)))],
        }
    );
}

#[test]
fn test_propagate_then_member_access() {
    // divide(a, b)?.code -> (divide(a, b)?).code
    assert_eq!(
        parse_expr_src("divide(a, b)?.code"),
        Expr::MemberAccess {
            object: Box::new(Expr::PropagateError(Box::new(Expr::Call {
                callee: Box::new(Expr::Identifier("divide".to_string())),
                arguments: vec![
                    (None, Expr::Identifier("a".to_string())),
                    (None, Expr::Identifier("b".to_string())),
                ],
            }))),
            property: "code".to_string(),
        }
    );
}

// COLLECTIONS

#[test]
fn test_list_literal() {
    assert_eq!(
        parse_expr_src("[1, 2, 3]"),
        Expr::List(vec![
            Expr::Literal(Literal::Int(1)),
            Expr::Literal(Literal::Int(2)),
            Expr::Literal(Literal::Int(3)),
        ])
    );
}

#[test]
fn test_list_literal_trailing_comma() {
    assert_eq!(
        parse_expr_src("[1, 2,]"),
        Expr::List(vec![
            Expr::Literal(Literal::Int(1)),
            Expr::Literal(Literal::Int(2)),
        ])
    );
}

#[test]
fn test_empty_list_literal() {
    assert_eq!(parse_expr_src("[]"), Expr::List(vec![]));
}

#[test]
fn test_map_literal() {
    assert_eq!(
        parse_expr_src(r#"{"key": 42}"#),
        Expr::Map(vec![(
            Expr::Literal(Literal::String("key".to_string())),
            Expr::Literal(Literal::Int(42)),
        )])
    );
}

#[test]
fn test_map_literal_trailing_comma() {
    assert_eq!(
        parse_expr_src(r#"{"a": 1, "b": 2,}"#),
        Expr::Map(vec![
            (
                Expr::Literal(Literal::String("a".to_string())),
                Expr::Literal(Literal::Int(1)),
            ),
            (
                Expr::Literal(Literal::String("b".to_string())),
                Expr::Literal(Literal::Int(2)),
            ),
        ])
    );
}

#[test]
fn test_block_vs_map_disambiguation() {
    // Empty braces {} is ALWAYS an empty block, never an empty map
    assert_eq!(parse_expr_src("{}"), Expr::Block(vec![]));

    // A block starting with a statement keyword must parse as a block
    match parse_expr_src("{ var x = 5; yield x; }") {
        Expr::Block(stmts) => assert_eq!(stmts.len(), 2),
        other => panic!("Expected Block, got {:?}", other),
    }

    // A block with an expression statement followed by ';' is a block
    match parse_expr_src("{ foo(); }") {
        Expr::Block(stmts) => assert_eq!(stmts.len(), 1),
        other => panic!("Expected Block, got {:?}", other),
    }
}

// STRUCT INIT EXPRESSIONS
#[test]
fn test_struct_init_expression() {
    assert_eq!(
        parse_expr_src(r#"User { name: "John", age: 522 }"#),
        Expr::StructInit {
            name: "User".to_string(),
            fields: vec![
                (
                    "name".to_string(),
                    Expr::Literal(Literal::String("John".to_string()))
                ),
                ("age".to_string(), Expr::Literal(Literal::Int(522))),
            ],
        }
    );
}

#[test]
fn test_struct_init_trailing_comma() {
    assert_eq!(
        parse_expr_src("Point { x: 1, y: 2, }"),
        Expr::StructInit {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), Expr::Literal(Literal::Int(1))),
                ("y".to_string(), Expr::Literal(Literal::Int(2))),
            ],
        }
    );
}

#[test]
fn test_struct_init_nested() {
    assert_eq!(
        parse_expr_src("Outer { inner: Inner { value: 5 } }"),
        Expr::StructInit {
            name: "Outer".to_string(),
            fields: vec![(
                "inner".to_string(),
                Expr::StructInit {
                    name: "Inner".to_string(),
                    fields: vec![("value".to_string(), Expr::Literal(Literal::Int(5)))],
                }
            )],
        }
    );
}

// SWITCH

#[test]
fn test_switch_expression() {
    assert_eq!(
        parse_expr_src("switch status { Status::Active -> 1, default -> 0 }"),
        Expr::Switch {
            target: Box::new(Expr::Identifier("status".to_string())),
            arms: vec![(
                Expr::EnumVariant {
                    enum_name: "Status".to_string(),
                    variant: "Active".to_string(),
                },
                Expr::Literal(Literal::Int(1)),
            )],
            default_arm: Box::new(Expr::Literal(Literal::Int(0))),
        }
    );
}

#[test]
fn test_switch_expression_multiple_arms_and_trailing_comma() {
    let expr = parse_expr_src(
        "switch status {
            Status::Active -> 1,
            Status::Pending -> 2,
            default -> 0,
        }",
    );
    match expr {
        Expr::Switch { arms, .. } => assert_eq!(arms.len(), 2),
        other => panic!("Expected Switch, got {:?}", other),
    }
}

#[test]
fn test_switch_with_only_default_arm() {
    assert_eq!(
        parse_expr_src("switch x { default -> 42 }"),
        Expr::Switch {
            target: Box::new(Expr::Identifier("x".to_string())),
            arms: vec![],
            default_arm: Box::new(Expr::Literal(Literal::Int(42))),
        }
    );
}

#[test]
fn test_switch_missing_default_arm_is_rejected() {
    assert!(try_parse("fn _t() { switch x { Status::Active -> 1 }; }").is_err());
}

// ERROR HANDLING SYNTAX

#[test]
fn test_propagate_error() {
    assert_eq!(
        parse_expr_src("divide(a, b)?"),
        Expr::PropagateError(Box::new(Expr::Call {
            callee: Box::new(Expr::Identifier("divide".to_string())),
            arguments: vec![
                (None, Expr::Identifier("a".to_string())),
                (None, Expr::Identifier("b".to_string())),
            ],
        }))
    );
}

#[test]
fn test_error_constructor_primitive() {
    assert_eq!(
        parse_expr_src(r#"Error(#NotFound, "Item missing")"#),
        Expr::Error {
            code: "NotFound".to_string(),
            message: Box::new(Expr::Literal(Literal::String("Item missing".to_string()))),
        }
    );
}

#[test]
fn test_or_inline_literal_desugaring() {
    // "a or 0" desugars in parser to OrFallback with Block([Yield(0)])
    assert_eq!(
        parse_expr_src("divide(a, b) or 0"),
        Expr::OrFallback {
            try_expr: Box::new(Expr::Call {
                callee: Box::new(Expr::Identifier("divide".to_string())),
                arguments: vec![
                    (None, Expr::Identifier("a".to_string())),
                    (None, Expr::Identifier("b".to_string())),
                ],
            }),
            error_binding: None,
            fallback: Box::new(Expr::Block(vec![Stmt::Yield(Expr::Literal(Literal::Int(
                0
            )))])),
        }
    );
}

#[test]
fn test_or_inline_variable() {
    assert_eq!(
        parse_expr_src("divide(a, b) or default_val"),
        Expr::OrFallback {
            try_expr: Box::new(Expr::Call {
                callee: Box::new(Expr::Identifier("divide".to_string())),
                arguments: vec![
                    (None, Expr::Identifier("a".to_string())),
                    (None, Expr::Identifier("b".to_string())),
                ],
            }),
            error_binding: None,
            fallback: Box::new(Expr::Block(vec![Stmt::Yield(Expr::Identifier(
                "default_val".to_string()
            ))])),
        }
    );
}

#[test]
fn test_or_inline_complex_expression() {
    assert_eq!(
        parse_expr_src("divide(a, b) or a + b"),
        Expr::OrFallback {
            try_expr: Box::new(Expr::Call {
                callee: Box::new(Expr::Identifier("divide".to_string())),
                arguments: vec![
                    (None, Expr::Identifier("a".to_string())),
                    (None, Expr::Identifier("b".to_string())),
                ],
            }),
            error_binding: None,
            fallback: Box::new(Expr::Block(vec![Stmt::Yield(Expr::Binary {
                left: Box::new(Expr::Identifier("a".to_string())),
                operator: Token::Plus,
                right: Box::new(Expr::Identifier("b".to_string())),
            })])),
        }
    );
}

#[test]
fn test_or_block_no_binding() {
    assert_eq!(
        parse_expr_src("divide(a, b) or { yield 0; }"),
        Expr::OrFallback {
            try_expr: Box::new(Expr::Call {
                callee: Box::new(Expr::Identifier("divide".to_string())),
                arguments: vec![
                    (None, Expr::Identifier("a".to_string())),
                    (None, Expr::Identifier("b".to_string())),
                ],
            }),
            error_binding: None,
            fallback: Box::new(Expr::Block(vec![Stmt::Yield(Expr::Literal(Literal::Int(
                0
            )))])),
        }
    );
}

#[test]
fn test_or_block_with_binding() {
    assert_eq!(
        parse_expr_src("divide(a, b) or (e) { yield 0; }"),
        Expr::OrFallback {
            try_expr: Box::new(Expr::Call {
                callee: Box::new(Expr::Identifier("divide".to_string())),
                arguments: vec![
                    (None, Expr::Identifier("a".to_string())),
                    (None, Expr::Identifier("b".to_string())),
                ],
            }),
            error_binding: Some("e".to_string()),
            fallback: Box::new(Expr::Block(vec![Stmt::Yield(Expr::Literal(Literal::Int(
                0
            )))])),
        }
    );
}

#[test]
fn test_or_inline_with_binding() {
    assert_eq!(
        parse_expr_src("divide(a, b) or (e) 0"),
        Expr::OrFallback {
            try_expr: Box::new(Expr::Call {
                callee: Box::new(Expr::Identifier("divide".to_string())),
                arguments: vec![
                    (None, Expr::Identifier("a".to_string())),
                    (None, Expr::Identifier("b".to_string())),
                ],
            }),
            error_binding: Some("e".to_string()),
            fallback: Box::new(Expr::Block(vec![Stmt::Yield(Expr::Literal(Literal::Int(
                0
            )))])),
        }
    );
}

#[test]
fn test_or_multiple_bindings_is_rejected() {
    // 'or' only accepts one error binding
    assert!(try_parse("fn _t() { divide(a, b) or (x, y) { yield 0; }; }").is_err());
}

// STRING INTERPOLATION - PARSER LEVEL

#[test]
fn test_plain_string_is_literal() {
    assert_eq!(
        parse_expr_src(r#""no interpolation here""#),
        Expr::Literal(Literal::String("no interpolation here".to_string()))
    );
}

#[test]
fn test_interpolated_string_simple() {
    assert_eq!(
        parse_expr_src(r#""Hello ${name}!""#),
        Expr::InterpolatedString(vec![
            StringSegment::Literal("Hello ".to_string()),
            StringSegment::Expr(Box::new(Expr::Identifier("name".to_string()))),
            StringSegment::Literal("!".to_string()),
        ])
    );
}

#[test]
fn test_interpolated_string_with_member_access() {
    assert_eq!(
        parse_expr_src(r#""User: ${user.name}""#),
        Expr::InterpolatedString(vec![
            StringSegment::Literal("User: ".to_string()),
            StringSegment::Expr(Box::new(Expr::MemberAccess {
                object: Box::new(Expr::Identifier("user".to_string())),
                property: "name".to_string(),
            })),
        ])
    );
}

#[test]
fn test_interpolated_string_with_complex_expression() {
    assert_eq!(
        parse_expr_src(r#""Result: ${1 + 2}""#),
        Expr::InterpolatedString(vec![
            StringSegment::Literal("Result: ".to_string()),
            StringSegment::Expr(Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Literal::Int(1))),
                operator: Token::Plus,
                right: Box::new(Expr::Literal(Literal::Int(2))),
            })),
        ])
    );
}

// ERROR

#[test]
fn test_missing_semicolon_is_rejected() {
    assert!(try_parse("fn _t() { var x = 5 }").is_err());
}

#[test]
fn test_missing_closing_brace_is_rejected() {
    assert!(try_parse("fn _t() { var x = 5;").is_err());
}

#[test]
fn test_unexpected_token_at_top_level_is_rejected() {
    assert!(try_parse("123").is_err());
}

#[test]
fn test_error_diagnostic_format_and_position() {
    let src = "fn _t() {\n    var x = 5\n}";
    match try_parse(src) {
        Err(e) => {
            assert_eq!(e.line, 3);
            let formatted = format!("{}", e);
            assert!(formatted.starts_with(&format!("{}:{}:", e.line, e.column)));
            assert!(formatted.contains("error:"));
        }
        Ok(_) => panic!("Expected parse error for missing semicolon"),
    }
}

#[test]
fn test_parser_unclosed_parenthesis_is_rejected() {
    assert!(try_parse("fn broken(").is_err());
}

#[test]
fn test_lexer_error_propagates_through_try_parse() {
    assert!(try_parse("fn _t() { var x = $; }").is_err());
}