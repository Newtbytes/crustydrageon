use crate::{
    ast::{
        BinOp, Block, BlockItem, Decl, Expr, ExprKind, Function, Identifier, Program, Stmt, UnOp,
    },
    visitor_trait,
};

visitor_trait! {
    Visitor {
        expr(e: Expr),
        expr_kind(k: ExprKind),
        var(#[leaf] _id: Identifier),
        constant(#[leaf] _v: i32),
        unary(#[leaf] _op: UnOp, e: Expr),
        binary(#[leaf] _op: BinOp, a: Expr, b: Expr),
        cond(c: Expr, t: Expr, f: Expr),
        stmt(s: Stmt),
        expr_stmt(e: Expr),
        return_stmt(e: Expr),
        if_stmt(c: Expr, t: Stmt, f: Option<Box<Stmt>>),
        decl(d: Decl),
        block(b: Block),
        block_item(i: BlockItem),
        function(f: Function),
        program(p: Program)
    }
}

pub trait MutVisitable {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V);
}

impl<T: MutVisitable> MutVisitable for Option<T> {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        if let Some(val) = self {
            val.accept(visitor);
        }
    }
}

impl<T: MutVisitable> MutVisitable for Box<T> {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        self.as_mut().accept(visitor);
    }
}

impl MutVisitable for Expr {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_expr_kind(&mut self.kind);
    }
}

impl MutVisitable for ExprKind {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        match self {
            Self::Const(v) => visitor.visit_constant(v),
            Self::Var(id) => visitor.visit_var(id),
            Self::Unary(op, expr) => visitor.visit_unary(op, expr),
            Self::Binary(op, a, b) => visitor.visit_binary(op, a, b),
            Self::Cond(c, t, f) => visitor.visit_cond(c, t, f),
        }
    }
}

impl MutVisitable for Stmt {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        match self {
            Self::Expr(e) => visitor.visit_expr(e),
            Self::Return(e) => visitor.visit_return_stmt(e),
            Self::If(c, t, f) => visitor.visit_if_stmt(c, t, f),
            Self::Compound(block) => visitor.visit_block(block),
            Self::Null => (),
        }
    }
}

impl MutVisitable for Decl {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        if let Some(init) = &mut self.init {
            visitor.visit_expr(init);
        }
    }
}

impl MutVisitable for Block {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        for i in self.iter_mut() {
            visitor.visit_block_item(i);
        }
    }
}

impl MutVisitable for BlockItem {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        match self {
            Self::Stmt(s) => visitor.visit_stmt(s),
            Self::Decl(d) => visitor.visit_decl(d),
        }
    }
}

impl MutVisitable for Function {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_block(&mut self.body);
    }
}

impl MutVisitable for Program {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_function(&mut self.body);
    }
}
