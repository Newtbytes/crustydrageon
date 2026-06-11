use std::collections::HashMap;

use crate::{
    ast::{BinOpKind, Decl, Expr, ExprKind, Function, Program, Stmt},
    diag::{Annotation, Diag, DiagLevel, Diagnostic},
    ir::VarID,
};

#[derive(Debug)]
pub enum ResolveError {
    InvalidLvalue(Expr),
    DuplicateDecl { decl: Decl, prev: Decl },
    UnknownVar(Expr),
}

impl Diagnostic for ResolveError {
    fn into_diag(self) -> Diag {
        let mut diag = Diag::new(DiagLevel::Error);

        match self {
            Self::InvalidLvalue(expr) => {
                diag.annotate(
                    Annotation::new(
                        expr.span,
                        "Illegal left-hand side for assignment".to_owned(),
                    )
                    // TODO: explain what a valid l-value is here
                    // variables are currently the only valid l-values, but in the future this will change
                    // TODO: add a ", but got a {description of expression}" clause
                    .with_note("Should be a variable".to_owned()),
                )
            }
            Self::DuplicateDecl { decl, prev } => diag
                .annotate(Annotation::new(
                    decl.span,
                    format!("Duplicate declaration of variable '{}'", decl.name),
                ))
                .annotate(Annotation::new(
                    prev.span,
                    "Previous declaration found here".to_owned(),
                )),
            Self::UnknownVar(expr) => diag.annotate(Annotation::new(
                expr.span.clone(),
                format!("Undeclared variable '{}'", expr.span),
            )),
        };

        diag
    }
}

pub type ResolveResult<T> = Result<T, ResolveError>;

type VarCtx = HashMap<String, (String, Decl)>;

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
                    return Err(ResolveError::UnknownVar(expr.clone()));
                }
            }
            ExprKind::Unary(_, expr) => self.resolve_expr(expr)?,
            ExprKind::Binary(op, a, b) => {
                if op.kind == BinOpKind::Assign && !matches!(a.kind, ExprKind::Var(_)) {
                    return Err(ResolveError::InvalidLvalue(*a.clone()));
                }
                self.resolve_expr(a)?;
                self.resolve_expr(b)?;
            }
            ExprKind::Const(_) => (),
        };

        Ok(())
    }

    fn resolve_decl(&mut self, decl: &mut Decl) -> ResolveResult<()> {
        if self.ctx.contains_key(decl.name.value()) {
            let (_, prev_decl) = self.ctx.get(decl.name.value()).unwrap();
            return Err(ResolveError::DuplicateDecl {
                decl: decl.clone(),
                prev: prev_decl.clone(),
            });
        }

        let prev_name = decl.name.value().to_owned();
        let curr_name = format!("{prev_name}.{}", VarID::new());
        decl.name.rename(curr_name.clone());
        self.ctx.insert(prev_name, (curr_name, decl.clone()));

        if let Some(expr) = &mut decl.init {
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
