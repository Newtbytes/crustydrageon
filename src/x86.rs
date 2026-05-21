use std::{collections::HashMap, fmt::Display};

use crate::{ast, ir};

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
        f.write_str(&format!("\tpushq %rbp\n"))?;
        f.write_str(&format!("\tmovq %rsp, %rbp\n"))?;
        for inst in &self.body {
            f.write_str(&format!("\t{inst}\n"))?;
        }
        Ok(())
    }
}

pub enum Register {
    AX,
    R10,
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Register::AX => "eax",
            Register::R10 => "r10d",
        })
    }
}

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
            Operand::Reg(register) => todo!(),
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

pub enum Instruction {
    Alloca(usize),
    Mov { src: Operand, dst: Operand },
    Unary(UnaryOp, Operand),
    Ret,
}

impl Instruction {
    fn get_operands_mut(&mut self) -> Vec<&mut Operand> {
        match self {
            Instruction::Mov { src, dst } => vec![src, dst],
            Instruction::Unary(_unary_op, operand) => vec![operand],
            _ => Vec::new(),
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Mov { src, dst } => f.write_str(&format!("movl {src}, {dst}")),
            Instruction::Ret => {
                // TODO: make adding the function epilogue a part of legalization perhaps?
                // alternatively it could be a part of lowering alloca instructions
                f.write_str("movq %rbp, %rsp\n")?;
                f.write_str("\tpopq %rbp\n")?;
                f.write_str("\tret")?;
                Ok(())
            }
            // TODO: add a alloca lowering pass instead of doing this during emission?
            Instruction::Alloca(size) => f.write_str(&format!("subq ${size}, %rsp")),
            Instruction::Unary(unary_op, operand) => f.write_str(&format!("{unary_op} {operand}")),
        }
    }
}

pub enum UnaryOp {
    Neg,
    Not,
}

impl From<ir::UnaryOp> for UnaryOp {
    fn from(op: ir::UnaryOp) -> Self {
        match op {
            ir::UnaryOp::Complement => UnaryOp::Neg,
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

#[must_use]
pub fn lower_expr(expr: ast::Expr) -> Operand {
    match expr {
        ast::Expr::Const(val) => Operand::Imm(val),
        ast::Expr::Unary(_op, _expr) => todo!(),
    }
}

#[must_use]
pub fn lower_stmt(stmt: ast::Stmt) -> Vec<Instruction> {
    match stmt {
        ast::Stmt::Return(expr) => vec![
            Instruction::Mov {
                src: lower_expr(expr),
                dst: Operand::Reg(Register::AX),
            },
            Instruction::Ret,
        ],
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

                if !stack_map.contains_key(&id) {
                    // allocate new stack slot
                    stack_size += operand.size_bytes() as u64;
                    stack_map.insert(id, -(stack_size as i32));
                }

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
