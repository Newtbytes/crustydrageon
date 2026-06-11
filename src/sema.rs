use std::collections::HashMap;

use crate::{
    ast::{BinaryOp, Decl, Expr, Function, Program, Stmt},
    ir::VarID,
};

pub type ResolveError = String;
pub type ResolveResult<T> = Result<T, ResolveError>;

type VarCtx = HashMap<String, String>;

struct VariableResolver {
    ctx: VarCtx,
}

impl VariableResolver {
    pub fn new() -> Self {
        Self { ctx: VarCtx::new() }
    }

    fn resolve_expr(&mut self, expr: &mut Expr) -> ResolveResult<()> {
        match expr {
            Expr::Var(id) => {
                if let None = self.ctx.get(id.value()) {
                    return Err("Undeclared variable".to_owned());
                }
            }
            Expr::Unary(_, expr) => self.resolve_expr(expr)?,
            Expr::Binary(op, a, b) => {
                if *op == BinaryOp::Assign && !matches!(**a, Expr::Var(_)) {
                    return Err("invalid lvalue".to_owned());
                }
                self.resolve_expr(a)?;
                self.resolve_expr(b)?;
            }
            Expr::Const(_) => (),
        };

        Ok(())
    }

    fn resolve_decl(&mut self, Decl { name, init }: &mut Decl) -> ResolveResult<()> {
        if self.ctx.contains_key(name.value()) {
            return Err("Duplicate variable definition".to_owned());
        }

        let prev_name = name.value().to_owned();
        let curr_name = format!("{prev_name}.{}", VarID::new());
        name.rename(curr_name.clone());
        self.ctx.insert(prev_name, curr_name);

        if let Some(expr) = init {
            self.resolve_expr(expr)?;
        }

        Ok(())
    }

    fn resolve_stmt(&mut self, stmt: &mut Stmt) -> ResolveResult<()> {
        match stmt {
            Stmt::Return(expr) | Stmt::Expr(expr) => self.resolve_expr(expr)?,
            Stmt::Null => (),
        };

        Ok(())
    }

    fn resolve_function(&mut self, func: &mut Function) -> ResolveResult<()> {
        for block_item in &mut func.body {
            match block_item {
                crate::ast::BlockItem::Stmt(stmt) => self.resolve_stmt(stmt)?,
                crate::ast::BlockItem::Decl(decl) => self.resolve_decl(decl)?,
            }
        }

        Ok(())
    }

    fn resolve_program(&mut self, prg: &mut Program) -> ResolveResult<()> {
        self.resolve_function(&mut prg.body)
    }
}

pub fn resolve(prg: &mut Program) -> ResolveResult<()> {
    let mut resolver = VariableResolver::new();
    resolver.resolve_program(prg)
}

#[cfg(test)]
mod tests {
    use crate::ast::Identifier;

    use super::*;

    #[test]
    fn test_resolve_invalid_lvalue() {
        let mut resolver = VariableResolver::new();

        let mut expr = Expr::Binary(
            BinaryOp::Assign,
            Expr::Const(5).into(),
            Expr::Var(Identifier::default()).into(),
        );

        resolver.resolve_expr(&mut expr).unwrap_err();
    }
}
