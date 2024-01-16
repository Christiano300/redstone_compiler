use std::{collections::HashMap, i32};

use crate::frontend::{Code, Expression, Operator, Parser, Statement};

use super::{Instruction, InstructionVariant};

pub enum CompilerError {
    NonexistentVar(String),
    TooManyVars,
}

type Res<T = ()> = Result<T, CompilerError>;

macro_rules! instr {
    ($self:ident, $variant:ident, $arg:expr) => {
        $self.instructions.push(Instruction {
            variant: &InstructionVariant::$variant,
            arg: Some($arg),
        })
    };
    ($self:ident, $variant:ident) => {
        $self.instructions.push(Instruction {
            variant: &InstructionVariant::$variant,
            arg: None,
        })
    };
}

pub fn compile_program(ast: Code) -> Vec<Instruction> {
    match ast {
        Code::Expr(..) => panic!(),
        Code::Stmt(stmt) => match stmt {
            Statement::Program { body } => {
                let compiler = Compiler::new();
                compiler.generate_assembly(body)
            }
            _ => panic!(),
        },
    }
}

pub fn compile_src(source_code: String) -> Vec<Instruction> {
    let mut parser = Parser::new();
    compile_program(parser.produce_ast(source_code).unwrap())
}

#[derive(Default, Copy, Clone)]
pub enum RegisterContents {
    Variable(u8),
    Number(i16),
    Result(i16),
    RamAddress(i32),
    #[default]
    Unknown,
}

#[derive(Default, Copy, Clone)]
pub struct ComputerState {
    pub reg_a: RegisterContents,
    pub reg_b: RegisterContents,
    pub reg_c: u8,
}

struct Scope {
    start_state: ComputerState,
    variables: HashMap<String, u8>,
    inline_variables: HashMap<String, i16>,
}

pub struct Compiler {
    scopes: Vec<Scope>,
    instructions: Vec<Instruction>,
}

impl Compiler {
    fn new() -> Compiler {
        Compiler {
            scopes: vec![Scope {
                start_state: Default::default(),
                variables: HashMap::new(),
                inline_variables: HashMap::new(),
            }],
            instructions: vec![],
        }
    }

    fn insert_inline_var(&mut self, symbol: String, value: i16) {
        let last_scope = self.scopes.last_mut().unwrap();
        last_scope.inline_variables.insert(symbol, value);
    }

    fn get_inline_var(&self, symbol: &String) -> Result<i16, ()> {
        for scope in self.scopes.iter().rev() {
            let entry = scope.inline_variables.get(symbol);
            if let Some(v) = entry {
                return Ok(*v);
            }
        }
        Err(())
    }

    fn insert_var(&mut self, symbol: &String) -> Res<u8> {
        let last_scope = self.scopes.last_mut().unwrap();
        let slot = match last_scope.variables.len().try_into() {
            Ok(v) => v,
            Err(_) => return Err(CompilerError::TooManyVars),
        };
        last_scope.variables.insert(symbol.clone(), slot);
        Ok(slot)
    }

    fn get_var(&self, symbol: &String) -> Res<u8> {
        for scope in self.scopes.iter().rev() {
            let entry = scope.variables.get(symbol);
            if let Some(v) = entry {
                return Ok(*v);
            }
        }
        Err(CompilerError::NonexistentVar(symbol.clone()))
    }

    fn insert_temp_var(&mut self) -> Res<(u8)> {
        let last_scope = self.scopes.last_mut().unwrap();
        let slot = match last_scope.variables.len().try_into() {
            Ok(v) => v,
            Err(_) => return Err(CompilerError::TooManyVars),
        };
        last_scope.variables.insert(format!(" {}", slot), slot);
        Ok(slot)
    }

    fn cleanup_temp_var(&mut self, index: u8) {
        let last_scope = self.scopes.last_mut().unwrap();
        last_scope
            .variables
            .remove(format!(" {}", index);
    }

    fn generate_assembly(mut self, body: Vec<Code>) -> Vec<Instruction> {
        body.into_iter().for_each(|line| match line {
            Code::Expr(expr) => match expr {
                Expression::Assignment { symbol, value } => {} // handle_assignment
                _ => unimplemented!(),
            },
            Code::Stmt(stmt) => match stmt {
                Statement::InlineDeclaration { symbol, value } => {
                    let value = self.eval_after_inline(value);
                    self.insert_inline_var(symbol, value)
                }
                _ => unimplemented!(),
            },
        });

        self.instructions
    }

    fn eval_after_inline(&mut self, expr: Expression) -> i16 {
        match expr {
            Expression::Identifier(name) => self.get_inline_var(&name).unwrap(),
            Expression::BinaryExpr {
                left,
                right,
                operator,
            } => {
                let left = self.eval_after_inline(*left);
                let right = self.eval_after_inline(*right);
                match operator {
                    Operator::Plus => left + right,
                    Operator::Minus => left - right,
                    Operator::Mult => left * right,
                    Operator::And => left & right,
                    Operator::Or => left | right,
                    Operator::Xor => left ^ right,
                }
            }
            Expression::NumericLiteral(value) => value,
            _ => {
                panic!()
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expression) -> Res {
        match expr {
            Expression::NumericLiteral(number) => todo!(),
            Expression::Identifier(ident) => todo!(),
            Expression::BinaryExpr {
                left,
                right,
                operator,
            } => self.eval_binary_expr(&left, &right, operator)?,
            _ => {}
        }
        Ok(())
    }

    fn is_simple(expr: &Expression) -> bool {
        use Expression::*;
        match expr {
            Identifier(..) | NumericLiteral(..) => true,
            BinaryExpr {
                left,
                right,
                operator,
            } => Compiler::is_simple(left) && Compiler::is_simple(right),
            _ => false,
        }
    }

    fn eval_binary_expr(
        &mut self,
        left: &Expression,
        right: &Expression,
        operator: &Operator,
    ) -> Res {
        use Expression::*;
        match (left, right) {
            (Identifier(..) | NumericLiteral(..), Identifier(..) | NumericLiteral(..)) => {
                self.eval_simple_expr(left, right, operator)?;
            }
            (Identifier(..) | NumericLiteral(..), _) | (_, Identifier(..) | NumericLiteral(..)) => {
            }
            _ => {}
        }
        Ok(())
    }

    fn eval_assignment(&mut self, symbol: &String, value: &Expression) -> Res {
        let slot = self.insert_var(symbol)?;

        self.eval_expr(value)?;

        instr!(self, SVA, slot);

        Ok(())
    }

    fn eval_simple_expr(
        &mut self,
        left: &Expression,
        right: &Expression,
        operator: &Operator,
    ) -> Res {
        // load a
        use Expression::*;
        match left {
            NumericLiteral(value) => {
                let bytes = value.to_le_bytes();
                instr!(self, LAL, bytes[0]);
                if bytes[1] != 0 {
                    instr!(self, LAH, bytes[1]);
                }
            }
            Identifier(symbol) => {
                instr!(self, LA, self.get_var(symbol)?);
            }
            _ => {}
        }

        match right {
            NumericLiteral(value) => {
                let bytes = value.to_le_bytes();
                instr!(self, LBL, bytes[0]);
                if bytes[1] != 0 {
                    instr!(self, LBH, bytes[1]);
                }
            }
            Identifier(symbol) => {
                instr!(self, LB, self.get_var(symbol)?);
            }
            _ => {}
        }

        use Operator::*;
        match operator {
            Plus => instr!(self, ADD),
            Minus => instr!(self, SUB),
            Mult => instr!(self, MUL),
            And => instr!(self, AND),
            Or => instr!(self, OR),
            Xor => instr!(self, XOR),
        }

        Ok(())
    }

    fn parse_expression(&mut self, value: Expression) {}
}
