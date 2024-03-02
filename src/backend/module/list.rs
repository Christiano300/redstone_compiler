/*
list.add(what) # also return new length of list
list.pop() # return
list.get_pointer() # get after last element
list.set_pointer()
list.last() # return
list.at(where) # return
*/

const INIT: &str = "list_init";
const POINTER: &str = "list_ptr";

use crate::{
    backend::{compiler::Compiler, RamPage, RegisterContents},
    frontend::{ExpressionType, Range},
    instr,
};

use super::{arg_parse, Arg, Call, Error, ErrorType, Res};

pub fn init(compiler: &mut Compiler, location: Range) -> Res {
    if is_initialized(compiler) {
        return Err(Error {
            typ: Box::new(ErrorType::ModuleInitTwice("list".to_string())),
            location,
        });
    }

    let slot: u8 = find_pointer_var_slot(&compiler.variables, location)?
        .try_into()
        .unwrap();
    compiler.module_state.insert(POINTER, Box::from(slot));
    compiler.module_state.insert(INIT, Box::from(true));
    Ok(())
}

pub fn module(compiler: &mut Compiler, call: &Call) -> Res {
    match call.method_name.as_str() {
        "add" => add(compiler, call),
        "pop" => pop(compiler, call),
        "get_pointer" => get_pointer(compiler, call),
        "set_pointer" => set_pointer(compiler, call),
        "last" => last(compiler, call),
        "at" => at(compiler, call),
        _ => Err(Error {
            typ: Box::new(ErrorType::UnknownMethod(call.method_name.clone())),
            location: call.location,
        }),
    }
}

fn add(compiler: &mut Compiler, call: &Call) -> Res {
    let value = arg_parse(compiler, [Arg::Number("value")], call.args, call.location)?[0];
    let pointer = *compiler.get_module_state::<u8>(POINTER).unwrap();
    compiler.eval_expr(value)?;
    if compiler.last_scope().state.b != RegisterContents::Variable(pointer) {
        instr!(compiler, LB, pointer);
    }
    instr!(compiler, RC);
    instr!(compiler, RW);
    instr!(compiler, LAL, 1);
    instr!(compiler, ADD);
    instr!(compiler, SVA, pointer);
    Ok(())
}

fn pop(compiler: &mut Compiler, call: &Call) -> Res {
    arg_parse(compiler, [], call.args, call.location)?;

    let pointer = *compiler.get_module_state::<u8>(POINTER).unwrap();

    if compiler.last_scope().state.a != RegisterContents::Variable(pointer) {
        instr!(compiler, LA, pointer);
    }
    instr!(compiler, LBL, 1);
    instr!(compiler, SUB);
    instr!(compiler, SVA, pointer);
    instr!(compiler, RC);
    instr!(compiler, RR);

    Ok(())
}

fn get_pointer(compiler: &mut Compiler, call: &Call) -> Res {
    arg_parse(compiler, [], call.args, call.location)?;
    let pointer = *compiler.get_module_state(POINTER).unwrap();
    instr!(compiler, LA, pointer);
    Ok(())
}

fn set_pointer(compiler: &mut Compiler, call: &Call) -> Res {
    let value = arg_parse(compiler, [Arg::Number("value")], call.args, call.location)?[0];

    let pointer = *compiler.get_module_state(POINTER).unwrap();
    compiler.eval_expr(value)?;

    instr!(compiler, SVA, pointer);
    Ok(())
}

fn last(compiler: &mut Compiler, call: &Call) -> Res {
    arg_parse(compiler, [], call.args, call.location)?;

    let pointer = *compiler.get_module_state(POINTER).unwrap();

    if compiler.last_scope().state.a != RegisterContents::Variable(pointer) {
        instr!(compiler, LA, pointer);
    }
    instr!(compiler, LBL, 1);
    instr!(compiler, SUB);
    instr!(compiler, RC);
    instr!(compiler, RR);

    Ok(())
}

fn at(compiler: &mut Compiler, call: &Call) -> Res {
    let address = arg_parse(compiler, [Arg::Number("address")], call.args, call.location)?[0];
    let location = call.args.first().unwrap().location;
    if Compiler::can_put_into_b(address) {
        compiler.put_into_b(address)?;
    } else {
        compiler.eval_expr(address)?;
        if let ExpressionType::Assignment { symbol, value: _ } = &address.typ {
            instr!(compiler, LB, compiler.get_var(symbol, location)?);
        } else {
            compiler.switch(location)?;
        }
    }

    match compiler.try_get_constant(address) {
        Some(value)
            if compiler.last_scope().state.ram_page == RamPage::ThisOne((value / 16) as u8) => {}
        _ => instr!(compiler, RC),
    }

    instr!(compiler, RR);

    Ok(())
}

#[inline]
fn is_initialized(compiler: &mut Compiler) -> bool {
    matches!(compiler.get_module_state(INIT), Some(true))
}

fn find_pointer_var_slot(slots: &[bool; 32], location: Range) -> Res<usize> {
    slots.iter().rev().position(|slot| !*slot).ok_or(Error {
        typ: Box::new(ErrorType::TooManyVars),
        location,
    })
}
