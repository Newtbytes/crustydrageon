use std::{collections::HashMap, fmt::Display};

use crate::{ast, ir, x86::Operand::Reg};

pub struct Program {
    pub func: Function,
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.func.fmt(f)?;

        #[cfg(target_os = "linux")]
        f.write_str(".section .note.GNU-stack,\"\",@progbits")?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    name: ast::Identifier,
    body: Vec<Instruction>,
}

impl Function {
    #[must_use]
    pub fn new(mut name: ast::Identifier, body: Vec<Instruction>) -> Self {
        if cfg!(target_os = "macos") {
            name.value = format!("_{}", name.value);
        }

        Self { name, body }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("\t.globl {}\n", self.name.value))?;
        f.write_str(&format!("{}:\n", self.name.value))?;
        // TODO: make adding the function prologue a part of legalization perhaps?
        // alternatively it could be a part of lowering alloca instructions
        f.write_str("\tpushq %rbp\n")?;
        f.write_str("\tmovq %rsp, %rbp\n")?;
        for inst in &self.body {
            f.write_str(&format!("\t{inst}\n"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    AX,
    DX,
    R10,
    R11,
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Register::AX => "eax",
            Register::DX => "edx",
            Register::R10 => "r10d",
            Register::R11 => "r11d",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Imm(i32),
    Reg(Register),
    Pseudo(usize),
    Stack(i32),
}

impl Operand {
    pub fn size_bytes(&self) -> usize {
        match self {
            Operand::Imm(_) => todo!(),
            Operand::Reg(_register) => todo!(),
            Operand::Pseudo(_) => 4,
            Operand::Stack(_) => todo!(),
        }
    }
}

impl From<Register> for Operand {
    fn from(reg: Register) -> Self {
        Self::Reg(reg)
    }
}

impl From<ir::Value> for Operand {
    fn from(value: ir::Value) -> Self {
        match value {
            ir::Value::Constant(val) => Operand::Imm(val),
            ir::Value::Var(id) => Operand::Pseudo(id),
        }
    }
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Imm(val) => f.write_str(&format!("${val}")),
            Operand::Reg(reg) => f.write_str(&format!("%{reg}")),
            Operand::Pseudo(id) => f.write_str(&format!("?{id}")),
            Operand::Stack(offset) => f.write_str(&format!("{offset}(%rbp)")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Alloca(usize),
    Mov { src: Operand, dst: Operand },
    Unary(UnaryOp, Operand),
    Binary(BinaryOp, Operand, Operand),
    Idiv(Operand),
    Cdq,
    Ret,
}

impl Instruction {
    fn get_operands_mut(&mut self) -> Vec<&mut Operand> {
        match self {
            Instruction::Mov { src, dst } => vec![src, dst],
            Instruction::Unary(_, operand) | Instruction::Idiv(operand) => vec![operand],
            Instruction::Binary(_, a, b) => vec![a, b],
            Instruction::Alloca(_) | Instruction::Ret | Instruction::Cdq => Vec::new(),
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Instruction::Mov { src, dst } => format!("movl {src}, {dst}"),
                Instruction::Ret => {
                    // TODO: make adding the function epilogue a part of legalization perhaps?
                    // alternatively it could be a part of lowering alloca instructions
                    write!(f, "movq %rbp, %rsp\n")?;
                    write!(f, "\tpopq %rbp\n")?;
                    write!(f, "\tret")?;
                    return Ok(());
                }
                // TODO: add a alloca lowering pass instead of doing this during emission?
                Instruction::Alloca(size) => format!("subq ${size}, %rsp"),
                Instruction::Unary(unary_op, operand) => format!("{unary_op} {operand}"),
                Instruction::Binary(binary_op, a, b) => format!("{binary_op} {a}, {b}"),
                Instruction::Idiv(operand) => format!("idivl {operand}"),
                Instruction::Cdq => "cdq".to_string(),
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
}

impl From<ir::UnaryOp> for UnaryOp {
    fn from(op: ir::UnaryOp) -> Self {
        match op {
            ir::UnaryOp::Complement => UnaryOp::Not,
            ir::UnaryOp::Negate => UnaryOp::Neg,
        }
    }
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UnaryOp::Neg => "negl",
                UnaryOp::Not => "notl",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinaryOp::Add => "addl",
                BinaryOp::Sub => "subl",
                BinaryOp::Mul => "imull",
            }
        )
    }
}

pub fn lower_op(insts: &mut Vec<Instruction>, op: ir::Operation) {
    match op {
        ir::Operation::Return(value) => {
            insts.push(Instruction::Mov {
                src: value.into(),
                dst: Register::AX.into(),
            });
            insts.push(Instruction::Ret);
        }
        ir::Operation::Unary { op, src, dst } => {
            insts.push(Instruction::Mov {
                src: src.into(),
                dst: dst.into(),
            });
            insts.push(Instruction::Unary(op.into(), dst.into()));
        }
        ir::Operation::Binary { op, a, b, dst } => match op {
            ir::BinaryOp::Add | ir::BinaryOp::Sub | ir::BinaryOp::Mul => {
                insts.extend([
                    Instruction::Mov {
                        src: a.into(),
                        dst: dst.into(),
                    },
                    Instruction::Binary(
                        match op {
                            ir::BinaryOp::Add => BinaryOp::Add,
                            ir::BinaryOp::Sub => BinaryOp::Sub,
                            ir::BinaryOp::Mul => BinaryOp::Mul,
                            _ => unreachable!(),
                        },
                        b.into(),
                        dst.into(),
                    ),
                ]);
            }
            ir::BinaryOp::Div | ir::BinaryOp::Rem => {
                insts.extend([
                    Instruction::Mov {
                        src: a.into(),
                        dst: Register::AX.into(),
                    },
                    Instruction::Cdq,
                    Instruction::Idiv(b.into()),
                    Instruction::Mov {
                        src: if op == ir::BinaryOp::Div {
                            Register::AX
                        } else {
                            Register::DX
                        }
                        .into(),
                        dst: dst.into(),
                    },
                ]);
            }
        },
    }
}

pub fn lower_block(block: Vec<ir::Operation>) -> Vec<Instruction> {
    let mut insts = Vec::new();

    for op in block {
        lower_op(&mut insts, op);
    }

    insts
}

pub fn legalize_inst(insts: &mut Vec<Instruction>, inst: Instruction) {
    match inst {
        Instruction::Mov {
            src: Operand::Stack(a),
            dst: Operand::Stack(b),
        } => {
            insts.push(Instruction::Mov {
                src: Operand::Stack(a),
                dst: Register::R10.into(),
            });

            insts.push(Instruction::Mov {
                src: Register::R10.into(),
                dst: Operand::Stack(b),
            });
        }
        Instruction::Binary(op, Operand::Stack(a), Operand::Stack(b))
            if op == BinaryOp::Add || op == BinaryOp::Sub =>
        {
            insts.extend([
                Instruction::Mov {
                    src: Operand::Stack(a),
                    dst: Register::R10.into(),
                },
                Instruction::Binary(op, Register::R10.into(), Operand::Stack(b)),
            ]);
        }
        Instruction::Binary(BinaryOp::Mul, a, Operand::Stack(b)) => {
            insts.extend([
                Instruction::Mov {
                    src: Operand::Stack(b),
                    dst: Register::R11.into(),
                },
                Instruction::Binary(BinaryOp::Mul, a, Register::R11.into()),
                Instruction::Mov {
                    src: Register::R11.into(),
                    dst: Operand::Stack(b),
                },
            ]);
        }
        Instruction::Idiv(Operand::Imm(val)) => insts.extend([
            Instruction::Mov {
                src: Operand::Imm(val).into(),
                dst: Register::R10.into(),
            },
            Instruction::Idiv(Register::R10.into()),
        ]),
        _ => insts.push(inst),
    }
}

pub fn legalize_block(block: Vec<Instruction>) -> Vec<Instruction> {
    let mut insts = Vec::new();

    // map from pseudo register to stack offset
    let mut stack_map: HashMap<usize, i32> = HashMap::new();
    let mut stack_size: u64 = 0;

    for mut inst in block {
        for operand in inst.get_operands_mut() {
            if let Operand::Pseudo(id) = operand {
                let id = *id;

                stack_map.entry(id).or_insert_with(|| {
                    // allocate new stack slot
                    stack_size += operand.size_bytes() as u64;
                    -(stack_size as i32)
                });

                *operand = Operand::Stack(*stack_map.get(&id).unwrap());
            }
        }

        legalize_inst(&mut insts, inst);
    }

    insts.insert(0, Instruction::Alloca(stack_size as usize));

    insts
}

#[must_use]
pub fn lower_func(func: ir::Function) -> Function {
    Function::new(func.id, legalize_block(lower_block(func.body)))
}

#[must_use]
pub fn lower_program(prg: ir::Program) -> Program {
    Program {
        func: lower_func(prg.body),
    }
}

#[cfg(test)]
mod strategy {
    use super::*;

    use proptest::prelude::*;

    pub fn arb_register() -> impl Strategy<Value = Register> {
        prop_oneof![Just(Register::AX), Just(Register::R10)]
    }

    pub fn arb_unary_op() -> impl Strategy<Value = UnaryOp> {
        prop_oneof![Just(UnaryOp::Neg), Just(UnaryOp::Not)]
    }

    pub fn arb_binary_op() -> impl Strategy<Value = BinaryOp> {
        prop_oneof![
            Just(BinaryOp::Add),
            Just(BinaryOp::Sub),
            Just(BinaryOp::Mul)
        ]
    }

    pub fn arb_operand() -> impl Strategy<Value = Operand> {
        prop_oneof![
            any::<i32>().prop_map(Operand::Imm),
            arb_register().prop_map(Operand::Reg),
            (0..100_usize).prop_map(Operand::Pseudo),
            (-128..128).prop_map(Operand::Stack),
        ]
    }

    pub fn arb_instruction() -> impl Strategy<Value = Instruction> {
        prop_oneof![
            Just(Instruction::Ret),
            (1..1000_usize).prop_map(Instruction::Alloca),
            (arb_operand(), arb_operand()).prop_map(|(src, dst)| Instruction::Mov { src, dst }),
            (arb_unary_op(), arb_operand())
                .prop_map(|(op, operand)| Instruction::Unary(op, operand)),
            (arb_binary_op(), arb_operand(), arb_operand())
                .prop_map(|(op, a, b)| Instruction::Binary(op, a, b)),
            (arb_operand()).prop_map(|operand| Instruction::Idiv(operand)),
            Just(Instruction::Cdq),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    use self::strategy::*;

    mod function {
        use super::*;

        proptest! {
            /// On macOS, function names must be prefixed with an underscore
            #[test]
            fn test_macos_name_prefix(id in ast::strategy::arb_identifier()) {
                let func = Function::new(id.clone(), Vec::new());

                if cfg!(target_os = "macos") {
                    assert_eq!(func.name.value, format!("_{}", id.value));
                } else {
                    assert_eq!(func.name.value, id.value);
                }
            }

            #[test]
            fn test_function_fmt(id in ast::strategy::arb_identifier(), block in prop::collection::vec(arb_instruction(), 0..10)) {
                let func = Function::new(id.clone(), block.clone());
                let fmt = format!("{}", func);

                // emitted code should contain the correct global directive and label for the function
                assert!(fmt.contains(&format!(".globl {}", func.name.value)));
                assert!(fmt.contains(&format!("{}:", func.name.value)));

                // emitted code should contain the function prologue
                assert!(fmt.contains("pushq %rbp"));
                assert!(fmt.contains("movq %rsp, %rbp"));
            }
        }
    }

    mod lowering {
        use super::*;

        #[test]
        fn test_from_ir_unary_op() {
            assert_eq!(UnaryOp::from(ir::UnaryOp::Complement), UnaryOp::Not);
            assert_eq!(UnaryOp::from(ir::UnaryOp::Negate), UnaryOp::Neg);
        }
    }

    mod legalization {
        use super::*;

        fn check_legalized(inst: Instruction) -> bool {
            !matches!(
                inst,
                Instruction::Mov {
                    src: Operand::Stack(_),
                    dst: Operand::Stack(_),
                }
            )
        }

        proptest! {
            #[test]
            fn test_mov_stack_to_stack(offset_a in -128..128, offset_b in -128..128) {
                let inst = Instruction::Mov {
                    src: Operand::Stack(offset_a),
                    dst: Operand::Stack(offset_b),
                };

                let mut insts = Vec::new();
                legalize_inst(&mut insts, inst);

                assert_eq!(insts.len(), 2);
                assert_eq!(
                    insts[0],
                    Instruction::Mov {
                        src: Operand::Stack(offset_a),
                        dst: Register::R10.into(),
                    }
                );
                assert_eq!(
                    insts[1],
                    Instruction::Mov {
                        src: Register::R10.into(),
                        dst: Operand::Stack(offset_b),
                    }
                );

                assert!(insts.iter().all(|inst| check_legalized(inst.clone())));
            }

            #[test]
            fn test_other_instructions(inst in arb_instruction()) {
                let mut insts = Vec::new();
                legalize_inst(&mut insts, inst.clone());
                assert!(insts.iter().all(|inst| check_legalized(inst.clone())));
            }
        }
    }
}
