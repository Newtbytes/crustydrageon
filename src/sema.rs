use std::collections::HashMap;

use crate::{
    ast::{
        BinOpKind, Block, Decl, Expr, ExprKind, Program, Stmt,
        visit::{MutVisitable, MutVisitor, MutWalkable},
    },
    diag::Annotation,
    ir::VarID,
    src,
};

#[derive(Debug, Clone)]
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

struct EnvCtx {
    scopes: Vec<HashMap<String, String>>,
}

impl EnvCtx {
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn end_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(k) {
                return Some(v);
            }
        }

        None
    }

    pub fn insert(&mut self, k: String, v: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(k, v);
        } else {
            self.begin_scope();
            self.insert(k, v);
        }
    }
}

struct VariableResolver {
    ctx: EnvCtx,
    errs: Vec<ResolveError>,
}

impl VariableResolver {
    pub fn new() -> Self {
        Self {
            ctx: EnvCtx::new(),
            errs: Vec::new(),
        }
    }

    fn emit_err(&mut self, e: ResolveError) {
        self.errs.push(e);
    }
}

impl MutVisitor for VariableResolver {
    fn visit_var(&mut self, id: &mut crate::ast::Identifier) {
        if let Some(renamed) = self.ctx.get(id.value()) {
            id.rename(renamed.clone());
        } else {
            self.emit_err(ResolveError::UnknownVar(id.span().clone()));
        }
    }

    fn visit_binary(&mut self, op: &mut crate::ast::BinOp, a: &mut Expr, b: &mut Expr) {
        if op.kind == BinOpKind::Assign && !matches!(a.kind, ExprKind::Var(_)) {
            self.emit_err(ResolveError::InvalidLvalue(a.span.clone()));
        }

        a.accept(self);
        b.accept(self);
    }

    fn visit_decl(&mut self, d: &mut Decl) {
        if self
            .ctx
            .scopes
            .last()
            .expect("resolving declarations should always happen after a scope has started")
            .contains_key(d.name.value())
        {
            self.emit_err(ResolveError::DuplicateDecl(d.span.clone()));
        }

        let prev_name = d.name.value().to_owned();
        let curr_name = format!("{prev_name}.{}", VarID::new());
        d.name.rename(curr_name.clone());
        self.ctx.insert(prev_name, curr_name);

        d.walk(self);
    }

    fn visit_for_stmt(
        &mut self,
        init: &mut either::Either<Decl, Option<Expr>>,
        cond: &mut Option<Expr>,
        post: &mut Option<Expr>,
        stmt: &mut Stmt,
        _label: &mut Option<crate::ast::LoopLabel>,
    ) {
        self.ctx.begin_scope();
        init.accept(self);
        cond.accept(self);
        post.accept(self);
        stmt.accept(self);
        self.ctx.end_scope();
    }

    fn visit_block(&mut self, b: &mut Block) {
        self.ctx.begin_scope();
        b.walk(self);
        self.ctx.end_scope();
    }
}

pub fn resolve(prg: &mut Program) -> ResolveResult<()> {
    let mut resolver = VariableResolver::new();
    prg.accept(&mut resolver);

    resolver.errs.first().cloned().map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::{fixture, rstest};

    #[fixture]
    fn resolver() -> VariableResolver {
        VariableResolver::new()
    }

    #[rstest]
    #[case::invalid_lvalue(Expr::binary(BinOpKind::Assign, Expr::constant(5), Expr::var(""),))]
    #[case::unknown_var(Expr::binary(BinOpKind::Add, Expr::var("a"), Expr::var("b"),))]
    fn test_resolve_expr_err(mut resolver: VariableResolver, #[case] mut expr: Expr) {
        expr.accept(&mut resolver);
        assert!(!resolver.errs.is_empty());
    }
}
