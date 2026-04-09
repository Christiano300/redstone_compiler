use super::super::error::Type as ErrorType;
use super::super::W4Compiler;
use super::{Call, Res};
use crate::error::Error;

pub fn module(compiler: &mut W4Compiler, call: &Call) -> Res {
    match call.method_name.as_str() {
        "gamepad" => gamepad(compiler, call),
        "mouse" => mouse(compiler, call),
        _ => Err(Error {
            typ: Box::new(ErrorType::UnknownMethod(call.method_name.clone())),
            location: call.location,
        }),
    }
}

fn gamepad(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 1 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "gamepad expects 1 argument: pad (1-4)".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}

fn mouse(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 1 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "mouse expects 1 argument: type (\"x\", \"y\", \"buttons\")".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}
