use std::borrow::Cow;

use crate::error::ErrorType;

#[derive(Debug, PartialEq, Eq)]
pub enum Type {
    NonexistentVar(String),
    NonexistentInlineVar(String),
    TooManyVars,
    ForbiddenInline,
    NonexistentModule(String),
    UnlodadedModule(String),
    UnknownMethod(String),
    InvalidArgs(String),
    CompileTimeArg(String),
    SomethingElseWentWrong(String),
    ModuleInitTwice(String),
    NumberTooBig,
    EqInNormalExpr,
    NormalInEqExpr,
    UseOutsideGlobalScope,
    NoConstants,
    NoFunctions,
    DataString,
}

impl ErrorType for Type {
    fn get_message(&self) -> Cow<'_, str> {
        match &self {
            Self::NonexistentVar(name) => Cow::from(format!("Varialble {name} is not defined")),
            Self::NonexistentInlineVar(name) => {
                Cow::from(format!("Inline variable {name} is not defined"))
            }
            Self::TooManyVars => Cow::from("There are too many variales"),
            Self::ForbiddenInline => {
                Cow::from("This expression cannot be used in an inline expression")
            }
            Self::NonexistentModule(name) => Cow::from(format!("The module {name} doesn't exist")),
            Self::UnlodadedModule(name) => Cow::from(format!("The module {name} is not loaded")),
            Self::UnknownMethod(name) => Cow::from(format!("The method {name} doesn't exist")),
            Self::InvalidArgs(args) => Cow::from(format!("The arguments {args} are invalid")),
            Self::SomethingElseWentWrong(e) => Cow::from(format!(
                "Something else has gone wrong: {e}. Please report this to the developer"
            )),
            Self::ModuleInitTwice(name) => {
                Cow::from(format!("The module {name} was initialilzed twice"))
            }
            Self::EqInNormalExpr => {
                Cow::from("You can't use an Equality Expression in a Normal Expression")
            }
            Self::NumberTooBig => {
                Cow::from("Value is too large for MCN-16 target, only i16 is supported")
            }
            Self::NormalInEqExpr => Cow::from("You can't use a normal Expression here"),
            Self::UseOutsideGlobalScope => Cow::from("You can only use 'use' in the global scope"),
            Self::CompileTimeArg(name) => {
                Cow::from(format!("{name} has to be known at compile-time"))
            }
            Self::NoConstants => Cow::from("Constants are only supported inside module calls"),
            Self::NoFunctions => Cow::from("Functions are not supported on the MCN-16 target"),
            Self::DataString => Cow::from("Data strings are not supported on the MCN-16 target"),
        }
    }
}
