use crate::{
    backend::{Compiler, CompilerError, ModuleCall},
    instr,
};

use super::Res;

pub fn io_module(compiler: &mut Compiler, call: ModuleCall) -> Res {
    match call.method_name.as_str() {
        "write" => write(compiler, call),
        "read" => read(compiler, call),
        _ => Err(CompilerError::UnknownMethod(call.method_name.clone())),
    }
}

fn read(compiler: &mut Compiler, call: ModuleCall) -> Res {
    if call.args.len() != 1 {
        return Err(CompilerError::InvalidArgs(format!("{:?}", call.args)));
    }

    let slot = match compiler.try_get_constant(call.args.first().unwrap()) {
        Some(value) => value,
        None => {
            return Err(CompilerError::InvalidArgs(
                "Input slot has to be known at compile time".to_string(),
            ))
        }
    };

    if !(0..8).contains(&slot) {
        return Err(CompilerError::InvalidArgs(
            "Input slot has to be from 0 to 7".to_string(),
        ));
    }

    let slot: u8 = slot.try_into().unwrap();

    instr!(compiler, LA, slot + 32);

    Ok(())
}

fn write(compiler: &mut Compiler, call: ModuleCall) -> Res {
    if call.args.len() != 2 {
        return Err(CompilerError::InvalidArgs(format!("{:?}", call.args)));
    }
    // out slot has to be known at compile-time
    let slot = match compiler.try_get_constant(call.args.last().unwrap()) {
        Some(value) => value,
        None => {
            return Err(CompilerError::InvalidArgs(
                "Output slot has to be known at compile time".to_string(),
            ))
        }
    };

    if !(0..8).contains(&slot) {
        return Err(CompilerError::InvalidArgs(
            "Output slot has to be from 0 to 7".to_string(),
        ));
    }

    let slot: u8 = slot.try_into().unwrap();

    compiler.eval_expr(call.args.first().unwrap())?;

    instr!(compiler, SVA, slot + 32);

    Ok(())
}
