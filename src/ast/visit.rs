use either::Either;

use crate::{
    ast::{
        BinOp, Block, BlockItem, Decl, Expr, ExprKind, Function, Identifier, LoopLabel, Program,
        Stmt, UnOp,
    },
    visitor_trait,
};

visitor_trait! {
    Visitor {
        expr(e: Expr),
        expr_kind(k: ExprKind),
        var(#[leaf] id: Identifier),
        constant(#[leaf] v: i32),
        unary(#[leaf] op: UnOp, #[branch] e: Expr),
        binary(#[leaf] op: BinOp, #[branch] a: Expr, #[branch] b: Expr),
        cond(#[branch] c: Expr, #[branch] t: Expr, #[branch] f: Expr),
        stmt(s: Stmt),
        expr_stmt(e: Expr),
        return_stmt(e: Expr),
        if_stmt(#[branch] c: Expr, #[branch] t: Stmt, #[branch] f: Option<Box<Stmt>>),
        loop_label(#[leaf] s: String),
        break_stmt(#[leaf] l: Option<LoopLabel>),
        continue_stmt(#[leaf] l: Option<LoopLabel>),
        while_stmt(#[branch] c: Expr, #[branch] s: Stmt, #[leaf] l: Option<LoopLabel>),
        do_while_stmt(#[branch] s: Stmt, #[branch] c: Expr, #[leaf] l: Option<LoopLabel>),
        for_stmt(
            i: Either<Decl, Option<Expr>>,
            #[branch] c: Option<Expr>,
            #[branch] p: Option<Expr>,
            #[branch] s: Stmt,
            #[leaf] l: Option<LoopLabel>
        ),
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

pub trait MutWalkable {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V);
}

impl<T: MutVisitable, U: MutVisitable> MutVisitable for Either<T, U> {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        match self {
            Self::Left(l) => l.accept(visitor),
            Self::Right(r) => r.accept(visitor),
        }
    }
}

impl MutVisitable for Expr {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_expr(self);
    }
}

impl MutWalkable for Expr {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        self.kind.accept(visitor);
    }
}

impl MutVisitable for ExprKind {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_expr_kind(self);
    }
}

impl MutWalkable for ExprKind {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        match self {
            Self::Const(v) => visitor.visit_constant(v),
            Self::Var(id) => visitor.visit_var(id),
            Self::Unary(op, expr) => visitor.visit_unary(op, expr),
            Self::Binary(op, a, b) => visitor.visit_binary(op, a, b),
            Self::Cond(c, t, f) => visitor.visit_cond(c, t, f),
        }
    }
}

// impl MutVisitable for Option<Box<Stmt>> {
//     fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
//         if let Some(stmt) = self {
//             visitor.visit_stmt(stmt);
//         }
//     }
// }

impl<T: MutWalkable> MutWalkable for Option<T> {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        if let Some(val) = self {
            val.walk(visitor);
        }
    }
}

impl<T: MutVisitable> MutVisitable for Option<T> {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        if let Some(val) = self {
            val.accept(visitor);
        }
    }
}

impl<T: MutWalkable> MutWalkable for Box<T> {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        self.as_mut().walk(visitor);
    }
}

impl<T: MutVisitable> MutVisitable for Box<T> {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        self.as_mut().accept(visitor);
    }
}

// impl MutVisitable for Option<LoopLabel> {
//     fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
//         if let Some(label) = self {
//             visitor.visit_loop_label(label);
//         }
//     }
// }

impl MutVisitable for LoopLabel {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_loop_label(self)
    }
}

// impl MutVisitable for Either<Decl, Option<Expr>> {
//     fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
//         match self {
//             Self::Left(decl) => visitor.visit_decl(decl),
//             Self::Right(expr) => expr.accept(visitor),
//         }
//     }
// }

impl<T: MutVisitable, U: MutVisitable> MutWalkable for Either<T, U> {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        match self {
            Self::Left(l) => l.accept(visitor),
            Self::Right(r) => r.accept(visitor),
        }
    }
}

// impl<T: MutVisitable, U: MutVisitable> MutVisitable for Either<T, U> {
//     fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
//         match self {
//             Self::Left(l) => l.accept(visitor),
//             Self::Right(r) => r.accept(visitor),
//         }
//     }
// }

// impl MutVisitable for Option<Expr> {
//     fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
//         if let Some(expr) = self {
//             visitor.visit_expr(expr);
//         }
//     }
// }

impl MutVisitable for Stmt {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_stmt(self);
    }
}

impl MutWalkable for Stmt {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        match self {
            Self::Expr(e) => e.accept(visitor),
            Self::Return(e) => visitor.visit_return_stmt(e),
            Self::If(c, t, f) => visitor.visit_if_stmt(c, t, f),
            Self::Compound(block) => block.walk(visitor),
            Self::Break(l) => visitor.visit_break_stmt(l),
            Self::Continue(l) => visitor.visit_continue_stmt(l),
            Self::While(c, s, l) => visitor.visit_while_stmt(c, s, l),
            Self::DoWhile(s, c, l) => visitor.visit_do_while_stmt(s, c, l),
            Self::For(i, c, p, s, l) => visitor.visit_for_stmt(i, c, p, s, l),
            Self::Null => (),
        }
    }
}

impl MutVisitable for Decl {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_decl(self);
    }
}

impl MutWalkable for Decl {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        self.init.accept(visitor);
        // if let Some(init) = &mut self.init {
        //     visitor.visit_expr(init);
        // }
    }
}

impl MutVisitable for Block {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_block(self);
    }
}

impl MutWalkable for Block {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        for i in self.iter_mut() {
            i.accept(visitor);
        }
    }
}

impl MutVisitable for BlockItem {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_block_item(self);
    }
}

impl MutWalkable for BlockItem {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        match self {
            Self::Stmt(s) => s.accept(visitor),
            Self::Decl(d) => d.accept(visitor),
        }
    }
}

impl MutVisitable for Function {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_function(self);
    }
}

impl MutWalkable for Function {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        self.body.accept(visitor);
        // visitor.visit_block(&mut self.body);
    }
}

impl MutVisitable for Program {
    fn accept<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        visitor.visit_program(self);
        // visitor.visit_function(&mut self.body);
    }
}

impl MutWalkable for Program {
    fn walk<V: MutVisitor + ?Sized>(&mut self, visitor: &mut V) {
        self.body.accept(visitor);
        // visitor.visit_function(&mut self.body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{BinOpKind, dummy::ident};

    use rstest::{fixture, rstest};

    #[derive(Default)]
    struct CountingVisitor {
        var_count: usize,
        decl_count: usize,
        expr_count: usize,
    }

    impl CountingVisitor {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl MutVisitor for CountingVisitor {
        fn visit_var(&mut self, _: &mut Identifier) {
            self.var_count += 1;
        }

        fn visit_decl(&mut self, d: &mut Decl) {
            self.decl_count += 1;
            d.walk(self);
        }

        fn visit_expr(&mut self, e: &mut Expr) {
            self.expr_count += 1;
            e.walk(self);
        }
    }

    #[fixture]
    fn counter() -> CountingVisitor {
        CountingVisitor::new()
    }

    #[rstest]
    #[case(Expr::constant(1), 0)]
    #[case(Expr::var(""), 1)]
    #[case(Expr::binary(BinOpKind::Add, Expr::var("a"), Expr::var("b")), 2)]
    #[case(
        Expr::binary(
            BinOpKind::Add,
            Expr::binary(BinOpKind::Add, Expr::var("a"), Expr::var("b")),
            Expr::var("c")
        ),
        3
    )]
    fn test_var_count_in_expr(
        mut counter: CountingVisitor,
        #[case] mut expr: Expr,
        #[case] expected_count: usize,
    ) {
        expr.accept(&mut counter);

        assert_eq!(counter.var_count, expected_count);
        assert!(counter.expr_count > 0);
    }

    #[rstest]
    #[case(Decl {
        name: ident("a"),
        init: Some(Expr::binary(BinOpKind::Add, Expr::var("a"), Expr::var("b"))),
        span: Default::default(),
    }, 2)]
    fn test_var_count_in_decl(
        mut counter: CountingVisitor,
        #[case] mut decl: Decl,
        #[case] expected_count: usize,
    ) {
        decl.accept(&mut counter);

        assert_eq!(counter.var_count, expected_count);
        assert_eq!(counter.decl_count, 1);
    }
}
