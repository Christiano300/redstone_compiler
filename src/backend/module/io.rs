use crate::{
    backend::{
        module::{arg_parse, Arg},
        Compiler,
    },
    err,
    error::Error,
    instr, modul,
};

use super::{Call, ErrorType, Res};

modul!(read write);

fn read(compiler: &mut Compiler, call: &Call) -> Res {
    let args = arg_parse(compiler, [Arg::Constant("Inslot")], call)?;

    let slot = compiler.try_get_constant(args[0]).unwrap();
    if !(0..8).contains(&slot) {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "Input slot has to be from 0 to 7".to_string(),
            )),
            location: call.args.first().unwrap().location,
        });
    }

    let slot: u8 = slot.try_into().unwrap_or(0);

    instr!(compiler, LA, slot + 32);

    Ok(())
}

fn write(compiler: &mut Compiler, call: &Call) -> Res {
    let args = arg_parse(
        compiler,
        [Arg::Number("value"), Arg::Constant("Outslot")],
        call,
    )?;

    let slot = compiler.try_get_constant(args[1]).unwrap();
    if !(0..8).contains(&slot) {
        return Err(Error {
            typ: Box::new(ErrorType::InvalidArgs(
                "Output slot has to be from 0 to 7".to_string(),
            )),
            location: call.args.get(1).unwrap().location,
        });
    }

    let slot: u8 = slot.try_into().unwrap_or(0);

    compiler.eval_expr(&call.args[0])?;

    instr!(compiler, SVA, slot + 32);

    Ok(())
}
