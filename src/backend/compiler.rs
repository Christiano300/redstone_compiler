use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Debug,
};

use vec1::{vec1, Vec1};

use crate::{
    backend::{module::Call, ComputerState, Instr, RegisterContents, Scope},
    frontend::{EqualityOperator, Expression, ExpressionType, Operator, Range},
};

use super::{
    module::{call, exist, init},
    Error, ErrorType, Instruction, InstructionVariant,
};

const VAR_SLOTS: usize = 32;

type Res<T = ()> = Result<T, Error>;

#[macro_export]
macro_rules! instr {
    ($self:ident, $variant:ident, $arg:expr) => {
        $self.push_instr($crate::backend::Instruction::new(
            $crate::backend::InstructionVariant::$variant,
            Some($arg),
        ))
    };
    ($self:ident, $variant:ident) => {
        $self.push_instr($crate::backend::Instruction::new(
            $crate::backend::InstructionVariant::$variant,
            None,
        ))
    };
}

/// compile that boi
///
/// # Panics
///
/// Panics if not a Program
///
/// # Errors
///
/// on any compiler error
///
/// # Examples
///
/// ```
/// use redstone_compiler::{frontend::{Expression, ExpressionType, Range, Location}, backend::{compile_program, Instruction, InstructionVariant}};
/// let ast = vec![Expression { typ: ExpressionType::NumericLiteral(5), location: Range(Location(0, 0), Location(0, 0)) }];
///
/// let compiled = compile_program(ast);
///
/// assert_eq!(
///     compiled,
///     Ok(vec![Instruction::new(InstructionVariant::LAL, Some(5))])
/// );
/// ```
pub fn compile_program(ast: Vec<Expression>) -> Res<Vec<Instruction>> {
    let compiler = Compiler::new();
    compiler.generate_assembly(ast)
}

#[derive(Debug)]
pub struct Compiler {
    scopes: Vec1<Scope>,
    main_scope: Vec<Instr>,
    modules: HashSet<String>,
    jump_marks: HashMap<u8, u8>,
    pub variables: [bool; VAR_SLOTS],
    pub module_state: HashMap<&'static str, Box<dyn Any>>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            scopes: vec1!(Scope::default()),
            modules: HashSet::new(),
            main_scope: vec![],
            jump_marks: HashMap::new(),
            variables: [false; VAR_SLOTS],
            module_state: HashMap::new(),
        }
    }

    fn scope_len(scope: &Vec<Instr>) -> u8 {
        let mut sum = 0;
        for i in scope {
            sum += match i {
                Instr::Code(_) => 1,
                Instr::Scope(s) => Self::scope_len(s),
            }
        }
        sum
    }

    pub fn get_module_state<'a, V: 'static>(&'a mut self, key: &'static str) -> Option<&'a mut V> {
        let value = self.module_state.get_mut(key)?;

        value.downcast_mut::<V>()
    }

    fn insert_inline_var(&mut self, symbol: String, value: i16) {
        let last_scope = self.last_scope_mut();
        last_scope.inline_variables.insert(symbol, value);
    }

    fn get_inline_var(&self, symbol: &String, location: Range) -> Res<i16> {
        for scope in self.scopes.iter().rev() {
            let entry = scope.inline_variables.get(symbol);
            if let Some(v) = entry {
                return Ok(*v);
            }
        }
        Err(Error {
            typ: ErrorType::NonexistentInlineVar(symbol.clone()),
            location,
        })
    }

    fn get_next_available_slot(&mut self) -> Option<u8> {
        let index = self.variables.iter().position(|slot| !*slot)?;
        self.variables[index] = true;
        Some(index.try_into().unwrap_or(0))
    }

    fn insert_var(&mut self, symbol: &str, location: Range) -> Res<u8> {
        for scope in self.scopes.iter().rev() {
            let entry = scope.variables.get(symbol);
            if let Some(v) = entry {
                return Ok(*v);
            }
        }
        let slot = self.get_next_available_slot().ok_or(Error {
            typ: ErrorType::TooManyVars,
            location,
        })?;
        self.last_scope_mut()
            .variables
            .insert(symbol.to_owned(), slot);
        Ok(slot)
    }

    /// get slot of a variable
    ///
    /// # Errors
    ///
    /// on any compiler error
    pub fn get_var(&self, symbol: &String, location: Range) -> Res<u8> {
        self.get_var_noerror(symbol).map_or_else(
            || {
                Err(Error {
                    typ: ErrorType::NonexistentVar(symbol.clone()),
                    location,
                })
            },
            Ok,
        )
    }

    #[must_use]
    pub fn get_var_noerror(&self, symbol: &String) -> Option<u8> {
        for scope in self.scopes.iter().rev() {
            let entry = scope.variables.get(symbol);
            if let Some(v) = entry {
                return Some(*v);
            }
        }
        None
    }

    /// Inserts a temporary variable
    ///
    /// # Errors
    ///
    /// When there are too many variables
    pub fn insert_temp_var(&mut self, location: Range) -> Res<u8> {
        self.get_next_available_slot().ok_or(Error {
            typ: ErrorType::TooManyVars,
            location,
        })
    }

    pub fn cleanup_temp_var(&mut self, index: u8) {
        self.variables[index as usize] = false;
    }

    /// use the "instr" macro
    pub fn push_instr(&mut self, instr: Instruction) {
        let last_scope = self.last_scope_mut();
        instr.execute(&mut last_scope.state);
        last_scope.instructions.push(Instr::Code(instr));
    }

    fn get_instructions(mut self) -> Vec<Instruction> {
        self.main_scope
            .push(Instr::Scope(self.scopes.split_off_first().0.instructions));
        let mut instructions = vec![];
        Self::flatten_scope(self.main_scope, &mut instructions);
        Self::insert_disc_jumps(&mut instructions, &mut self.jump_marks);
        Self::replace_jump_marks(&mut instructions, &self.jump_marks);
        instructions
    }

    fn flatten_scope(scope: Vec<Instr>, into: &mut Vec<Instruction>) {
        scope.into_iter().for_each(|i| match i {
            Instr::Code(instr) => into.push(instr),
            Instr::Scope(s) => Self::flatten_scope(s, into),
        });
    }

    #[must_use]
    pub fn last_scope(&self) -> &Scope {
        self.scopes.last()
    }

    pub fn last_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut()
    }

    fn is_root_scope(&self) -> bool {
        self.scopes.len() == 1
    }

    fn generate_assembly(mut self, body: Vec<Expression>) -> Res<Vec<Instruction>> {
        body.into_iter()
            .try_for_each(|line| self.eval_statement(line))?;

        Ok(self.get_instructions())
    }

    fn insert_jump_mark(&mut self) -> u8 {
        let id = self.jump_marks.len() as u8;
        self.jump_marks.insert(id, 0);
        id
    }

    fn eval_statement(&mut self, line: Expression) -> Res {
        match line.typ {
            ExpressionType::InlineDeclaration { symbol, value } => {
                let value = self.eval_after_inline(&value)?;
                self.insert_inline_var(symbol, value);
                Ok(())
            }
            ExpressionType::Use(module) => {
                if !self.is_root_scope() {
                    return Err(Error {
                        typ: ErrorType::UseOutsideGlobalScope,
                        location: line.location,
                    });
                }
                if !exist(&module) {
                    return Err(Error {
                        typ: ErrorType::UnknownModule(module),
                        location: line.location,
                    });
                }
                init(&module, self, line.location)?;
                self.modules.insert(module);

                Ok(())
            }
            ExpressionType::VarDeclaration { symbol } => {
                self.insert_var(symbol.as_str(), line.location)?;
                Ok(())
            }
            ExpressionType::Pass => Ok(()),
            ExpressionType::EndlessLoop { body } => {
                let mark = Self::scope_len(&self.scopes.first().instructions);
                let id = self.insert_jump_mark();
                self.jump_marks.insert(id, mark);

                self.push_scope(body, ComputerState::default())?;
                self.pop_scope();

                instr!(self, JMP, id);

                Ok(())
            }
            ExpressionType::WhileLoop { condition, body } => {
                let (left, right, operator) = eval_condition(*condition)?;

                let start_id = self.insert_jump_mark();
                let end_id = self.insert_jump_mark();

                self.put_comparison((&left, &right, operator.opposite()), end_id)?;

                let start = Self::scope_len(&self.scopes.first().instructions);

                self.jump_marks.insert(start_id, start);

                self.push_scope(body, self.last_scope().state)?;

                self.put_comparison((&left, &right, operator), start_id)?;

                self.pop_scope();
                let end = Self::scope_len(&self.scopes.first().instructions);

                self.jump_marks.insert(end_id, end);

                Ok(())
            }
            ExpressionType::Conditional {
                condition,
                body,
                paths,
                alternate,
            } => self.eval_conditional(*condition, body, paths, alternate)?,
            _ => self.eval_expr(&line),
        }?;
        Ok(())
    }

    fn eval_conditional(
        &mut self,
        condition: Expression,
        body: Vec<Expression>,
        paths: Vec<(Expression, Vec<Expression>)>,
        alternate: Option<Vec<Expression>>,
    ) -> Result<Result<(), Error>, Error> {
        let (left, right, operator) = eval_condition(condition)?;
        let end_id = self.insert_jump_mark();
        let mut next_mark_id = self.insert_jump_mark();

        self.put_comparison((&left, &right, operator.opposite()), next_mark_id)?;

        let mut last_state = self.last_scope().state;

        self.push_scope(body, last_state)?;
        if !paths.is_empty() || alternate.is_some() {
            instr!(self, JMP, end_id);
        }
        self.pop_scope();
        self.jump_marks.insert(
            next_mark_id,
            Self::scope_len(&self.scopes.first().instructions),
        );
        let path_len = paths.len();
        paths.into_iter().enumerate().try_for_each(|path| {
            let (index, (condition, body)) = path;
            let (left, right, operator) = eval_condition(condition)?;

            next_mark_id = self.insert_jump_mark();

            self.put_comparison((&left, &right, operator.opposite()), next_mark_id)?;

            last_state = self.last_scope().state;

            self.push_scope(body, last_state)?;

            if index != path_len - 1 || alternate.is_some() {
                instr!(self, JMP, end_id);
            }

            self.pop_scope();
            self.jump_marks.insert(
                next_mark_id,
                Self::scope_len(&self.scopes.first().instructions),
            );

            Ok(())
        })?;
        if let Some(body) = alternate {
            self.push_scope(body, last_state)?;
            self.pop_scope();
        }
        self.jump_marks
            .insert(end_id, Self::scope_len(&self.scopes.first().instructions));
        Ok(Ok(()))
    }

    fn pop_scope(&mut self) {
        let scope = self.scopes.pop().unwrap();
        self.last_scope_mut()
            .instructions
            .push(Instr::Scope(scope.instructions));
        for i in scope.variables {
            let (_, slot) = i;
            self.variables[slot as usize] = false;
        }
    }

    fn push_scope(&mut self, body: Vec<Expression>, state: ComputerState) -> Res {
        self.scopes.push(Scope::with_state(state));
        body.into_iter()
            .try_for_each(|line| self.eval_statement(line))?;
        Ok(())
    }

    fn put_comparison(
        &mut self,
        condition: (&Expression, &Expression, EqualityOperator),
        jump_to: u8,
    ) -> Res {
        let (left, right, operator) = condition;
        let op = if self.put_ab(left, right, true)? {
            operator.turnaround()
        } else {
            operator
        };
        self.push_instr(Instruction::new(
            InstructionVariant::from_op(op),
            Some(jump_to),
        ));
        Ok(())
    }

    fn eval_after_inline(&mut self, expr: &Expression) -> Res<i16> {
        match &expr.typ {
            ExpressionType::Identifier(name) => self.get_inline_var(name, expr.location),
            ExpressionType::BinaryExpr {
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
            ExpressionType::NumericLiteral(value) => Ok(*value),
            _ => Err(Error {
                typ: ErrorType::ForbiddenInline,
                location: expr.location,
            }),
        }
    }

    /// evaluate expression and put result into a register
    /// # Errors
    ///
    /// on any compiler error
    pub fn eval_expr(&mut self, expr: &Expression) -> Res {
        match &expr.typ {
            ExpressionType::NumericLiteral(..) => self.put_into_a(expr)?,
            ExpressionType::Identifier(..) => self.put_into_a(expr)?,
            ExpressionType::BinaryExpr {
                left,
                right,
                operator,
            } => self.eval_binary_expr(left, right, *operator)?,
            ExpressionType::Assignment { symbol, value } => {
                self.eval_assignment(symbol, value)?;
            }
            ExpressionType::Call { args, function } => self.eval_call(function, args)?,
            ExpressionType::EqExpr { .. } => {
                return Err(Error {
                    typ: ErrorType::EqInNormalExpr,
                    location: expr.location,
                })
            }
            ExpressionType::Debug => instr!(self, LAL, 17),

            _ => todo!("unsupported expression: {:?}", expr),
        }
        Ok(())
    }

    #[must_use]
    pub const fn can_put_into_a(expr: &Expression) -> bool {
        use ExpressionType as E;
        match &expr.typ {
            E::NumericLiteral(..) | E::Identifier(..) => true,
            E::Assignment { symbol: _, value } => Self::can_put_into_a(value),
            _ => false,
        }
    }

    #[must_use]
    pub const fn can_put_into_b(expr: &Expression) -> bool {
        use ExpressionType as E;
        matches!(expr.typ, E::NumericLiteral(..) | E::Identifier(..))
    }

    fn eval_binary_expr(
        &mut self,
        left: &Expression,
        right: &Expression,
        operator: Operator,
    ) -> Res {
        self.put_ab(left, right, operator.is_commutative())?;

        self.put_op(operator);
        Ok(())
    }

    /// # Returns
    /// if the arguments were swapped
    fn put_ab(&mut self, left: &Expression, right: &Expression, is_commutative: bool) -> Res<bool> {
        let mut swapped = false;
        match (Self::can_put_into_a(left), Self::can_put_into_b(right)) {
            (true, true) => {
                if is_commutative
                    && ((self.is_in_a(right) || self.is_in_b(left))
                        || (matches!(right.typ, ExpressionType::Identifier(..))
                            && matches!(left.typ, ExpressionType::NumericLiteral(..))))
                {
                    self.put_into_a(right)?;
                    self.put_into_b(left)?;
                    return Ok(true);
                }
                self.put_into_a(left)?;
                self.put_into_b(right)?;
            }
            (true, false) => {
                self.eval_expr(right)?;
                if is_commutative && Self::can_put_into_b(left) {
                    self.put_into_b(left)?;
                    swapped = true;
                } else {
                    // if we just saved a variable we use it to switch
                    if let ExpressionType::Assignment { symbol, value: _ } = &right.typ {
                        instr!(self, LB, self.get_var(symbol, Range::default()).unwrap());
                    } else {
                        self.switch(left.location)?;
                    }
                    self.put_into_a(left)?;
                }
            }
            (false, true) => {
                self.eval_expr(left)?;
                self.put_into_b(right)?;
            }
            (false, false) => {
                self.eval_expr(right)?;
                if let ExpressionType::Assignment { symbol, value: _ } = &right.typ {
                    self.eval_expr(left)?;
                    instr!(self, LB, self.get_var(symbol, Range::default()).unwrap());
                } else {
                    let temp = self.insert_temp_var(left.location)?;
                    instr!(self, SVA, temp);
                    self.eval_expr(left)?;
                    instr!(self, LB, temp);
                    self.cleanup_temp_var(temp);
                }
            }
        }
        Ok(swapped)
    }

    fn eval_assignment(&mut self, symbol: &str, value: &Expression) -> Res {
        let slot = self.insert_var(symbol, value.location)?;

        self.eval_expr(value)?;

        instr!(self, SVA, slot);

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
    ///
    /// # Errors
    ///
    /// on any compiler error
    pub fn try_get_constant(&mut self, value: &Expression) -> Res<Option<i16>> {
        Ok(match &value.typ {
            ExpressionType::NumericLiteral(value) => Some(*value),
            ExpressionType::Identifier(symbol) => self.get_inline_var(symbol, value.location).ok(),
            ExpressionType::BinaryExpr { .. } => match self.eval_after_inline(value) {
                Ok(value) => Some(value),
                Err(err) => match &err.typ {
                    ErrorType::NonexistentInlineVar(..) | ErrorType::ForbiddenInline => None,
                    _ => return Err(err),
                },
            },
            _ => None,
        })
    }

    /// puts a into b
    ///
    /// # Errors
    ///
    /// if there are too many variables
    pub fn switch(&mut self, location: Range) -> Res {
        let temp = self.insert_temp_var(location)?;
        instr!(self, SVA, temp);
        instr!(self, LB, temp);
        self.cleanup_temp_var(temp);
        Ok(())
    }

    /// expr should be either `NumericLiteral`, `Identifier` or `Assignment`
    ///
    /// # Errors
    ///
    /// if variable doesn't exist or called on a wrong expression
    pub fn put_into_a(&mut self, expr: &Expression) -> Res {
        use ExpressionType as E;
        match &expr.typ {
            E::NumericLiteral(value) => {
                self.put_a_number(*value);
            }
            E::Identifier(symbol) => {
                if let Ok(value) = self.get_inline_var(symbol, expr.location) {
                    self.put_a_number(value);
                } else {
                    let var = self.get_var(symbol, expr.location)?;
                    if let RegisterContents::Variable(v) = self.last_scope().state.a {
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
                    return Err(Error {
                        typ: ErrorType::SomethingElseWentWrong("put_a".to_string()),
                        location: expr.location,
                    });
                }
            }
            _ => {
                return Err(Error {
                    typ: ErrorType::SomethingElseWentWrong(
                        "put_a called on wrong expression".to_string(),
                    ),
                    location: expr.location,
                })
            }
        }
        Ok(())
    }

    /// expr should be either `NumericLiteral` or `Identifier`
    ///
    /// # Errors
    ///
    /// if variable doesn't exist or called on a wrong expression
    pub fn put_into_b(&mut self, expr: &Expression) -> Res {
        use ExpressionType as E;
        match &expr.typ {
            E::NumericLiteral(value) => {
                self.put_b_number(*value);
            }
            E::Identifier(symbol) => {
                if let Ok(value) = self.get_inline_var(symbol, expr.location) {
                    self.put_b_number(value);
                } else {
                    let var = self.get_var(symbol, expr.location)?;
                    if let RegisterContents::Variable(v) = self.last_scope().state.b {
                        if v == var {
                            return Ok(());
                        }
                    }
                    instr!(self, LB, var);
                }
            }
            _ => {
                return Err(Error {
                    typ: ErrorType::SomethingElseWentWrong(
                        "put_b called on wrong expression".to_string(),
                    ),
                    location: expr.location,
                })
            }
        }
        Ok(())
    }

    fn is_in_a(&mut self, expr: &Expression) -> bool {
        use ExpressionType as E;
        match &expr.typ {
            E::NumericLiteral(value) => {
                self.last_scope().state.a == RegisterContents::Number(*value)
            }
            E::Identifier(symbol) => {
                RegisterContents::Variable(match self.get_var_noerror(symbol) {
                    Some(v) => v,
                    None => return false,
                }) == self.last_scope().state.a
            }
            _ => false,
        }
    }

    fn is_in_b(&mut self, expr: &Expression) -> bool {
        use ExpressionType as E;
        match &expr.typ {
            E::NumericLiteral(value) => {
                self.last_scope().state.b == RegisterContents::Number(*value)
            }
            E::Identifier(symbol) => {
                RegisterContents::Variable(match self.get_var_noerror(symbol) {
                    Some(v) => v,
                    None => return false,
                }) == self.last_scope().state.b
            }
            _ => false,
        }
    }

    pub fn put_a_number(&mut self, value: i16) {
        if self.last_scope().state.a == RegisterContents::Number(value) {
            return;
        }
        let bytes = value.to_le_bytes();
        instr!(self, LAL, bytes[0]);
        if bytes[1] != 0 {
            instr!(self, LAH, bytes[1]);
        }
    }

    pub fn put_b_number(&mut self, value: i16) {
        if self.last_scope().state.b == RegisterContents::Number(value) {
            return;
        }
        let bytes = value.to_le_bytes();
        instr!(self, LBL, bytes[0]);
        if bytes[1] != 0 {
            instr!(self, LBH, bytes[1]);
        }
    }

    fn eval_call(&mut self, function: &Expression, args: &Vec<Expression>) -> Res {
        use ExpressionType as E;
        let module;
        let method;
        match &function.typ {
            E::Member { object, property } => match &object.typ {
                E::Identifier(symbol) => {
                    module = symbol;
                    method = property;
                }
                _ => {
                    return Err(Error {
                        typ: ErrorType::UnknownModule(format!("{object:?}")),
                        location: function.location,
                    })
                }
            },
            _ => {
                return Err(Error {
                    typ: ErrorType::UnknownMethod(format!("{function:?}")),
                    location: function.location,
                })
            }
        }
        if !self.modules.contains(module) {
            return Err(Error {
                typ: ErrorType::UnknownModule(module.clone()),
                location: function.location,
            });
        }

        call(
            module,
            self,
            &Call {
                method_name: method,
                args,
                location: function.location,
            },
        )
    }

    fn replace_jump_marks(instructions: &mut [Instruction], jump_marks: &HashMap<u8, u8>) {
        for i in instructions.iter_mut() {
            if i.variant.is_jump() {
                i.arg = Some(
                    *jump_marks
                        .get(&i.arg.expect("jump does not have arg"))
                        .expect("Invalid jump mark"),
                );
            }
        }
    }

    fn move_jump_marks(jump_marks: &mut HashMap<u8, u8>, from: u8, by: u8) {
        for (_, value) in jump_marks.iter_mut() {
            if *value >= from {
                *value += by;
            }
        }
    }

    fn insert_disc_jumps(instructions: &mut Vec<Instruction>, jump_marks: &mut HashMap<u8, u8>) {
        loop {
            let mut changes = false;

            let mut i = 0;
            while i < instructions.len() {
                let instr = instructions
                    .get_mut(i)
                    .expect("Tried getting invalid instruction in insert_disc_jumps loop");
                if instr.variant.is_jump() && !instr.variant.disc_jump() {
                    let mark = instr.arg.expect("Jump instruction doesn't have arg");
                    let current_page = i / 64;
                    let jump_page = jump_marks.get(&mark).expect("Invalid jump mark") / 64;
                    if current_page != jump_page as usize {
                        instr.variant = instr.variant.to_disc_jump();
                        instructions.insert(
                            i,
                            Instruction::new(InstructionVariant::LCL, Some(jump_page)),
                        );
                        Self::move_jump_marks(jump_marks, i as u8, 1);
                        i += 1;
                        changes = true;
                    }
                }
                i += 1;
            }

            if !changes {
                break;
            }
        }
    }
}

fn eval_condition(
    condition: Expression,
) -> Res<(Box<Expression>, Box<Expression>, EqualityOperator)> {
    let ExpressionType::EqExpr {
        left,
        right,
        operator,
    } = condition.typ
    else {
        return Err(Error {
            typ: ErrorType::NormalInEqExpr,
            location: condition.location,
        });
    };
    Ok((left, right, operator))
}
