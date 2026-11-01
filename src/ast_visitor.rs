use crate::ast::*;

/// Visitor trait for traversing the AST.
///
/// Each checker pass implements this trait and overrides only the visit_*
/// methods it cares about. The walk_* methods are the actual traversal
pub trait AstVisitor {
    fn visit_program(&mut self, program: &Program) {
        self.walk_program(program);
    }

    fn visit_item(&mut self, item: &Item) {
        self.walk_item(item);
    }

    fn visit_function_decl(&mut self, decl: &FunctionDecl) {
        self.walk_function_decl(decl);
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        self.walk_stmt(stmt);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        self.walk_expr(expr);
    }


    fn walk_program(&mut self, program: &Program) {
        for item in &program.items {
            self.visit_item(item);
        }
    }

    fn walk_item(&mut self, item: &Item) {
        match item {
            Item::Struct { .. }
            | Item::Enum { .. }
            | Item::TypeAlias { .. }
            | Item::Use { .. } => {}

            Item::Function(decl) => {
                self.visit_function_decl(decl);
            }

            Item::Impl { methods, .. } => {
                for method in methods {
                    self.visit_function_decl(method);
                }
            }
        }
    }

    fn walk_function_decl(&mut self, decl: &FunctionDecl) {
        // Visit default values of parameters.
        for param in &decl.parameters {
            if let Some(default) = &param.default_value {
                self.visit_expr(default);
            }
        }
        // Visit the function body.
        for stmt in &decl.body {
            self.visit_stmt(stmt);
        }
    }

    fn walk_stmt(&mut self, stmt: &Stmt) {
        match &stmt.node {
            StmtNode::VarDecl { initializer, .. } => {
                if let Some(expr) = initializer {
                    self.visit_expr(expr);
                }
            }

            StmtNode::Assignment { target, value, .. } => {
                self.walk_assign_target(target);
                self.visit_expr(value);
            }

            StmtNode::Expression(expr) => {
                self.visit_expr(expr);
            }

            StmtNode::If { condition, then_branch, else_branch } => {
                self.visit_expr(condition);
                self.visit_stmt(then_branch);
                if let Some(else_stmt) = else_branch {
                    self.visit_stmt(else_stmt);
                }
            }

            StmtNode::While { condition, body } => {
                self.visit_expr(condition);
                self.visit_stmt(body);
            }

            StmtNode::For { iterator, body, .. } => {
                self.visit_expr(iterator);
                self.visit_stmt(body);
            }

            StmtNode::Break | StmtNode::Continue => {}

            StmtNode::Return(expr) => {
                if let Some(e) = expr {
                    self.visit_expr(e);
                }
            }

            StmtNode::Yield(expr) => {
                self.visit_expr(expr);
            }

            StmtNode::Defer(inner) => {
                self.visit_stmt(inner);
            }
        }
    }

    /// Internal helper - recurses into exprs embedded in an AssignTarget.
    /// Not an override point because passes don't need to hook on targets
    /// separately from the Assignment stmt that contains them.
    fn walk_assign_target(&mut self, target: &AssignTarget) {
        match target {
            // Just a name - nothing to recurse into.
            AssignTarget::Identifier(_) => {}

            AssignTarget::MemberAccess { object, .. } => {
                self.visit_expr(object);
            }

            AssignTarget::Index { list, index } => {
                self.visit_expr(list);
                self.visit_expr(index);
            }
        }
    }

    fn walk_expr(&mut self, expr: &Expr) {
        match &expr.node {
            ExprNode::Literal(_)
            | ExprNode::ECodeLit(_)
            | ExprNode::Identifier(_)
            | ExprNode::SelfKw
            | ExprNode::EnumVariant { .. } => {}

            ExprNode::Unary { right, .. } => {
                self.visit_expr(right);
            }

            ExprNode::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }

            ExprNode::Call { callee, arguments } => {
                self.visit_expr(callee);
                for (_, arg) in arguments {
                    self.visit_expr(arg);
                }
            }

            ExprNode::MemberAccess { object, .. } => {
                self.visit_expr(object);
            }

            ExprNode::Index { list, index } => {
                self.visit_expr(list);
                self.visit_expr(index);
            }

            ExprNode::List(items) => {
                for item in items {
                    self.visit_expr(item);
                }
            }

            ExprNode::Map(pairs) => {
                for (key, value) in pairs {
                    self.visit_expr(key);
                    self.visit_expr(value);
                }
            }

            ExprNode::InterpolatedString(segments) => {
                for seg in segments {
                    if let StringSegment::Expr(e) = seg {
                        self.visit_expr(e);
                    }
                }
            }

            ExprNode::StructInit { fields, .. } => {
                for (_, value) in fields {
                    self.visit_expr(value);
                }
            }

            ExprNode::Switch { target, arms, default_arm } => {
                self.visit_expr(target);
                for (pattern, result) in arms {
                    self.visit_expr(pattern);
                    self.visit_expr(result);
                }
                self.visit_expr(default_arm);
            }

            ExprNode::Block(stmts) => {
                for stmt in stmts {
                    self.visit_stmt(stmt);
                }
            }

            ExprNode::PropagateError(inner) => {
                self.visit_expr(inner);
            }

            ExprNode::OrFallback { try_expr, fallback, .. } => {
                self.visit_expr(try_expr);
                self.visit_expr(fallback);
            }

            ExprNode::Error { message, .. } => {
                self.visit_expr(message);
            }
        }
    }
}
