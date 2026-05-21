use std::sync::atomic;

use crate::ast;

#[derive(Debug)]
pub struct Program {
    pub body: Function,
}

#[derive(Debug)]
pub struct Function {
    pub id: ast::Identifier,
    pub body: Vec<Operation>,
}

#[derive(Debug)]
pub enum Operation {
    Return(Value),
    Unary { op: UnaryOp, src: Value, dst: Value },
}

#[derive(Debug)]
pub enum UnaryOp {
    Complement,
    Negate,
}

impl From<ast::UnaryOp> for UnaryOp {
    fn from(op: ast::UnaryOp) -> Self {
        match op {
            ast::UnaryOp::Complement => Self::Complement,
            ast::UnaryOp::Negate => Self::Negate,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Constant(i32),
    Var(usize),
}

impl Value {
    #[must_use]
    #[inline]
    pub fn new_id() -> usize {
        static TMP_ID_COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

        TMP_ID_COUNTER.fetch_add(1, atomic::Ordering::Relaxed)
    }

    pub fn new_var() -> Self {
        Self::Var(Self::new_id())
    }
}

pub fn lower_unary_op(unary_op: ast::UnaryOp) -> UnaryOp {
    match unary_op {
        ast::UnaryOp::Complement => UnaryOp::Complement,
        ast::UnaryOp::Negate => UnaryOp::Negate,
    }
}

pub fn lower_expr(ops: &mut Vec<Operation>, expr: ast::Expr) -> Value {
    match expr {
        ast::Expr::Const(val) => Value::Constant(val),
        ast::Expr::Unary(unary_op, expr) => {
            let src = lower_expr(ops, *expr);
            let dst = Value::new_var();

            ops.push(Operation::Unary {
                op: unary_op.into(),
                src,
                dst,
            });

            dst
        }
    }
}

pub fn lower_stmt(ops: &mut Vec<Operation>, stmt: ast::Stmt) {
    match stmt {
        ast::Stmt::Return(expr) => {
            let value = lower_expr(ops, expr);
            ops.push(Operation::Return(value));
        }
    }
}

pub fn lower_func(func: ast::Function) -> Function {
    let mut ops = Vec::new();

    lower_stmt(&mut ops, func.body);

    Function {
        id: func.name,
        body: ops,
    }
}

pub fn lower_program(program: ast::Program) -> Program {
    Program {
        body: lower_func(program.body),
    }
}
