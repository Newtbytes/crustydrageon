use crate::ast::{
    BinOp, Block, BlockItem, Decl, Expr, ExprKind, Function, Identifier, Program, Stmt, UnOp,
};

pub trait MutVisitor
where
    Self: Sized,
{
    fn visit_expr(&mut self, e: &mut Expr) {
        e.accept(self);
    }
    fn visit_expr_kind(&mut self, k: &mut ExprKind) {
        k.accept(self);
    }
    fn visit_const(&mut self, _v: &mut i32) {}
    fn visit_var(&mut self, _id: &mut Identifier) {}
    fn visit_unary(&mut self, _op: &mut UnOp, expr: &mut Expr) {
        expr.accept(self);
    }
    fn visit_binary(&mut self, _op: &mut BinOp, a: &mut Expr, b: &mut Expr) {
        a.accept(self);
        b.accept(self);
    }
    fn visit_cond(&mut self, c: &mut Expr, t: &mut Expr, f: &mut Expr) {
        c.accept(self);
        t.accept(self);
        f.accept(self);
    }

    fn visit_stmt(&mut self, s: &mut Stmt) {
        s.accept(self);
    }
    fn visit_expr_stmt(&mut self, e: &mut Expr) {
        e.accept(self);
    }
    fn visit_return(&mut self, e: &mut Expr) {
        e.accept(self);
    }
    fn visit_if(&mut self, c: &mut Expr, t: &mut Stmt, f: &mut Option<Box<Stmt>>) {
        c.accept(self);
        t.accept(self);
        if let Some(f) = f {
            f.accept(self);
        }
    }

    fn visit_decl(&mut self, d: &mut Decl) {
        d.accept(self);
    }

    fn visit_block(&mut self, b: &mut Block) {
        b.accept(self);
    }

    fn visit_block_item(&mut self, i: &mut BlockItem) {
        i.accept(self);
    }

    fn visit_function(&mut self, f: &mut Function) {
        f.accept(self);
    }

    fn visit_program(&mut self, p: &mut Program) {
        p.accept(self);
    }
}

pub trait MutVisitable {
    fn accept(&mut self, visitor: &mut impl MutVisitor);
}

impl MutVisitable for Expr {
    fn accept(&mut self, visitor: &mut impl MutVisitor) {
        visitor.visit_expr_kind(&mut self.kind);
    }
}

impl MutVisitable for ExprKind {
    fn accept(&mut self, visitor: &mut impl MutVisitor) {
        match self {
            Self::Const(v) => visitor.visit_const(v),
            Self::Var(id) => visitor.visit_var(id),
            Self::Unary(op, expr) => visitor.visit_unary(op, expr),
            Self::Binary(op, a, b) => visitor.visit_binary(op, a, b),
            Self::Cond(c, t, f) => visitor.visit_cond(c, t, f),
        }
    }
}

impl MutVisitable for Stmt {
    fn accept(&mut self, visitor: &mut impl MutVisitor) {
        match self {
            Self::Expr(e) => visitor.visit_expr(e),
            Self::Return(e) => visitor.visit_return(e),
            Self::If(c, t, f) => visitor.visit_if(c, t, f),
            Self::Compound(block) => visitor.visit_block(block),
            Self::Null => (),
        }
    }
}

impl MutVisitable for Decl {
    fn accept(&mut self, visitor: &mut impl MutVisitor) {
        if let Some(init) = &mut self.init {
            visitor.visit_expr(init);
        }
    }
}

impl MutVisitable for Block {
    fn accept(&mut self, visitor: &mut impl MutVisitor) {
        for i in self.iter_mut() {
            visitor.visit_block_item(i);
        }
    }
}

impl MutVisitable for BlockItem {
    fn accept(&mut self, visitor: &mut impl MutVisitor) {
        match self {
            Self::Stmt(s) => visitor.visit_stmt(s),
            Self::Decl(d) => visitor.visit_decl(d),
        }
    }
}

impl MutVisitable for Function {
    fn accept(&mut self, visitor: &mut impl MutVisitor) {
        visitor.visit_block(&mut self.body);
    }
}

impl MutVisitable for Program {
    fn accept(&mut self, visitor: &mut impl MutVisitor) {
        visitor.visit_function(&mut self.body);
    }
}
