use std::{
    any::Any,
    collections::{HashMap, HashSet},
    mem,
};

use vec1::{vec1, Vec1};

use crate::{
    backend::target::Target,
    error::Error,
    frontend::{Expr, Expression, Fragment, Ident, Range},
};

use super::module::Call;

const WASM4_FRAMEBUFFER: u32 = 0xa0;

#[derive(Debug)]
pub struct W4Compiler {
    scopes: Vec1<W4Scope>,
    modules: HashSet<String>,
    pub variables: Vec<Variable>,
    pub module_state: HashMap<&'static str, Box<dyn Any>>,
    strings: Vec<String>,
    next_var_offset: usize,
}

#[derive(Debug)]
struct W4Scope {
    variables: HashMap<String, usize>,
}

impl Default for W4Scope {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct Variable {
    name: String,
    offset: usize,
}

#[derive(Debug)]
pub struct W4Output(Vec<u8>);

impl W4Compiler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: vec1!(W4Scope::default()),
            modules: HashSet::new(),
            variables: Vec::new(),
            module_state: HashMap::new(),
            strings: Vec::new(),
            next_var_offset: WASM4_FRAMEBUFFER as usize + (160 * 160 / 4),
        }
    }

    pub fn add_string(&mut self, s: &str) -> usize {
        let offset = self.next_var_offset;
        self.next_var_offset += s.len() + 1;
        self.strings.push(s.to_string());
        offset
    }

    fn get_variable_offset(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(&offset) = scope.variables.get(name) {
                return Some(offset);
            }
        }
        None
    }

    fn allocate_variable(&mut self, name: String) -> usize {
        let offset = self.next_var_offset;
        self.variables.push(Variable { name, offset });
        self.next_var_offset += 4;
        offset
    }

    fn push_scope(&mut self, body: Fragment) -> Result<(), Error> {
        self.scopes.push(W4Scope::default());
        for line in body {
            self.eval_statement(line)?;
        }
        self.scopes.pop();
        Ok(())
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
                        typ: Box::new(super::error::Type::NonexistentModule(format!(
                            "{:?}",
                            object
                        ))),
                        location: function.location,
                    });
                }
            },
            _ => {
                return Err(Error {
                    typ: Box::new(super::error::Type::UnknownMethod(format!("{:?}", function))),
                    location: function.location,
                });
            }
        }
        if !self.modules.contains(module) {
            return Err(Error {
                typ: Box::new(super::error::Type::UnlodadedModule(module.clone())),
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
}

type Res<T = ()> = Result<T, Error>;

impl Default for W4Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Target for W4Compiler {
    type Output = W4Output;

    fn visit_inline_decl(&mut self, _ident: Ident, _value: Expression) -> Result<(), Error> {
        Ok(())
    }

    fn visit_var_decl(&mut self, ident: Ident) -> Result<(), Error> {
        let offset = self.allocate_variable(ident.symbol.clone());
        self.scopes
            .last_mut()
            .variables
            .insert(ident.symbol, offset);
        Ok(())
    }

    fn visit_use(&mut self, modules: Vec1<Ident>, _location: Range) -> Result<(), Error> {
        use super::module::{exist, init};

        for module in modules {
            if !exist(&module.symbol) {
                return Err(Error {
                    typ: Box::new(super::error::Type::NonexistentModule(module.symbol.clone())),
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
        self.push_scope(body)?;
        for (_cond, branch_body) in paths {
            self.push_scope(branch_body)?;
        }
        if let Some(alt_body) = alternate {
            self.push_scope(alt_body)?;
        }
        let _ = condition;
        Ok(())
    }

    fn visit_endless(&mut self, body: Fragment, _location: Range) -> Result<(), Error> {
        self.push_scope(body)
    }

    fn visit_while(&mut self, condition: Expression, body: Fragment) -> Result<(), Error> {
        self.push_scope(body)?;
        let _ = condition;
        Ok(())
    }

    fn eval_expr(&mut self, expr: &Expr, location: Range) -> Result<(), Error> {
        match expr {
            Expr::NumericLiteral(_value) => {}
            Expr::Identifier(name) => {
                if self.get_variable_offset(name).is_none() {
                    return Err(Error {
                        typ: Box::new(super::error::Type::NonexistentVar(name.clone())),
                        location,
                    });
                }
            }
            Expr::BinaryExpr {
                left,
                right,
                operator,
            } => {
                self.eval_expr(&left.typ, location)?;
                self.eval_expr(&right.typ, location)?;
                let _ = operator;
            }
            Expr::Assignment { ident, value } => {
                self.eval_expr(&value.typ, location)?;
                if self.get_variable_offset(&ident.symbol).is_none() {
                    return Err(Error {
                        typ: Box::new(super::error::Type::NonexistentVar(ident.symbol.clone())),
                        location,
                    });
                }
            }
            Expr::IAssignment {
                ident,
                value,
                operator,
            } => {
                self.eval_expr(&value.typ, location)?;
                if self.get_variable_offset(&ident.symbol).is_none() {
                    return Err(Error {
                        typ: Box::new(super::error::Type::NonexistentVar(ident.symbol.clone())),
                        location,
                    });
                }
                let _ = operator;
            }
            Expr::Call { args, function } => {
                for arg in args {
                    self.eval_expr(&arg.typ, location)?;
                }
                self.eval_call(function, args)?;
            }
            Expr::EqExpr { .. } => {
                return Err(Error {
                    typ: Box::new(super::error::Type::EqInNormalExpr),
                    location,
                });
            }
            Expr::Debug => {}
            Expr::Member { .. } => {
                return Err(Error {
                    typ: Box::new(super::error::Type::NoConstants),
                    location,
                });
            }
        }
        Ok(())
    }

    fn get_output(&mut self) -> Self::Output {
        let wasm = generate_minimal_wasm();
        W4Output(wasm)
    }

    fn reset(&mut self) {
        drop(mem::take(self));
    }
}

fn generate_minimal_wasm() -> Vec<u8> {
    let mut wasm = Vec::new();

    wasm.extend_from_slice(b"\x00\x61\x73\x6d");
    wasm.push(1);
    wasm.push(0);
    wasm.push(0);
    wasm.push(0);

    wasm.push(0x01);
    wasm.push(0x07);
    wasm.push(0x01);
    wasm.push(0x66);
    wasm.push(0x75);
    wasm.push(0x6e);
    wasm.push(0x63);
    wasm.push(0x74);
    wasm.push(0x69);
    wasm.push(0x6f);
    wasm.push(0x6e);
    wasm.push(0x00);
    wasm.push(0x00);

    wasm.extend_from_slice(&[0x03, 0x02, 0x01, 0x00]);

    wasm.push(0x07);
    wasm.push(0x09);
    wasm.push(0x01);
    wasm.push(0x06);
    wasm.push(0x75);
    wasm.push(0x70);
    wasm.push(0x64);
    wasm.push(0x61);
    wasm.push(0x74);
    wasm.push(0x65);
    wasm.push(0x00);
    wasm.push(0x00);

    wasm.push(0x0a);
    wasm.push(0x09);
    wasm.push(0x01);
    wasm.push(0x07);
    wasm.push(0x00);
    wasm.push(0x41);
    wasm.push(0x00);
    wasm.push(0x0b);

    wasm
}

impl crate::backend::target::Output for W4Output {
    fn repr(&self) -> String {
        format!("WASM module ({} bytes)", self.0.len())
    }

    fn repr_bin(&self) -> Option<String> {
        None
    }

    fn repr_loc(&self) -> Option<String> {
        None
    }
}
