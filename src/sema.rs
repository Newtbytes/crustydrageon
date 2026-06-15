use std::collections::HashMap;

use crate::{
    ast::{BinOpKind, Block, Decl, Expr, ExprKind, Function, Program, Stmt},
    diag::Annotation,
    ir::VarID,
    src,
};

#[derive(Debug)]
pub enum ResolveError {
    InvalidLvalue(src::Span),
    DuplicateDecl(src::Span),
    UnknownVar(src::Span),
}

impl Annotation for ResolveError {
    fn span(&self) -> &src::Span {
        match self {
            Self::UnknownVar(span) | Self::DuplicateDecl(span) | Self::InvalidLvalue(span) => span,
        }
    }

    fn message(&self) -> String {
        match self {
            ResolveError::InvalidLvalue(_) => "Invalid lvalue",
            ResolveError::DuplicateDecl(_) => "Duplicate declaration",
            ResolveError::UnknownVar(_) => "Undeclared variable",
        }
        .to_owned()
    }
}

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
        let kind = &mut expr.kind;
        match kind {
            ExprKind::Var(id) => {
                if !self.ctx.contains_key(id.value()) {
                    return Err(ResolveError::UnknownVar(id.span().clone()));
                }
            }
            ExprKind::Unary(_, expr) => self.resolve_expr(expr)?,
            ExprKind::Binary(op, a, b) => {
                if op.kind == BinOpKind::Assign && !matches!(a.kind, ExprKind::Var(_)) {
                    return Err(ResolveError::InvalidLvalue(a.span.clone()));
                }
                self.resolve_expr(a)?;
                self.resolve_expr(b)?;
            }
            ExprKind::Cond(cond, if_true, if_false) => {
                self.resolve_expr(cond)?;
                self.resolve_expr(if_true)?;
                self.resolve_expr(if_false)?;
            }
            ExprKind::Const(_) => (),
        };

        Ok(())
    }

    fn resolve_decl(&mut self, decl: &mut Decl) -> ResolveResult<()> {
        if self.ctx.contains_key(decl.name.value()) {
            return Err(ResolveError::DuplicateDecl(decl.span.clone()));
        }

        let prev_name = decl.name.value().to_owned();
        let curr_name = format!("{prev_name}.{}", VarID::new());
        decl.name.rename(curr_name.clone());
        self.ctx.insert(prev_name, curr_name);

        if let Some(expr) = &mut decl.init {
            self.resolve_expr(expr)?;
        }

        Ok(())
    }

    fn resolve_stmt(&mut self, stmt: &mut Stmt) -> ResolveResult<()> {
        match stmt {
            Stmt::Return(expr) | Stmt::Expr(expr) => self.resolve_expr(expr)?,
            Stmt::If(cond, if_true, if_false) => {
                self.resolve_expr(cond)?;
                self.resolve_stmt(if_true)?;
                if let Some(if_false) = if_false {
                    self.resolve_stmt(if_false)?;
                }
            }
            Stmt::Compound(block) => self.resolve_block(block)?,
            Stmt::Null => (),
        };

        Ok(())
    }

    fn resolve_block(&mut self, block: &mut Block) -> ResolveResult<()> {
        for item in block.iter_mut() {
            match item {
                crate::ast::BlockItem::Stmt(stmt) => self.resolve_stmt(stmt)?,
                crate::ast::BlockItem::Decl(decl) => self.resolve_decl(decl)?,
            }
        }

        Ok(())
    }

    fn resolve_function(&mut self, func: &mut Function) -> ResolveResult<()> {
        self.resolve_block(&mut func.body)
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
    use super::*;

    #[test]
    fn test_resolve_invalid_lvalue() {
        VariableResolver::new()
            .resolve_expr(&mut Expr::binary(
                BinOpKind::Assign,
                Expr::constant(5),
                Expr::var(""),
            ))
            .unwrap_err();
    }
}
