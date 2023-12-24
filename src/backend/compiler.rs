use std::{collections::HashMap, i32};

use crate::frontend::{Code, Expression, Operator, Parser, Statement};

use super::Instruction;

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

#[derive(Default)]
enum RegisterContents {
    Variable(u8),
    Result(i16),
    RamAddress(i32),
    #[default]
    Unknown,
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

    fn insert_inline(&mut self, symbol: String, value: i16) {
        let last_scope = self.scopes.last_mut().unwrap();
        last_scope.inline_variables.insert(symbol, value);
    }

    fn get_inline(&mut self, symbol: &String) -> Result<i16, ()> {
        for scope in self.scopes.iter_mut().rev() {
            let entry = scope.inline_variables.get(symbol);
            if let Some(v) = entry {
                return Ok(*v);
            }
        }
        Err(())
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
                    self.insert_inline(symbol, value)
                }
                _ => unimplemented!(),
            },
        });

        self.instructions
    }

    fn eval_after_inline(&mut self, expr: Expression) -> i16 {
        match expr {
            Expression::Identifier(name) => self.get_inline(&name).unwrap(),
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

    //fn get_variable_slot(&mut self, name: String) -> u8 {
    //  if let Some(slot) = self.variables.get(&name) {
    //    return *slot;
    //    }
    //    let slot = (self.variables.len() + 1) as u8;
    //    self.variables.insert(name, slot);
    //    slot
    //}

    //fn handle_assignment(&mut self, symbol: String, value: Expression) {
    //    let slot = self.get_variable_slot(symbol);

    //    self.parse_expression(value);
    //}

    fn parse_expression(&mut self, value: Expression) {}
}
