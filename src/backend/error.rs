use crate::frontend::Range;
use std::fmt::{Debug, Display};

#[derive(Debug, PartialEq, Eq)]
pub enum Type {
    NonexistentVar(String),
    NonexistentInlineVar(String),
    TooManyVars,
    ForbiddenInline,
    UnknownModule(String),
    UnknownMethod(String),
    InvalidArgs(String),
    CompileTimeArg(String),
    SomethingElseWentWrong(String),
    ModuleInitTwice(String),
    EqInNormalExpr,
    NormalInEqExpr,
    UseOutsideGlobalScope,
}

#[derive(PartialEq, Eq)]
pub struct Error {
    pub typ: Type,
    pub location: Range,
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_string(f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_string(f)
    }
}

impl Error {
    fn to_string(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}",
            match &self.typ {
                Type::NonexistentVar(name) => {
                    format!("Varialble {name} is not defined")
                }
                Type::NonexistentInlineVar(name) => {
                    format!("Inline variable {name} is not defined")
                }
                Type::TooManyVars => "There are too many variales".to_string(),
                Type::ForbiddenInline => {
                    "This expression cannot be used in an inline expression".to_string()
                }
                Type::UnknownModule(name) => {
                    format!("The module {name} is either not loaded or doesn't exist")
                }
                Type::UnknownMethod(name) => {
                    format!("The method {name} doesn't exist")
                }
                Type::InvalidArgs(args) => {
                    format!("The arguments {args} are invalid")
                }
                Type::SomethingElseWentWrong(e) => format!(
                    "Something else has gone wrong: {e}. Please report this to the developer"
                ),
                Type::ModuleInitTwice(name) => {
                    format!("The module {name} was initialilzed twice")
                }
                Type::EqInNormalExpr => {
                    "You can't use an Equality Expression in a Normal Expression".to_string()
                }
                Type::NormalInEqExpr => "You can't use a normal Expression here".to_string(),
                Type::UseOutsideGlobalScope =>
                    "You can only use 'use' in the global scope".to_string(),
                Type::CompileTimeArg(name) => {
                    format!("{name} has to be known at compile-time")
                }
            },
        ))?;
        Ok(())
    }
}
