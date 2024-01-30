use crate::{
    backend::{Compiler, Error, ModuleCall},
    instr,
};

use super::Res;

pub fn module(compiler: &mut Compiler, call: &ModuleCall) -> Res {
    match call.method_name.as_str() {
        "write" => write(compiler, call),
        "read" => read(compiler, call),
        _ => Err(Error::UnknownMethod(call.method_name.clone())),
    }
}

#[allow(clippy::unwrap_used)]
fn read(compiler: &mut Compiler, call: &ModuleCall) -> Res {
    if call.args.len() != 1 {
        return Err(Error::InvalidArgs(format!("{:?}", call.args)));
    }

    let Some(slot) = compiler.try_get_constant(call.args.first().unwrap())? else {
        return Err(Error::InvalidArgs(
            "Input slot has to be known at compile time".to_string(),
        ));
    };

    if !(0..8).contains(&slot) {
        return Err(Error::InvalidArgs(
            "Input slot has to be from 0 to 7".to_string(),
        ));
    }

    let slot: u8 = slot.try_into().unwrap();

    instr!(compiler, LA, slot + 32);

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn write(compiler: &mut Compiler, call: &ModuleCall) -> Res {
    if call.args.len() != 2 {
        return Err(Error::InvalidArgs(format!("{:?}", call.args)));
    }
    // out slot has to be known at compile-time
    let Some(slot) = compiler.try_get_constant(call.args.last().unwrap())? else {
        return Err(Error::InvalidArgs(
            "Output slot has to be known at compile time".to_string(),
        ));
    };

    if !(0..8).contains(&slot) {
        return Err(Error::InvalidArgs(
            "Output slot has to be from 0 to 7".to_string(),
        ));
    }

    let slot: u8 = slot.try_into().unwrap();

    compiler.eval_expr(call.args.first().unwrap())?;

    instr!(compiler, SVA, slot + 32);

    Ok(())
}
