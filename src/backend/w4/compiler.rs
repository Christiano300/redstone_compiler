use std::{
    any::Any,
    collections::{HashMap, HashSet},
    mem,
};

use vec1::{Vec1, vec1};
use wasm_encoder::{
    CodeSection, Encode, Function, FunctionSection, ImportSection, Instruction, InstructionSink,
    Module, TypeSection, ValType,
};
use wasmprinter::{Config, PrintFmtWrite};

use crate::{
    backend::{Output, target::Target},
    error::Error,
    frontend::{
        EqualityOperator, Expr, Expression, Fragment, Ident, Operator, Range, Statement, Stmt,
    },
};

use super::error::Type as ErrorType;

use super::module::Call;

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
}

#[derive(Debug)]
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
                        typ: Box::new(ErrorType::NonexistentModule(format!("{:?}", object))),
                        location: function.location,
                    });
                }
            },
            E::Identifier(_symbol) => {
                todo!("Function calling");
            }
            _ => {
                return Err(Error {
                    typ: Box::new(ErrorType::UnknownMethod(format!("{:?}", function))),
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

        super::module::call(
            module,
            self,
            &Call {
                method_name: &method.symbol,
                args,
                location: function.location,
            },
        )
    }

    fn scan_functions(&mut self, program: &Fragment) -> Res {
        for line in program {
            if let Stmt::FunctionDeclaration { ident, args, .. } = &line.typ {
                if self.functions.contains_key(&ident.symbol) {
                    return Err(Error {
                        typ: Box::new(ErrorType::DuplicateFunction(ident.symbol.clone())),
                        location: ident.location,
                    });
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
                self.function_scopes.push(FunctionScope::new(args));
                let last = body.pop().unwrap(); // Body cannot be empty
                for line in body {
                    self.eval_statement(line, false)?;
                }
                self.eval_statement(last, true)?;
                // We already checked for duplicate functions in the first pass, so we can safely unwrap here
                let func = self.functions.get(&ident.symbol).unwrap();
                let scope = self.function_scopes.pop().unwrap(); // TODO: Prob remove vec and use mem::take
                let mut wasm_func = Function::new([(scope.max_locals, ValType::I32)]);
                wasm_func.raw(scope.instr_bytes).instructions().end();
                self.compiled_functions
                    .insert(func.func_index, (func.type_index, wasm_func));
            }
            Stmt::InlineDeclaration { ident, value } => todo!(),
            Stmt::VarDeclaration { ident } => todo!(),
            Stmt::Use(vec1) => todo!(),
            Stmt::Conditional {
                condition,
                body,
                paths,
                alternate,
            } => todo!(),
            Stmt::EndlessLoop { body } => todo!(),
            Stmt::WhileLoop { condition, body } => todo!(),
            Stmt::Pass => todo!(),
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
                todo!()
            }
            Expr::Call { args, function } => {
                for arg in args {
                    self.compile_expr(&arg.typ, true, location)?;
                }
                self.eval_call(function, args)?;
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
        for i in 0..self.compiled_functions.len() {
            functions.function(self.compiled_functions.get(&(i as u32)).unwrap().0);
        }
        let mut module = Module::new();
        module.section(&self.types);
        module.section(&self.imports);
        module.section(&functions);
        let mut code = CodeSection::new();
        for i in 0..self.compiled_functions.len() {
            code.function(&self.compiled_functions.get(&(i as u32)).unwrap().1);
        }
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
            .print(self, &mut PrintFmtWrite(&mut buf))
            .inspect_err(|e| eprintln!("{e:?}"))
            .unwrap();
        buf
    }

    fn repr_bin(&self) -> Option<String> {
        None
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
        // First pass check for functions
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
