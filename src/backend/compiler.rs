use std::collections::{HashMap, HashSet};

use vec1::{vec1, Vec1};

use crate::frontend::{Expression, Operator};

use super::{module::MODULES, Instruction};

#[derive(Debug)]
pub enum Error {
    NonexistentVar(String),
    NonexistentInlineVar(String),
    TooManyVars,
    ForbiddenInline,
    UnknownModule(String),
    UnknownMethod(String),
    InvalidArgs(String),
    SomethingElseWentWrong(String),
}

type Res<T = ()> = Result<T, Error>;

#[macro_export]
macro_rules! instr {
    ($self:ident, $variant:ident, $arg:expr) => {
        $self.push_instr($crate::backend::Instruction::new(
            &$crate::backend::InstructionVariant::$variant,
            Some($arg),
        ))
    };
    ($self:ident, $variant:ident) => {
        $self.push_instr($crate::backend::Instruction::new(
            &$crate::backend::InstructionVariant::$variant,
            None,
        ))
    };
}

pub fn compile_program(ast: Expression) -> Res<Vec<Instruction>> {
    if let Expression::Program(body) = ast {
        let compiler = Compiler::new();
        compiler.generate_assembly(body)
    } else {
        panic!()
    }
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
#[allow(unused)]
pub enum RegisterContents {
    Variable(u8),
    Number(i16),
    RamAddress(i32),
    #[default]
    Unknown,
}

#[derive(Default, Copy, Clone, Debug)]
pub struct ComputerState {
    pub a: RegisterContents,
    pub b: RegisterContents,
    pub c: RegisterContents,
}

#[derive(Debug)]
pub enum Instr {
    Code(Instruction),
    Scope(Vec<Instr>),
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Compiler {
    scopes: Vec1<Scope>,
    main_scope: Vec<Instr>,
    modules: HashSet<String>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            scopes: vec1!(Scope {
                start_state: ComputerState::default(),
                variables: HashMap::new(),
                inline_variables: HashMap::new(),
                instructions: vec![],
            }),
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
        Err(Error::NonexistentInlineVar(format!(
            "Inline {:?}, {:#?}",
            symbol, self.scopes
        )))
    }

    fn insert_var(&mut self, symbol: &str) -> Res<u8> {
        let last_scope = self.last_scope();
        if let Some(slot) = last_scope.variables.get(symbol) {
            return Ok(*slot);
        }
        let Ok(slot) = last_scope.variables.len().try_into() else {
            return Err(Error::TooManyVars);
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
        Err(Error::NonexistentVar(format!(
            "{:?}, {:#?}",
            symbol, self.scopes
        )))
    }

    pub fn insert_temp_var(&mut self) -> Res<u8> {
        let last_scope = self.last_scope();
        let Ok(slot) = last_scope.variables.len().try_into() else {
            return Err(Error::TooManyVars);
        };
        last_scope.variables.insert(format!(" {slot}"), slot);
        Ok(slot)
    }

    pub fn cleanup_temp_var(&mut self, index: u8) {
        let last_scope = self.last_scope();
        last_scope.variables.remove(&format!(" {index}"));
    }

    /// use the "instr" macro
    pub fn push_instr(&mut self, instr: Instruction) {
        let last_scope = self.last_scope();
        instr.execute(&mut last_scope.start_state);
        last_scope.instructions.push(Instr::Code(instr));
    }

    fn get_instructions(mut self) -> Vec<Instruction> {
        self.main_scope
            .push(Instr::Scope(self.scopes.split_off_first().0.instructions));
        let mut instructions = vec![];
        Self::resolve_scope(self.main_scope, &mut instructions);
        instructions
    }

    fn resolve_scope(scope: Vec<Instr>, into: &mut Vec<Instruction>) {
        scope.into_iter().for_each(|i| match i {
            Instr::Code(instr) => into.push(instr),
            Instr::Scope(s) => Self::resolve_scope(s, into),
        });
    }

    fn last_scope(&mut self) -> &mut Scope {
        self.scopes.last_mut()
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
                        return Err(Error::UnknownModule(format!(
                            "{module}, that module doesn't exist"
                        )));
                    }
                    self.modules.insert(module);
                    Ok(())
                }
                expr => self.eval_expr(&expr),
            }?;
            Ok::<(), Error>(())
        })?;

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
            _ => Err(Error::ForbiddenInline),
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
            } => self.eval_binary_expr(left, right, *operator)?,
            Expression::Assignment { symbol, value } => self.eval_assignment(symbol, value)?,
            Expression::Call { args, function } => self.eval_call(function, args)?,
            _ => todo!("unsupported expression: {:?}", expr),
        }
        Ok(())
    }

    pub const fn can_put_into_a(expr: &Expression) -> bool {
        use Expression as E;
        match expr {
            E::NumericLiteral(..) | E::Identifier(..) => true,
            E::Assignment { symbol: _, value } => Self::can_put_into_a(value),
            _ => false,
        }
    }

    pub const fn can_put_into_b(expr: &Expression) -> bool {
        use Expression as E;
        matches!(expr, E::NumericLiteral(..) | E::Identifier(..))
    }

    fn eval_binary_expr(
        &mut self,
        left: &Expression,
        right: &Expression,
        operator: Operator,
    ) -> Res {
        match (Self::can_put_into_a(left), Self::can_put_into_b(right)) {
            (true, true) => {
                if Self::can_put_into_a(right) && Self::can_put_into_b(left) {
                    self.eval_simple_expr(right, left, operator)?;
                } else {
                    self.eval_simple_expr(left, right, operator)?;
                }
            }
            (true, false) => {
                self.eval_expr(right)?;
                if operator.is_commutative() && Self::can_put_into_b(left) {
                    self.put_into_b(left)?;
                } else {
                    // if we just saved a variable we use it to switch
                    match right {
                        Expression::Assignment { symbol, value: _ } => {
                            instr!(self, LB, self.get_var(symbol)?);
                        }
                        _ => self.switch()?,
                    }
                    self.put_into_a(left)?;
                }
                self.put_op(operator);
            }
            (false, true) => {
                self.eval_expr(left)?;
                self.put_into_b(right)?;
                self.put_op(operator);
            }
            (false, false) => {
                self.eval_expr(right)?;
                if let Expression::Assignment { symbol, value: _ } = right {
                    self.eval_expr(left)?;
                    instr!(self, LB, self.get_var(symbol)?);
                } else {
                    let temp = self.insert_temp_var()?;
                    instr!(self, SVA, temp);
                    self.eval_expr(left)?;
                    instr!(self, LB, temp);
                    self.cleanup_temp_var(temp);
                }
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
        operator: Operator,
    ) -> Res {
        self.put_into_a(left)?;

        self.put_into_b(right)?;

        self.put_op(operator);

        Ok(())
    }

    fn put_op(&mut self, operator: Operator) {
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
    pub fn try_get_constant(&mut self, value: &Expression) -> Res<Option<i16>> {
        Ok(match value {
            Expression::NumericLiteral(value) => Some(*value),
            Expression::Identifier(symbol) => self.get_inline_var(symbol).ok(),
            Expression::BinaryExpr { .. } => match self.eval_after_inline(value) {
                Ok(value) => Some(value),
                Err(Error::ForbiddenInline | Error::NonexistentInlineVar(..)) => None,
                Err(other) => return Err(other),
            },
            _ => None,
        })
    }

    /// puts a into b
    pub fn switch(&mut self) -> Res {
        let temp = self.insert_temp_var()?;
        instr!(self, SVA, temp);
        instr!(self, LB, temp);
        self.cleanup_temp_var(temp);
        Ok(())
    }

    /// expr should be either `NumericLiteral`, `Identifier` or `Assignment`
    pub fn put_into_a(&mut self, expr: &Expression) -> Res {
        use Expression as E;
        match expr {
            E::NumericLiteral(value) => {
                self.put_a_number(*value);
            }
            E::Identifier(symbol) => {
                if let Ok(value) = self.get_inline_var(symbol) {
                    self.put_a_number(value);
                } else {
                    let var = self.get_var(symbol)?;
                    if let RegisterContents::Variable(v) = self.last_scope().start_state.a {
                        if v == var {
                            return Ok(());
                        }
                    }
                    instr!(self, LA, var);
                }
            }
            E::Assignment { .. } => {
                if Self::can_put_into_a(expr) {
                    self.eval_expr(expr)?;
                } else {
                    return Err(Error::SomethingElseWentWrong(
                        "put_a called on wrong assignment, report to developer".to_string(),
                    ));
                }
            }
            _ => {
                return Err(Error::SomethingElseWentWrong(
                    "put_a called on wrong expression".to_string(),
                ))
            }
        }
        Ok(())
    }

    /// expr should be either `NumericLiteral` or `Identifier`
    pub fn put_into_b(&mut self, expr: &Expression) -> Res {
        use Expression as E;
        match expr {
            E::NumericLiteral(value) => {
                self.put_b_number(*value);
            }
            E::Identifier(symbol) => {
                if let Ok(value) = self.get_inline_var(symbol) {
                    self.put_b_number(value);
                } else {
                    let var = self.get_var(symbol)?;
                    if let RegisterContents::Variable(v) = self.last_scope().start_state.b {
                        if v == var {
                            return Ok(());
                        }
                    }
                    instr!(self, LB, var);
                }
            }
            _ => {
                return Err(Error::SomethingElseWentWrong(
                    "put_b called on wrong expression".to_string(),
                ))
            }
        }
        Ok(())
    }

    pub fn put_a_number(&mut self, value: i16) {
        if self.last_scope().start_state.a == RegisterContents::Number(value) {
            return;
        }
        let bytes = value.to_le_bytes();
        instr!(self, LAL, bytes[0]);
        if bytes[1] != 0 {
            instr!(self, LAH, bytes[1]);
        }
    }

    pub fn put_b_number(&mut self, value: i16) {
        if self.last_scope().start_state.b == RegisterContents::Number(value) {
            return;
        }
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
                    return Err(Error::UnknownModule(format!(
                        "{object:?}, you cant use that as a module"
                    )))
                }
            },
            _ => {
                return Err(Error::UnknownMethod(format!(
                    "{function:?}, you cant use that as a function"
                )))
            }
        }
        if !self.modules.contains(module) {
            return Err(Error::UnknownModule(format!(
                "{}, not loaded in",
                module.clone()
            )));
        }

        // don't ask
        MODULES.with(|modules| {
            (modules
                .borrow_mut()
                .get_mut(module)
                .ok_or_else(|| {
                    Error::UnknownModule(format!("{} not a valid module", module.clone()))
                })?
                .handler)(
                self,
                &ModuleCall {
                    method_name: method,
                    args,
                },
            )?;

            Ok(())
        })
    }
}
