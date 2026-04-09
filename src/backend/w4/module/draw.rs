use super::super::error::Type as ErrorType;
use super::super::W4Compiler;
use super::{Call, Res};
use crate::error::Error;

pub fn module(compiler: &mut W4Compiler, call: &Call) -> Res {
    match call.method_name.as_str() {
        "rect" => rect(compiler, call),
        "line" => line(compiler, call),
        "oval" => oval(compiler, call),
        "text" => text(compiler, call),
        "blit" => blit(compiler, call),
        "hline" => hline(compiler, call),
        "vline" => vline(compiler, call),
        _ => Err(Error {
            typ: Box::new(ErrorType::UnknownMethod(call.method_name.clone())),
            location: call.location,
        }),
    }
}

fn rect(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 4 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "rect expects 4 arguments: x, y, width, height".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}

fn line(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 4 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "line expects 4 arguments: x1, y1, x2, y2".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}

fn oval(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 4 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "oval expects 4 arguments: x, y, width, height".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}

fn text(compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 3 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "text expects 3 arguments: string, x, y".to_string(),
            )),
            location: call.location,
        });
    }
    let _ = compiler.add_string("placeholder");
    Ok(())
}

fn blit(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 6 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "blit expects 6 arguments: sprite, x, y, width, height, flags".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}

fn hline(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 3 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "hline expects 3 arguments: x, y, len".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}

fn vline(_compiler: &mut W4Compiler, call: &Call) -> Res {
    if call.args.len() != 3 {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "vline expects 3 arguments: x, y, len".to_string(),
            )),
            location: call.location,
        });
    }
    Ok(())
}
