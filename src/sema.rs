use std::collections::HashMap;

use crate::{
    ast::{
        BinOpKind, Block, Decl, Expr, ExprKind, LoopLabel, Program, Stmt,
        visit::{MutVisitable, MutVisitor, MutWalkable},
    },
    diag::Annotation,
    error::{CompilerError, CompilerResult},
    ir::VarID,
    src::{self, Source},
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

#[derive(Debug, Clone)]
pub enum LoopLabelingError {
    BreakOutsideLoop(src::Span),
    ContinueOutsideLoop(src::Span),
}

impl Annotation for LoopLabelingError {
    fn span(&self) -> &src::Span {
        match self {
            Self::BreakOutsideLoop(span) | Self::ContinueOutsideLoop(span) => span,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::BreakOutsideLoop(_) => "break statement must be inside of a loop",
            Self::ContinueOutsideLoop(_) => "continue statement must be inside of a loop",
        }
        .to_owned()
    }
}

#[derive(Default)]
struct LoopLabeler {
    depth: usize,
    curr_root_loop_id: usize,
    next_root_loop_id: usize,
    errs: Vec<LoopLabelingError>,
}

impl LoopLabeler {
    pub fn new() -> Self {
        Self {
            depth: Default::default(),
            curr_root_loop_id: Default::default(),
            next_root_loop_id: Default::default(),
            errs: Vec::new(),
        }
    }
}

impl LoopLabeler {
    fn emit_err(&mut self, err: LoopLabelingError) {
        self.errs.push(err);
    }

    fn inside_loop(&mut self) -> bool {
        self.depth > 0
    }

    fn start_loop(&mut self) -> LoopLabel {
        if !self.inside_loop() {
            self.curr_root_loop_id = self.next_root_loop_id;
        }

        self.depth += 1;
        self.next_root_loop_id += 1;

        self.current_label()
    }

    fn current_label(&self) -> LoopLabel {
        LoopLabel(self.curr_root_loop_id + self.depth)
    }

    fn end_loop(&mut self) {
        assert_ne!(
            self.depth, 0,
            "curr_loop_id is 0; loop should have been started before being ended"
        );

        self.depth -= 1;
    }
}

impl MutVisitor for LoopLabeler {
    fn visit_stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Break(label) | Stmt::Continue(label) => {
                if !self.inside_loop() {
                    let span = src::Span::default(); // TODO: add a span field to Stmt
                    self.emit_err(if matches!(stmt, Stmt::Break(..)) {
                        LoopLabelingError::BreakOutsideLoop(span)
                    } else {
                        LoopLabelingError::ContinueOutsideLoop(span)
                    })
                } else {
                    // TODO: test bug where self.start_loop() is incorrectly used here
                    *label = Some(self.current_label());
                }
            }
            Stmt::While(cond, body, label) => {
                *label = Some(self.start_loop());
                self.visit_while_stmt(cond, body, label);
                self.end_loop();
            }
            Stmt::DoWhile(body, cond, label) => {
                *label = Some(self.start_loop());
                self.visit_do_while_stmt(body, cond, label);
                self.end_loop();
            }
            Stmt::For(init, cond, post, body, label) => {
                *label = Some(self.start_loop());
                self.visit_for_stmt(init, cond, post, body, label);
                self.end_loop();
            }
            s if s.is_loop() => todo!("loop labeling for statement: {s}"),
            _ => stmt.walk(self),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SemanticsError {
    VariableResolution(ResolveError),
    LoopLabeling(LoopLabelingError),
}

impl Annotation for SemanticsError {
    fn span(&self) -> &src::Span {
        match self {
            Self::VariableResolution(err) => err.span(),
            Self::LoopLabeling(err) => err.span(),
        }
    }

    fn message(&self) -> String {
        match self {
            Self::VariableResolution(err) => err.message(),
            Self::LoopLabeling(err) => err.message(),
        }
    }
}

pub fn resolve(prg: &mut Program) -> ResolveResult<()> {
    let mut resolver = VariableResolver::new();
    prg.accept(&mut resolver);

    resolver.errs.first().cloned().map_or(Ok(()), Err)
}

pub fn label_loops(prg: &mut Program) -> Result<(), LoopLabelingError> {
    let mut labeler = LoopLabeler::new();
    prg.accept(&mut labeler);

    labeler.errs.first().cloned().map_or(Ok(()), Err)
}

pub fn analyze_semantics(src: &Source, prg: &mut Program) -> CompilerResult<()> {
    resolve(prg).map_err(|e| CompilerError::SourceDiagnostic(src.clone(), Box::new(e)))?;
    label_loops(prg).map_err(|e| CompilerError::SourceDiagnostic(src.clone(), Box::new(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::{fixture, rstest};

    #[fixture]
    fn resolver() -> VariableResolver {
        VariableResolver::new()
    }

    #[fixture]
    fn labeler() -> LoopLabeler {
        LoopLabeler::new()
    }

    #[rstest]
    #[case::invalid_lvalue(Expr::binary(BinOpKind::Assign, Expr::constant(5), Expr::var(""),))]
    #[case::unknown_var(Expr::binary(BinOpKind::Add, Expr::var("a"), Expr::var("b"),))]
    fn test_resolve_expr_err(mut resolver: VariableResolver, #[case] mut expr: Expr) {
        expr.accept(&mut resolver);
        assert!(!resolver.errs.is_empty());
    }

    #[rstest]
    #[case::break_outside_loop(Stmt::If(Expr::constant(1), Box::new(Stmt::Break(None)), None))]
    fn test_loop_labeling_stmt_err(mut labeler: LoopLabeler, #[case] mut stmt: Stmt) {
        stmt.accept(&mut labeler);
        assert!(!labeler.errs.is_empty());
    }

    #[rstest]
    #[case(
        Stmt::While(
            Expr::binary(BinOpKind::LessThan, Expr::var("a"), Expr::var("b")),
            Box::new(Stmt::Null),
            None,
        ),
        1
    )]
    #[case(
        Stmt::While(
            Expr::binary(BinOpKind::LessThan, Expr::var("a"), Expr::var("b")),
            Box::new(Stmt::While(
                Expr::binary(BinOpKind::LessThan, Expr::var("a"), Expr::var("b")),
                Box::new(Stmt::Null),
                None,
            )),
            None,
        ),
        2
    )]
    fn test_next_root_loop_id(
        mut labeler: LoopLabeler,
        #[case] mut stmt: Stmt,
        #[case] expected: usize,
    ) {
        assert_eq!(labeler.depth, 0);
        assert_eq!(labeler.curr_root_loop_id, 0);

        stmt.accept(&mut labeler);

        assert_eq!(labeler.depth, 0);
        assert_eq!(labeler.next_root_loop_id, expected);
    }
}
