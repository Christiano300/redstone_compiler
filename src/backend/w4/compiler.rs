use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Write,
    mem,
};

use vec1::{Vec1, vec1};
use wasm_encoder::{
    CodeSection, Encode, ExportSection, Function, FunctionSection, ImportSection, Instruction,
    InstructionSink, MemArg, MemoryType, Module, TypeSection, ValType,
};
use wasmprinter::{Config, PrintFmtWrite};

use crate::{
    backend::{Output, target::Target},
    error::Error,
    frontend::{
        EqualityOperator, Expr, Expression, Fragment, Ident, Operator, Range, Statement, Stmt,
        args_location,
    },
};

use super::error::Type as ErrorType;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OptLevel {
    #[default]
    None,
    Basic,
    Full,
}

impl OptLevel {
    fn opt(&self) -> bool {
        matches!(self, Self::Basic | Self::Full)
    }

    fn full(&self) -> bool {
        matches!(self, Self::Full)
    }
}

#[derive(Debug, Default)]
struct BlockScope {
    variables: HashMap<String, u32>,
    inline_variables: HashMap<String, i32>,
}

#[derive(Debug, Clone)]
struct W4Function {
    arg_count: u16,
    func_index: u32,
    type_index: u32,
}

#[derive(Debug, Default)]
struct FunctionScope {
    instr_bytes: Vec<u8>,
    scopes: Vec1<BlockScope>,
    next_local: u32,
    max_locals: u32,
    pending_assignments: HashMap<String, ()>,
}

impl FunctionScope {
    fn new(args: Vec<Ident>) -> Self {
        let mut scope = BlockScope::default();
        let len = args.len() as u32;
        for (i, arg) in args.into_iter().enumerate() {
            scope.variables.insert(arg.symbol, i as u32);
        }
        Self {
            scopes: vec1!(scope),
            next_local: len,
            max_locals: len,
            ..Default::default()
        }
    }

    fn sink(&mut self) -> InstructionSink<'_> {
        InstructionSink::new(&mut self.instr_bytes)
    }

    fn get_inline_const(&self, name: &str) -> Option<i32> {
        for scope in self.scopes.iter().rev() {
            if let Some(&value) = scope.inline_variables.get(name) {
                return Some(value);
            }
        }
        None
    }

    fn get_local_index(&self, name: &str) -> Option<u32> {
        for scope in self.scopes.iter().rev() {
            if let Some(&index) = scope.variables.get(name) {
                return Some(index);
            }
        }
        None
    }

    fn insert_var(&mut self, name: &str) -> u32 {
        let index = self.next_local;
        self.next_local += 1;
        if self.next_local > self.max_locals {
            self.max_locals = self.next_local;
        }
        self.scopes
            .last_mut()
            .variables
            .insert(name.to_string(), index);
        index
    }

    fn push_scope(&mut self) {
        self.scopes.push(BlockScope::default());
    }
}

#[derive(Debug, Default)]
pub struct W4Compiler {
    function_scopes: Vec1<FunctionScope>,
    modules: HashSet<String>,
    types: TypeSection,
    functions: HashMap<String, W4Function>,
    /// Maps function index to (type index, wasm-encoder function object)
    compiled_functions: HashMap<u32, (u32, Function)>,
    pub module_state: HashMap<&'static str, Box<dyn Any>>,
    strings: Vec<String>,
    imports: ImportSection,
    pub opt_level: OptLevel,
}

impl W4Compiler {
    fn func(&mut self) -> &mut FunctionScope {
        self.function_scopes.last_mut()
    }

    pub fn instr(&mut self) -> InstructionSink<'_> {
        self.func().sink()
    }

    fn try_get_const(&mut self, expr: &Expr) -> Option<i32> {
        match &expr {
            Expr::NumericLiteral(value) => Some(*value),
            Expr::BinaryExpr {
                left,
                right,
                operator,
            } => {
                let left_val = self.try_get_const(&left.typ)?;
                let right_val = self.try_get_const(&right.typ)?;
                Some(match operator {
                    Operator::Plus => left_val + right_val,
                    Operator::Minus => left_val - right_val,
                    Operator::Mult => left_val * right_val,
                    Operator::And => left_val & right_val,
                    Operator::Or => left_val | right_val,
                    Operator::Xor => left_val ^ right_val,
                })
            }
            Expr::EqExpr {
                left,
                right,
                operator,
            } => {
                let left_val = self.try_get_const(&left.typ)?;
                let right_val = self.try_get_const(&right.typ)?;
                Some(match operator {
                    EqualityOperator::EqualTo => left_val == right_val,
                    EqualityOperator::NotEqual => left_val != right_val,
                    EqualityOperator::Greater => left_val > right_val,
                    EqualityOperator::Less => left_val < right_val,
                    EqualityOperator::GreaterEq => left_val >= right_val,
                    EqualityOperator::LessEq => left_val <= right_val,
                } as i32)
            }
            _ => None,
        }
    }

    fn eval_call(&mut self, function: &Expression, args: &Vec<Expression>, location: Range) -> Res {
        use Expr as E;
        match &function.typ {
            E::Member { object, property } => {
                if let E::Identifier(symbol) = &object.typ {
                    todo!("Module call");
                } else {
                    return Err(Error {
                        typ: Box::new(ErrorType::NonexistentModule(format!("{:?}", object))),
                        location: function.location,
                    });
                }
            }
            E::Identifier(symbol) => {
                let Some(func) = self.functions.get(symbol).cloned() else {
                    return err!(
                        ErrorType::NonexistentFunc(symbol.clone()),
                        function.location
                    );
                };
                if func.arg_count as usize != args.len() {
                    return err!(
                        ErrorType::WrongArgs {
                            supplied: args.len(),
                            takes: func.arg_count,
                        },
                        args_location(args).unwrap_or(location)
                    );
                }
                for arg in args {
                    self.compile_expr(&arg.typ, true, arg.location)?;
                }
                self.instr().call(func.func_index);
                Ok(())
            }
            _ => {
                return Err(Error {
                    typ: Box::new(ErrorType::UnknownMethod(format!("{:?}", function))),
                    location: function.location,
                });
            }
        }
    }

    fn scan_functions(&mut self, program: &Fragment) -> Res {
        for line in program {
            if let Stmt::FunctionDeclaration { ident, args, .. } = &line.typ {
                if self.functions.contains_key(&ident.symbol) {
                    return err!(
                        ErrorType::DuplicateFunction(ident.symbol.clone()),
                        ident.location
                    );
                }
                let type_index = self.types.len();
                // TODO: Optimize type section by reusing types, maybe just predefined types
                self.types
                    .ty()
                    .function(vec![ValType::I32; args.len()], [ValType::I32]);
                let func_index = self.functions.len() as u32;
                self.functions.insert(
                    ident.symbol.clone(),
                    W4Function {
                        arg_count: args.len() as u16,
                        func_index,
                        type_index,
                    },
                );
            }
        }
        Ok(())
    }

    fn eval_statement(&mut self, statement: Statement, push: bool) -> Res {
        Ok(match statement.typ {
            Stmt::Expr(expr) => {
                self.compile_expr(&expr, push, statement.location)?;
            }
            Stmt::FunctionDeclaration {
                ident,
                args,
                mut body,
            } => {
                let args_len = args.len();
                self.function_scopes.push(FunctionScope::new(args));
                let last = body.pop().unwrap(); // Body cannot be empty
                for line in body {
                    self.eval_statement(line, false)?;
                }
                self.eval_statement(last, true)?;
                // We already checked for duplicate functions in the first pass, so we can safely unwrap here
                let func = self.functions.get(&ident.symbol).unwrap();
                let scope = self.function_scopes.pop().unwrap(); // TODO: Prob remove vec and use mem::take
                let mut wasm_func =
                    Function::new([(scope.max_locals - args_len as u32, ValType::I32)]);
                wasm_func.raw(scope.instr_bytes).instructions().end();
                self.compiled_functions
                    .insert(func.func_index, (func.type_index, wasm_func));
            }
            Stmt::InlineDeclaration { ident, value } => {
                if self.func().get_local_index(&ident.symbol).is_some()
                    || self.func().get_inline_const(&ident.symbol).is_some()
                {
                    return err!(
                        ErrorType::DuplicateVar(ident.symbol.clone()),
                        ident.location
                    );
                }
                let value = self.try_get_const(&value.typ).ok_or(Error {
                    typ: Box::new(ErrorType::ForbiddenInline),
                    location: value.location,
                })?;
                self.func()
                    .scopes
                    .last_mut()
                    .inline_variables
                    .insert(ident.symbol, value);
            }
            Stmt::VarDeclaration { ident } => {
                if self.func().get_local_index(&ident.symbol).is_some()
                    || self.func().get_inline_const(&ident.symbol).is_some()
                {
                    return err!(
                        ErrorType::DuplicateVar(ident.symbol.clone()),
                        ident.location
                    );
                }
            }
            Stmt::Use(vec1) => todo!("use"),
            Stmt::Conditional {
                condition,
                body,
                paths,
                alternate,
            } => todo!("conditionals"),
            Stmt::EndlessLoop { body } => {
                todo!("forever")
            }
            Stmt::WhileLoop { condition, body } => todo!("while"),
            Stmt::Pass => todo!("pass"),
            Stmt::DataDeclaration { ident, value } => todo!("data"),
        })
    }

    fn compile_expr(&mut self, expr: &Expr, push: bool, location: Range) -> Result<(), Error> {
        Ok(match expr {
            Expr::NumericLiteral(value) => {
                if push {
                    self.instr().i32_const(*value);
                }
            }
            Expr::Identifier(name) => {
                if let Some(val) = self.func().get_inline_const(&name)
                    && push
                {
                    self.instr().i32_const(val);
                    return Ok(());
                }
                let var = self.func().get_local_index(name).ok_or(Error {
                    typ: Box::new(ErrorType::UnknownVariable(name.clone())),
                    location,
                })?;
                if push {
                    self.instr().local_get(var);
                }
            }
            Expr::BinaryExpr {
                left,
                right,
                operator,
            } => {
                if self.opt_level.opt()
                    && let Some(value) = self.try_get_const(expr)
                {
                    if push {
                        self.instr().i32_const(value);
                    }
                    return Ok(());
                }
                self.compile_expr(&left.typ, push, location)?;
                self.compile_expr(&right.typ, push, location)?;
                if push {
                    match operator {
                        Operator::Plus => Instruction::I32Add,
                        Operator::Minus => Instruction::I32Sub,
                        Operator::Mult => Instruction::I32Mul,
                        Operator::And => Instruction::I32And,
                        Operator::Or => Instruction::I32Or,
                        Operator::Xor => Instruction::I32Xor,
                    }
                    .encode(&mut self.func().instr_bytes);
                }
            }
            Expr::Assignment { ident, value } => {
                if let Some(_) = self.func().get_inline_const(&ident.symbol) {
                    return err!(TrySetInline, ident.location);
                }
                self.compile_expr(&value.typ, true, location)?;
                let var = self
                    .func()
                    .get_local_index(&ident.symbol)
                    .unwrap_or_else(|| self.func().insert_var(&ident.symbol));
                if push {
                    self.instr().local_tee(var);
                } else {
                    self.instr().local_set(var);
                }
            }
            Expr::IAssignment {
                ident,
                value,
                operator,
            } => {
                todo!("Iassignments")
            }
            Expr::Call { args, function } => {
                self.eval_call(function, args, location)?;
            }
            Expr::EqExpr { .. } => {
                todo!()
            }
            Expr::Debug => {}
            Expr::Member { .. } => {
                todo!()
            }
        })
    }

    fn get_output(&mut self) -> Vec<u8> {
        let mut functions = FunctionSection::new();
        let mut code = CodeSection::new();
        for i in 0..self.compiled_functions.len() {
            functions.function(self.compiled_functions.get(&(i as u32)).unwrap().0);
            code.function(&self.compiled_functions.get(&(i as u32)).unwrap().1);
        }
        let mut exports = ExportSection::new();
        for i in self.functions.iter() {
            exports.export(&i.0, wasm_encoder::ExportKind::Func, i.1.func_index);
        }
        let mut module = Module::new();
        module.section(&self.types);
        module.section(&self.imports);
        module.section(&functions);
        module.section(&exports);
        module.section(&code);
        module.finish()
    }
}

type Res<T = ()> = Result<T, Error>;

impl Output for Vec<u8> {
    fn repr(&self) -> String {
        let mut buf = String::new();
        Config::new()
            .fold_instructions(false)
            .name_unnamed(true)
            .print(self, &mut PrintFmtWrite(&mut buf))
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        buf
    }

    fn repr_bin(&self) -> Option<String> {
        let mut buf = String::with_capacity(self.len() * 2);
        for byte in self {
            write!(buf, "{byte:x}").ok()?;
        }
        Some(buf)
    }

    fn repr_loc(&self) -> Option<String> {
        None
    }
}

impl Target for W4Compiler {
    type Output = Vec<u8>;

    fn reset(&mut self) {
        drop(mem::take(self));
    }

    fn compile_program(&mut self, program: Fragment) -> Result<Self::Output, Vec<Error>> {
        self.imports.import(
            "env",
            "memory",
            MemoryType {
                minimum: 1,
                maximum: None,
                memory64: false,
                page_size_log2: None,
                shared: false,
            },
        );
        // First pass check for functions
        // TODO: Rewrite so that functions and outer context are handled differently
        self.scan_functions(&program).map_err(|err| vec![err])?;

        let errors = program
            .into_iter()
            .filter_map(|line| self.eval_statement(line, false).err())
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(self.get_output())
    }
}
