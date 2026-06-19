use crate::lexer::Token;

// =====================================================================
// 1. TYPES (Syntactic Representation)
// =====================================================================

/// Represents a data type as written by the user.
#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    /// A standard type like `Int`, `String`, `Bool`, or `AStruct`.
    Basic(String),
    /// An array type. eg: `[Int]`
    Array(Box<Type>),
    /// A map
    Map {
        key: Box<Type>,
        value: Box<Type>,
    },
    /// A function signature type. eg: `fn(Int, String) -> Bool`
    Function {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },
}

/// Represents the raw datatypes which are hardcoded into Antimatter.
#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

/// Represents a valid memory location on the Left Hand Side (L-Value) of an assignment.
#[derive(Debug, PartialEq, Clone)]
pub enum AssignTarget {
    Identifier(String), //eg: var a = 123;

    //eg: user.name = "Stark";
    MemberAccess {
        object: Box<Expr>,
        property: String,
    },
    //eg: list[5] = 6;
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },
}

/// Represents anything that computes to a value.
#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    ///Normal Core data types.
    Literal(Literal),

    /// Names
    Identifier(String),

    /// The `self` keyword.
    SelfKw,

    /// A single operator applied to a value. (-5, !active)
    Unary { operator: Token, right: Box<Expr> },

    /// Binary Operations (1+1,a*b)
    Binary { left: Box<Expr>, operator: Token, right: Box<Expr> },

    /// Executing a function. eg: `process(val, secure=true)`
    Call { callee: Box<Expr>, arguments: Vec<(Option<String>, Expr)> },

    /// Access a property on an object eg: `user.name`
    MemberAccess { object: Box<Expr>, property: String },

    /// Reading from an array. (List[0])
    Index { array: Box<Expr>, index: Box<Expr> },

    /// List
    Array(Vec<Expr>),

    /// Map
    Map(Vec<(Expr, Expr)>),

    /// A string with variables eg: `"Hello ${user.name}"`
    InterpolatedString(Vec<Expr>),

    /// Instantiating a struct in memory. `User { name: "Jack Sparrow" }`
    StructInit {
        name: Box<Expr>,
        fields: Vec<(String, Expr)>,
    },

    ///Switch
    Switch { target: Box<Expr>, arms: Vec<(Expr, Expr)>, default_arm: Box<Expr> },

    /// A group of statements that optionally resolves to a value via `Yield`.
    ///eg: `{ var x = 10; yield x * 2; }`
    Block(Vec<Stmt>),
}

// STATEMENTS
/// Represents a distinct action or control flow step.
#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    /// Variable Declaration.
    VarDecl {
        is_const: bool,
        name: String,
        type_annotation: Option<Type>,
        initializer: Option<Expr>,
    },

    /// Assignment.
    Assignment {
        target: AssignTarget,
        operator: Token,
        value: Expr,
    },

    /// Expression executing just for function without . so we don't use returned data.
    Expression(Expr),

    /// IF/ELSE
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },

    /// While Loop
    While {
        condition: Expr,
        body: Box<Stmt>,
    },

    /// Immediately exits the current loop.
    Break,
    /// Skips to the next iteration of the current loop.
    Continue,
    /// Return
    Return(Option<Expr>),
    /// Exits the current Block Expression and returns a value to the outer scope.
    Yield(Expr),
    /// Schedules a statement to run at the end of the current scope.
    Defer(Box<Stmt>),
}

// TOP-LEVEL
/// A parameter inside a function definition.
#[derive(Debug, PartialEq, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_name: Type,
    pub default_value: Option<Expr>,
}

/// A property defined inside a Struct blueprint.
#[derive(Debug, PartialEq, Clone)]
pub struct StructField {
    pub is_pub: bool,
    pub name: String,
    pub type_name: Type,
}

///Function. Extracted into its own struct so it
/// can be shared securely between standalone functions and Impl blocks.
#[derive(Debug, PartialEq, Clone)]
pub struct FunctionDecl {
    /// eg: @depricated
    pub directives: Vec<String>,

    pub is_pub: bool,

    /// The name of the function.
    pub name: String,

    /// The input parameters.
    pub parameters: Vec<Parameter>,

    /// The return type.
    pub return_type: Option<Type>,

    /// Code inside the function.
    pub body: Vec<Stmt>,
}

/// An Item is a top-level declaration. It is the outer shell of the program.
/// We cannot put a `struct` or a `fn` inside a `while` loop; they live here.
#[derive(Debug, PartialEq, Clone)]
pub enum Item {
    /// The Struct
    Struct {
        directives: Vec<String>,
        is_pub: bool,
        name: String,
        fields: Vec<StructField>,
    },

    /// Simple Enum
    Enum {
        directives: Vec<String>,
        is_pub: bool,
        name: String,
        values: Vec<String>,
    },

    /// Function Declaration
    Function(FunctionDecl),

    /// Attaches methods to a struct.
    Impl {
        /// The struct this implementation belongs to.
        target: String,
        /// The methods attached to the struct.
        methods: Vec<FunctionDecl>,
    },

    /// Renames a type for convenience. eg: `type Port = Int;`
    TypeAlias {
        directives: Vec<String>,
        is_pub: bool,
        name: String,
        target_type: Type,
    },

    /// Imports code from other modules.
    Use {
        /// Stored as a path array.
        path: Vec<String>,
    },
}

/// The root node of every single Antimatter file.
/// The Parser will read a text file and output exactly one `Program`.
#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}