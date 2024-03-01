use crate::{
    backend::compiler::Compiler,
    frontend::{Expression, Range},
    instr,
};

use super::{arg_parse, Arg, Call, Error, ErrorType, Res};

/*
Screen:
Position: [7] 0b**XXXXXX_**YYYYYY (only required on on, invert and off)
Operation: [6] 1 | 2 | 4 | 8 | 16
    1: flip
    2: clear buffer
    4: on
    8: invert
    16: off
*/

const BASE_OUT_REG: u8 = 32;
const SCREENOP_REG: u8 = BASE_OUT_REG + 6;
const SCREENPOS_REG: u8 = BASE_OUT_REG + 7;

pub fn module(compiler: &mut Compiler, call: &Call) -> Res {
    match call.method_name.as_str() {
        "flip" => screen_operation(compiler, call, 1),
        "clear" => screen_operation(compiler, call, 2),
        "set_at" => pixel_operation(compiler, call, 4),
        "invert_at" => pixel_operation(compiler, call, 8),
        "off_at" => pixel_operation(compiler, call, 16),
        "set" => whole_pixel_operation(compiler, call, 4),
        "invert" => whole_pixel_operation(compiler, call, 8),
        "off" => whole_pixel_operation(compiler, call, 16),
        _ => Err(Error {
            typ: ErrorType::UnknownMethod(call.method_name.clone()),
            location: call.location,
        }),
    }
}

fn pixel_operation(compiler: &mut Compiler, call: &Call, op: u8) -> Res {
    let args = arg_parse(
        compiler,
        [Arg::Number("x"), Arg::Number("y")],
        call.args,
        call.location,
    )?;

    write_screenpos(compiler, args[0], args[1], call.location)?;
    write_screenop(compiler, op);
    Ok(())
}

fn screen_operation(compiler: &mut Compiler, call: &Call, op: u8) -> Res {
    let _ = arg_parse(compiler, [], call.args, call.location)?;
    write_screenop(compiler, op);
    Ok(())
}

fn whole_pixel_operation(compiler: &mut Compiler, call: &Call, op: u8) -> Res {
    let args = arg_parse(compiler, [Arg::Number("pos")], call.args, call.location)?;

    compiler.eval_expr(args[0])?;
    instr!(compiler, SVA, SCREENPOS_REG);
    write_screenop(compiler, op);

    Ok(())
}

fn write_screenpos(
    compiler: &mut Compiler,
    x: &Expression,
    y: &Expression,
    location: Range,
) -> Res {
    match (compiler.try_get_constant(x)?, compiler.try_get_constant(y)?) {
        (Some(x), Some(y)) => {
            compiler.put_a_number(x << 8 | y);
        }
        (Some(x), None) => {
            compiler.eval_expr(y)?;
            compiler.put_b_number(x << 8);
            instr!(compiler, OR);
        }
        (None, Some(y)) => {
            compiler.eval_expr(x)?;
            instr!(compiler, SUP, 8);
            compiler.put_b_number(y);
            instr!(compiler, OR);
        }
        (None, None) => {
            let simple = Compiler::can_put_into_b(y);
            compiler.eval_expr(y)?;
            if simple {
                compiler.put_into_b(y)?;

                compiler.eval_expr(x)?;
                instr!(compiler, SUP, 8);
            } else {
                let temp = compiler.insert_temp_var(location)?;
                instr!(compiler, SVA, temp);
                compiler.eval_expr(x)?;
                instr!(compiler, SUP, 8);
                compiler.cleanup_temp_var(temp);
                instr!(compiler, LB, temp);
            }
            instr!(compiler, OR);
        }
    }
    instr!(compiler, SVA, SCREENPOS_REG);
    Ok(())
}

fn write_screenop(compiler: &mut Compiler, op: u8) {
    instr!(compiler, LAL, op);
    instr!(compiler, SVA, SCREENOP_REG);
}
