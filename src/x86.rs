use std::fmt::Display;

use crate::ast;

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
    pub fn new(name: ast::Identifier, body: Vec<Instruction>) -> Self {
        if !cfg!(target_os = "macos") {
            Self { name, body }
        } else {
            Self {
                name: ast::Identifier {
                    value: format!("_{}", name.value),
                },
                body,
            }
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("\t.globl {}\n", self.name.value))?;
        f.write_str(&format!("{}:\n", self.name.value))?;
        for inst in &self.body {
            f.write_str(&format!("\t{}\n", inst))?;
        }
        Ok(())
    }
}

pub enum Register {
    EAX,
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Register::EAX => f.write_str("eax"),
        }
    }
}

pub enum Operand {
    Imm(i32),
    Reg(Register),
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Imm(val) => f.write_str(&format!("${}", val)),
            Operand::Reg(reg) => f.write_str(&format!("%{}", reg)),
        }
    }
}

pub enum Instruction {
    Mov { src: Operand, dst: Operand },
    Ret,
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Mov { src, dst } => f.write_str(&format!("movl {}, {}", src, dst)),
            Instruction::Ret => f.write_str("ret"),
        }
    }
}

pub fn lower_expr(expr: ast::Expr) -> Operand {
    match expr {
        ast::Expr::Const(val) => Operand::Imm(val),
    }
}

pub fn lower_stmt(stmt: ast::Stmt) -> Vec<Instruction> {
    match stmt {
        ast::Stmt::Return(expr) => vec![
            Instruction::Mov {
                src: lower_expr(expr),
                dst: Operand::Reg(Register::EAX),
            },
            Instruction::Ret,
        ],
    }
}

pub fn lower_func(func: ast::Function) -> Function {
    Function::new(func.name, lower_stmt(func.body))
}

pub fn lower_program(prg: ast::Program) -> Program {
    Program {
        func: lower_func(prg.body),
    }
}
