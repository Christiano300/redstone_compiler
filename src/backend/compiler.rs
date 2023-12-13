use core::panic;
use std::{collections::HashMap, i32};

use crate::frontend::{Code, Expression, Operator, Parser, Statement};

use super::Instruction;

pub fn compile_program(ast: Code) -> Vec<Instruction> {
    match ast {
        Code::Expr(..) => panic!(),
        Code::Stmt(stmt) => match stmt {
            Statement::Program { body } => {
                let mut compiler = Compiler::new();
                return compiler.generate_assembly(body);
            }
            _ => panic!(),
        },
    }
}

pub fn compile_src(source_code: String) -> Vec<Instruction> {
    let mut parser = Parser::new();
    compile_program(parser.produce_ast(source_code))
}

enum RegisterContents {
    Variable(u8),
    Result(i16),
    RamAddress(i32),
    Unknown,
}

impl Default for RegisterContents {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Default)]
struct ComputerState {
    reg_a: RegisterContents,
    reg_b: RegisterContents,
    reg_c: u8,
}

struct Scope {
    start_state: ComputerState,
    variables: HashMap<String, u8>,
    inline_variables: HashMap<String, i16>,
}

#[derive(Default)]
pub struct Compiler {
    variables: HashMap<String, u8>,
    inline_variables: HashMap<String, i16>,
    instructions: Vec<Instruction>,
    reg_a: RegisterContents,
    reg_b: RegisterContents,
}

impl Compiler {
    fn new() -> Compiler {
        Default::default()
    }

    fn generate_assembly(mut self, body: Vec<Code>) -> Vec<Instruction> {
        body.into_iter().for_each(|line| match line {
            Code::Expr(expr) => match expr {
                Expression::Assignment { symbol, value } => {
                    self.handle_assignment(symbol, *value);
                }
                _ => unimplemented!(),
            },
            Code::Stmt(stmt) => match stmt {
                Statement::InlineDeclaration { symbol, value } => {
                    let value = self.eval_after_inline(value);
                    self.inline_variables.insert(symbol, value);
                }
                _ => unimplemented!(),
            },
        });

        self.instructions
    }

    fn eval_after_inline(&mut self, expr: Expression) -> i16 {
        match expr {
            Expression::Identifier(name) => {
                if self.inline_variables.contains_key(&name) {
                    *self.inline_variables.get(&name).unwrap()
                } else {
                    panic!()
                }
            }
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
            Expression::Member { .. } | Expression::Call { .. } | Expression::Assignment { .. } => {
                panic!()
            }
        }
    }

    fn get_variable_slot(&mut self, name: String) -> u8 {
        if let Some(slot) = self.variables.get(&name) {
            return *slot;
        }
        let slot = (self.variables.len() + 1) as u8;
        self.variables.insert(name, slot);
        slot
    }

    fn handle_assignment(&mut self, symbol: String, value: Expression) {
        let slot = self.get_variable_slot(symbol);

        self.parse_expression(value);
    }

    fn parse_expression(&mut self, value: Expression) {}
}
