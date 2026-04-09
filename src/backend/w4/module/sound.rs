use super::super::error::Type as ErrorType;
use super::super::W4Compiler;
use super::{Call, Res};
use crate::error::Error;

pub fn module(compiler: &mut W4Compiler, call: &Call) -> Res {
    match call.method_name.as_str() {
        "tone" => tone(compiler, call),
        _ => Err(Error {
            typ: Box::new(ErrorType::UnknownMethod(call.method_name.clone())),
            location: call.location,
        }),
    }
}

fn tone(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 4 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "tone expects 4 arguments: frequency, duration, volume, flags".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}
