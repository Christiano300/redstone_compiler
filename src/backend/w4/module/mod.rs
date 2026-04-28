use crate::{
    error::Error,
    frontend::{Expression, Range},
};

use super::{compiler::W4Compiler, error::Type as ErrorType};

pub fn call(name: &str, _compiler: &mut W4Compiler, call: &Call) -> Res {
    err!(Err ErrorType::NonexistentModule(name.to_string()), call.location)
}

pub fn exist(name: &str) -> bool {
    matches!(name, "draw" | "input" | "sound" | "storage")
}

pub fn init(_name: &str, _compiler: &mut W4Compiler, _location: Range) -> Res {
    Ok(())
}

pub struct Call<'a> {
    pub method_name: &'a String,
    pub args: &'a Vec<Expression>,
    pub location: Range,
}

type Res<T = ()> = Result<T, Error>;
