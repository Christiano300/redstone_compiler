mod colorscreen;
mod io;
mod list;
mod ram;
mod screen;

use crate::{
    err,
    error::Error,
    frontend::{Expression, Range},
};

use super::{Compiler, ErrorType};

pub fn call(name: &str, compiler: &mut Compiler, call: &Call) -> Res {
    match name {
        "io" => io::module(compiler, call),
        "screen" => screen::module(compiler, call),
        "ram" => ram::module(compiler, call),
        "list" => list::module(compiler, call),
        "colorscreen" => colorscreen::module(compiler, call),
        _ => Err(Error {
            typ: Box::new(ErrorType::NonexistentModule(call.method_name.clone())),
            location: call.location,
        }),
    }
}

pub fn exist(name: &str) -> bool {
    matches!(name, "io" | "screen" | "ram" | "list" | "colorscreen")
}

pub fn init(name: &str, compiler: &mut Compiler, location: Range) -> Res {
    match name {
        "list" => list::init(compiler, location),
        _ => Ok(()),
    }
}

pub struct Call<'a> {
    pub method_name: &'a String,
    pub args: &'a Vec<Expression>,
    pub location: Range,
}

enum Arg {
    Number(&'static str),
    Constant(&'static str),
}

fn arg_parse<'a, const COUNT: usize>(
    compiler: &mut Compiler,
    types: [Arg; COUNT],
    call: &'a Call,
) -> Res<[&'a Expression; COUNT]> {
    if types.len() != call.args.len() {
        return err!(
            ErrorType::InvalidArgs("Wrong number of Arguments".to_string()),
            call.location
        );
    }
    types
        .into_iter()
        .zip(call.args.iter())
        .try_for_each(|(typ, arg)| match typ {
            Arg::Constant(name) => match compiler.try_get_constant(arg) {
                Some(_) => Ok(()), // if we can get the value at compile-time, its ok
                None => Err(Error {
                    typ: Box::new(ErrorType::CompileTimeArg(name.to_string())),
                    location: arg.location,
                }), // otherwise we error
            },
            Arg::Number(..) => Ok(()),
        })?;

    let mut iter = call.args.iter();
    let res = [(); COUNT].map(|_res| iter.next().unwrap());
    assert_eq!(res.len(), COUNT);
    Ok(res)
}

type Res<T = ()> = Result<T, Error>;
