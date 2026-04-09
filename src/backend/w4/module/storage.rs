use super::super::error::Type as ErrorType;
use super::super::W4Compiler;
use super::{Call, Res};
use crate::error::Error;

pub fn module(compiler: &mut W4Compiler, call: &Call) -> Res {
    match call.method_name.as_str() {
        "diskr" => diskr(compiler, call),
        "diskw" => diskw(compiler, call),
        _ => Err(Error {
            typ: Box::new(ErrorType::UnknownMethod(call.method_name.clone())),
            location: call.location,
        }),
    }
}

fn diskr(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 2 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "diskr expects 2 arguments: dest, size".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}

fn diskw(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 2 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "diskw expects 2 arguments: src, size".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}
