use crate::error::ErrorType;

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

impl ErrorType for Type {
    fn get_message(&self) -> String {
        match &self {
            Self::NonexistentVar(name) => {
                format!("Varialble {name} is not defined")
            }
            Self::NonexistentInlineVar(name) => {
                format!("Inline variable {name} is not defined")
            }
            Self::TooManyVars => "There are too many variales".to_string(),
            Self::ForbiddenInline => {
                "This expression cannot be used in an inline expression".to_string()
            }
            Self::UnknownModule(name) => {
                format!("The module {name} is either not loaded or doesn't exist")
            }
            Self::UnknownMethod(name) => {
                format!("The method {name} doesn't exist")
            }
            Self::InvalidArgs(args) => {
                format!("The arguments {args} are invalid")
            }
            Self::SomethingElseWentWrong(e) => {
                format!("Something else has gone wrong: {e}. Please report this to the developer")
            }
            Self::ModuleInitTwice(name) => {
                format!("The module {name} was initialilzed twice")
            }
            Self::EqInNormalExpr => {
                "You can't use an Equality Expression in a Normal Expression".to_string()
            }
            Self::NormalInEqExpr => "You can't use a normal Expression here".to_string(),
            Self::UseOutsideGlobalScope => "You can only use 'use' in the global scope".to_string(),
            Self::CompileTimeArg(name) => {
                format!("{name} has to be known at compile-time")
            }
        }
    }
}
