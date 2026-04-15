use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Debug,
    mem,
};

use vec1::{Vec1, vec1};

use crate::{
    backend::target::Target,
    error::Error,
    frontend::{
        EqualityOperator, Expr, Expression, Fragment, Ident, Operator, Range, Statement, Stmt,
    },
};

use super::{ComputerState, Instr, RegisterContents, Scope, error::Type as ErrorType};

use super::{
    Instruction, InstructionVariant,
    module::{Call, call, exist, init},
};

use static_assertions::const_assert;

use crate::backend::w4::compiler::OptLevel;

const VAR_SLOTS: usize = 32;

type Res<T = (), E = Error> = Result<T, E>;

macro_rules! instr {
    ($self:ident, $variant:ident, $arg:expr, $loc:expr) => {{
        const_assert!($crate::backend::InstructionVariant::$variant.has_arg(),);
        $self.push_instr($crate::backend::Instruction::new(
            $crate::backend::InstructionVariant::$variant,
            Some($arg),
            $loc,
        ))
    }};
    ($self:ident, $variant:ident, $loc:expr) => {{
        const_assert!(!$crate::backend::InstructionVariant::$variant.has_arg());
        $self.push_instr($crate::backend::Instruction::new(
            $crate::backend::InstructionVariant::$variant,
            None,
            $loc,
        ))
    }};
}

#[derive(Debug)]
pub struct Compiler {
    scopes: Vec1<Scope>,
    modules: HashSet<String>,
    jump_marks: HashMap<u8, u8>,
    pub variables: [bool; VAR_SLOTS],
    pub module_state: HashMap<&'static str, Box<dyn Any>>,
    pub opt_level: OptLevel,
}

impl Compiler {
    #[must_use]
    pub fn new() -> Self {
        Self::with_opt_level(OptLevel::None)
    }

    #[must_use]
    pub fn with_opt_level(opt_level: OptLevel) -> Self {
        Self {
            scopes: vec1!(Scope::default()),
            modules: HashSet::new(),
            jump_marks: HashMap::new(),
            variables: [false; VAR_SLOTS],
            module_state: HashMap::new(),
            opt_level,
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
            typ: Box::new(ErrorType::NonexistentInlineVar(symbol.clone())),
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
            typ: Box::new(ErrorType::TooManyVars),
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
                    typ: Box::new(ErrorType::NonexistentVar(symbol.clone())),
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
            typ: Box::new(ErrorType::TooManyVars),
            location,
        })
    }

    pub const fn cleanup_temp_var(&mut self, index: u8) {
        self.variables[index as usize] = false;
    }

    /// use the "instr" macro
    pub fn push_instr(&mut self, instr: Instruction) {
        let last_scope = self.last_scope_mut();
        instr.execute(&mut last_scope.state);
        last_scope.instructions.push(Instr::Code(instr));
    }

    fn flatten_scope(scope: Vec<Instr>, into: &mut Vec<Instruction>) {
        for i in scope {
            match i {
                Instr::Code(instr) => into.push(instr),
                Instr::Scope(s) => Self::flatten_scope(s, into),
            }
        }
    }

    #[must_use]
    pub fn last_scope(&self) -> &Scope {
        self.scopes.last()
    }

    #[must_use]
    pub fn last_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut()
    }

    #[inline]
    #[must_use]
    fn is_root_scope(&self) -> bool {
        self.scopes.len() == 1
    }

    fn insert_jump_mark(&mut self) -> u8 {
        let id = self.jump_marks.len() as u8;
        self.jump_marks.insert(id, 0);
        id
    }

    fn eval_statement(&mut self, statement: Statement) -> Result<(), Error> {
        match statement.typ {
            Stmt::Expr(expr) => self.eval_expr(&expr, statement.location),
            Stmt::InlineDeclaration { ident, value } => self.visit_inline_decl(ident, *value),
            Stmt::VarDeclaration { ident } => self.visit_var_decl(ident),
            Stmt::Use(modules) => self.visit_use(modules, statement.location),
            Stmt::Conditional {
                condition,
                body,
                paths,
                alternate,
            } => self.visit_conditional(*condition, body, paths, alternate),
            Stmt::EndlessLoop { body } => self.visit_endless(body, statement.location),
            Stmt::WhileLoop { condition, body } => self.visit_while(*condition, body),
            Stmt::Pass => Ok(self.visit_pass(statement.location)),
            Stmt::FunctionDeclaration { ident, args, body } => {
                self.visit_function_decl(ident, args, body)
            }
        }
    }

    fn eval_conditional(
        &mut self,
        condition: Expression,
        body: Fragment,
        paths: Vec<(Expression, Fragment)>,
        alternate: Option<Fragment>,
    ) -> Result<Result<(), Error>, Error> {
        let location = condition.location;
        let (left, right, operator) = eval_condition(condition)?;
        let end_id = self.insert_jump_mark();
        let mut next_mark_id = self.insert_jump_mark();

        self.put_comparison((&left, &right, operator.opposite()), location, next_mark_id)?;

        let mut last_state = self.last_scope().state;

        self.push_scope(body, last_state)?;
        if !paths.is_empty() || alternate.is_some() {
            instr!(self, JMP, end_id, location);
        }
        self.pop_scope();
        self.jump_marks.insert(
            next_mark_id,
            Self::scope_len(&self.scopes.first().instructions),
        );
        let path_len = paths.len();
        paths.into_iter().enumerate().try_for_each(|path| {
            let (index, (condition, body)) = path;
            let location = condition.location;
            let (left, right, operator) = eval_condition(condition)?;

            next_mark_id = self.insert_jump_mark();

            self.put_comparison((&left, &right, operator.opposite()), location, next_mark_id)?;

            last_state = self.last_scope().state;

            self.push_scope(body, last_state)?;

            if index != path_len - 1 || alternate.is_some() {
                instr!(self, JMP, end_id, location);
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

    fn push_scope(&mut self, body: Fragment, state: ComputerState) -> Res {
        self.scopes.push(Scope::with_state(state));
        body.into_iter()
            .try_for_each(|line| self.eval_statement(line))?;
        Ok(())
    }

    fn put_comparison(
        &mut self,
        condition: (&Expression, &Expression, EqualityOperator),
        location: Range,
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
            location,
        ));
        Ok(())
    }

    fn try_eval_const(&mut self, expr: &Expression) -> Res<Option<i16>> {
        match &expr.typ {
            Expr::Identifier(name) => self
                .get_inline_var(name, expr.location)
                .or(err!(
                    ErrorType::NonexistentInlineVar(name.clone()),
                    expr.location
                ))
                .map(Some),
            Expr::BinaryExpr {
                left,
                right,
                operator,
            } => {
                let left = self.try_eval_const(left)?;
                let right = self.try_eval_const(right)?;
                let Some(left) = left else {
                    return Ok(None);
                };
                let Some(right) = right else {
                    return Ok(None);
                };
                Ok(Some(match operator {
                    Operator::Plus => left + right,
                    Operator::Minus => left - right,
                    Operator::Mult => left * right,
                    Operator::And => left & right,
                    Operator::Or => left | right,
                    Operator::Xor => left ^ right,
                }))
            }
            Expr::NumericLiteral(value) => i16::try_from(*value)
                .or(err!(ErrorType::NumberTooBig, expr.location))
                .map(Some),
            _ => err!(ErrorType::ForbiddenInline, expr.location),
        }
    }

    #[must_use]
    pub const fn can_put_into_a(expr: &Expr) -> bool {
        use Expr as E;
        match &expr {
            E::NumericLiteral(..) | E::Identifier(..) => true,
            E::Assignment { ident: _, value } => Self::can_put_into_a(&value.typ),
            _ => false,
        }
    }

    #[must_use]
    pub const fn can_put_into_b(expr: &Expression) -> bool {
        use Expr as E;
        matches!(expr.typ, E::NumericLiteral(..) | E::Identifier(..))
    }

    fn eval_binary_expr(
        &mut self,
        left: &Expression,
        right: &Expression,
        operator: Operator,
        location: Range,
    ) -> Res {
        self.put_ab(left, right, operator.is_commutative())?;

        self.put_op(operator, location);
        Ok(())
    }

    /// # Returns
    /// if the arguments were swapped
    fn put_ab(&mut self, left: &Expression, right: &Expression, is_commutative: bool) -> Res<bool> {
        let mut swapped = false;
        match (Self::can_put_into_a(&left.typ), Self::can_put_into_b(right)) {
            (true, true) => {
                if is_commutative
                    && ((self.is_in_a(right) || self.is_in_b(left))
                        || (matches!(right.typ, Expr::Identifier(..))
                            && matches!(left.typ, Expr::NumericLiteral(..))))
                {
                    self.put_into_a(&right.typ, right.location)?;
                    self.put_into_b(left)?;
                    return Ok(true);
                }
                self.put_into_a(&left.typ, left.location)?;
                self.put_into_b(right)?;
            }
            (true, false) => {
                self.eval_expression(right)?;
                if is_commutative && Self::can_put_into_b(left) {
                    self.put_into_b(left)?;
                    swapped = true;
                } else {
                    // if we just saved a variable we use it to switch
                    if let Expr::Assignment { ident, value: _ } = &right.typ {
                        instr!(
                            self,
                            LB,
                            self.get_var(&ident.symbol, Range::default()).unwrap(),
                            right.location
                        );
                    } else {
                        self.switch(left.location)?;
                    }
                    self.put_into_a(&left.typ, left.location)?;
                }
            }
            (false, true) => {
                self.eval_expression(left)?;
                self.put_into_b(right)?;
            }
            (false, false) => {
                self.eval_expression(right)?;
                if let Expr::Assignment { ident, value: _ } = &right.typ {
                    self.eval_expression(left)?;
                    instr!(
                        self,
                        LB,
                        self.get_var(&ident.symbol, Range::default()).unwrap(),
                        right.location
                    );
                } else {
                    let temp = self.insert_temp_var(left.location)?;
                    instr!(self, SVA, temp, left.location);
                    self.eval_expression(left)?;
                    instr!(self, LB, temp, left.location);
                    self.cleanup_temp_var(temp);
                }
            }
        }
        Ok(swapped)
    }

    pub fn eval_expression(&mut self, expr: &Expression) -> Result<(), Error> {
        self.eval_expr(&expr.typ, expr.location)
    }

    fn visit_inline_decl(&mut self, ident: Ident, value: Expression) -> Result<(), Error> {
        let value = self.try_eval_const(&value)?.ok_or(Error {
            typ: Box::new(ErrorType::ForbiddenInline),
            location: value.location,
        })?;
        self.insert_inline_var(ident.symbol, value);
        Ok(())
    }

    fn visit_var_decl(&mut self, ident: Ident) -> Result<(), Error> {
        self.insert_var(&ident.symbol, ident.location)?;
        Ok(())
    }

    fn visit_use(&mut self, modules: Vec1<Ident>, location: Range) -> Result<(), Error> {
        for module in modules {
            if !self.is_root_scope() {
                return Err(Error {
                    typ: Box::new(ErrorType::UseOutsideGlobalScope),
                    location,
                });
            }
            if !exist(&module.symbol) {
                return Err(Error {
                    typ: Box::new(ErrorType::NonexistentModule(module.symbol)),
                    location: module.location,
                });
            }
            init(&module.symbol, self, module.location)?;
            self.modules.insert(module.symbol);
        }
        Ok(())
    }

    fn visit_conditional(
        &mut self,
        condition: Expression,
        body: Fragment,
        paths: Vec<(Expression, Fragment)>,
        alternate: Option<Fragment>,
    ) -> Result<(), Error> {
        self.eval_conditional(condition, body, paths, alternate)
            .flatten()
    }

    fn visit_endless(&mut self, body: Fragment, location: Range) -> Result<(), Error> {
        let mark = Self::scope_len(&self.scopes.first().instructions);
        let id = self.insert_jump_mark();
        self.jump_marks.insert(id, mark);

        self.push_scope(body, ComputerState::default())?;
        self.pop_scope();

        instr!(self, JMP, id, location);

        Ok(())
    }

    fn visit_while(&mut self, condition: Expression, body: Fragment) -> Result<(), Error> {
        let condition_loc = condition.location;
        let (left, right, operator) = eval_condition(condition)?;

        let start_id = self.insert_jump_mark();
        let end_id = self.insert_jump_mark();

        self.put_comparison((&left, &right, operator.opposite()), condition_loc, end_id)?;

        let start = Self::scope_len(&self.scopes.first().instructions);

        self.jump_marks.insert(start_id, start);

        self.push_scope(body, self.last_scope().state)?;

        self.put_comparison((&left, &right, operator), condition_loc, start_id)?;

        self.pop_scope();
        let end = Self::scope_len(&self.scopes.first().instructions);

        self.jump_marks.insert(end_id, end);

        Ok(())
    }

    fn visit_function_decl(
        &mut self,
        ident: Ident,
        _args: Vec<Ident>,
        _body: Fragment,
    ) -> Result<(), Error> {
        Err(Error {
            typ: Box::new(super::error::Type::NoFunctions),
            location: ident.location,
        })
    }

    fn eval_expr(&mut self, expr: &Expr, location: Range) -> Result<(), Error> {
        match &expr {
            Expr::NumericLiteral(..) | Expr::Identifier(..) => {
                self.put_into_a(expr, location)?;
            }
            Expr::BinaryExpr {
                left,
                right,
                operator,
            } => self.eval_binary_expr(left, right, *operator, location)?,
            Expr::Assignment { ident, value } => {
                self.eval_assignment(&ident.symbol, value)?;
            }
            Expr::IAssignment {
                ident,
                value,
                operator,
            } => {
                self.eval_iassignment(ident, value, *operator)?;
            }
            Expr::Call { args, function } => self.eval_call(function, args)?,
            Expr::EqExpr { .. } => {
                return err!(EqInNormalExpr, location);
            }
            Expr::Debug => instr!(self, LAL, 17, location),
            Expr::Member { .. } => return err!(NoConstants, location),
        }
        Ok(())
    }

    fn get_output(&mut self) -> <Self as Target>::Output {
        let main_scope = vec![Instr::Scope(
            mem::take(self.scopes.first_mut()).instructions,
        )];
        let mut instructions = vec![];
        Self::flatten_scope(main_scope, &mut instructions);
        Self::insert_disc_jumps(&mut instructions, &mut self.jump_marks);
        Self::replace_jump_marks(&mut instructions, &self.jump_marks);
        instructions
    }

    fn eval_assignment(&mut self, symbol: &str, value: &Expression) -> Res {
        self.eval_expression(value)?;

        let slot = self.insert_var(symbol, value.location)?;

        instr!(self, SVA, slot, value.location);

        Ok(())
    }

    fn eval_iassignment(&mut self, ident: &Ident, value: &Expression, operator: Operator) -> Res {
        self.eval_expression(value)?;
        self.put_into_b(&Expression {
            typ: Expr::Identifier(ident.symbol.clone()),
            location: value.location,
        })?;

        self.put_op(operator, value.location);

        let slot = self.get_var(&ident.symbol, value.location)?;

        self.save_to(slot, value.location);
        Ok(())
    }

    fn put_op(&mut self, operator: Operator, location: Range) {
        use Operator as O;
        match operator {
            O::Plus => instr!(self, ADD, location),
            O::Minus => instr!(self, SUB, location),
            O::Mult => instr!(self, MUL, location),
            O::And => instr!(self, AND, location),
            O::Or => instr!(self, OR, location),
            O::Xor => instr!(self, XOR, location),
        }
    }

    /// tries to get the value known at compile time
    ///
    /// # Errors
    ///
    /// on any compiler error
    pub fn try_get_constant(&mut self, value: &Expression) -> Res<Option<i16>> {
        match &value.typ {
            Expr::NumericLiteral(num) => {
                i16::try_from(*num).map_or(err!(NumberTooBig, value.location), |v| Ok(Some(v)))
            }
            Expr::Identifier(symbol) => self.get_inline_var(symbol, value.location).map(Some),
            Expr::BinaryExpr { .. } => self.try_eval_const(value),
            _ => Ok(None),
        }
    }

    /// puts a into b
    ///
    /// # Errors
    ///
    /// if there are too many variables
    pub fn switch(&mut self, location: Range) -> Res {
        let temp = self.insert_temp_var(location)?;
        self.save_to(temp, location);
        instr!(self, LB, temp, location);
        self.cleanup_temp_var(temp);
        Ok(())
    }

    /// expr should be either `NumericLiteral`, `Identifier` or `Assignment`
    ///
    /// # Errors
    ///
    /// if variable doesn't exist or called on a wrong expression
    pub fn put_into_a(&mut self, expr: &Expr, location: Range) -> Res {
        use Expr as E;
        match &expr {
            E::NumericLiteral(value) => {
                self.put_a_number(
                    i16::try_from(*value).or(err!(NumberTooBig, location))?,
                    location,
                );
            }
            E::Identifier(symbol) => {
                if let Ok(value) = self.get_inline_var(symbol, location) {
                    self.put_a_number(value, location);
                } else {
                    let var = self.get_var(symbol, location)?;
                    if let RegisterContents::Variable(v) = self.last_scope().state.a
                        && v == var
                    {
                        return Ok(());
                    }
                    instr!(self, LA, var, location);
                }
            }
            E::Assignment { .. } => {
                if Self::can_put_into_a(expr) {
                    self.eval_expr(expr, location)?;
                } else {
                    return Err(Error {
                        typ: Box::new(ErrorType::SomethingElseWentWrong("put_a".to_string())),
                        location,
                    });
                }
            }
            _ => {
                return Err(Error {
                    typ: Box::new(ErrorType::SomethingElseWentWrong(
                        "put_a called on wrong expression".to_string(),
                    )),
                    location,
                });
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
        use Expr as E;
        match &expr.typ {
            E::NumericLiteral(value) => {
                self.put_b_number(
                    i16::try_from(*value).or(err!(NumberTooBig, expr.location))?,
                    expr.location,
                );
            }
            E::Identifier(symbol) => {
                if let Ok(value) = self.get_inline_var(symbol, expr.location) {
                    self.put_b_number(value, expr.location);
                } else {
                    let var = self.get_var(symbol, expr.location)?;
                    if let RegisterContents::Variable(v) = self.last_scope().state.b
                        && v == var
                    {
                        return Ok(());
                    }
                    instr!(self, LB, var, expr.location);
                }
            }
            _ => {
                return Err(Error {
                    typ: Box::new(ErrorType::SomethingElseWentWrong(
                        "put_b called on wrong expression".to_string(),
                    )),
                    location: expr.location,
                });
            }
        }
        Ok(())
    }

    #[inline]
    pub fn save_to_out(&mut self, port: u8, location: Range) {
        self.save_to(port + 32, location);
    }

    #[inline]
    pub fn save_to(&mut self, slot: u8, location: Range) {
        instr!(self, SVA, slot, location);
    }

    fn is_in_a(&self, expr: &Expression) -> bool {
        use Expr as E;
        match &expr.typ {
            E::NumericLiteral(value) => {
                matches!(self.last_scope().state.a, RegisterContents::Number(v) if v as i32 == *value)
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

    fn is_in_b(&self, expr: &Expression) -> bool {
        use Expr as E;
        match &expr.typ {
            E::NumericLiteral(value) => {
                matches!(self.last_scope().state.b, RegisterContents::Number(v) if v as i32 == *value)
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

    pub fn put_a_number(&mut self, value: i16, location: Range) {
        if self.last_scope().state.a == RegisterContents::Number(value) {
            return;
        }
        let bytes = value.to_le_bytes();
        instr!(self, LAL, bytes[0], location);
        if bytes[1] != 0 {
            instr!(self, LAH, bytes[1], location);
        }
    }

    pub fn put_b_number(&mut self, value: i16, location: Range) {
        if self.last_scope().state.b == RegisterContents::Number(value) {
            return;
        }
        let bytes = value.to_le_bytes();
        instr!(self, LBL, bytes[0], location);
        if bytes[1] != 0 {
            instr!(self, LBH, bytes[1], location);
        }
    }

    fn eval_call(&mut self, function: &Expression, args: &Vec<Expression>) -> Res {
        use Expr as E;
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
                        typ: Box::new(ErrorType::NonexistentModule(format!("{object:?}"))),
                        location: function.location,
                    });
                }
            },
            _ => {
                return Err(Error {
                    typ: Box::new(ErrorType::UnknownMethod(format!("{function:?}"))),
                    location: function.location,
                });
            }
        }
        if !self.modules.contains(module) {
            return Err(Error {
                typ: Box::new(ErrorType::UnlodadedModule(module.clone())),
                location: function.location,
            });
        }

        call(
            module,
            self,
            &Call {
                method_name: &method.symbol,
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
                let location = instr.orig_location;
                if instr.variant.is_jump() && !instr.variant.disc_jump() {
                    let mark = instr.arg.expect("Jump instruction doesn't have arg");
                    let current_page = i / 64;
                    let jump_page = jump_marks.get(&mark).expect("Invalid jump mark") / 64;
                    if current_page != jump_page as usize {
                        instr.variant = instr.variant.to_disc_jump();
                        instructions.insert(
                            i,
                            Instruction::new(InstructionVariant::LCL, Some(jump_page), location),
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

    fn visit_pass(&mut self, location: Range) {
        instr!(self, LAL, 17, location);
    }
}

fn eval_condition(
    condition: Expression,
) -> Res<(Box<Expression>, Box<Expression>, EqualityOperator)> {
    let Expr::EqExpr {
        left,
        right,
        operator,
    } = condition.typ
    else {
        return Err(Error {
            typ: Box::new(ErrorType::NormalInEqExpr),
            location: condition.location,
        });
    };
    Ok((left, right, operator))
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Target for Compiler {
    type Output = Vec<Instruction>;

    fn compile_program(&mut self, program: Fragment) -> Result<Self::Output, Vec<Error>> {
        let errors = program
            .into_iter()
            .filter_map(|line| self.eval_statement(line).err())
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(self.get_output())
    }

    fn reset(&mut self) {
        drop(mem::take(self));
    }
}
