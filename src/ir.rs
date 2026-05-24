use std::fmt::Display;

use crate::ast;

#[derive(Debug)]
pub struct Program {
    pub body: Function,
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.body)
    }
}

#[derive(Debug)]
pub struct Function {
    pub id: ast::Identifier,
    pub body: Vec<Operation>,
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "fn {}() {{", self.id.value)?;

        for op in &self.body {
            writeln!(f, "   {}", op)?;
        }

        writeln!(f, "}}")?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Operation {
    Return(Value),
    Unary {
        op: UnaryOp,
        src: Value,
        dst: Value,
    },
    Binary {
        op: BinaryOp,
        a: Value,
        b: Value,
        dst: Value,
    },
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Operation::Return(value) => format!("return {}", value),
                Operation::Unary { op, src, dst } => format!("{} = {} {}", dst, op, src),
                Operation::Binary { op, a, b, dst } => format!("{} = {} {}, {}", dst, op, a, b),
            }
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
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

impl Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UnaryOp::Complement => "not",
                UnaryOp::Negate => "neg",
            }
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinaryOp::Add => "add",
                BinaryOp::Sub => "sub",
                BinaryOp::Mul => "mul",
                BinaryOp::Div => "div",
                BinaryOp::Rem => "rem",
            }
        )
    }
}

impl From<ast::BinaryOp> for BinaryOp {
    fn from(op: ast::BinaryOp) -> Self {
        match op {
            ast::BinaryOp::Add => Self::Add,
            ast::BinaryOp::Subtract => Self::Sub,
            ast::BinaryOp::Multiply => Self::Mul,
            ast::BinaryOp::Divide => Self::Div,
            ast::BinaryOp::Modulo => Self::Rem,
        }
    }
}

/// A counter for generating unique variable IDs for temporary variables created during IR generation.
#[allow(non_snake_case)]
mod VarID {
    use std::sync::atomic;

    static COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

    /// Create a new temporary variable with a unique ID.
    #[must_use]
    #[inline]
    pub fn new() -> usize {
        COUNTER.fetch_add(1, atomic::Ordering::Relaxed)
    }

    /// Reset the variable ID counter to zero.
    #[inline]
    #[allow(dead_code)] // currently only used in tests
    pub fn reset() {
        COUNTER.store(0, atomic::Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Constant(i32),
    Var(usize),
}

impl Value {
    #[must_use]
    pub fn new_var() -> Self {
        Self::Var(VarID::new())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Value::Constant(val) => val.to_string(),
                Value::Var(id) => format!("${}", id),
            }
        )
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
        ast::Expr::Binary(binary_op, a, b) => {
            let a = lower_expr(ops, *a);
            let b = lower_expr(ops, *b);
            let dst = Value::new_var();

            ops.push(Operation::Binary {
                op: binary_op.into(),
                a,
                b,
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

#[must_use]
pub fn lower_func(func: ast::Function) -> Function {
    let mut ops = Vec::new();

    lower_stmt(&mut ops, func.body);

    Function {
        id: func.name,
        body: ops,
    }
}

#[must_use]
pub fn lower_program(program: ast::Program) -> Program {
    Program {
        body: lower_func(program.body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;
    use rstest::rstest;
    use serial_test::serial;

    mod var_id {
        use super::*;

        #[test]
        #[serial]
        fn test_id_counter() {
            VarID::reset();

            let v0 = VarID::new();

            assert_eq!(v0, 0);

            let v1 = VarID::new();

            assert_eq!(v1, 1);
            assert_eq!(v0, 0); // ensure first var's id remains unaffected

            assert_eq!(VarID::new(), 2);
            assert_eq!(VarID::new(), 3);
            assert_eq!(VarID::new(), 4);
        }

        #[test]
        #[serial]
        fn test_increasing() {
            let v0 = VarID::new();
            let v1 = VarID::new();

            if v0 < usize::MAX {
                assert!(v1 > v0);
            } else {
                assert_ne!(v0, v1);
            }
        }

        #[test]
        #[serial]
        fn test_reset_id() {
            VarID::reset();

            assert_eq!(VarID::new(), 0);
            assert_eq!(VarID::new(), 1);
            assert_eq!(VarID::new(), 2);
            assert_eq!(VarID::new(), 3);

            VarID::reset();

            assert_eq!(VarID::new(), 0);
            assert_eq!(VarID::new(), 1);
        }
    }

    mod lower {
        use super::*;

        #[rstest]
        #[case::constants(ast::Expr::Const(5), vec![], Value::Constant(5))]
        #[case::negate(
            // given: -5
            // expect:
            //  negate #5 -> %0
            ast::Expr::Unary(ast::UnaryOp::Negate, Box::new(ast::Expr::Const(5))),
            vec![Operation::Unary {
                    op: UnaryOp::Negate,
                    src: Value::Constant(5),
                    dst: Value::Var(0),
                }], Value::Var(0)
        )]
        #[case::nested_negate_and_complement(
            // given: ~(-42)
            // expect:
            //  negate      #42 -> %0
            //  complement  %0  -> %1
            ast::Expr::Unary(ast::UnaryOp::Complement, Box::new(
                ast::Expr::Unary(ast::UnaryOp::Negate, Box::new(ast::Expr::Const(42)))
            )),
            vec![
                Operation::Unary {
                    op: UnaryOp::Negate,
                    src: Value::Constant(42),
                    dst: Value::Var(0),
                },
                Operation::Unary {
                    op: UnaryOp::Complement,
                    src: Value::Var(0),
                    dst: Value::Var(1),
                }
            ], Value::Var(1)
        )]
        #[serial]
        fn test_lower_expr(
            #[case] expr: ast::Expr,
            #[case] expect_ops: Vec<Operation>,
            #[case] expect_val: Value,
        ) {
            VarID::reset();

            let mut ops: Vec<Operation> = Vec::new();

            let value = lower_expr(&mut ops, expr);

            assert_eq!(ops, expect_ops);
            assert_eq!(value, expect_val);
        }

        proptest! {
            #[test]
            fn test_lower_stmt(expr in ast::strategy::arb_expr()) {
                let stmt = ast::Stmt::Return(expr.clone());

                VarID::reset();
                let mut expected_ops = Vec::new();
                let expected_val = lower_expr(&mut expected_ops, expr);

                VarID::reset();
                let mut actual_ops = Vec::new();
                lower_stmt(&mut actual_ops, stmt);

                prop_assert_eq!(actual_ops.last(), Some(&Operation::Return(expected_val)));
                prop_assert_eq!(actual_ops.len(), expected_ops.len() + 1);
                prop_assert_eq!(&actual_ops[..actual_ops.len() - 1], &expected_ops[..]);
            }
        }
    }
}
