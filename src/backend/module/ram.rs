/*
ram.read(where) # return
ram.write(what, where) # also return
ram.copy(from, to) # also return
*/

use crate::{
    backend::{compiler::Compiler, RamPage},
    err,
    frontend::{Expression, ExpressionType, Range},
    instr, modul,
};

use super::{arg_parse, Arg, Call, ErrorType, Res};

modul!(read write copy);

fn copy(compiler: &mut Compiler, call: &Call) -> Res {
    let [from, to] = arg_parse(compiler, [Arg::Number("from"), Arg::Number("to")], call)?;
    put_address(compiler, from, call.location)?;
    instr!(compiler, RR, call.location);
    if Compiler::can_put_into_b(to) {
        put_address(compiler, to, call.location)?;
    } else {
        let temp = compiler.insert_temp_var(call.location)?;
        instr!(compiler, SVA, temp, call.location);
        put_address(compiler, to, call.location)?;
        instr!(compiler, LA, temp, call.location);
    }
    instr!(compiler, RW, call.location);
    Ok(())
}

fn write(compiler: &mut Compiler, call: &Call) -> Res {
    let [value, address] = arg_parse(
        compiler,
        [Arg::Number("value"), Arg::Number("address")],
        call,
    )?;

    match (
        Compiler::can_put_into_a(value),
        Compiler::can_put_into_b(address),
    ) {
        (true, _) => {
            put_address(compiler, address, call.location)?;
            compiler.put_into_a(value)?;
        }
        (false, true) => {
            compiler.eval_expr(value)?;
            put_address(compiler, address, call.location)?;
        }
        (false, false) => {
            compiler.eval_expr(value)?;
            if let ExpressionType::Assignment { ident, value: _ } = &value.typ {
                put_address(compiler, address, call.location)?;
                instr!(
                    compiler,
                    LA,
                    compiler.get_var(&ident.symbol, call.location)?,
                    call.location
                );
            } else {
                let temp = compiler.insert_temp_var(call.location)?;
                instr!(compiler, SVA, temp, call.location);
                put_address(compiler, address, call.location)?;
                instr!(compiler, LA, temp, call.location);
                compiler.cleanup_temp_var(temp);
            }
        }
    }
    instr!(compiler, RW, call.location);
    Ok(())
}

fn read(compiler: &mut Compiler, call: &Call) -> Res {
    let address = arg_parse(compiler, [Arg::Number("address")], call)?[0];
    put_address(compiler, address, call.location)?;

    instr!(compiler, RR, call.location);
    Ok(())
}

/// puts the address in the B register and calls RC if neccessary
fn put_address(compiler: &mut Compiler, address: &Expression, location: Range) -> Res {
    if let Some(value) = compiler.try_get_constant(address) {
        if compiler.last_scope().state.ram_page != RamPage::ThisOne((value / 16) as u8) {
            instr!(compiler, RC, location);
        }
        compiler.put_b_number(value, location);
    } else {
        instr!(compiler, RC, location);
        if Compiler::can_put_into_b(address) {
            compiler.put_into_b(address)?;
        } else if Compiler::can_put_into_a(address) {
            // if can_put_into_b is false and
            // can_put_into_a is true is must be an assigmnent
            compiler.put_into_a(address)?;
            if let ExpressionType::Assignment { ident, value: _ } = &address.typ {
                instr!(
                    compiler,
                    LB,
                    compiler.get_var(&ident.symbol, location)?,
                    address.location
                );
            }
        } else {
            compiler.eval_expr(address)?;
            compiler.switch(location)?;
        }
    }
    Ok(())
}
