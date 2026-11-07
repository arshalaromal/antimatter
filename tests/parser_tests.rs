// Parser tests

use antimatter::ast::*;
use antimatter::error::CompilerResult;
use antimatter::lexer::*;
use antimatter::parser::Parser;
use antimatter::span::Span;

macro_rules! ex {
    ($node:expr) => {
        Expr::new($node, Span::new(0, 0))
    };
}
macro_rules! en {
    ($node:expr) => {
        Box::new(ex!($node))
    };
}
macro_rules! st {
    ($node:expr) => {
        Stmt::new($node, Span::new(0, 0))
    };
}
macro_rules! sn {
    ($node:expr) => {
        Box::new(st!($node))
    };
}

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
        Item::Function(f) => match &f.body[0].node {
            StmtNode::Expression(e) => e.clone(),
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
        Item::Function(f) => match &f.body[0].node {
            StmtNode::VarDecl {
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
        Item::Function(FunctionDecl { span: Span::new(0, 0),
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
        Item::Function(FunctionDecl { span: Span::new(0, 0),
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
        Item::Function(FunctionDecl { span: Span::new(0, 0),
            directives: vec![],
            is_pub: false,
            name: "add".to_string(),
            parameters: vec![
                Parameter { span: Span::new(0, 0),
                    name: "a".to_string(),
                    type_name: Type::Basic("Int".to_string()),
                    default_value: None,
                },
                Parameter { span: Span::new(0, 0),
                    name: "b".to_string(),
                    type_name: Type::Basic("Int".to_string()),
                    default_value: None,
                },
            ],
            return_type: Some(Type::Basic("Int".to_string())),
            body: vec![st!(StmtNode::Return(Some(ex!(ExprNode::Identifier("a".to_string())))))],
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
        Item::Function(FunctionDecl { span: Span::new(0, 0),
            directives: vec![],
            is_pub: false,
            name: "greet".to_string(),
            parameters: vec![Parameter { span: Span::new(0, 0),
                name: "name".to_string(),
                type_name: Type::Basic("String".to_string()),
                default_value: Some(ex!(ExprNode::Literal(Literal::String("World".to_string())))),
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
                Some(ex!(ExprNode::Literal(Literal::Int(8080))))
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
        Item::Function(FunctionDecl { span: Span::new(0, 0),
            directives: vec!["extern".to_string()],
            is_pub: false,
            name: "sin".to_string(),
            parameters: vec![Parameter { span: Span::new(0, 0),
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
        Item::Struct { span: Span::new(0, 0),
            directives: vec![],
            is_pub: false,
            name: "User".to_string(),
            fields: vec![
                StructField { span: Span::new(0, 0),
                    directives: vec![],
                    is_pub: false,
                    name: "name".to_string(),
                    type_name: Type::Basic("String".to_string()),
                },
                StructField { span: Span::new(0, 0),
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
        Item::Struct { span: Span::new(0, 0),
            directives: vec![],
            is_pub: true,
            name: "Node".to_string(),
            fields: vec![
                StructField { span: Span::new(0, 0),
                    directives: vec![],
                    is_pub: true,
                    name: "name".to_string(),
                    type_name: Type::Basic("String".to_string()),
                },
                StructField { span: Span::new(0, 0),
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
        Item::Struct { span: Span::new(0, 0),
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
        Item::Enum { span: Span::new(0, 0),
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
        Item::Enum { span: Span::new(0, 0),
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
        Item::Enum { span: Span::new(0, 0),
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
        Item::Impl { span: Span::new(0, 0),
            target: "User".to_string(),
            methods: vec![FunctionDecl { span: Span::new(0, 0),
                directives: vec![],
                is_pub: true,
                name: "get_name".to_string(),
                parameters: vec![],
                return_type: Some(Type::Basic("String".to_string())),
                body: vec![st!(StmtNode::Return(Some(ex!(ExprNode::MemberAccess {
                    object: en!(ExprNode::SelfKw),
                    property: "name".to_string(),
                }))))],
            }],
        }
    );
}

#[test]
fn test_impl_block_multiple_methods() {
    let item = parse_single_item("impl User { fn a() { } fn b() { } }");
    match item {
        Item::Impl { target, methods, .. } => {
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
        Item::TypeAlias { span: Span::new(0, 0),
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
        Item::TypeAlias { span: Span::new(0, 0),
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
        Item::Use { span: Span::new(0, 0),
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
        Item::Use { span: Span::new(0, 0),
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
        Item::Use { span: Span::new(0, 0),
            path: vec!["net".to_string(), "http".to_string()],
            items: vec!["Request".to_string(), "Response".to_string()],
        }
    );
}

#[test]
fn test_use_multi_import_trailing_comma() {
    assert_eq!(
        parse_single_item("use a::{b, c,};"),
        Item::Use { span: Span::new(0, 0),
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
        st!(StmtNode::VarDecl {
            is_const: false,
            name: "x".to_string(),
            type_annotation: Some(Type::Basic("Int".to_string())),
            initializer: Some(ex!(ExprNode::Literal(Literal::Int(42)))),
        })
    );
}

#[test]
fn test_var_decl_type_inferred() {
    assert_eq!(
        parse_stmt_src("var y = 5;"),
        st!(StmtNode::VarDecl {
            is_const: false,
            name: "y".to_string(),
            type_annotation: None,
            initializer: Some(ex!(ExprNode::Literal(Literal::Int(5)))),
        })
    );
}

#[test]
fn test_var_decl_type_only_no_initializer() {
    assert_eq!(
        parse_stmt_src("var z: Int;"),
        st!(StmtNode::VarDecl {
            is_const: false,
            name: "z".to_string(),
            type_annotation: Some(Type::Basic("Int".to_string())),
            initializer: None,
        })
    );
}

#[test]
fn test_var_decl_bare_no_type_no_initializer() {
    // Parses in grammar; semantic checker later catches/rejects it
    assert_eq!(
        parse_stmt_src("var bare;"),
        st!(StmtNode::VarDecl {
            is_const: false,
            name: "bare".to_string(),
            type_annotation: None,
            initializer: None,
        })
    );
}

#[test]
fn test_const_decl() {
    assert_eq!(
        parse_stmt_src("const PI = 3.14;"),
        st!(StmtNode::VarDecl {
            is_const: true,
            name: "PI".to_string(),
            type_annotation: None,
            initializer: Some(ex!(ExprNode::Literal(Literal::Float(3.14)))),
        })
    );
}

// ASSIGNMENT STATEMENTS
#[test]
fn test_assignment_to_identifier() {
    assert_eq!(
        parse_stmt_src("x = 5;"),
        st!(StmtNode::Assignment {
            target: AssignTarget::Identifier("x".to_string()),
            operator: Token::Assign,
            value: ex!(ExprNode::Literal(Literal::Int(5))),
        })
    );
}

#[test]
fn test_assignment_to_member_access() {
    assert_eq!(
        parse_stmt_src(r#"user.name = "Stark";"#),
        st!(StmtNode::Assignment {
            target: AssignTarget::MemberAccess {
                object: en!(ExprNode::Identifier("user".to_string())),
                property: "name".to_string(),
            },
            operator: Token::Assign,
            value: ex!(ExprNode::Literal(Literal::String("Stark".to_string()))),
        })
    );
}

#[test]
fn test_assignment_to_nested_member_access() {
    assert_eq!(
        parse_stmt_src("a.b.c = 10;"),
        st!(StmtNode::Assignment {
            target: AssignTarget::MemberAccess {
                object: en!(ExprNode::MemberAccess {
                    object: en!(ExprNode::Identifier("a".to_string())),
                    property: "b".to_string(),
                }),
                property: "c".to_string(),
            },
            operator: Token::Assign,
            value: ex!(ExprNode::Literal(Literal::Int(10))),
        })
    );
}

#[test]
fn test_assignment_to_index() {
    assert_eq!(
        parse_stmt_src("list[0] = 9;"),
        st!(StmtNode::Assignment {
            target: AssignTarget::Index {
                list: en!(ExprNode::Identifier("list".to_string())),
                index: en!(ExprNode::Literal(Literal::Int(0))),
            },
            operator: Token::Assign,
            value: ex!(ExprNode::Literal(Literal::Int(9))),
        })
    );
}

#[test]
fn test_compound_assignments() {
    assert_eq!(
        parse_stmt_src("x += 1;"),
        st!(StmtNode::Assignment {
            target: AssignTarget::Identifier("x".to_string()),
            operator: Token::PlusEq,
            value: ex!(ExprNode::Literal(Literal::Int(1))),
        })
    );
    assert_eq!(
        parse_stmt_src("flags &= mask;"),
        st!(StmtNode::Assignment {
            target: AssignTarget::Identifier("flags".to_string()),
            operator: Token::BitAndEq,
            value: ex!(ExprNode::Identifier("mask".to_string())),
        })
    );
    assert_eq!(
        parse_stmt_src("val <<= 2;"),
        st!(StmtNode::Assignment {
            target: AssignTarget::Identifier("val".to_string()),
            operator: Token::ShlEq,
            value: ex!(ExprNode::Literal(Literal::Int(2))),
        })
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
        st!(StmtNode::If {
            condition: ex!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("x".to_string())),
                operator: Token::Gt,
                right: en!(ExprNode::Literal(Literal::Int(0))),
            }),
            then_branch: sn!(StmtNode::Expression(ex!(ExprNode::Block(vec![st!(StmtNode::Return(Some(
                ex!(ExprNode::Literal(Literal::Int(1)))
            )))])))),
            else_branch: None,
        })
    );
}

#[test]
fn test_if_else() {
    let stmt = parse_stmt_src("if x > 0 { return 1; } else { return 0; }");
    match &stmt.node {
        StmtNode::If { else_branch, .. } => assert!(else_branch.is_some()),
        _ => panic!("Expected If"),
    }
}

#[test]
fn test_else_if_chain() {
    let stmt = parse_stmt_src("if a { } else if b { } else { }");
    match &stmt.node {
        StmtNode::If { else_branch, .. } => match &else_branch.as_ref().expect("Expected else branch").node {
            StmtNode::If {
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
        st!(StmtNode::If {
            condition: ex!(ExprNode::Identifier("active".to_string())),
            then_branch: sn!(StmtNode::Return(Some(ex!(ExprNode::Literal(Literal::Bool(true)))))),
            else_branch: None,
        })
    );
}

#[test]
fn test_while_loop() {
    assert_eq!(
        parse_stmt_src("while active { break; }"),
        st!(StmtNode::While {
            condition: ex!(ExprNode::Identifier("active".to_string())),
            body: sn!(StmtNode::Expression(ex!(ExprNode::Block(vec![st!(StmtNode::Break)])))),
        })
    );
}

#[test]
fn test_braceless_while() {
    assert_eq!(
        parse_stmt_src("while active break;"),
        st!(StmtNode::While {
            condition: ex!(ExprNode::Identifier("active".to_string())),
            body: sn!(StmtNode::Break),
        })
    );
}

#[test]
fn test_for_loop() {
    assert_eq!(
        parse_stmt_src("for item in list { continue; }"),
        st!(StmtNode::For {
            item_name: "item".to_string(),
            iterator: ex!(ExprNode::Identifier("list".to_string())),
            body: sn!(StmtNode::Expression(ex!(ExprNode::Block(vec![st!(StmtNode::Continue)])))),
        })
    );
}

#[test]
fn test_for_loop_with_discard_variable() {
    assert_eq!(
        parse_stmt_src("for _ in list { continue; }"),
        st!(StmtNode::For {
            item_name: "_".to_string(),
            iterator: ex!(ExprNode::Identifier("list".to_string())),
            body: sn!(StmtNode::Expression(ex!(ExprNode::Block(vec![st!(StmtNode::Continue)])))),
        })
    );
}

#[test]
fn test_condition_not_confused_with_struct_init() {
    // Heads of if/while/for/switch parse expressions with allow_struct = false
    assert_eq!(
        parse_stmt_src("if active { return true; }"),
        st!(StmtNode::If {
            condition: ex!(ExprNode::Identifier("active".to_string())),
            then_branch: sn!(StmtNode::Expression(ex!(ExprNode::Block(vec![st!(StmtNode::Return(Some(
                ex!(ExprNode::Literal(Literal::Bool(true)))
            )))])))),
            else_branch: None,
        })
    );

    assert_eq!(
        parse_stmt_src("while running { break; }"),
        st!(StmtNode::While {
            condition: ex!(ExprNode::Identifier("running".to_string())),
            body: sn!(StmtNode::Expression(ex!(ExprNode::Block(vec![st!(StmtNode::Break)])))),
        })
    );

    assert_eq!(
        parse_stmt_src("for x in items { continue; }"),
        st!(StmtNode::For {
            item_name: "x".to_string(),
            iterator: ex!(ExprNode::Identifier("items".to_string())),
            body: sn!(StmtNode::Expression(ex!(ExprNode::Block(vec![st!(StmtNode::Continue)])))),
        })
    );
}

#[test]
fn test_struct_init_allowed_inside_parens_in_condition() {
    // Parentheses re-enable struct init in conditional contexts
    let stmt = parse_stmt_src(r#"if (User { active: true }).active { return true; }"#);
    match &stmt.node {
        StmtNode::If { condition, .. } => match &condition.node {
            ExprNode::MemberAccess { object, .. } => match &object.node {
                ExprNode::StructInit { name, fields } => {
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
    assert_eq!(parse_stmt_src("break;"), st!(StmtNode::Break));
    assert_eq!(parse_stmt_src("continue;"), st!(StmtNode::Continue));
}

#[test]
fn test_return_statements() {
    assert_eq!(parse_stmt_src("return;"), st!(StmtNode::Return(None)));
    assert_eq!(
        parse_stmt_src("return 42;"),
        st!(StmtNode::Return(Some(ex!(ExprNode::Literal(Literal::Int(42))))))
    );
}

#[test]
fn test_yield_statement() {
    assert_eq!(
        parse_stmt_src("yield x * 2;"),
        st!(StmtNode::Yield(ex!(ExprNode::Binary {
            left: en!(ExprNode::Identifier("x".to_string())),
            operator: Token::Star,
            right: en!(ExprNode::Literal(Literal::Int(2))),
        })))
    );
}

#[test]
fn test_defer_statement() {
    assert_eq!(
        parse_stmt_src("defer file.close();"),
        st!(StmtNode::Defer(sn!(StmtNode::Expression(ex!(ExprNode::Call {
            callee: en!(ExprNode::MemberAccess {
                object: en!(ExprNode::Identifier("file".to_string())),
                property: "close".to_string(),
            }),
            arguments: vec![],
        })))))
    );
}

// LITERALS & IDENTIFIERS

#[test]
fn test_literals() {
    assert_eq!(parse_expr_src("42"), ex!(ExprNode::Literal(Literal::Int(42))));
    assert_eq!(parse_expr_src("3.14"), ex!(ExprNode::Literal(Literal::Float(3.14))));
    assert_eq!(
        parse_expr_src(r#""hello""#),
        ex!(ExprNode::Literal(Literal::String("hello".to_string())))
    );
    assert_eq!(
        parse_expr_src("`raw string`"),
        ex!(ExprNode::Literal(Literal::String("raw string".to_string())))
    );
    assert_eq!(parse_expr_src("true"), ex!(ExprNode::Literal(Literal::Bool(true))));
    assert_eq!(parse_expr_src("false"), ex!(ExprNode::Literal(Literal::Bool(false))));
}

#[test]
fn test_identifiers_and_self() {
    assert_eq!(
        parse_expr_src("my_var"),
        ex!(ExprNode::Identifier("my_var".to_string()))
    );
    assert_eq!(parse_expr_src("_"), ex!(ExprNode::Identifier("_".to_string())));
    assert_eq!(parse_expr_src("self"), ex!(ExprNode::SelfKw));
}

#[test]
fn test_ecode_literal_standalone() {
    assert_eq!(
        parse_expr_src("#NotFound"),
        ex!(ExprNode::ECodeLit("NotFound".to_string()))
    );
}

#[test]
fn test_ecode_literal_in_comparison() {
    assert_eq!(
        parse_expr_src("e.code == #NotFound"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::MemberAccess {
                object: en!(ExprNode::Identifier("e".to_string())),
                property: "code".to_string(),
            }),
            operator: Token::EqEq,
            right: en!(ExprNode::ECodeLit("NotFound".to_string())),
        })
    );
}

#[test]
fn test_enum_variant_standalone() {
    assert_eq!(
        parse_expr_src("Status::Active"),
        ex!(ExprNode::EnumVariant {
            enum_name: "Status".to_string(),
            variant: "Active".to_string(),
        })
    );
}

#[test]
fn test_enum_variant_in_comparison() {
    assert_eq!(
        parse_expr_src("Status::Active == Status::Pending"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::EnumVariant {
                enum_name: "Status".to_string(),
                variant: "Active".to_string(),
            }),
            operator: Token::EqEq,
            right: en!(ExprNode::EnumVariant {
                enum_name: "Status".to_string(),
                variant: "Pending".to_string(),
            }),
        })
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
        ex!(ExprNode::Unary {
            operator: Token::Minus,
            right: en!(ExprNode::Literal(Literal::Int(13))),
        })
    );
    assert_eq!(
        parse_expr_src("!active"),
        ex!(ExprNode::Unary {
            operator: Token::Not,
            right: en!(ExprNode::Identifier("active".to_string())),
        })
    );
    assert_eq!(
        parse_expr_src("~mask"),
        ex!(ExprNode::Unary {
            operator: Token::BitNot,
            right: en!(ExprNode::Identifier("mask".to_string())),
        })
    );
}

#[test]
fn test_grouping_overrides_precedence() {
    assert_eq!(
        parse_expr_src("(2 + 3) * 4"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Binary {
                left: en!(ExprNode::Literal(Literal::Int(2))),
                operator: Token::Plus,
                right: en!(ExprNode::Literal(Literal::Int(3))),
            }),
            operator: Token::Star,
            right: en!(ExprNode::Literal(Literal::Int(4))),
        })
    );
}

#[test]
fn test_precedence_factor_over_term() {
    // 2 + 3 * 4 -> 2 + (3 * 4)
    assert_eq!(
        parse_expr_src("2 + 3 * 4"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Literal(Literal::Int(2))),
            operator: Token::Plus,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Literal(Literal::Int(3))),
                operator: Token::Star,
                right: en!(ExprNode::Literal(Literal::Int(4))),
            }),
        })
    );

    // 10 - 8 / 2 -> 10 - (8 / 2)
    assert_eq!(
        parse_expr_src("10 - 8 / 2"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Literal(Literal::Int(10))),
            operator: Token::Minus,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Literal(Literal::Int(8))),
                operator: Token::Slash,
                right: en!(ExprNode::Literal(Literal::Int(2))),
            }),
        })
    );

    // 5 + 7 % 3 -> 5 + (7 % 3)
    assert_eq!(
        parse_expr_src("5 + 7 % 3"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Literal(Literal::Int(5))),
            operator: Token::Plus,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Literal(Literal::Int(7))),
                operator: Token::Percent,
                right: en!(ExprNode::Literal(Literal::Int(3))),
            }),
        })
    );
}

#[test]
fn test_precedence_term_over_shift() {
    // a << 1 + 2 -> a << (1 + 2)
    assert_eq!(
        parse_expr_src("a << 1 + 2"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Identifier("a".to_string())),
            operator: Token::Shl,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Literal(Literal::Int(1))),
                operator: Token::Plus,
                right: en!(ExprNode::Literal(Literal::Int(2))),
            }),
        })
    );
}

#[test]
fn test_precedence_shift_over_comparison() {
    // a < b << 1 -> a < (b << 1)
    assert_eq!(
        parse_expr_src("a < b << 1"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Identifier("a".to_string())),
            operator: Token::Lt,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("b".to_string())),
                operator: Token::Shl,
                right: en!(ExprNode::Literal(Literal::Int(1))),
            }),
        })
    );
}

#[test]
fn test_precedence_comparison_over_equality() {
    // a == b < c -> a == (b < c)
    assert_eq!(
        parse_expr_src("a == b < c"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Identifier("a".to_string())),
            operator: Token::EqEq,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("b".to_string())),
                operator: Token::Lt,
                right: en!(ExprNode::Identifier("c".to_string())),
            }),
        })
    );
}

#[test]
fn test_precedence_equality_over_bitand() {
    // a & b == c -> a & (b == c)
    assert_eq!(
        parse_expr_src("a & b == c"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Identifier("a".to_string())),
            operator: Token::BitAnd,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("b".to_string())),
                operator: Token::EqEq,
                right: en!(ExprNode::Identifier("c".to_string())),
            }),
        })
    );
}

#[test]
fn test_precedence_bitand_over_bitxor() {
    // a ^ b & c -> a ^ (b & c)
    assert_eq!(
        parse_expr_src("a ^ b & c"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Identifier("a".to_string())),
            operator: Token::BitXor,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("b".to_string())),
                operator: Token::BitAnd,
                right: en!(ExprNode::Identifier("c".to_string())),
            }),
        })
    );
}

#[test]
fn test_precedence_bitxor_over_bitor() {
    // a | b ^ c -> a | (b ^ c)
    assert_eq!(
        parse_expr_src("a | b ^ c"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Identifier("a".to_string())),
            operator: Token::BitOr,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("b".to_string())),
                operator: Token::BitXor,
                right: en!(ExprNode::Identifier("c".to_string())),
            }),
        })
    );
}

#[test]
fn test_precedence_bitor_over_and() {
    // a && b | c -> a && (b | c)
    assert_eq!(
        parse_expr_src("a && b | c"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Identifier("a".to_string())),
            operator: Token::And,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("b".to_string())),
                operator: Token::BitOr,
                right: en!(ExprNode::Identifier("c".to_string())),
            }),
        })
    );
}

#[test]
fn test_precedence_and_over_or() {
    // a || b && c -> a || (b && c)
    assert_eq!(
        parse_expr_src("a || b && c"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Identifier("a".to_string())),
            operator: Token::Or,
            right: en!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("b".to_string())),
                operator: Token::And,
                right: en!(ExprNode::Identifier("c".to_string())),
            }),
        })
    );
}

#[test]
fn test_left_associativity_arithmetic() {
    // 5 - 3 - 1 -> (5 - 3) - 1
    assert_eq!(
        parse_expr_src("5 - 3 - 1"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Binary {
                left: en!(ExprNode::Literal(Literal::Int(5))),
                operator: Token::Minus,
                right: en!(ExprNode::Literal(Literal::Int(3))),
            }),
            operator: Token::Minus,
            right: en!(ExprNode::Literal(Literal::Int(1))),
        })
    );

    // 8 / 4 / 2 -> (8 / 4) / 2
    assert_eq!(
        parse_expr_src("8 / 4 / 2"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Binary {
                left: en!(ExprNode::Literal(Literal::Int(8))),
                operator: Token::Slash,
                right: en!(ExprNode::Literal(Literal::Int(4))),
            }),
            operator: Token::Slash,
            right: en!(ExprNode::Literal(Literal::Int(2))),
        })
    );
}

#[test]
fn test_left_associativity_comparisons() {
    // a < b < c -> (a < b) < c
    assert_eq!(
        parse_expr_src("a < b < c"),
        ex!(ExprNode::Binary {
            left: en!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("a".to_string())),
                operator: Token::Lt,
                right: en!(ExprNode::Identifier("b".to_string())),
            }),
            operator: Token::Lt,
            right: en!(ExprNode::Identifier("c".to_string())),
        })
    );
}

// CALLS, MEMBER ACCESS, INDEXING - INCLUDING CHAINED POSTFIX

#[test]
fn test_function_call_positional() {
    assert_eq!(
        parse_expr_src("foo(1, 2)"),
        ex!(ExprNode::Call {
            callee: en!(ExprNode::Identifier("foo".to_string())),
            arguments: vec![
                (None, ex!(ExprNode::Literal(Literal::Int(1)))),
                (None, ex!(ExprNode::Literal(Literal::Int(2)))),
            ],
        })
    );
}

#[test]
fn test_function_call_named_arguments() {
    assert_eq!(
        parse_expr_src(r#"connect(host="localhost", port=8080)"#),
        ex!(ExprNode::Call {
            callee: en!(ExprNode::Identifier("connect".to_string())),
            arguments: vec![
                (
                    Some("host".to_string()),
                    ex!(ExprNode::Literal(Literal::String("localhost".to_string())))
                ),
                (Some("port".to_string()), ex!(ExprNode::Literal(Literal::Int(8080)))),
            ],
        })
    );
}

#[test]
fn test_function_call_mixed_arguments() {
    assert_eq!(
        parse_expr_src(r#"connect("localhost", port=8080)"#),
        ex!(ExprNode::Call {
            callee: en!(ExprNode::Identifier("connect".to_string())),
            arguments: vec![
                (None, ex!(ExprNode::Literal(Literal::String("localhost".to_string())))),
                (Some("port".to_string()), ex!(ExprNode::Literal(Literal::Int(8080)))),
            ],
        })
    );
}

#[test]
fn test_function_call_trailing_comma() {
    assert_eq!(
        parse_expr_src("foo(1, 2,)"),
        ex!(ExprNode::Call {
            callee: en!(ExprNode::Identifier("foo".to_string())),
            arguments: vec![
                (None, ex!(ExprNode::Literal(Literal::Int(1)))),
                (None, ex!(ExprNode::Literal(Literal::Int(2)))),
            ],
        })
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
        ex!(ExprNode::MemberAccess {
            object: en!(ExprNode::MemberAccess {
                object: en!(ExprNode::Identifier("user".to_string())),
                property: "profile".to_string(),
            }),
            property: "name".to_string(),
        })
    );
}

#[test]
fn test_index_access_chain() {
    assert_eq!(
        parse_expr_src("grid[0][1]"),
        ex!(ExprNode::Index {
            list: en!(ExprNode::Index {
                list: en!(ExprNode::Identifier("grid".to_string())),
                index: en!(ExprNode::Literal(Literal::Int(0))),
            }),
            index: en!(ExprNode::Literal(Literal::Int(1))),
        })
    );
}

#[test]
fn test_chained_call() {
    assert_eq!(
        parse_expr_src("make_adder(5)(3)"),
        ex!(ExprNode::Call {
            callee: en!(ExprNode::Call {
                callee: en!(ExprNode::Identifier("make_adder".to_string())),
                arguments: vec![(None, ex!(ExprNode::Literal(Literal::Int(5))))],
            }),
            arguments: vec![(None, ex!(ExprNode::Literal(Literal::Int(3))))],
        })
    );
}

#[test]
fn test_propagate_then_member_access() {
    // divide(a, b)?.code -> (divide(a, b)?).code
    assert_eq!(
        parse_expr_src("divide(a, b)?.code"),
        ex!(ExprNode::MemberAccess {
            object: en!(ExprNode::PropagateError(en!(ExprNode::Call {
                callee: en!(ExprNode::Identifier("divide".to_string())),
                arguments: vec![
                    (None, ex!(ExprNode::Identifier("a".to_string()))),
                    (None, ex!(ExprNode::Identifier("b".to_string()))),
                ],
            }))),
            property: "code".to_string(),
        })
    );
}

// COLLECTIONS

#[test]
fn test_list_literal() {
    assert_eq!(
        parse_expr_src("[1, 2, 3]"),
        ex!(ExprNode::List(vec![
            ex!(ExprNode::Literal(Literal::Int(1))),
            ex!(ExprNode::Literal(Literal::Int(2))),
            ex!(ExprNode::Literal(Literal::Int(3))),
        ]))
    );
}

#[test]
fn test_list_literal_trailing_comma() {
    assert_eq!(
        parse_expr_src("[1, 2,]"),
        ex!(ExprNode::List(vec![
            ex!(ExprNode::Literal(Literal::Int(1))),
            ex!(ExprNode::Literal(Literal::Int(2))),
        ]))
    );
}

#[test]
fn test_empty_list_literal() {
    assert_eq!(parse_expr_src("[]"), ex!(ExprNode::List(vec![])));
}

#[test]
fn test_map_literal() {
    assert_eq!(
        parse_expr_src(r#"{"key": 42}"#),
        ex!(ExprNode::Map(vec![(
            ex!(ExprNode::Literal(Literal::String("key".to_string()))),
            ex!(ExprNode::Literal(Literal::Int(42))),
        )]))
    );
}

#[test]
fn test_map_literal_trailing_comma() {
    assert_eq!(
        parse_expr_src(r#"{"a": 1, "b": 2,}"#),
        ex!(ExprNode::Map(vec![
            (
                ex!(ExprNode::Literal(Literal::String("a".to_string()))),
                ex!(ExprNode::Literal(Literal::Int(1))),
            ),
            (
                ex!(ExprNode::Literal(Literal::String("b".to_string()))),
                ex!(ExprNode::Literal(Literal::Int(2))),
            ),
        ]))
    );
}

#[test]
fn test_block_vs_map_disambiguation() {
    // Empty braces {} is ALWAYS an empty block, never an empty map
    assert_eq!(parse_expr_src("{}"), ex!(ExprNode::Block(vec![])));

    // A block starting with a statement keyword must parse as a block
    match parse_expr_src("{ var x = 5; yield x; }").node {
        ExprNode::Block(stmts) => assert_eq!(stmts.len(), 2),
        other => panic!("Expected Block, got {:?}", other),
    }

    // A block with an expression statement followed by ';' is a block
    match parse_expr_src("{ foo(); }").node {
        ExprNode::Block(stmts) => assert_eq!(stmts.len(), 1),
        other => panic!("Expected Block, got {:?}", other),
    }
}

// STRUCT INIT EXPRESSIONS
#[test]
fn test_struct_init_expression() {
    assert_eq!(
        parse_expr_src(r#"User { name: "John", age: 522 }"#),
        ex!(ExprNode::StructInit {
            name: "User".to_string(),
            fields: vec![
                (
                    "name".to_string(),
                    ex!(ExprNode::Literal(Literal::String("John".to_string())))
                ),
                ("age".to_string(), ex!(ExprNode::Literal(Literal::Int(522)))),
            ],
        })
    );
}

#[test]
fn test_struct_init_trailing_comma() {
    assert_eq!(
        parse_expr_src("Point { x: 1, y: 2, }"),
        ex!(ExprNode::StructInit {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), ex!(ExprNode::Literal(Literal::Int(1)))),
                ("y".to_string(), ex!(ExprNode::Literal(Literal::Int(2)))),
            ],
        })
    );
}

#[test]
fn test_struct_init_nested() {
    assert_eq!(
        parse_expr_src("Outer { inner: Inner { value: 5 } }"),
        ex!(ExprNode::StructInit {
            name: "Outer".to_string(),
            fields: vec![(
                "inner".to_string(),
                ex!(ExprNode::StructInit {
                    name: "Inner".to_string(),
                    fields: vec![("value".to_string(), ex!(ExprNode::Literal(Literal::Int(5))))],
                })
            )],
        })
    );
}

// SWITCH

#[test]
fn test_switch_expression() {
    assert_eq!(
        parse_expr_src("switch status { Status::Active -> 1, default -> 0 }"),
        ex!(ExprNode::Switch {
            target: en!(ExprNode::Identifier("status".to_string())),
            arms: vec![(
                ex!(ExprNode::EnumVariant {
                    enum_name: "Status".to_string(),
                    variant: "Active".to_string(),
                }),
                ex!(ExprNode::Literal(Literal::Int(1))),
            )],
            default_arm: en!(ExprNode::Literal(Literal::Int(0))),
        })
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
    match &expr.node {
        ExprNode::Switch { arms, .. } => assert_eq!(arms.len(), 2),
        other => panic!("Expected Switch, got {:?}", other),
    }
}

#[test]
fn test_switch_with_only_default_arm() {
    assert_eq!(
        parse_expr_src("switch x { default -> 42 }"),
        ex!(ExprNode::Switch {
            target: en!(ExprNode::Identifier("x".to_string())),
            arms: vec![],
            default_arm: en!(ExprNode::Literal(Literal::Int(42))),
        })
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
        ex!(ExprNode::PropagateError(en!(ExprNode::Call {
            callee: en!(ExprNode::Identifier("divide".to_string())),
            arguments: vec![
                (None, ex!(ExprNode::Identifier("a".to_string()))),
                (None, ex!(ExprNode::Identifier("b".to_string()))),
            ],
        })))
    );
}

#[test]
fn test_error_constructor_primitive() {
    assert_eq!(
        parse_expr_src(r#"Error(#NotFound, "Item missing")"#),
        ex!(ExprNode::Error {
            code: "NotFound".to_string(),
            message: en!(ExprNode::Literal(Literal::String("Item missing".to_string()))),
        })
    );
}

#[test]
fn test_or_inline_literal_desugaring() {
    // "a or 0" desugars in parser to OrFallback with Block([Yield(0)])
    assert_eq!(
        parse_expr_src("divide(a, b) or 0"),
        ex!(ExprNode::OrFallback {
            try_expr: en!(ExprNode::Call {
                callee: en!(ExprNode::Identifier("divide".to_string())),
                arguments: vec![
                    (None, ex!(ExprNode::Identifier("a".to_string()))),
                    (None, ex!(ExprNode::Identifier("b".to_string()))),
                ],
            }),
            error_binding: None,
            fallback: en!(ExprNode::Block(vec![st!(StmtNode::Yield(ex!(ExprNode::Literal(Literal::Int(
                0
            )))))])),
        })
    );
}

#[test]
fn test_or_inline_variable() {
    assert_eq!(
        parse_expr_src("divide(a, b) or default_val"),
        ex!(ExprNode::OrFallback {
            try_expr: en!(ExprNode::Call {
                callee: en!(ExprNode::Identifier("divide".to_string())),
                arguments: vec![
                    (None, ex!(ExprNode::Identifier("a".to_string()))),
                    (None, ex!(ExprNode::Identifier("b".to_string()))),
                ],
            }),
            error_binding: None,
            fallback: en!(ExprNode::Block(vec![st!(StmtNode::Yield(ex!(ExprNode::Identifier(
                "default_val".to_string()
            ))))])),
        })
    );
}

#[test]
fn test_or_inline_complex_expression() {
    assert_eq!(
        parse_expr_src("divide(a, b) or a + b"),
        ex!(ExprNode::OrFallback {
            try_expr: en!(ExprNode::Call {
                callee: en!(ExprNode::Identifier("divide".to_string())),
                arguments: vec![
                    (None, ex!(ExprNode::Identifier("a".to_string()))),
                    (None, ex!(ExprNode::Identifier("b".to_string()))),
                ],
            }),
            error_binding: None,
            fallback: en!(ExprNode::Block(vec![st!(StmtNode::Yield(ex!(ExprNode::Binary {
                left: en!(ExprNode::Identifier("a".to_string())),
                operator: Token::Plus,
                right: en!(ExprNode::Identifier("b".to_string())),
            })))])),
        })
    );
}

#[test]
fn test_or_block_no_binding() {
    assert_eq!(
        parse_expr_src("divide(a, b) or { yield 0; }"),
        ex!(ExprNode::OrFallback {
            try_expr: en!(ExprNode::Call {
                callee: en!(ExprNode::Identifier("divide".to_string())),
                arguments: vec![
                    (None, ex!(ExprNode::Identifier("a".to_string()))),
                    (None, ex!(ExprNode::Identifier("b".to_string()))),
                ],
            }),
            error_binding: None,
            fallback: en!(ExprNode::Block(vec![st!(StmtNode::Yield(ex!(ExprNode::Literal(Literal::Int(
                0
            )))))])),
        })
    );
}

#[test]
fn test_or_block_with_binding() {
    assert_eq!(
        parse_expr_src("divide(a, b) or (e) { yield 0; }"),
        ex!(ExprNode::OrFallback {
            try_expr: en!(ExprNode::Call {
                callee: en!(ExprNode::Identifier("divide".to_string())),
                arguments: vec![
                    (None, ex!(ExprNode::Identifier("a".to_string()))),
                    (None, ex!(ExprNode::Identifier("b".to_string()))),
                ],
            }),
            error_binding: Some("e".to_string()),
            fallback: en!(ExprNode::Block(vec![st!(StmtNode::Yield(ex!(ExprNode::Literal(Literal::Int(
                0
            )))))])),
        })
    );
}

#[test]
fn test_or_inline_with_binding() {
    assert_eq!(
        parse_expr_src("divide(a, b) or (e) 0"),
        ex!(ExprNode::OrFallback {
            try_expr: en!(ExprNode::Call {
                callee: en!(ExprNode::Identifier("divide".to_string())),
                arguments: vec![
                    (None, ex!(ExprNode::Identifier("a".to_string()))),
                    (None, ex!(ExprNode::Identifier("b".to_string()))),
                ],
            }),
            error_binding: Some("e".to_string()),
            fallback: en!(ExprNode::Block(vec![st!(StmtNode::Yield(ex!(ExprNode::Literal(Literal::Int(
                0
            )))))])),
        })
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
        ex!(ExprNode::Literal(Literal::String("no interpolation here".to_string())))
    );
}

#[test]
fn test_interpolated_string_simple() {
    assert_eq!(
        parse_expr_src(r#""Hello ${name}!""#),
        ex!(ExprNode::InterpolatedString(vec![
            StringSegment::Literal("Hello ".to_string()),
            StringSegment::Expr(en!(ExprNode::Identifier("name".to_string()))),
            StringSegment::Literal("!".to_string()),
        ]))
    );
}

#[test]
fn test_interpolated_string_with_member_access() {
    assert_eq!(
        parse_expr_src(r#""User: ${user.name}""#),
        ex!(ExprNode::InterpolatedString(vec![
            StringSegment::Literal("User: ".to_string()),
            StringSegment::Expr(en!(ExprNode::MemberAccess {
                object: en!(ExprNode::Identifier("user".to_string())),
                property: "name".to_string(),
            })),
        ]))
    );
}

#[test]
fn test_interpolated_string_with_complex_expression() {
    assert_eq!(
        parse_expr_src(r#""Result: ${1 + 2}""#),
        ex!(ExprNode::InterpolatedString(vec![
            StringSegment::Literal("Result: ".to_string()),
            StringSegment::Expr(en!(ExprNode::Binary {
                left: en!(ExprNode::Literal(Literal::Int(1))),
                operator: Token::Plus,
                right: en!(ExprNode::Literal(Literal::Int(2))),
            })),
        ]))
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


// SPAN ACCURACY TESTS (Part 3 §10.2 G3-3)

#[test]
fn test_span_binary_expression_accuracy() {
    let prog = parse("fn main() {\n    a + b;\n}");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0].node {
            StmtNode::Expression(expr) => {
                assert_eq!(expr.span, Span::new(2, 5));
                match &expr.node {
                    ExprNode::Binary { left, right, .. } => {
                        assert_eq!(left.span, Span::new(2, 5));
                        assert_eq!(right.span, Span::new(2, 9));
                    }
                    _ => panic!("Expected Binary expression"),
                }
            }
            _ => panic!("Expected expression stmt"),
        },
        _ => panic!("Expected Function"),
    }
}

#[test]
fn test_span_statement_accuracy() {
    let prog = parse("fn main() {\n    var x = 5;\n}");
    match &prog.items[0] {
        Item::Function(f) => {
            let stmt = &f.body[0];
            assert_eq!(stmt.span, Span::new(2, 5));
            match &stmt.node {
                StmtNode::VarDecl { initializer, .. } => {
                    assert_eq!(initializer.as_ref().unwrap().span, Span::new(2, 13));
                }
                _ => panic!("Expected VarDecl statement"),
            }
        }
        _ => panic!("Expected Function"),
    }
}

#[test]
fn test_span_item_function_accuracy() {
    let item = parse_single_item("fn foo() {}");
    assert_eq!(item.span(), Span::new(1, 1));
    match item {
        Item::Function(f) => {
            assert_eq!(f.span, Span::new(1, 1));
        }
        _ => panic!("Expected Function item"),
    }
}

#[test]
fn test_span_item_struct_and_fields_accuracy() {
    let item = parse_single_item("struct User {\n    name: String,\n    age: Int,\n}");
    assert_eq!(item.span(), Span::new(1, 1));
    match item {
        Item::Struct { fields, .. } => {
            assert_eq!(fields[0].span, Span::new(2, 5));
            assert_eq!(fields[1].span, Span::new(3, 5));
        }
        _ => panic!("Expected Struct item"),
    }
}

#[test]
fn test_span_multiline_construct_anchors_at_start() {
    let prog = parse("fn main() {\n    if false {\n        var y = 10;\n    }\n}");
    match &prog.items[0] {
        Item::Function(f) => {
            let stmt = &f.body[0];
            assert_eq!(stmt.span, Span::new(2, 5));
        }
        _ => panic!("Expected Function"),
    }
}

#[test]
fn test_span_call_expression_accuracy() {
    let prog = parse("fn main() {\n    foo(bar, 42);\n}");
    match &prog.items[0] {
        Item::Function(f) => match &f.body[0].node {
            StmtNode::Expression(expr) => {
                assert_eq!(expr.span, Span::new(2, 5));
                match &expr.node {
                    ExprNode::Call { callee, arguments } => {
                        assert_eq!(callee.span, Span::new(2, 5));
                        assert_eq!(arguments[0].1.span, Span::new(2, 9));
                        assert_eq!(arguments[1].1.span, Span::new(2, 14));
                    }
                    _ => panic!("Expected Call expression"),
                }
            }
            _ => panic!("Expected expression stmt"),
        },
        _ => panic!("Expected Function"),
    }
}

#[test]
fn test_empty_string_interpolation_is_rejected() {
    let result = try_parse(r#"fn _t() { var x = "${}"; }"#);
    assert!(result.is_err(), "Expected error for empty interpolation, got Ok");
}

#[test]
fn test_string_interpolation_with_trailing_tokens_is_rejected() {
    let result = try_parse(r#"fn _t() { var x = "${a b}"; }"#);
    assert!(result.is_err(), "Expected error for leftover tokens in interpolation, got Ok");
}

#[test]
fn test_expression_truncated_at_eof_reports_unexpected_end_of_input() {
    let result = try_parse("fn _t() { var x = 1 +");
    match result {
        Err(e) => {
            assert!(
                e.message.contains("Unexpected end of input"),
                "Expected 'Unexpected end of input', got: {}",
                e.message
            );
        }
        Ok(_) => panic!("Expected parse error for truncated expression"),
    }
}
