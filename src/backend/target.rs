use std::fmt::Debug;

use vec1::Vec1;

use crate::{
    Error,
    frontend::{Expr, Expression, Fragment, Ident, Range, Statement, Stmt},
};

#[allow(clippy::missing_errors_doc)] // All the same compiler error
pub trait Target {
    type Output: Output;

    fn visit_inline_decl(&mut self, ident: Ident, value: Expression) -> Result<(), Error>;

    fn visit_var_decl(&mut self, ident: Ident) -> Result<(), Error>;

    fn visit_use(&mut self, modules: Vec1<Ident>, location: Range) -> Result<(), Error>;

    fn visit_conditional(
        &mut self,
        condition: Expression,
        body: Fragment,
        paths: Vec<(Expression, Fragment)>,
        alternate: Option<Fragment>,
    ) -> Result<(), Error>;

    fn visit_endless(&mut self, body: Fragment, location: Range) -> Result<(), Error>;

    fn visit_while(&mut self, condition: Expression, body: Fragment) -> Result<(), Error>;

    fn visit_pass(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn eval_expression(&mut self, expr: &Expression) -> Result<(), Error> {
        self.eval_expr(&expr.typ, expr.location)
    }

    fn eval_expr(&mut self, expr: &Expr, location: Range) -> Result<(), Error>;

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
            Stmt::Pass => self.visit_pass(),
        }
    }

    fn get_output(&mut self) -> Self::Output;

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

    fn reset(&mut self) {}
}

pub trait Output: Debug {
    fn repr(&self) -> String;

    fn repr_bin(&self) -> Option<String>;

    fn repr_loc(&self) -> Option<String>;
}
