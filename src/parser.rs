use crate::ast::*;
use crate::error::{CompilerError, CompilerResult};
use crate::lexer::{SpannedToken, Token};

/// The Recursive Descent Parser for Antimatter.
/// Consumes a flat `Vec<SpannedToken>` and builds a typed AST.
///
/// Every method that can encounter bad syntax returns `CompilerResult<T>`
pub struct Parser {
    tokens: Vec<SpannedToken>,
    current: usize,
}

/// Operator precedence levels, lowest to highest.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum Precedence {
    None = 0,
    OrFallback = 1, // or { }
    Or = 2,         // ||
    And = 3,        // &&
    BitOr = 4,      // |
    BitXor = 5,     // ^
    BitAnd = 6,     // &
    Equality = 7,   // == !=
    Comparison = 8, // < > <= >=
    Shift = 9,      // << >>
    Term = 10,      // + -
    Factor = 11,    // * / %
    Unary = 12,     // ! - ~
    Call = 13,      // . () [] ?
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, current: 0 }
    }

    //CURSOR & TOKEN MANAGEMENT
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn peek(&self) -> Option<&SpannedToken> {
        if self.is_at_end() {
            None
        } else {
            Some(&self.tokens[self.current])
        }
    }

    fn previous(&self) -> &SpannedToken {
        &self.tokens[self.current - 1]
    }

    fn check(&self, token_type: &Token) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek().map(|st| &st.token) == Some(token_type)
    }

    /// Returns the line/column of the current (not-yet-consumed) token.
    fn current_pos(&self) -> (usize, usize) {
        if self.is_at_end() {
            if self.tokens.is_empty() {
                return (1, 1);
            }
            let span = &self.tokens.last().unwrap().span;
            return (span.line, span.column);
        }
        let span = &self.peek().unwrap().span;
        (span.line, span.column)
    }

    fn advance(&mut self) -> &SpannedToken {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    /// Tries each token in order. Advances and returns `true` on the first match.
    fn match_tokens(&mut self, tokens: &[Token]) -> bool {
        for token in tokens {
            if self.check(token) {
                self.advance();
                return true;
            }
        }
        false
    }

    /// Advances past `expected` or returns an error describing what was found instead.
    fn consume(&mut self, expected: Token, message: &str) -> CompilerResult<Token> {
        if self.check(&expected) {
            return Ok(self.advance().token.clone());
        }
        let (line, col) = self.current_pos();
        let got = match self.peek() {
            Some(st) => format!("{:?}", st.token),
            None => "end of file".to_string(),
        };
        Err(CompilerError::new(
            line,
            col,
            format!("{} (expected {:?}, found {})", message, expected, got),
        ))
    }

    /// Advances past the current `Ident` token and returns its name.
    /// Peeks before advancing so the cursor never moves on failure.
    fn consume_ident(&mut self, message: &str) -> CompilerResult<String> {
        if let Some(st) = self.peek() {
            if let Token::Ident(name) = &st.token {
                let name_clone = name.clone();
                self.advance();
                return Ok(name_clone);
            }
        }
        let (line, col) = self.current_pos();
        Err(CompilerError::new(line, col, message.to_string()))
    }

    /// Parses zero or more `@directive` annotations followed by an optional `pub`.
    fn parse_modifiers(&mut self) -> CompilerResult<(Vec<String>, bool)> {
        let mut directives = Vec::new();
        while self.match_tokens(&[Token::At]) {
            directives.push(self.consume_ident("Expected directive name after '@'")?);
        }
        let is_pub = self.match_tokens(&[Token::Pub]);
        Ok((directives, is_pub))
    }

    // TOP-LEVEL DECLARATIONS
    pub fn parse(&mut self) -> CompilerResult<Program> {
        let mut items = Vec::new();
        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    /// Routes the parser to the correct top-level declaration.
    fn parse_item(&mut self) -> CompilerResult<Item> {
        if self.match_tokens(&[Token::Use]) {
            return self.parse_use();
        }
        if self.match_tokens(&[Token::Impl]) {
            return self.parse_impl();
        }

        let (directives, is_pub) = self.parse_modifiers()?;

        if self.match_tokens(&[Token::Type]) {
            return self.parse_type_alias(directives, is_pub);
        }
        if self.match_tokens(&[Token::Struct]) {
            return self.parse_struct(directives, is_pub);
        }
        if self.match_tokens(&[Token::Enum]) {
            return self.parse_enum(directives, is_pub);
        }
        if self.match_tokens(&[Token::Fn]) {
            return Ok(Item::Function(
                self.parse_function_decl(directives, is_pub)?,
            ));
        }

        let (line, col) = self.current_pos();
        Err(CompilerError::new(
            line,
            col,
            format!(
                "Unexpected token at top level: {:?}",
                self.peek().map(|t| &t.token)
            ),
        ))
    }

    /// Parses `use`
    fn parse_use(&mut self) -> CompilerResult<Item> {
        let mut path = Vec::new();
        let mut items = Vec::new();

        loop {
            if self.match_tokens(&[Token::LBrace]) {
                loop {
                    if self.check(&Token::RBrace) {
                        break;
                    } // trailing comma
                    items.push(self.consume_ident("Expected item name in use block")?);
                    if !self.match_tokens(&[Token::Comma]) {
                        break;
                    }
                }
                self.consume(Token::RBrace, "Expected '}' after use items")?;
                break;
            }

            let name = self.consume_ident("Expected module or item name in 'use' statement")?;
            if self.match_tokens(&[Token::DoubleColon]) {
                path.push(name);
            } else {
                items.push(name);
                break;
            }
        }

        self.consume(Token::SemiColon, "Expected ';' after 'use' statement")?;
        Ok(Item::Use { path, items })
    }

    ///Parses Type Aliases.
    ///
    ///eg: `type Port = Int;`
    fn parse_type_alias(&mut self, directives: Vec<String>, is_pub: bool) -> CompilerResult<Item> {
        let name = self.consume_ident("Expected name for type alias")?;
        self.consume(Token::Assign, "Expected '=' in type alias declaration")?;
        let target_type = self.parse_type()?;
        self.consume(Token::SemiColon, "Expected ';' after type alias")?;
        Ok(Item::TypeAlias {
            directives,
            is_pub,
            name,
            target_type,
        })
    }

    /// Parses Enums
    fn parse_enum(&mut self, directives: Vec<String>, is_pub: bool) -> CompilerResult<Item> {
        let name = self.consume_ident("Expected name for enum declaration")?;
        self.consume(Token::LBrace, "Expected '{' before enum values")?;
        let mut values = Vec::new();

        if !self.check(&Token::RBrace) {
            loop {
                if self.check(&Token::RBrace) {
                    break;
                } // trailing comma
                values.push(self.consume_ident("Expected enum value name")?);
                if !self.match_tokens(&[Token::Comma]) {
                    break;
                }
            }
        }

        self.consume(Token::RBrace, "Expected '}' after enum values")?;
        Ok(Item::Enum {
            directives,
            is_pub,
            name,
            values,
        })
    }

    /// Parses a struct definition with optional per-field directives and visibility.
    fn parse_struct(&mut self, directives: Vec<String>, is_pub: bool) -> CompilerResult<Item> {
        let name = self.consume_ident("Expected struct name")?;
        self.consume(Token::LBrace, "Expected '{' before struct fields")?;
        let mut fields = Vec::new();

        if !self.check(&Token::RBrace) {
            loop {
                if self.check(&Token::RBrace) {
                    break;
                } // trailing comma

                let (field_directives, field_is_pub) = self.parse_modifiers()?;
                let field_name = self.consume_ident("Expected field name")?;
                self.consume(Token::Colon, "Expected ':' after field name")?;
                let field_type = self.parse_type()?;

                fields.push(StructField {
                    directives: field_directives,
                    is_pub: field_is_pub,
                    name: field_name,
                    type_name: field_type,
                });
                if !self.match_tokens(&[Token::Comma]) {
                    break;
                }
            }
        }

        self.consume(Token::RBrace, "Expected '}' after struct fields")?;
        Ok(Item::Struct {
            directives,
            is_pub,
            name,
            fields,
        })
    }

    /// Parses a complete function
    fn parse_function_decl(
        &mut self,
        directives: Vec<String>,
        is_pub: bool,
    ) -> CompilerResult<FunctionDecl> {
        let name = self.consume_ident("Expected function name")?;
        self.consume(Token::LParen, "Expected '(' after function name")?;
        let mut parameters = Vec::new();

        if !self.check(&Token::RParen) {
            loop {
                if self.check(&Token::RParen) {
                    break;
                } // trailing comma

                let param_name = self.consume_ident("Expected parameter name")?;
                self.consume(Token::Colon, "Expected ':' after parameter name")?;
                let param_type = self.parse_type()?;

                let mut default_value = None;
                if self.match_tokens(&[Token::Assign]) {
                    default_value = Some(self.parse_expr(Precedence::None)?);
                }

                parameters.push(Parameter {
                    name: param_name,
                    type_name: param_type,
                    default_value,
                });
                if !self.match_tokens(&[Token::Comma]) {
                    break;
                }
            }
        }

        self.consume(Token::RParen, "Expected ')' after parameters")?;

        let mut return_type = None;
        if self.match_tokens(&[Token::Arrow]) {
            return_type = Some(self.parse_type()?);
        }

        let body = if directives.contains(&"extern".to_string()) {
            self.consume(Token::SemiColon, "Expected ';' after @extern function declaration")?;
            vec![]
        } else {
            self.consume(Token::LBrace, "Expected '{' before function body")?;
            self.parse_block()?
        };

        Ok(FunctionDecl {
            directives,
            is_pub,
            name,
            parameters,
            return_type,
            body,
        })
    }

    /// Parses `impl`
    fn parse_impl(&mut self) -> CompilerResult<Item> {
        let target = self.consume_ident("Expected struct name after 'impl'")?;
        self.consume(Token::LBrace, "Expected '{' before impl methods")?;
        let mut methods = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let (directives, is_pub) = self.parse_modifiers()?;
            self.consume(Token::Fn, "Expected 'fn' inside impl block")?;
            methods.push(self.parse_function_decl(directives, is_pub)?);
        }

        self.consume(Token::RBrace, "Expected '}' after impl methods")?;
        Ok(Item::Impl { target, methods })
    }

    //TYPE PARSER
    /// Recursively parses any Antimatter type expression.
    fn parse_type(&mut self) -> CompilerResult<Type> {
        // Failable wrapper: `!Type`
        if self.match_tokens(&[Token::Not]) {
            let inner = self.parse_type()?;
            if matches!(inner, Type::Failable(_)) {
                let (line, col) = self.current_pos();
                return Err(CompilerError::new(
                    line,
                    col,
                    "Redundant '!' — type is already failable. Write '!T', not '!!T'.".to_string(),
                ));
            }
            return Ok(Type::Failable(Box::new(inner)));
        }
        // List type: `[Int]`
        if self.match_tokens(&[Token::LBracket]) {
            let inner = self.parse_type()?;
            self.consume(Token::RBracket, "Expected ']' after list type")?;
            return Ok(Type::List(Box::new(inner)));
        }
        // Map type: `{String: Int}`
        if self.match_tokens(&[Token::LBrace]) {
            let key = self.parse_type()?;
            self.consume(Token::Colon, "Expected ':' in map type definition")?;
            let value = self.parse_type()?;
            self.consume(Token::RBrace, "Expected '}' after map type")?;
            return Ok(Type::Map {
                key: Box::new(key),
                value: Box::new(value),
            });
        }
        // Function type: `fn(Int, String) -> Bool`
        if self.match_tokens(&[Token::Fn]) {
            self.consume(Token::LParen, "Expected '(' after 'fn' in function type")?;
            let mut parameters = Vec::new();
            if !self.check(&Token::RParen) {
                loop {
                    if self.check(&Token::RParen) {
                        break;
                    }
                    parameters.push(self.parse_type()?);
                    if !self.match_tokens(&[Token::Comma]) {
                        break;
                    }
                }
            }
            self.consume(Token::RParen, "Expected ')' after function type parameters")?;
            self.consume(Token::Arrow, "Expected '->' after function type parameters")?;
            return Ok(Type::Function {
                parameters,
                return_type: Box::new(self.parse_type()?),
            });
        }
        // Named type: `Int`, `String`, `Struct`
        let name = self.consume_ident("Expected a valid type name")?;
        match name.as_str() {
            "Void" => Ok(Type::Void),
            "ECode" => Ok(Type::ECode),
            _ => Ok(Type::Basic(name)),
        }
    }

    //STATEMENT PARSER
    /// Routes to the correct statement parser.
    fn parse_stmt(&mut self) -> CompilerResult<Stmt> {
        if self.match_tokens(&[Token::Var]) {
            return self.parse_var_decl(false);
        }
        if self.match_tokens(&[Token::Const]) {
            return self.parse_var_decl(true);
        }
        if self.match_tokens(&[Token::If]) {
            return self.parse_if_stmt();
        }
        if self.match_tokens(&[Token::While]) {
            return self.parse_while_stmt();
        }
        if self.match_tokens(&[Token::For]) {
            return self.parse_for_stmt();
        }
        if self.match_tokens(&[Token::Return]) {
            return self.parse_return_stmt();
        }
        if self.match_tokens(&[Token::Break]) {
            self.consume(Token::SemiColon, "Expected ';' after 'break'")?;
            return Ok(Stmt::Break);
        }
        if self.match_tokens(&[Token::Continue]) {
            self.consume(Token::SemiColon, "Expected ';' after 'continue'")?;
            return Ok(Stmt::Continue);
        }
        if self.match_tokens(&[Token::Yield]) {
            return self.parse_yield_stmt();
        }
        if self.match_tokens(&[Token::Defer]) {
            return self.parse_defer_stmt();
        }

        self.parse_expression_stmt()
    }

    /// Parses variable/constant declarations.
    fn parse_var_decl(&mut self, is_const: bool) -> CompilerResult<Stmt> {
        let name = self.consume_ident("Expected variable name")?;

        let mut type_annotation = None;
        if self.match_tokens(&[Token::Colon]) {
            type_annotation = Some(self.parse_type()?);
        }

        let mut initializer = None;
        if self.match_tokens(&[Token::Assign]) {
            initializer = Some(self.parse_expr(Precedence::None)?);
        }

        self.consume(Token::SemiColon, "Expected ';' after variable declaration")?;
        Ok(Stmt::VarDecl {
            is_const,
            name,
            type_annotation,
            initializer,
        })
    }

    /// Parses `if`
    fn parse_if_stmt(&mut self) -> CompilerResult<Stmt> {
        let condition = self.parse_expr_no_struct(Precedence::None)?;

        let then_branch = if self.match_tokens(&[Token::LBrace]) {
            Box::new(Stmt::Expression(Expr::Block(self.parse_block()?)))
        } else {
            Box::new(self.parse_stmt()?)
        };

        let mut else_branch = None;
        if self.match_tokens(&[Token::Else]) {
            if self.match_tokens(&[Token::If]) {
                else_branch = Some(Box::new(self.parse_if_stmt()?));
            } else if self.match_tokens(&[Token::LBrace]) {
                else_branch = Some(Box::new(Stmt::Expression(Expr::Block(self.parse_block()?))));
            } else {
                else_branch = Some(Box::new(self.parse_stmt()?));
            }
        }

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    /// Parses `while`
    fn parse_while_stmt(&mut self) -> CompilerResult<Stmt> {
        let condition = self.parse_expr_no_struct(Precedence::None)?;

        let body = if self.match_tokens(&[Token::LBrace]) {
            Box::new(Stmt::Expression(Expr::Block(self.parse_block()?)))
        } else {
            Box::new(self.parse_stmt()?)
        };

        Ok(Stmt::While { condition, body })
    }

    /// Parses `for` statement
    fn parse_for_stmt(&mut self) -> CompilerResult<Stmt> {
        let item_name = self.consume_ident("Expected iterator variable name after 'for'")?;
        self.consume(Token::In, "Expected 'in' after for loop variable")?;

        let iterator = self.parse_expr_no_struct(Precedence::None)?;

        let body = if self.match_tokens(&[Token::LBrace]) {
            Box::new(Stmt::Expression(Expr::Block(self.parse_block()?)))
        } else {
            Box::new(self.parse_stmt()?)
        };

        Ok(Stmt::For {
            item_name,
            iterator,
            body,
        })
    }

    /// Parses `return;` or `return expr;`
    fn parse_return_stmt(&mut self) -> CompilerResult<Stmt> {
        let value = if !self.check(&Token::SemiColon) {
            Some(self.parse_expr(Precedence::None)?)
        } else {
            None
        };
        self.consume(Token::SemiColon, "Expected ';' after return statement")?;
        Ok(Stmt::Return(value))
    }

    /// Parses `yield expr;`
    fn parse_yield_stmt(&mut self) -> CompilerResult<Stmt> {
        let value = self.parse_expr(Precedence::None)?;
        self.consume(Token::SemiColon, "Expected ';' after yield expression")?;
        Ok(Stmt::Yield(value))
    }

    /// Parses `defer expr;`
    fn parse_defer_stmt(&mut self) -> CompilerResult<Stmt> {
        Ok(Stmt::Defer(Box::new(self.parse_stmt()?)))
    }

    /// Parses an expression statement or assignment.
    /// eg: `foo();`, `x = 5;`, `list[0] += 1;`
    fn parse_expression_stmt(&mut self) -> CompilerResult<Stmt> {
        let expr = self.parse_expr(Precedence::None)?;

        if self.match_tokens(&[
            Token::Assign,
            Token::PlusEq,
            Token::MinusEq,
            Token::StarEq,
            Token::SlashEq,
            Token::BitAndEq,
            Token::BitOrEq,
            Token::BitXorEq,
            Token::ShlEq,
            Token::ShrEq,
        ]) {
            let operator = self.previous().token.clone();

            let target = match expr {
                Expr::Identifier(name) => AssignTarget::Identifier(name),
                Expr::MemberAccess { object, property } => {
                    AssignTarget::MemberAccess { object, property }
                }
                Expr::Index { list, index } => AssignTarget::Index { list, index },
                _ => {
                    let (line, col) = self.current_pos();
                    return Err(CompilerError::new(
                        line, col,
                        "Invalid assignment target. Must be a variable, field access, or index expression.".to_string(),
                    ));
                }
            };

            let value = self.parse_expr(Precedence::None)?;
            self.consume(Token::SemiColon, "Expected ';' after assignment")?;
            return Ok(Stmt::Assignment {
                target,
                operator,
                value,
            });
        }

        self.consume(Token::SemiColon, "Expected ';' after expression statement")?;
        Ok(Stmt::Expression(expr))
    }

    //EXPRESSION PARSER (PRATT)

    /// Maps the current token to its infix binding power.
    /// Returns `Precedence::None` for tokens that are not infix operators.
    fn peek_binding_power(&self) -> Precedence {
        if self.is_at_end() {
            return Precedence::None;
        }
        match self.peek().unwrap().token {
            Token::OrKw => Precedence::OrFallback,
            Token::Or => Precedence::Or,
            Token::And => Precedence::And,
            Token::BitOr => Precedence::BitOr,
            Token::BitXor => Precedence::BitXor,
            Token::BitAnd => Precedence::BitAnd,
            Token::EqEq | Token::NotEq => Precedence::Equality,
            Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => Precedence::Comparison,
            Token::Shl | Token::Shr => Precedence::Shift,
            Token::Plus | Token::Minus => Precedence::Term,
            Token::Star | Token::Slash | Token::Percent => Precedence::Factor,
            Token::LParen | Token::Dot | Token::LBracket | Token::Question | Token::DoubleColon => {
                Precedence::Call
            }
            _ => Precedence::None,
        }
    }

    /// Full expression parser. Struct initializers are allowed.
    fn parse_expr(&mut self, precedence: Precedence) -> CompilerResult<Expr> {
        self.parse_expr_inner(precedence, true)
    }

    /// Used in `if`/`while`/`for`/`switch` conditions to prevent `if name {`
    /// from being misread as `if (name {})`. Struct initializers are not allowed.
    fn parse_expr_no_struct(&mut self, precedence: Precedence) -> CompilerResult<Expr> {
        self.parse_expr_inner(precedence, false)
    }

    /// The Pratt parsing master loop.
    fn parse_expr_inner(
        &mut self,
        precedence: Precedence,
        allow_struct: bool,
    ) -> CompilerResult<Expr> {
        let mut left = self.parse_prefix(allow_struct)?;

        while precedence < self.peek_binding_power() {
            // Capture binding power BEFORE consuming the operator so left-associativity
            // is correct. (Consuming first and then peeking returns the next token's
            // power, which makes every binary operator right-associative.)
            let infix_bp = self.peek_binding_power();
            left = self.parse_infix(left, infix_bp, allow_struct)?;
        }

        Ok(left)
    }

    /// Parses a prefix (the start of any expression).
    fn parse_prefix(&mut self, allow_struct: bool) -> CompilerResult<Expr> {
        let token = self.advance().token.clone();

        match token {
            //Literals
            Token::IntLit(v) => Ok(Expr::Literal(Literal::Int(v))),
            Token::FloatLit(v) => Ok(Expr::Literal(Literal::Float(v))),
            Token::StringLit(v) | Token::RawString(v) => Ok(Expr::Literal(Literal::String(v))),

            Token::InterpolatedStringLit(parts) => {
                let mut segments = Vec::new();
                for part in parts {
                    match part {
                        crate::lexer::StringPart::Literal(s) => {
                            segments.push(StringSegment::Literal(s));
                        }
                        crate::lexer::StringPart::ExprSource(src) => {
                            // Re lex and reparse the expression source found inside ${}.
                            let inner_tokens = crate::lexer::tokenize(&src)?;
                            let mut inner_parser = Parser::new(inner_tokens);
                            let expr = inner_parser.parse_expr(Precedence::None)?;
                            segments.push(StringSegment::Expr(Box::new(expr)));
                        }
                    }
                }
                Ok(Expr::InterpolatedString(segments))
            }

            Token::True => Ok(Expr::Literal(Literal::Bool(true))),
            Token::False => Ok(Expr::Literal(Literal::Bool(false))),
            Token::SelfKw => Ok(Expr::SelfKw),

            //Error constructor
            Token::Error => self.parse_error_primitive(),
            Token::Hash => {
                let code = self.consume_ident("Expected error code name after '#'")?;
                Ok(Expr::ECodeLit(code))
            }

            //Identifier or struct init
            Token::Ident(name) => {
                // Struct initializer
                // Only allowed when not in a no-struct context.
                if allow_struct && self.check(&Token::LBrace) {
                    return self.parse_struct_init(name);
                }
                Ok(Expr::Identifier(name))
            }

            //Unary prefix operators
            Token::Minus | Token::Not | Token::BitNot => {
                let right = self.parse_expr_inner(Precedence::Unary, allow_struct)?;
                Ok(Expr::Unary {
                    operator: token,
                    right: Box::new(right),
                })
            }

            //Grouped expression
            Token::LParen => {
                let expr = self.parse_expr(Precedence::None)?;
                self.consume(Token::RParen, "Expected ')' after grouped expression")?;
                Ok(expr)
            }

            //Block expression or Map literal
            Token::LBrace => {
                if self.is_map_literal() {
                    return self.parse_map_literal();
                }
                Ok(Expr::Block(self.parse_block()?))
            }

            //List
            Token::LBracket => {
                let mut items = Vec::new();
                if !self.check(&Token::RBracket) {
                    loop {
                        if self.check(&Token::RBracket) {
                            break;
                        }
                        items.push(self.parse_expr(Precedence::None)?);
                        if !self.match_tokens(&[Token::Comma]) {
                            break;
                        }
                    }
                }
                self.consume(Token::RBracket, "Expected ']' after list elements")?;
                Ok(Expr::List(items))
            }

            //Switch expression
            Token::Switch => {
                let target = Box::new(self.parse_expr_no_struct(Precedence::None)?);
                self.consume(Token::LBrace, "Expected '{' after switch target")?;

                let mut arms = Vec::new();
                let mut default_arm = None;

                while !self.check(&Token::RBrace) && !self.is_at_end() {
                    if self.match_tokens(&[Token::Default]) {
                        self.consume(Token::Arrow, "Expected '->' after 'default'")?;
                        default_arm = Some(Box::new(self.parse_expr(Precedence::None)?));
                        self.match_tokens(&[Token::Comma]);
                        break;
                    }
                    let case_expr = self.parse_expr(Precedence::None)?;
                    self.consume(Token::Arrow, "Expected '->' after switch case")?;
                    let result_expr = self.parse_expr(Precedence::None)?;
                    arms.push((case_expr, result_expr));
                    if !self.match_tokens(&[Token::Comma]) {
                        break;
                    }
                }

                // Check for missing `default` BEFORE consuming `}` so the error
                // points at the end of the switch body, not the next line.
                if default_arm.is_none() {
                    let (line, col) = self.current_pos();
                    return Err(CompilerError::new(
                        line,
                        col,
                        "Switch expressions must have a 'default ->' arm.".to_string(),
                    ));
                }

                self.consume(Token::RBrace, "Expected '}' after switch arms")?;
                Ok(Expr::Switch {
                    target,
                    arms,
                    default_arm: default_arm.unwrap(),
                })
            }

            _ => {
                let (line, col) = self.current_pos();
                Err(CompilerError::new(
                    line,
                    col,
                    format!("Expected an expression, but found {:?}", token),
                ))
            }
        }
    }

    /// Parses an infix or postfix operator given the left-hand expression.
    fn parse_infix(
        &mut self,
        left: Expr,
        precedence: Precedence,
        allow_struct: bool,
    ) -> CompilerResult<Expr> {
        let operator = self.advance().token.clone();

        match operator {
            //Standard binary operators
            Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Percent
            | Token::EqEq
            | Token::NotEq
            | Token::Lt
            | Token::Gt
            | Token::LtEq
            | Token::GtEq
            | Token::And
            | Token::Or
            | Token::BitAnd
            | Token::BitOr
            | Token::BitXor
            | Token::Shl
            | Token::Shr => {
                // Pass the captured precedence (not a fresh peek) for correct
                // left-associativity.
                let right = self.parse_expr_inner(precedence, allow_struct)?;
                Ok(Expr::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                })
            }

            //Error propagation postfix
            Token::Question => Ok(Expr::PropagateError(Box::new(left))),

            // ::
            Token::DoubleColon => {
                let variant = self.consume_ident("Expected enum variant name after '::'")?;
                match left {
                    Expr::Identifier(enum_name) => Ok(Expr::EnumVariant { enum_name, variant }),
                    _ => {
                        let (line, col) = self.current_pos();
                        Err(CompilerError::new(
                            line,
                            col,
                            "Left side of '::' must be an enum name".to_string(),
                        ))
                    }
                }
            }

            //`or { }` fallback block
            Token::OrKw => {
                let error_binding = if self.match_tokens(&[Token::LParen]) {
                    let name = self.consume_ident("Expected error variable name")?;
                    if self.check(&Token::Comma) {
                        let (line, col) = self.current_pos();
                        return Err(CompilerError::new(
                            line,
                            col,
                            "'or' only accepts one error binding: or (e) { }".to_string(),
                        ));
                    }
                    self.consume(Token::RParen, "Expected ')' after error binding")?;
                    Some(name)
                } else {
                    None
                };

                let fallback = if self.check(&Token::LBrace) {
                    self.advance();
                    Expr::Block(self.parse_block()?)
                } else {
                    let expr = self.parse_expr(Precedence::None)?;
                    Expr::Block(vec![Stmt::Yield(expr)])
                };

                Ok(Expr::OrFallback {
                    try_expr: Box::new(left),
                    error_binding,
                    fallback: Box::new(fallback),
                })
            }

            //Function call
            Token::LParen => Ok(Expr::Call {
                callee: Box::new(left),
                arguments: self.parse_call_arguments()?,
            }),

            //Member access
            Token::Dot => {
                let property = self.consume_ident("Expected property name after '.'")?;
                Ok(Expr::MemberAccess {
                    object: Box::new(left),
                    property,
                })
            }

            //Index access
            Token::LBracket => {
                let index = self.parse_expr(Precedence::None)?;
                self.consume(Token::RBracket, "Expected ']' after index")?;
                Ok(Expr::Index {
                    list: Box::new(left),
                    index: Box::new(index),
                })
            }

            _ => unreachable!("Token not in peek_binding_power but reached parse_infix"),
        }
    }

    /// Parses statements until `}` is found, then consumes the `}`.
    /// The opening `{` must already be consumed by the caller.
    fn parse_block(&mut self) -> CompilerResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }
        self.consume(Token::RBrace, "Expected '}' at end of block")?;
        Ok(stmts)
    }

    /// Parses the argument list inside a function call.
    /// Supports positional `call(1, 2)` and named `call(port=8080)` arguments.
    fn parse_call_arguments(&mut self) -> CompilerResult<Vec<(Option<String>, Expr)>> {
        let mut arguments = Vec::new();
        let mut saw_named = false;

        if !self.check(&Token::RParen) {
            loop {
                if self.check(&Token::RParen) {
                    break;
                }

                let mut name = None;
                let mut value = self.parse_expr(Precedence::None)?;

                if self.match_tokens(&[Token::Assign]) {
                    if let Expr::Identifier(arg_name) = value {
                        saw_named = true;
                        name = Some(arg_name);
                        value = self.parse_expr(Precedence::None)?;
                    } else {
                        let (line, col) = self.current_pos();
                        return Err(CompilerError::new(
                            line,
                            col,
                            "Left side of '=' in a named argument must be a plain identifier."
                                .to_string(),
                        ));
                    }
                } else if saw_named {
                    let (line, col) = self.current_pos();
                    return Err(CompilerError::new(
                        line,
                        col,
                        "Positional argument cannot follow a named argument.".to_string(),
                    ));
                }

                arguments.push((name, value));
                if !self.match_tokens(&[Token::Comma]) {
                    break;
                }
            }
        }

        self.consume(Token::RParen, "Expected ')' after function arguments")?;
        Ok(arguments)
    }
    /// Parses the built-in Error constructor: `Error(#Code, "message")`
    fn parse_error_primitive(&mut self) -> CompilerResult<Expr> {
        self.consume(Token::LParen, "Expected '(' after 'Error'")?;
        self.consume(Token::Hash, "Expected '#' before error code")?;
        let code = self.consume_ident("Expected error code identifier")?;
        self.consume(Token::Comma, "Expected ',' after error code")?;
        let message = self.parse_expr(Precedence::None)?;
        self.consume(Token::RParen, "Expected ')' after error message")?;
        Ok(Expr::Error {
            code,
            message: Box::new(message),
        })
    }

    /// Parses a struct initializer: `User { name: "Stark", age: 280 }`
    /// The struct name has already been consumed by the caller.
    fn parse_struct_init(&mut self, name: String) -> CompilerResult<Expr> {
        self.consume(Token::LBrace, "Expected '{' in struct initializer")?;
        let mut fields = Vec::new();

        if !self.check(&Token::RBrace) {
            loop {
                if self.check(&Token::RBrace) {
                    break;
                } // trailing comma
                let field_name = self.consume_ident("Expected field name in struct initializer")?;
                self.consume(Token::Colon, "Expected ':' after field name")?;
                // Field values can themselves be struct initializers.
                let value = self.parse_expr(Precedence::None)?;
                fields.push((field_name, value));
                if !self.match_tokens(&[Token::Comma]) {
                    break;
                }
            }
        }

        self.consume(Token::RBrace, "Expected '}' after struct fields")?;
        Ok(Expr::StructInit { name, fields })
    }

    /// Called AFTER the opening `{` has been consumed, so `self.current` points
    /// to the first token inside the braces. `depth` starts at 1 to represent
    /// that we are already one level deep inside the outer `{`.
    ///
    /// This is read-only lookahead. it can never fail, so it returns `bool`
    /// rather than a `CompilerResult`.
    fn is_map_literal(&self) -> bool {
        //{}
        if self.check(&Token::RBrace) {
            return false;
        }

        // If the first token is a statement keyword, it cannot be a map key.
        if let Some(st) = self.peek() {
            match &st.token {
                Token::Var
                | Token::Const
                | Token::If
                | Token::While
                | Token::For
                | Token::Return
                | Token::Break
                | Token::Continue
                | Token::Yield
                | Token::Defer
                | Token::Switch => return false,
                _ => {}
            }
        }

        // Scan for a colon (map) or a semicolon (block) at the top level.
        let mut depth = 1;
        let mut i = self.current;

        while i < self.tokens.len() {
            match &self.tokens[i].token {
                Token::LBrace | Token::LParen | Token::LBracket => depth += 1,
                Token::RBrace | Token::RParen | Token::RBracket => {
                    depth -= 1;
                    if depth == 0 {
                        return false;
                    }
                }
                Token::Colon if depth == 1 => return true, // colon -> map
                Token::SemiColon if depth == 1 => return false, // semicolon -> block
                _ => {}
            }
            i += 1;
        }

        false
    }

    /// Parses a map literal body. The opening `{` must already be consumed.
    fn parse_map_literal(&mut self) -> CompilerResult<Expr> {
        let mut pairs: Vec<(Expr, Expr)> = Vec::new();

        if !self.check(&Token::RBrace) {
            loop {
                if self.check(&Token::RBrace) {
                    break;
                }
                let key = self.parse_expr(Precedence::None)?;
                self.consume(Token::Colon, "Expected ':' after map key")?;
                let value = self.parse_expr(Precedence::None)?;
                pairs.push((key, value));
                if !self.match_tokens(&[Token::Comma]) {
                    break;
                }
            }
        }

        self.consume(Token::RBrace, "Expected '}' after map literal")?;
        Ok(Expr::Map(pairs))
    }
}
