use std::collections::HashMap;

use crate::{
    ast::{BinOpKind, Decl, Expr, ExprKind, Function, Program, Stmt},
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

// #[cfg(test)]
// mod tests {
//     use crate::ast::Identifier;

//     use super::*;

//     #[test]
//     fn test_resolve_invalid_lvalue() {
//         let mut resolver = VariableResolver::new();

//         let mut expr = Expr::Binary(
//             BinaryOp::Assign,
//             Expr::Const(5).into(),
//             Expr::Var(Identifier::default()).into(),
//         );

//         resolver.resolve_expr(&mut expr).unwrap_err();
//     }
// }
