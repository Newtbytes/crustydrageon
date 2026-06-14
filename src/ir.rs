use std::fmt::Display;

#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::ast;

#[derive(Debug)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Program {
    pub body: Function,
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.body)
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Function {
    pub id: ast::Identifier,
    pub body: Vec<Operation>,
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        cov_mark::hit!(ir_pp_function);

        writeln!(f, "fn {}() {{", self.id)?;

        for op in &self.body {
            if !matches!(op, Operation::Label(_)) {
                writeln!(f, "  {op}")?;
            } else {
                writeln!(f, "{op}")?;
            }
        }

        writeln!(f, "}}")?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum Label {
    Named(ast::Identifier),
    Anon(usize),
}

impl Label {
    fn new() -> Self {
        Self::Anon(VarID::new())
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        cov_mark::hit!(ir_pp_label);

        match self {
            Label::Named(id) => write!(f, "{id}"),
            Label::Anon(id) => write!(f, "{id}"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
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
    Copy {
        src: Value,
        dst: Value,
    },
    Branch(Label),
    BranchIf {
        cond: Value,
        then_label: Label,
        else_label: Label,
    },
    BranchWhen {
        cond: Value,
        when_label: Label,
    },
    Label(Label),
}

impl Operation {
    pub fn get_operands(&self) -> Vec<&Value> {
        match self {
            Operation::Return(value) => vec![value],
            Operation::Unary { op: _, src, dst } | Operation::Copy { src, dst } => vec![src, dst],
            Operation::Binary { op: _, a, b, dst } => vec![a, b, dst],
            Operation::Branch(_) | Operation::Label(_) => Vec::new(),
            Operation::BranchIf {
                cond,
                then_label: _,
                else_label: _,
            }
            | Operation::BranchWhen {
                cond,
                when_label: _,
            } => vec![cond],
        }
    }

    pub fn is_branch(&self) -> bool {
        match self {
            Operation::Branch(_) | Operation::BranchIf { .. } | Operation::BranchWhen { .. } => {
                true
            }
            Operation::Return(_)
            | Operation::Unary { .. }
            | Operation::Binary { .. }
            | Operation::Copy { .. }
            | Operation::Label(_) => false,
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        cov_mark::hit!(ir_pp_op);

        write!(
            f,
            "{}",
            match self {
                Operation::Return(value) => format!("return {value}"),
                Operation::Unary { op, src, dst } => format!("{dst} = {op} {src}"),
                Operation::Binary { op, a, b, dst } => format!("{dst} = {op} {a}, {b}"),
                Operation::Copy { src, dst } => format!("copy {dst} = {src}"),
                Operation::Branch(label) => format!("branch {label}"),
                Operation::BranchIf {
                    cond,
                    then_label,
                    else_label,
                } => format!("branchif {cond} then {then_label} else {else_label}"),
                Operation::BranchWhen { cond, when_label } =>
                    format!("branch {when_label} when {cond}"),
                Operation::Label(label) => format!("{label}:"),
            }
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum UnaryOp {
    Complement,
    Negate,
    Not,
}

impl From<ast::UnOpKind> for UnaryOp {
    fn from(op: ast::UnOpKind) -> Self {
        match op {
            ast::UnOpKind::Complement => Self::Complement,
            ast::UnOpKind::Negate => Self::Negate,
            ast::UnOpKind::Not => Self::Not,
        }
    }
}

impl From<ast::UnOp> for UnaryOp {
    fn from(op: ast::UnOp) -> Self {
        Self::from(op.kind)
    }
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        cov_mark::hit!(ir_pp_unary_op_kind);

        write!(
            f,
            "{}",
            match self {
                UnaryOp::Complement => "not",
                UnaryOp::Negate => "neg",
                UnaryOp::Not => "not",
            }
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    /// Bitwise and.
    And,
    /// Bitwise or.
    Or,
    Xor,
    /// Logical shift left.
    ///
    /// Shift bits to the left, filling new bits to the right with zeros.
    Ashl,
    /// Logical shift right.
    ///
    /// Shift bits to the right, filling new bits to the left with zeros.
    Ashr,
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        cov_mark::hit!(ir_pp_binary_op_kind);

        write!(
            f,
            "{}",
            match self {
                Self::Add => "add",
                Self::Sub => "sub",
                Self::Mul => "mul",
                Self::Div => "div",
                Self::Rem => "rem",
                Self::Eq => "eq",
                Self::Neq => "neq",
                Self::Lt => "lt",
                Self::Lte => "lte",
                Self::Gt => "gt",
                Self::Gte => "gte",
                Self::And => "and",
                Self::Or => "or",
                Self::Xor => "xor",
                Self::Ashl => "ashl",
                Self::Ashr => "ashr",
            }
        )
    }
}

impl TryFrom<ast::BinOpKind> for BinaryOp {
    type Error = ();

    fn try_from(kind: ast::BinOpKind) -> Result<Self, Self::Error> {
        match kind {
            ast::BinOpKind::Add => Ok(Self::Add),
            ast::BinOpKind::Subtract => Ok(Self::Sub),
            ast::BinOpKind::Multiply => Ok(Self::Mul),
            ast::BinOpKind::Divide => Ok(Self::Div),
            ast::BinOpKind::Modulo => Ok(Self::Rem),
            ast::BinOpKind::Equal => Ok(Self::Eq),
            ast::BinOpKind::NotEqual => Ok(Self::Neq),
            ast::BinOpKind::LessThan => Ok(Self::Lt),
            ast::BinOpKind::LessOrEqual => Ok(Self::Lte),
            ast::BinOpKind::GreaterThan => Ok(Self::Gt),
            ast::BinOpKind::GreaterOrEqual => Ok(Self::Gte),
            ast::BinOpKind::BitAnd => Ok(Self::And),
            ast::BinOpKind::BitOr => Ok(Self::Or),
            ast::BinOpKind::Xor => Ok(Self::Xor),
            ast::BinOpKind::LShift => Ok(Self::Ashl),
            ast::BinOpKind::RShift => Ok(Self::Ashr),
            ast::BinOpKind::Assign | ast::BinOpKind::And | ast::BinOpKind::Or => Err(()), // handled separately in lower_expr
        }
    }
}

impl TryFrom<ast::BinOp> for BinaryOp {
    type Error = ();

    fn try_from(op: ast::BinOp) -> Result<Self, Self::Error> {
        op.kind.try_into()
    }
}

/// A counter for generating unique variable IDs for temporary variables created during IR generation.
#[allow(non_snake_case)]
pub mod VarID {
    use std::cell::Cell;

    thread_local! {
        static COUNTER: Cell<usize> = const { Cell::new(0) };
    }

    /// Create a new temporary variable with a unique ID.
    #[must_use]
    pub fn new() -> usize {
        cov_mark::hit!(ir_var_id_created);
        let val = COUNTER.get() + 1;
        COUNTER.set(val);
        val - 1
    }
    /// Reset the variable ID counter to zero.
    pub fn reset() {
        COUNTER.set(0);
        cov_mark::hit!(ir_var_id_counter_reset);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum Value {
    Constant(i32),
    Var(String),
}

impl Value {
    #[must_use]
    pub fn new_var() -> Self {
        cov_mark::hit!(ir_var_created);
        Self::Var(VarID::new().to_string())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        cov_mark::hit!(ir_pp_value);

        write!(
            f,
            "{}",
            match self {
                Value::Constant(val) => val.to_string(),
                Value::Var(id) => format!("${id}"),
            }
        )
    }
}

fn binary(ops: &mut Vec<Operation>, op: BinaryOp, a: Value, b: Value) -> Value {
    let dst = Value::new_var();
    ops.push(Operation::Binary {
        op,
        a,
        b,
        dst: dst.clone(),
    });
    dst
}

fn jeq(ops: &mut Vec<Operation>, a: Value, b: Value, label: Label) {
    let eq = binary(ops, BinaryOp::Eq, a, b);
    ops.push(Operation::BranchWhen {
        cond: eq,
        when_label: label,
    });
}

fn jneq(ops: &mut Vec<Operation>, a: Value, b: Value, label: Label) {
    let neq = binary(ops, BinaryOp::Neq, a, b);
    ops.push(Operation::BranchWhen {
        cond: neq,
        when_label: label,
    });
}

pub fn lower_expr(ops: &mut Vec<Operation>, expr: ast::Expr) -> Value {
    let dst = match expr.kind {
        ast::ExprKind::Const(val) => Value::Constant(val),
        ast::ExprKind::Unary(unary_op, expr) => {
            let src = lower_expr(ops, *expr);
            let dst = Value::new_var();

            ops.push(Operation::Unary {
                op: unary_op.into(),
                src,
                dst: dst.clone(),
            });

            cov_mark::hit!(ir_unary_op_lowered);

            dst
        }
        ast::ExprKind::Binary(op, a, b)
            if op.kind == ast::BinOpKind::And || op.kind == ast::BinOpKind::Or =>
        {
            let skip_label = Label::new();
            let end_label = Label::new();
            let result = Value::new_var();

            let a = lower_expr(ops, *a);
            if op.kind == ast::BinOpKind::And {
                jeq(ops, a, Value::Constant(0), skip_label.clone());
            } else {
                jneq(ops, a, Value::Constant(0), skip_label.clone());
            }

            let b = lower_expr(ops, *b);
            if op.kind == ast::BinOpKind::And {
                jeq(ops, b, Value::Constant(0), skip_label.clone());
            } else {
                jneq(ops, b, Value::Constant(0), skip_label.clone());
            }

            ops.extend([
                Operation::Copy {
                    src: Value::Constant(if op.kind == ast::BinOpKind::And { 1 } else { 0 }),
                    dst: result.clone(),
                },
                Operation::Branch(end_label.clone()),
                Operation::Label(skip_label),
                Operation::Copy {
                    src: Value::Constant(if op.kind == ast::BinOpKind::And { 0 } else { 1 }),
                    dst: result.clone(),
                },
                Operation::Label(end_label),
            ]);

            result
        }
        ast::ExprKind::Binary(op, dst, src) if op.kind == ast::BinOpKind::Assign => {
            let dst = lower_expr(ops, *dst);
            let src = lower_expr(ops, *src);

            ops.push(Operation::Copy {
                src,
                dst: dst.clone(),
            });

            dst
        }
        ast::ExprKind::Binary(binary_op, a, b) => {
            let a = lower_expr(ops, *a);
            let b = lower_expr(ops, *b);
            let dst = Value::new_var();

            ops.push(Operation::Binary {
                op: BinaryOp::try_from(binary_op).unwrap(),
                a,
                b,
                dst: dst.clone(),
            });

            cov_mark::hit!(ir_binary_op_lowered);

            dst
        }
        ast::ExprKind::Var(identifier) => Value::Var(identifier.to_string()),
        ast::ExprKind::Cond(cond, if_true, if_false) => {
            let cond = lower_expr(ops, *cond);
            let dst = Value::new_var();

            let when_false = Label::new();
            let end = Label::new();

            jeq(ops, cond, Value::Constant(0), when_false.clone());

            let true_val = lower_expr(ops, *if_true);
            ops.extend([
                Operation::Copy {
                    src: true_val,
                    dst: dst.clone(),
                },
                Operation::Branch(end.clone()),
            ]);

            ops.push(Operation::Label(when_false));
            let false_val = lower_expr(ops, *if_false);
            ops.extend([Operation::Copy {
                src: false_val,
                dst: dst.clone(),
            }]);

            ops.push(Operation::Label(end));

            dst
        }
    };

    cov_mark::hit!(ir_expr_lowered);

    dst
}

pub fn lower_stmt(ops: &mut Vec<Operation>, stmt: ast::Stmt) {
    match stmt {
        ast::Stmt::Return(expr) => {
            let value = lower_expr(ops, expr);
            ops.push(Operation::Return(value));
            cov_mark::hit!(ir_return_stmt_lowered);
        }
        ast::Stmt::Expr(expr) => {
            lower_expr(ops, expr);
            cov_mark::hit!(ir_expr_stmt_lowered);
        }
        ast::Stmt::Null => {
            cov_mark::hit!(ir_null_stmt_lowered);
        }
        ast::Stmt::If(cond, if_true, if_false) => {
            let cond = lower_expr(ops, cond);
            let end = Label::new();

            if let Some(if_false) = if_false {
                let when_false = Label::new();

                jeq(ops, cond, Value::Constant(0), when_false.clone());
                lower_stmt(ops, *if_true);
                ops.push(Operation::Branch(end.clone()));

                ops.push(Operation::Label(when_false));
                lower_stmt(ops, *if_false);
            } else {
                jeq(ops, cond, Value::Constant(0), end.clone());
                lower_stmt(ops, *if_true);
            }

            ops.push(Operation::Label(end));
        }
    }

    cov_mark::hit!(ir_stmt_lowered);
}

pub fn lower_decl(ops: &mut Vec<Operation>, decl: ast::Decl) {
    if let Some(init) = decl.init {
        let init = lower_expr(ops, init);

        ops.push(Operation::Copy {
            src: init,
            dst: Value::Var(decl.name.to_string()),
        });
    }
}

pub fn lower_block_item(ops: &mut Vec<Operation>, block_item: ast::BlockItem) {
    match block_item {
        ast::BlockItem::Stmt(stmt) => lower_stmt(ops, stmt),
        ast::BlockItem::Decl(decl) => lower_decl(ops, decl),
    }
}

#[must_use]
pub fn lower_func(func: ast::Function) -> Function {
    let mut ops = Vec::new();

    if !func.body.is_empty() {
        for block_item in func.body {
            lower_block_item(&mut ops, block_item);
        }
    }

    ops.push(Operation::Return(Value::Constant(0)));

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

            cov_mark::check_count!(ir_var_id_created, 5);

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
            cov_mark::check_count!(ir_var_id_created, 2);

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
            cov_mark::check_count!(ir_var_id_counter_reset, 2);

            VarID::reset();

            assert_eq!(VarID::new(), 0);
            assert_eq!(VarID::new(), 1);
            assert_eq!(VarID::new(), 2);
            assert_eq!(VarID::new(), 3);

            VarID::reset();

            assert_eq!(VarID::new(), 0);
            assert_eq!(VarID::new(), 1);
        }

        #[test]
        #[serial]
        fn test_var_uses_var_id() {
            cov_mark::check_count!(ir_var_id_created, 2);
            cov_mark::check_count!(ir_var_created, 1);

            VarID::reset();

            let _ = VarID::new();
            let _ = Value::new_var();
        }
    }

    // TODO: use rstest_reuse to consolidate all operation test cases into a template

    mod lower {
        use super::*;

        #[rstest]
        #[case::constants(ast::Expr::constant(5), vec![], Value::Constant(5))]
        #[case::negate(
            // given: -5
            // expect:
            //  negate #5 -> %0
            ast::Expr::unary(ast::UnOpKind::Negate, ast::Expr::constant(5)),
            vec![Operation::Unary {
                    op: UnaryOp::Negate,
                    src: Value::Constant(5),
                    dst: Value::Var(0.to_string()),
                }], Value::Var(0.to_string())
        )]
        #[case::nested_negate_and_complement(
            // given: ~(-42)
            // expect:
            //  negate      #42 -> %0
            //  complement  %0  -> %1
            ast::Expr::unary(ast::UnOpKind::Complement,
                ast::Expr::unary(ast::UnOpKind::Negate, ast::Expr::constant(42))
            ),
            vec![
                Operation::Unary {
                    op: UnaryOp::Negate,
                    src: Value::Constant(42),
                    dst: Value::Var(0.to_string()),
                },
                Operation::Unary {
                    op: UnaryOp::Complement,
                    src: Value::Var(0.to_string()),
                    dst: Value::Var(1.to_string()),
                }
            ], Value::Var(1.to_string())
        )]
        #[serial]
        fn test_lower_expr(
            #[case] expr: ast::Expr,
            #[case] expect_ops: Vec<Operation>,
            #[case] expect_val: Value,
        ) {
            cov_mark::check!(ir_expr_lowered);

            VarID::reset();

            let mut ops: Vec<Operation> = Vec::new();

            let value = lower_expr(&mut ops, expr);

            assert_eq!(ops, expect_ops);
            assert_eq!(value, expect_val);
        }

        // proptest! {
        //     #[test]
        //     #[serial]
        //     fn test_lower_stmt(expr: ast::Expr) {
        //         cov_mark::check!(ir_stmt_lowered);
        //         cov_mark::check!(ir_expr_lowered);
        //         cov_mark::check_count!(ir_return_stmt_lowered, 1);

        //         let stmt = ast::Stmt::Return(expr.clone());

        //         VarID::reset();
        //         let mut expected_ops = Vec::new();
        //         let expected_val = lower_expr(&mut expected_ops, expr);

        //         VarID::reset();
        //         let mut actual_ops = Vec::new();
        //         lower_stmt(&mut actual_ops, stmt);

        //         prop_assert_eq!(actual_ops.last(), Some(&Operation::Return(expected_val)));
        //         prop_assert_eq!(actual_ops.len(), expected_ops.len() + 1);
        //         prop_assert_eq!(&actual_ops[..actual_ops.len() - 1], &expected_ops[..]);
        //     }

        //     #[test]
        //     #[ignore]
        //     fn test_lowered_func_contains_more_ops(func: ast::Function) {
        //         let ir_func = lower_func(func.clone());

        //         prop_assert!(
        //             ir_func.body.len() >= func.body.iter()
        //                 .filter(|item| {
        //                     matches!(item, ast::BlockItem::Stmt(_))
        //                     && !matches!(item, ast::BlockItem::Stmt(ast::Stmt::Null))
        //                 }).count(),
        //             "ir opcount of {} should be >= ast stmt count {}:\n{}",
        //             ir_func.body.len(), func.body.len(), ir_func
        //         );
        //     }
        // }
    }

    /// Test pretty printing & Display implementations
    mod pretty {
        use super::*;

        mod value {
            use super::*;

            proptest! {
                #[test]
                fn test_no_whitespace(val in any::<i32>(), id in any::<usize>()) {
                    let c = Value::Constant(val).to_string();
                    let v = Value::Var(id.to_string()).to_string();

                    prop_assert_eq!(c.trim(), &c);
                    prop_assert_eq!(v.trim(), &v);
                }

                #[test]
                fn test_variants_differ(val in any::<u16>() /* u16 so it fits in usize & i32 */) {
                    let c = Value::Constant(val.into()).to_string();
                    let v = Value::Var(val.to_string()).to_string();

                    prop_assert_ne!(c, v);
                }

                #[test]
                fn test_value_const_pretty(val in any::<i32>()) {
                    let c = Value::Constant(val);

                    prop_assert_eq!(c.to_string(), val.to_string());
                }

                #[test]
                fn test_value_var_pretty(id in any::<usize>()) {
                    let v = Value::Var(id.to_string());
                    let s = v.to_string();

                    prop_assert!(s.starts_with('$'));
                    prop_assert!(s.contains(&id.to_string()));
                }
            }
        }

        mod label {
            use super::*;

            proptest! {
                #[test]
                fn differing_value_implies_differing_pp(id1 in any::<usize>(), id2 in any::<usize>()) {
                    let l1 = Label::Anon(id1);
                    let l2 = Label::Anon(id2);

                    assert_eq!(l1 == l2, l1.to_string() == l2.to_string());
                }

                #[test]
                fn pp_contains_anon_id(id in any::<usize>()) {
                    assert!(Label::Anon(id).to_string().contains(&id.to_string()));
                }

                #[test]
                fn pp_contains_named_id(id in any::<ast::Identifier>()) {
                    let l = Label::Named(id.clone()).to_string();
                    assert!(l.contains(&id.to_string()));
                }
            }
        }

        mod op {
            use super::*;

            #[rstest]
            #[case(
                Operation::Binary {
                    op: BinaryOp::Add,
                    a: Value::Constant(5), b: Value::Var(3.to_string()),
                    dst: Value::Var(3.to_string())
                }
            )]
            #[case(
                Operation::Unary { op: UnaryOp::Negate, src: Value::Var(2.to_string()), dst: Value::Var(5.to_string()) }
            )]
            #[case(Operation::Branch(Label::Anon(5)))]
            #[case(Operation::BranchIf {
                cond: Value::Var(2.to_string()),
                then_label: Label::Anon(5),
                else_label: Label::Named(
                    ast::Identifier::default()
                )
            })]
            #[case(Operation::Label(Label::Anon(2)))]
            #[case(Operation::Label(Label::Named(ast::Identifier::default())))]
            #[case(Operation::BranchWhen { cond: Value::Constant(5), when_label: Label::Anon(2) })]
            fn test_op_contains_info(#[case] op: Operation) {
                cov_mark::check!(ir_pp_op);

                let op_pp = op.to_string();

                // TODO: once rstest_reuse is used for making templates of operation cases, separate checks for the presence of '=' into a separate test

                match op {
                    Operation::Return(_) | Operation::Copy { .. } => {}
                    Operation::Unary { op, src: _, dst: _ } => {
                        cov_mark::check!(ir_pp_unary_op_kind);

                        assert!(op_pp.contains(&op.to_string()));

                        assert!(op_pp.contains('='));
                    }
                    Operation::Binary {
                        op,
                        a: _,
                        b: _,
                        dst: _,
                    } => {
                        cov_mark::check!(ir_pp_binary_op_kind);

                        assert!(op_pp.contains(&op.to_string()));

                        assert!(op_pp.contains('='));
                    }
                    Operation::Branch(label) => {
                        cov_mark::check!(ir_pp_label);

                        assert!(op_pp.contains("branch"));

                        assert!(op_pp.contains(&label.to_string()));
                    }
                    Operation::BranchIf {
                        cond: _,
                        then_label,
                        else_label,
                    } => {
                        cov_mark::check!(ir_pp_label);

                        assert!(op_pp.contains("branch"));
                        assert!(op_pp.contains("if"));

                        assert!(op_pp.contains(&then_label.to_string()));
                        assert!(op_pp.contains(&else_label.to_string()));
                    }
                    Operation::BranchWhen {
                        cond: _,
                        when_label,
                    } => {
                        cov_mark::check!(ir_pp_label);

                        assert!(op_pp.contains("when"));

                        assert!(op_pp.contains(&when_label.to_string()));
                    }
                    Operation::Label(label) => {
                        cov_mark::check!(ir_pp_label);

                        assert!(op_pp.contains(&label.to_string()));
                    }
                }
            }

            proptest! {
                #[test]
                fn test_op_contains_operands(op: Operation) {
                    cov_mark::check!(ir_pp_op);

                    let op_pp = op.to_string();

                    for operand in op.get_operands() {
                        cov_mark::check!(ir_pp_value);

                        prop_assert!(op_pp.contains(&operand.to_string()));
                    }
                }

                #[test]
                fn test_branch_ops(op: Operation) {
                    if op.is_branch() {
                        // ensure branches contain labels
                        cov_mark::check!(ir_pp_label);
                        let _ =  op.to_string();
                    }

                    let op_pp = op.to_string();

                    prop_assert_eq!(op.is_branch(), op_pp.contains("branch"));
                }
            }
        }

        // TODO: test Function pretty printing

        // TODO: test Program pretty printing
    }
}
