/*
ram.read(where) # return
ram.write(what, where) # also return
ram.copy(from, to) # also return

ram.use_list()
ram.add_to_list(what) # also return
ram.pop_list() # return
*/

use crate::{
    backend::compiler::{Compiler, Error, ModuleCall, RamPage},
    frontend::Expression,
    instr,
};

use super::{arg_parse, Arg, Res};

pub fn module(compiler: &mut Compiler, call: &ModuleCall) -> Res {
    match call.method_name.as_str() {
        "read" => ram_read(compiler, call),
        "write" => ram_write(compiler, call),
        "copy" => ram_copy(compiler, call),
        _ => Err(Error::UnknownMethod(call.method_name.clone())),
    }
}

fn ram_copy(compiler: &mut Compiler, call: &ModuleCall) -> Res {
    let [from, to] = arg_parse(
        compiler,
        [Arg::Number("from"), Arg::Number("to")],
        call.args,
    )?;
    put_address(compiler, from)?;
    instr!(compiler, RR);
    if Compiler::can_put_into_b(to) {
        put_address(compiler, to)?;
    } else {
        let temp = compiler.insert_temp_var()?;
        instr!(compiler, SVA, temp);
        put_address(compiler, to)?;
        instr!(compiler, LA, temp);
    }
    instr!(compiler, RW);
    Ok(())
}

fn ram_write(compiler: &mut Compiler, call: &ModuleCall) -> Res {
    let [value, address] = arg_parse(
        compiler,
        [Arg::Number("value"), Arg::Number("address")],
        call.args,
    )?;

    match (
        Compiler::can_put_into_a(value),
        Compiler::can_put_into_b(address),
    ) {
        (true, _) => {
            put_address(compiler, address)?;
            compiler.put_into_a(value)?;
        }
        (false, true) => {
            compiler.eval_expr(value)?;
            put_address(compiler, address)?;
        }
        (false, false) => {
            compiler.eval_expr(value)?;
            if let Expression::Assignment { symbol, value: _ } = value {
                put_address(compiler, address)?;
                instr!(compiler, LA, compiler.get_var(symbol)?);
            } else {
                let temp = compiler.insert_temp_var()?;
                instr!(compiler, SVA, temp);
                put_address(compiler, address)?;
                instr!(compiler, LA, temp);
                compiler.cleanup_temp_var(temp);
            }
        }
    }
    instr!(compiler, RW);
    Ok(())
}

fn ram_read(compiler: &mut Compiler, call: &ModuleCall) -> Res {
    let address = arg_parse(compiler, [Arg::Number("address")], call.args)?[0];
    put_address(compiler, address)?;

    instr!(compiler, RR);
    Ok(())
}

/// puts the address in the B register and calls RC if neccessary
fn put_address(compiler: &mut Compiler, address: &Expression) -> Res {
    if let Some(value) = compiler.try_get_constant(address)? {
        if compiler.last_scope().state.ram_page != RamPage::ThisOne((value / 16) as u8) {
            instr!(compiler, RC);
        }
        compiler.put_b_number(value);
    } else {
        instr!(compiler, RC);
        if Compiler::can_put_into_b(address) {
            compiler.put_into_b(address)?;
        } else if Compiler::can_put_into_a(address) {
            // if can_put_into_b is false and
            // can_put_into_a is true is must be an assigmnent
            compiler.put_into_a(address)?;
            if let Expression::Assignment { symbol, value: _ } = address {
                instr!(compiler, LB, compiler.get_var(symbol)?);
            }
        } else {
            compiler.eval_expr(address)?;
            compiler.switch()?;
        }
    }
    Ok(())
}

fn find_pointer_var_slot(slots: &[bool; 32]) -> Res<usize> {
    slots
        .iter()
        .rev()
        .position(|slot| !*slot)
        .ok_or(Error::TooManyVars)
}
