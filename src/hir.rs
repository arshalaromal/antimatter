//! High-level Intermediate Representation (HIR) for Antimatter.

/// A fully resolved Antimatter type. No Type::Basic(String) ambiguity.
#[derive(Debug, PartialEq, Clone)]
pub enum HirType {
    Int,
    Float,
    Bool,
    String,
    ECode,
    Void,
    List(Box<HirType>),
    Map {
        key: Box<HirType>,
        value: Box<HirType>,
    },
    /// A validated struct name. Checker guarantees it exists.
    Struct(String),
    /// A validated enum name. Checker guarantees it exists.
    Enum(String),
    Function {
        params: Vec<HirType>,
        ret: Box<HirType>,
    },
    Failable(Box<HirType>),
}

// Operators
#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

// Expressions
/// A typed HIR expression.
#[derive(Debug, PartialEq, Clone)]
pub struct HirExpr {
    pub kind: HirExprKind,
    pub ty: HirType,
}

#[derive(Debug, PartialEq, Clone)]
pub enum HirExprKind {
    //Literals
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    ECodeLit(String),

    /// A resolved variable reference.
    Var(String),

    /// `self` inside an impl method. ty is always HirType::Struct(owner).
    SelfRef,

    EnumVariant {
        enum_name: String,
        variant: String,
    },

    // Operators
    Unary {
        op: UnaryOp,
        right: Box<HirExpr>,
    },
    Binary {
        left: Box<HirExpr>,
        op: BinaryOp,
        right: Box<HirExpr>,
    },

    // Calls
    /// A normal resolved function call by name. eg: `add(1, 2)`
    /// Named args are gone. positional only.
    DirectCall {
        name: String,
        args: Vec<HirExpr>,
    },

    /// Calling through a variable that holds a function value.
    /// eg: `var f: fn(Int) -> Bool = is_valid; f(5);`
    /// Only emitted when callee resolves to a variable of Function type.
    IndirectCall {
        callee: Box<HirExpr>,
        args: Vec<HirExpr>,
    },

    /// A method call on a receiver.
    /// eg: `user.greet("hello")` → receiver=user, method="greet", args=\["hello"]
    ///
    /// Also covers desugared shorthand init:
    /// eg: `User(a, b)` → receiver=HirType::Struct("User"), method="new", args=[a, b]
    MethodCall {
        receiver: Box<HirExpr>,
        method: String,
        args: Vec<HirExpr>,
    },

    //Data access
    Field {
        object: Box<HirExpr>,
        name: String,
    },

    /// Works on both List and Map. Type checker validates the index type.
    /// List: index must be Int. Map: index must match key type.
    Index {
        collection: Box<HirExpr>,
        index: Box<HirExpr>,
    },

    //Collection literals
    List(Vec<HirExpr>),
    Map(Vec<(HirExpr, HirExpr)>),

    /// Always the full named-field form.
    /// `User(a, b)` shorthand is desugared to MethodCall { method: "new", ... }.
    StructInit {
        struct_name: String,
        fields: Vec<(String, HirExpr)>,
    },

    //Block expression
    /// A group of statements that optionally produces a value via `yield`.
    /// If yield_expr is None, the block is Void.
    /// If yield_expr is Some(e), the block's ty = e's ty.
    Block {
        stmts: Vec<HirStmt>,
        yield_expr: Option<Box<HirExpr>>,
    },

    // Pattern matching
    /// target: the expression being matched on.
    /// arms: Vec of (pattern_value, result_value).
    /// default_arm: result when no arm matches.
    Switch {
        target: Box<HirExpr>,
        arms: Vec<(HirExpr, HirExpr)>,
        default_arm: Box<HirExpr>,
    },

    //Error handling
    /// `expr?` - if expr is an error, return that error from the current function.
    /// If expr succeeds, unwrap and produce the inner value.
    /// Error propagates up the call stack until someone handles it with OrFallback.
    PropagateError(Box<HirExpr>),

    /// `expr or { fallback }` or `expr or (e) { fallback }`
    ///
    /// The source `or (e) { body }` is lowered so that the fallback block's
    /// first statement is an injected VarDecl:
    ///   VarDecl { name: "e", ty: HirType::ECode, init: <extracted error> }
    /// followed by the original body. The HIR does this.
    ///
    /// `or { body }` (no binding) has no injected VarDecl.
    OrFallback {
        try_expr: Box<HirExpr>,
        fallback: Box<HirExpr>,
    },

    /// `Error(#NotFound, "message")` - constructs a failable error value.
    ErrorVal {
        code: String,
        message: Box<HirExpr>,
    },

    //String interpolation
    Format(Vec<HirFormatPart>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum HirFormatPart {
    Lit(String),
    Val(HirExpr),
}


/// HIR statements.
#[derive(Debug, PartialEq, Clone)]
pub enum HirStmt {
    /// ty is always resolved - never unknown.
    VarDecl {
        is_const: bool,
        name: String,
        ty: HirType,
        init: Option<HirExpr>,
    },

    Assign {
        target: HirAssignTarget,
        value: HirExpr,
    },

    Expression(HirExpr),

    If {
        condition: HirExpr,
        then_block: Vec<HirStmt>,
        else_block: Option<Vec<HirStmt>>,
    },

    /// All loops in HIR are While.
    While {
        condition: HirExpr,
        body: Vec<HirStmt>,
    },

    Return(Option<HirExpr>),
    Yield(HirExpr),
    Break,
    Continue,

    Defer(Box<HirStmt>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum HirAssignTarget {
    Var(String),
    Field {
        object: Box<HirExpr>,
        name: String,
    },
    Index {
        collection: Box<HirExpr>,
        index: Box<HirExpr>,
    },
}

// Functions, structs, enums, type aliases

/// A resolved parameter. No default values. defaults are resolved at call
/// sites by the lowering pass and injected as explicit args.
#[derive(Debug, PartialEq, Clone)]
pub struct HirParam {
    pub name: String,
    pub ty: HirType,
}

/// A fully resolved function.
///
/// For impl methods, self is always the explicit first param:
///   HirParam { name: "self", ty: HirType::Struct(owner_name) }
///
/// return_type is never None - void functions use HirType::Void.
///
/// is_extern: true if declared with @extern - body will be empty.
///
/// directives: everything else (@deprecated, @experimental, etc.)
#[derive(Debug, PartialEq, Clone)]
pub struct HirFunction {
    pub name: String,
    pub is_pub: bool,
    pub is_extern: bool,
    pub directives: Vec<String>,
    pub params: Vec<HirParam>,
    pub return_type: HirType,
    pub body: Vec<HirStmt>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct HirStructField {
    pub name: String,
    pub ty: HirType,
    pub is_pub: bool,
    pub directives: Vec<String>,
}

/// A resolved struct. Impl block methods are merged in during lowering.
/// There are no separate Impl items in HIR.
#[derive(Debug, PartialEq, Clone)]
pub struct HirStruct {
    pub name: String,
    pub is_pub: bool,
    pub directives: Vec<String>,
    pub fields: Vec<HirStructField>,
    /// All methods from all `impl StructName { }` blocks, merged and lowered.
    pub methods: Vec<HirFunction>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct HirEnum {
    pub name: String,
    pub is_pub: bool,
    pub directives: Vec<String>,
    pub variants: Vec<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct HirTypeAlias {
    pub name: String,
    pub is_pub: bool,
    pub directives: Vec<String>,
    pub target: HirType,
}

// Program root

/// A top-level HIR item.
#[derive(Debug, PartialEq, Clone)]
pub enum HirItem {
    Struct(HirStruct),
    Enum(HirEnum),
    Function(HirFunction),
    TypeAlias(HirTypeAlias),
}

/// Root of the HIR tree. One per source file.
#[derive(Debug, PartialEq, Clone)]
pub struct HirProgram {
    pub items: Vec<HirItem>,
}