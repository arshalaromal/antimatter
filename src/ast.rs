use crate::lexer::Token;
use crate::span::Span;

/// Represents a data type
#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    /// A standard type like `Int`, `String`, `Bool`, or `AStruct`.
    Basic(String),
    /// A List type. eg: `[Int]`
    List(Box<Type>),
    /// A map type. eg: `{String: Int}`
    Map {
        key: Box<Type>,
        value: Box<Type>,
    },
    /// A function signature type. eg: `fn(Int, String) -> Bool`
    Function {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },
    /// Wraps ANY type in a failable state. e.g., `![Int]` or `!{String: Int}`
    Failable(Box<Type>),
    Void,
    ECode,
}

/// Represents the raw datatypes which are hardcoded into Antimatter.
#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

/// One segment inside an interpolated string.
#[derive(Debug, PartialEq, Clone)]
pub enum StringSegment {
    Literal(String),   // plain text portion
    Expr(Box<Expr>),   // evaluated expression
}

/// Represents a valid memory location on the Left Hand Side of an assignment.
#[derive(Debug, PartialEq, Clone)]
pub enum AssignTarget {
    /// eg: `a = 123;`
    Identifier(String),
    /// eg: `user.name = "Stark";`
    MemberAccess {
        object: Box<Expr>,
        property: String,
    },
    /// eg: `list[5] = 6;`
    Index {
        list: Box<Expr>,
        index: Box<Expr>,
    },
}

/// An expression paired with its exact position in the source code.
#[derive(Debug, Clone)]
pub struct Expr {
    pub node: ExprNode,
    pub span: Span,
}

impl Expr {
    pub fn new(node: ExprNode, span: Span) -> Self {
        Self { node, span }
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

/// Represents anything that computes to a value.
#[derive(Debug, PartialEq, Clone)]
pub enum ExprNode {
    /// Normal core data types.
    Literal(Literal),

    ECodeLit(String),

    /// A named variable reference.
    Identifier(String),

    /// The `self` keyword.
    SelfKw,

    /// A single operator applied to a value. eg: `-5`, `!active`, `~mask`
    Unary { operator: Token, right: Box<Expr> },

    /// Binary operations. eg: `1 + 1`, `a * b`, `x && y`
    Binary { left: Box<Expr>, operator: Token, right: Box<Expr> },

    /// Executing a function. eg: `process(val, secure=true)`
    Call { callee: Box<Expr>, arguments: Vec<(Option<String>, Expr)> },

    /// An enum variant. eg: `Status::Active`
    EnumVariant {
        enum_name: String,
        variant: String,
    },

    /// Access a property on an object. eg: `user.name`
    MemberAccess { object: Box<Expr>, property: String },

    /// Reading from a list. eg: `list[0]`
    Index { list: Box<Expr>, index: Box<Expr> },

    /// A list literal. eg: `[1, 2, 3]`
    List(Vec<Expr>),

    /// A map literal. eg: `{"key": 42}`
    Map(Vec<(Expr, Expr)>),

    /// A string with embedded expressions. eg: `"Hello ${user.name}"`
    InterpolatedString(Vec<StringSegment>),

    /// Instantiating a struct in memory. eg: `User { name: "Jack Sparrow" }`
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    },

    /// A pattern-matching expression.
    /// eg: `switch status { Active -> 1, Pending -> 0, default -> -1 }`
    Switch {
        target: Box<Expr>,
        arms: Vec<(Expr, Expr)>,
        default_arm: Box<Expr>,
    },

    /// A group of statements that optionally resolves to a value via `yield`.
    /// eg: `{ var x = 10; yield x * 2; }`
    Block(Vec<Stmt>),

    /// The `?` postfix operator. Propagates errors upward.
    /// eg: `divide(10, 0)?`
    PropagateError(Box<Expr>),

    /// The `or { }` fallback block. Handles a failable expression.
    /// eg: `divide(10, 0) or { yield 0; }`
    OrFallback {
        try_expr: Box<Expr>,
        error_binding: Option<String>,  // Some("e") for `or (e) {`, None for `or {`
        fallback: Box<Expr>,
    },

    /// The built-in Error constructor.
    /// eg: `Error(#NotFound, "Item does not exist")`
    Error {
        code: String,
        message: Box<Expr>,
    },
}

/// A statement paired with its exact position in the source code.
#[derive(Debug, Clone)]
pub struct Stmt {
    pub node: StmtNode,
    pub span: Span,
}

impl Stmt {
    pub fn new(node: StmtNode, span: Span) -> Self {
        Self { node, span }
    }
}

impl PartialEq for Stmt {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

/// Represents a distinct action or control flow step.
#[derive(Debug, PartialEq, Clone)]
pub enum StmtNode {
    /// Variable or constant declaration.
    /// eg: `var x: Int = 5;` or `const PI = 3.14;`
    VarDecl {
        is_const: bool,
        name: String,
        type_annotation: Option<Type>,
        initializer: Option<Expr>,
    },

    /// An assignment to an existing location.
    /// eg: `x = 5;`, `user.name = "Stark";`, `list[0] += 1;`
    Assignment {
        target: AssignTarget,
        operator: Token,
        value: Expr,
    },

    /// An expression used as a statement (e.g. a function call whose return value is discarded).
    Expression(Expr),

    /// An if/else branch.
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },

    /// A while loop.
    While {
        condition: Expr,
        body: Box<Stmt>,
    },

    /// A for-each loop. eg: `for item in list { ... }`
    For {
        item_name: String,
        iterator: Expr,
        body: Box<Stmt>,
    },

    /// Immediately exits the current loop.
    Break,

    /// Skips to the next iteration of the current loop.
    Continue,

    /// Returns a value (or nothing) from the current function.
    Return(Option<Expr>),

    /// Exits the current Block expression and produces a value.
    /// eg: `yield x * 2;`
    Yield(Expr),

    /// Schedules a statement to run when the current scope exits.
    /// eg: `defer file.close();`
    Defer(Box<Stmt>),
}

/// A parameter inside a function definition.
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_name: Type,
    pub default_value: Option<Expr>,
    pub span: Span,
}

impl PartialEq for Parameter {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.type_name == other.type_name
            && self.default_value == other.default_value
    }
}

/// A field defined inside a Struct blueprint.
#[derive(Debug, Clone)]
pub struct StructField {
    pub directives: Vec<String>,
    pub is_pub: bool,
    pub name: String,
    pub type_name: Type,
    pub span: Span,
}

impl PartialEq for StructField {
    fn eq(&self, other: &Self) -> bool {
        self.directives == other.directives
            && self.is_pub == other.is_pub
            && self.name == other.name
            && self.type_name == other.type_name
    }
}

/// A function declaration. Shared between top-level functions and impl blocks.
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    /// Directives applied to this function. eg: `@deprecated`
    pub directives: Vec<String>,
    pub is_pub: bool,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

impl PartialEq for FunctionDecl {
    fn eq(&self, other: &Self) -> bool {
        self.directives == other.directives
            && self.is_pub == other.is_pub
            && self.name == other.name
            && self.parameters == other.parameters
            && self.return_type == other.return_type
            && self.body == other.body
    }
}

/// A top-level declaration. The outer shell of the program.
/// Functions, structs, enums, and impls can only live here.
#[derive(Debug, Clone)]
pub enum Item {
    Struct {
        directives: Vec<String>,
        is_pub: bool,
        name: String,
        fields: Vec<StructField>,
        span: Span,
    },

    Enum {
        directives: Vec<String>,
        is_pub: bool,
        name: String,
        values: Vec<String>,
        span: Span,
    },

    Function(FunctionDecl),

    /// Attaches methods to a struct.
    Impl {
        target: String,
        methods: Vec<FunctionDecl>,
        span: Span,
    },

    /// A type alias. eg: `type Port = Int;`
    TypeAlias {
        directives: Vec<String>,
        is_pub: bool,
        name: String,
        target_type: Type,
        span: Span,
    },

    /// An import declaration. eg: `use net::http::{Request, Response};`
    Use {
        path: Vec<String>,
        items: Vec<String>,
        span: Span,
    },
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Item::Struct {
                    directives: d1,
                    is_pub: p1,
                    name: n1,
                    fields: f1,
                    ..
                },
                Item::Struct {
                    directives: d2,
                    is_pub: p2,
                    name: n2,
                    fields: f2,
                    ..
                },
            ) => d1 == d2 && p1 == p2 && n1 == n2 && f1 == f2,
            (
                Item::Enum {
                    directives: d1,
                    is_pub: p1,
                    name: n1,
                    values: v1,
                    ..
                },
                Item::Enum {
                    directives: d2,
                    is_pub: p2,
                    name: n2,
                    values: v2,
                    ..
                },
            ) => d1 == d2 && p1 == p2 && n1 == n2 && v1 == v2,
            (Item::Function(f1), Item::Function(f2)) => f1 == f2,
            (
                Item::Impl {
                    target: t1,
                    methods: m1,
                    ..
                },
                Item::Impl {
                    target: t2,
                    methods: m2,
                    ..
                },
            ) => t1 == t2 && m1 == m2,
            (
                Item::TypeAlias {
                    directives: d1,
                    is_pub: p1,
                    name: n1,
                    target_type: t1,
                    ..
                },
                Item::TypeAlias {
                    directives: d2,
                    is_pub: p2,
                    name: n2,
                    target_type: t2,
                    ..
                },
            ) => d1 == d2 && p1 == p2 && n1 == n2 && t1 == t2,
            (
                Item::Use {
                    path: p1,
                    items: i1,
                    ..
                },
                Item::Use {
                    path: p2,
                    items: i2,
                    ..
                },
            ) => p1 == p2 && i1 == i2,
            _ => false,
        }
    }
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Function(f) => f.span,
            Item::Struct { span, .. } => *span,
            Item::Enum { span, .. } => *span,
            Item::Impl { span, .. } => *span,
            Item::TypeAlias { span, .. } => *span,
            Item::Use { span, .. } => *span,
        }
    }
}

/// The root node of every Antimatter source file.
#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}
