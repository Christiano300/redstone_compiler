use std::collections::{HashMap, HashSet};

use crate::frontend::{Expression, Operator, Parser};

use super::{module::MODULES, Instruction};

#[derive(Debug)]
pub enum CompilerError {
    NonexistentVar(String),
    TooManyVars,
    ForbiddenInline,
    UnknownModule(String),
    UnknownMethod(String),
    InvalidArgs(String),
    SomethingElseWentWrong(String),
}

type Res<T = ()> = Result<T, CompilerError>;

#[macro_export]
macro_rules! instr {
    ($self:ident, $variant:ident, $arg:expr) => {
        $self.push_instr($crate::backend::Instruction {
            variant: &$crate::backend::InstructionVariant::$variant,
            arg: Some($arg),
        })
    };
    ($self:ident, $variant:ident) => {{
        $self.push_instr($crate::backend::Instruction {
            variant: &$crate::backend::InstructionVariant::$variant,
            arg: None,
        })
    }};
}

pub fn compile_program(ast: Expression) -> Res<Vec<Instruction>> {
    match ast {
        Expression::Program(body) => {
            let compiler = Compiler::new();
            compiler.generate_assembly(body)
        }
        _ => panic!(),
    }
}

pub fn compile_src(source_code: String) -> Res<Vec<Instruction>> {
    let mut parser = Parser::new();
    compile_program(parser.produce_ast(source_code).unwrap())
}

#[derive(Default, Copy, Clone, Debug)]
#[allow(unused)]
pub enum RegisterContents {
    Variable(u8),
    Number(i16),
    Result(i16),
    RamAddress(i32),
    #[default]
    Unknown,
}

#[derive(Default, Copy, Clone, Debug)]
pub struct ComputerState {
    pub reg_a: RegisterContents,
    pub reg_b: RegisterContents,
    pub reg_c: u8,
}

pub enum Instr {
    Code(Instruction),
    Scope(Vec<Instr>),
}

struct Scope {
    start_state: ComputerState,
    variables: HashMap<String, u8>,
    inline_variables: HashMap<String, i16>,
    instructions: Vec<Instr>,
}

pub struct ModuleCall<'a> {
    pub method_name: &'a String,
    pub args: &'a Vec<Expression>,
}

pub struct Compiler {
    scopes: Vec<Scope>,
    main_scope: Vec<Instr>,
    modules: HashSet<String>,
}

impl Compiler {
    fn new() -> Compiler {
        Compiler {
            scopes: vec![Scope {
                start_state: Default::default(),
                variables: HashMap::new(),
                inline_variables: HashMap::new(),
                instructions: vec![],
            }],
            modules: HashSet::new(),
            main_scope: vec![],
        }
    }

    fn insert_inline_var(&mut self, symbol: String, value: i16) {
        let last_scope = self.last_scope();
        last_scope.inline_variables.insert(symbol, value);
    }

    fn get_inline_var(&self, symbol: &String) -> Res<i16> {
        for scope in self.scopes.iter().rev() {
            let entry = scope.inline_variables.get(symbol);
            if let Some(v) = entry {
                return Ok(*v);
            }
        }
        Err(CompilerError::NonexistentVar(symbol.clone()))
    }

    fn insert_var(&mut self, symbol: &str) -> Res<u8> {
        let last_scope = self.scopes.last_mut().unwrap();
        if let Some(slot) = last_scope.variables.get(symbol) {
            return Ok(*slot);
        }
        let slot = match last_scope.variables.len().try_into() {
            Ok(v) => v,
            Err(_) => return Err(CompilerError::TooManyVars),
        };
        last_scope.variables.insert(symbol.to_owned(), slot);
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

    fn insert_temp_var(&mut self) -> Res<u8> {
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
        last_scope.variables.remove(&format!(" {}", index));
    }

    /// use the "instr" macro
    pub fn push_instr(&mut self, instr: Instruction) {
        let last_scope = self.scopes.last_mut().unwrap();
        instr.execute(&mut last_scope.start_state);
        last_scope.instructions.push(Instr::Code(instr));
    }

    fn get_instructions(self) -> Vec<Instruction> {
        let mut instructions = vec![];
        Compiler::resolve_scope(self.main_scope, &mut instructions);
        instructions
    }

    fn resolve_scope(scope: Vec<Instr>, into: &mut Vec<Instruction>) {
        scope.into_iter().for_each(|i| match i {
            Instr::Code(instr) => into.push(instr),
            Instr::Scope(s) => Compiler::resolve_scope(s, into),
        })
    }

    fn last_scope(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    fn generate_assembly(mut self, body: Vec<Expression>) -> Res<Vec<Instruction>> {
        body.into_iter().try_for_each(|line| {
            match line {
                Expression::InlineDeclaration { symbol, value } => {
                    let value = self.eval_after_inline(&value)?;
                    self.insert_inline_var(symbol, value);
                    Ok(())
                }
                Expression::Use(module) => {
                    if !MODULES.with(|modules| modules.borrow().contains_key(&module)) {
                        return Err(CompilerError::UnknownModule(format!(
                            "{}, that module doesn't exist",
                            module
                        )));
                    }
                    self.modules.insert(module);
                    Ok(())
                }
                expr => self.eval_expr(&expr),
            }?;
            Ok::<(), CompilerError>(())
        })?;

        self.main_scope
            .push(Instr::Scope(self.scopes.pop().unwrap().instructions));

        Ok(self.get_instructions())
    }

    fn eval_after_inline(&mut self, expr: &Expression) -> Res<i16> {
        match expr {
            Expression::Identifier(name) => self.get_inline_var(name),
            Expression::BinaryExpr {
                left,
                right,
                operator,
            } => {
                let left = self.eval_after_inline(left)?;
                let right = self.eval_after_inline(right)?;
                Ok(match operator {
                    Operator::Plus => left + right,
                    Operator::Minus => left - right,
                    Operator::Mult => left * right,
                    Operator::And => left & right,
                    Operator::Or => left | right,
                    Operator::Xor => left ^ right,
                })
            }
            Expression::NumericLiteral(value) => Ok(*value),
            _ => Err(CompilerError::ForbiddenInline),
        }
    }

    pub fn eval_expr(&mut self, expr: &Expression) -> Res {
        match expr {
            Expression::NumericLiteral(..) => self.put_into_a(expr)?,
            Expression::Identifier(..) => self.put_into_a(expr)?,
            Expression::BinaryExpr {
                left,
                right,
                operator,
            } => self.eval_binary_expr(left, right, operator)?,
            Expression::Assignment { symbol, value } => self.eval_assignment(symbol, value)?,
            Expression::Call { args, function } => self.eval_call(function, args)?,
            _ => todo!("unsupported expression: {:?}", expr),
        }
        Ok(())
    }

    fn eval_binary_expr(
        &mut self,
        left: &Expression,
        right: &Expression,
        operator: &Operator,
    ) -> Res {
        use Expression as E;
        match (left, right) {
            (
                E::Identifier(..) | E::NumericLiteral(..),
                E::Identifier(..) | E::NumericLiteral(..),
            ) => {
                self.eval_simple_expr(left, right, operator)?;
            }
            (E::Identifier(..) | E::NumericLiteral(..), _) => {
                self.eval_expr(right)?;
                if matches!(operator, Operator::Minus) {
                    self.switch()?;
                    self.put_into_a(left)?;
                } else {
                    self.put_into_b(left)?;
                }
                self.put_op(operator);
            }
            (_, E::Identifier(..) | E::NumericLiteral(..)) => {
                self.eval_expr(left)?;
                self.put_into_b(right)?;
                self.put_op(operator);
            }
            _ => {
                self.eval_expr(right)?;
                let temp = self.insert_temp_var()?;
                instr!(self, SVA, temp);
                self.eval_expr(left)?;
                instr!(self, LB, temp);
                self.cleanup_temp_var(temp);
                self.put_op(operator);
            }
        }
        Ok(())
    }

    fn eval_assignment(&mut self, symbol: &str, value: &Expression) -> Res {
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
        self.put_into_a(left)?;

        self.put_into_b(right)?;

        self.put_op(operator);

        Ok(())
    }

    fn put_op(&mut self, operator: &Operator) {
        use Operator as O;
        match operator {
            O::Plus => instr!(self, ADD),
            O::Minus => instr!(self, SUB),
            O::Mult => instr!(self, MUL),
            O::And => instr!(self, AND),
            O::Or => instr!(self, OR),
            O::Xor => instr!(self, XOR),
        }
    }

    /// tries to get the value known at compile time
    pub fn try_get_constant(&self, value: &Expression) -> Option<i16> {
        match value {
            Expression::NumericLiteral(value) => Some(*value),
            Expression::Identifier(symbol) => match self.get_inline_var(symbol) {
                Ok(value) => Some(value),
                Err(_) => None,
            },
            _ => None,
        }
    }

    /// puts a into b
    pub fn switch(&mut self) -> Res {
        let temp = self.insert_temp_var()?;
        instr!(self, SVA, temp);
        self.cleanup_temp_var(temp);
        Ok(())
    }

    /// expr should be either NumericLiteral or Identifier
    pub fn put_into_a(&mut self, expr: &Expression) -> Res {
        use Expression as E;
        match expr {
            E::NumericLiteral(value) => {
                self.put_a_number(*value);
            }
            E::Identifier(symbol) => match self.get_inline_var(symbol) {
                Ok(value) => self.put_a_number(value),
                Err(_) => {
                    let var = self.get_var(symbol)?;
                    if let RegisterContents::Variable(v) = self.last_scope().start_state.reg_a {
                        if v == var {
                            return Ok(());
                        }
                    }
                    instr!(self, LA, var)
                }
            },
            _ => {
                return Err(CompilerError::SomethingElseWentWrong(
                    "put_a called on wrong expression".to_string(),
                ))
            }
        }
        Ok(())
    }

    /// expr should be either NumericLiteral or Identifier
    pub fn put_into_b(&mut self, expr: &Expression) -> Res {
        use Expression as E;
        match expr {
            E::NumericLiteral(value) => {
                self.put_b_number(*value);
            }
            E::Identifier(symbol) => match self.get_inline_var(symbol) {
                Ok(value) => self.put_b_number(value),
                Err(_) => {
                    let var = self.get_var(symbol)?;
                    if let RegisterContents::Variable(v) = self.last_scope().start_state.reg_b {
                        if v == var {
                            return Ok(());
                        }
                    }
                    instr!(self, LB, var)
                }
            },
            _ => {
                return Err(CompilerError::SomethingElseWentWrong(
                    "put_a called on wrong expression".to_string(),
                ))
            }
        }
        Ok(())
    }

    fn put_a_number(&mut self, value: i16) {
        let bytes = value.to_le_bytes();
        instr!(self, LAL, bytes[0]);
        if bytes[1] != 0 {
            instr!(self, LAH, bytes[1]);
        }
    }

    fn put_b_number(&mut self, value: i16) {
        let bytes = value.to_le_bytes();
        instr!(self, LBL, bytes[0]);
        if bytes[1] != 0 {
            instr!(self, LBH, bytes[1]);
        }
    }

    fn eval_call(&mut self, function: &Expression, args: &Vec<Expression>) -> Res {
        use Expression as E;
        let module;
        let method;
        match function {
            E::Member { object, property } => match object.as_ref() {
                E::Identifier(symbol) => {
                    module = symbol;
                    method = property;
                }
                _ => {
                    return Err(CompilerError::UnknownModule(format!(
                        "{object:?}, you cant use that as a module"
                    )))
                }
            },
            _ => {
                return Err(CompilerError::UnknownMethod(format!(
                    "{function:?}, you cant use that as a function"
                )))
            }
        }
        if !self.modules.contains(module) {
            return Err(CompilerError::UnknownModule(format!(
                "{}, not loaded in",
                module.clone()
            )));
        }

        // don't ask
        MODULES.with(|modules| {
            (modules.borrow_mut().get_mut(module).unwrap().handler)(
                self,
                ModuleCall {
                    method_name: method,
                    args,
                },
            )?;

            Ok(())
        })
    }
}
