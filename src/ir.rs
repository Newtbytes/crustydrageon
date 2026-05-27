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

#[derive(Debug, PartialEq, Eq)]
pub enum Operation {
    Return(Value),
    Unary { op: UnaryOp, src: Value, dst: Value },
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
    pub fn new_var() -> Self {
        Self::Var(VarID::new())
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
        ast::Expr::Binary(binary_op, expr, expr1) => todo!("lowering to IR binary ops"),
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
