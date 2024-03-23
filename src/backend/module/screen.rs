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
            typ: Box::new(ErrorType::UnknownMethod(call.method_name.clone())),
            location: call.location,
        }),
    }
}

fn pixel_operation(compiler: &mut Compiler, call: &Call, op: u8) -> Res {
    let args = arg_parse(compiler, [Arg::Number("x"), Arg::Number("y")], call)?;

    write_screenpos(compiler, args[0], args[1], call.location)?;
    write_screenop(compiler, op);
    Ok(())
}

fn screen_operation(compiler: &mut Compiler, call: &Call, op: u8) -> Res {
    let _ = arg_parse(compiler, [], call)?;
    write_screenop(compiler, op);
    Ok(())
}

fn whole_pixel_operation(compiler: &mut Compiler, call: &Call, op: u8) -> Res {
    let args = arg_parse(compiler, [Arg::Number("pos")], call)?;

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
    put_xy(compiler, x, y, location, 8)?;
    instr!(compiler, SVA, SCREENPOS_REG);
    Ok(())
}

pub fn put_xy(
    compiler: &mut Compiler,
    upper: &Expression,
    lower: &Expression,
    location: Range,
    offset: u8,
) -> Res {
    match (
        compiler.try_get_constant(upper),
        compiler.try_get_constant(lower),
    ) {
        (Some(upper), Some(lower)) => {
            compiler.put_a_number(upper << offset | lower);
        }
        (Some(upper), None) => {
            compiler.eval_expr(lower)?;
            compiler.put_b_number(upper << offset);
            instr!(compiler, OR);
        }
        (None, Some(lower)) => {
            compiler.eval_expr(upper)?;
            instr!(compiler, SUP, offset);
            compiler.put_b_number(lower);
            instr!(compiler, OR);
        }
        (None, None) => {
            let simple = Compiler::can_put_into_b(lower);
            if simple {
                compiler.eval_expr(upper)?;
                instr!(compiler, SUP, offset);
                compiler.put_into_b(lower)?;
            } else {
                let temp = compiler.insert_temp_var(location)?;
                compiler.eval_expr(lower)?;
                instr!(compiler, SVA, temp);
                compiler.eval_expr(upper)?;
                instr!(compiler, SUP, offset);
                instr!(compiler, LB, temp);
                compiler.cleanup_temp_var(temp);
            }
            instr!(compiler, OR);
        }
    }
    Ok(())
}

fn write_screenop(compiler: &mut Compiler, op: u8) {
    instr!(compiler, LAL, op);
    instr!(compiler, SVA, SCREENOP_REG);
}
