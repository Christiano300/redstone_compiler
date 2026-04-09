mod draw;
mod input;
mod sound;
mod storage;

use crate::{
    error::Error,
    frontend::{Expression, Range},
};

use super::compiler::W4Compiler;

pub fn call(name: &str, compiler: &mut W4Compiler, call: &Call) -> Res {
    match name {
        "draw" => draw::module(compiler, call),
        "input" => input::module(compiler, call),
        "sound" => sound::module(compiler, call),
        "storage" => storage::module(compiler, call),
        _ => Err(Error {
            typ: Box::new(super::error::Type::NonexistentModule(
                call.method_name.clone(),
            )),
            location: call.location,
        }),
    }
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
