use crate::{
    backend::{
        module::{arg_parse, Arg},
        Compiler, Error, ModuleCall,
    },
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

fn read(compiler: &mut Compiler, call: &ModuleCall) -> Res {
    let args = arg_parse(compiler, [Arg::Constant("Inslot")], call.args)?;

    #[allow(clippy::unwrap_used)]
    let slot = compiler.try_get_constant(args[0])?.unwrap();
    if !(0..8).contains(&slot) {
        return Err(Error::InvalidArgs(
            "Input slot has to be from 0 to 7".to_string(),
        ));
    }

    let slot: u8 = slot.try_into().unwrap_or(0);

    instr!(compiler, LA, slot + 32);

    Ok(())
}

fn write(compiler: &mut Compiler, call: &ModuleCall) -> Res {
    let args = arg_parse(
        compiler,
        [Arg::Number("value"), Arg::Constant("Outslot")],
        call.args,
    )?;

    #[allow(clippy::unwrap_used)]
    let slot = compiler.try_get_constant(args[1])?.unwrap();
    if !(0..8).contains(&slot) {
        return Err(Error::InvalidArgs(
            "Output slot has to be from 0 to 7".to_string(),
        ));
    }

    let slot: u8 = slot.try_into().unwrap_or(0);

    compiler.eval_expr(&call.args[0])?;

    instr!(compiler, SVA, slot + 32);

    Ok(())
}
