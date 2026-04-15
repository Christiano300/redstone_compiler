use std::borrow::Cow;

use crate::error::ErrorType;

#[derive(Debug, PartialEq, Eq)]
pub enum Type {
    NonexistentVar(String),
    NonexistentInlineVar(String),
    UnknownMethod(String),
    NonexistentModule(String),
    UnlodadedModule(String),
    InvalidArgs(String),
    DuplicateFunction(String),
    UnknownVariable(String),
}

impl ErrorType for Type {
    fn get_message(&self) -> Cow<'_, str> {
        match &self {
            Self::NonexistentVar(name) => Cow::from(format!("Variable {name} is not defined")),
            Self::NonexistentInlineVar(name) => {
                Cow::from(format!("Inline variable {name} is not defined"))
            }
            Self::UnknownMethod(name) => Cow::from(format!("The method {name} doesn't exist")),
            Self::NonexistentModule(name) => Cow::from(format!("The module {name} doesn't exist")),
            Self::UnlodadedModule(name) => Cow::from(format!("The module {name} is not loaded")),
            Self::InvalidArgs(msg) => Cow::from(msg),
            Self::DuplicateFunction(name) => {
                Cow::from(format!("The function {name} is already defined"))
            }
            Self::UnknownVariable(name) => Cow::from(format!("The variable {name} is not defined")),
        }
    }
}
