/*
Position 1 [7]: 0bCCCC_XXXX XXYY_YYYY
Position 2 [6]: 0b****_XXXX XXYY_YYYY
Operation [5]: 1 | 2
    1: Set (can make rectangle with different positions)
    2: Refresh

Colors:
0: White
1: Orange
2: Magenta
3: Light Blue
4: Yellow
5: Lime
6: Pink
7: Gray
8: Light Gray
9: Cyan
10: Purple
11: Blue
12: Brown
13: Green
14: Red
15: Black
*/

/*
colorscreen.set(pos, color)
colorscreen.set_at(x, y, color)
colorscreen.flip()
colorscreen.fill(from, to, color)
colorscreen.fill_xy(x1, y1, x2, y2, color)
colorscreen.fill_screen(color)
colorscreen.orange = 0x1000
colorscreen.color_of(color_idx)
*/

const SCREENOP_REG: u8 = 5;
const SCREENPOS1_REG: u8 = 7;
const SCREENPOS2_REG: u8 = 6;

const PAINT: i16 = 1;
const FLIP: i16 = 2;

use std::num::NonZeroI16;

use crate::{
    backend::compiler::Compiler,
    err,
    frontend::{Expression, ExpressionType},
    instr, modul,
};

use super::{arg_parse, screen::put_xy, Arg, Call, ErrorType, Res};

modul!(set set_at fill fill_xy fill_screen flip color_of);

fn fill_screen(compiler: &mut Compiler, call: &Call) -> Res {
    let [color] = arg_parse(compiler, [Arg::Number("color")], call)?;
    match is_const_color(color) {
        Some(color) => compiler.put_a_number(color.into(), call.location),
        None => compiler.eval_expr(color)?,
    }
    compiler.save_to_out(SCREENPOS1_REG, call.location);
    compiler.put_a_number(0x0FFF, call.location);
    compiler.save_to_out(SCREENPOS2_REG, call.location);
    compiler.put_a_number(PAINT, call.location);
    compiler.save_to_out(SCREENOP_REG, call.location);

    Ok(())
}

fn set_at(compiler: &mut Compiler, call: &Call) -> Res {
    let [x, y, color] = arg_parse(
        compiler,
        [Arg::Number("x"), Arg::Number("y"), Arg::Number("color")],
        call,
    )?;
    put_xy_color(compiler, color, x, y, call)?;
    compiler.save_to_out(SCREENPOS1_REG, call.location);
    compiler.save_to_out(SCREENPOS2_REG, call.location);
    compiler.put_a_number(PAINT, call.location);
    compiler.save_to_out(SCREENOP_REG, call.location);

    Ok(())
}

fn put_xy_color(
    compiler: &mut Compiler,
    color: &Expression,
    x: &Expression,
    y: &Expression,
    call: &Call<'_>,
) -> Res {
    match is_const_color(color) {
        Some(color) => {
            put_xy(compiler, x, y, call.location, 6)?;
            compiler.put_b_number(color.into(), call.location);
            instr!(compiler, OR, call.location);
        }
        None => {
            if Compiler::can_put_into_b(color) {
                put_xy(compiler, x, y, call.location, 6)?;
                compiler.put_into_b(color)?;
                instr!(compiler, OR, call.location);
            } else if is_color_of_call(&color.typ) {
                put_xy(compiler, x, y, call.location, 6)?;
                compiler.eval_expr(color)?;
                instr!(compiler, OR, call.location);
            } else if compiler.try_get_constant(x).is_some()
                && compiler.try_get_constant(y).is_some()
            {
                compiler.eval_expr(color)?;
                compiler.switch(call.location)?;
                put_xy(compiler, x, y, call.location, 6)?;
            } else {
                put_xy(compiler, x, y, call.location, 6)?;
                let temp = compiler.insert_temp_var(call.location)?;
                compiler.save_to(temp, call.location);
                compiler.eval_expr(color)?;
                instr!(compiler, LB, temp, call.location);
                compiler.cleanup_temp_var(temp);
            }
        }
    }
    Ok(())
}

fn flip(compiler: &mut Compiler, call: &Call) -> Res {
    let _ = arg_parse(compiler, [], call)?;
    compiler.put_a_number(FLIP, call.location);
    compiler.save_to_out(SCREENOP_REG, call.location);
    Ok(())
}

fn set(compiler: &mut Compiler, call: &Call) -> Res {
    let [position, color] = arg_parse(
        compiler,
        [Arg::Number("position"), Arg::Number("color")],
        call,
    )?;

    load_position_color(compiler, position, color, call)?;
    compiler.save_to_out(SCREENPOS1_REG, call.location);
    compiler.save_to_out(SCREENPOS2_REG, call.location);
    compiler.put_a_number(PAINT, call.location);
    compiler.save_to_out(SCREENOP_REG, call.location);
    Ok(())
}

fn fill(compiler: &mut Compiler, call: &Call) -> Res {
    let [from, to, color] = arg_parse(
        compiler,
        [Arg::Number("from"), Arg::Number("to"), Arg::Number("color")],
        call,
    )?;
    load_position_color(compiler, from, color, call)?;
    compiler.save_to_out(SCREENPOS1_REG, call.location);
    compiler.eval_expr(to)?;
    compiler.save_to_out(SCREENPOS2_REG, call.location);
    compiler.put_a_number(PAINT, call.location);
    compiler.save_to_out(SCREENOP_REG, call.location);
    Ok(())
}

fn fill_xy(compiler: &mut Compiler, call: &Call) -> Res {
    let [x1, y1, x2, y2, color] = arg_parse(
        compiler,
        [
            Arg::Number("x1"),
            Arg::Number("y1"),
            Arg::Number("x2"),
            Arg::Number("y2"),
            Arg::Number("color"),
        ],
        call,
    )?;

    put_xy_color(compiler, color, x1, y1, call)?;
    compiler.save_to_out(SCREENPOS1_REG, call.location);
    put_xy(compiler, x2, y2, call.location, 6)?;
    compiler.save_to_out(SCREENPOS2_REG, call.location);
    compiler.put_a_number(PAINT, call.location);
    compiler.save_to_out(SCREENOP_REG, call.location);
    Ok(())
}

fn load_position_color(
    compiler: &mut Compiler,
    position: &Expression,
    color: &Expression,
    call: &Call,
) -> Res {
    match (compiler.try_get_constant(position), is_const_color(color)) {
        (None, None) => {
            let temp = compiler.insert_temp_var(call.location)?;
            compiler.eval_expr(color)?;
            compiler.save_to(temp, call.location);
            compiler.eval_expr(position)?;
            instr!(compiler, LB, temp, call.location);
            compiler.cleanup_temp_var(temp);
            instr!(compiler, OR, call.location);
        }
        (None, Some(color)) => {
            compiler.eval_expr(position)?;
            compiler.put_b_number(color.into(), call.location);
            instr!(compiler, OR, call.location);
        }
        (Some(pos), None) => {
            compiler.eval_expr(color)?;
            compiler.put_b_number(pos, call.location);
            instr!(compiler, OR, call.location);
        }
        (Some(pos), Some(color)) => compiler.put_a_number(pos | i16::from(color), call.location),
    }
    Ok(())
}

fn color_of(compiler: &mut Compiler, call: &Call) -> Res {
    let color = arg_parse(compiler, [Arg::Number("color")], call)?[0];

    match compiler.try_get_constant(color) {
        Some(number) => compiler.put_a_number(number, call.location),
        None => compiler.eval_expr(color)?,
    }
    Ok(())
}

fn get_color(color: &str) -> Option<NonZeroI16> {
    NonZeroI16::new(
        match color {
            "white" => 0,
            "orange" => 1,
            "magenta" => 2,
            "light_blue" => 3,
            "yellow" => 4,
            "lime" => 5,
            "pink" => 6,
            "gray" => 7,
            "light_gray" => 8,
            "cyan" => 9,
            "purple" => 10,
            "blue" => 11,
            "brown" => 12,
            "green" => 13,
            "red" => 14,
            "black" => 15,
            _ => return None,
        } << 12,
    )
}

fn is_const_color(expr: &Expression) -> Option<NonZeroI16> {
    match &expr.typ {
        ExpressionType::Member { object, property } => {
            let color = get_color(&property.symbol);
            match color {
                Some(_) if matches!(&object.typ, ExpressionType::Identifier(name) if name == "colorscreen") => {
                    color
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_color_of_call(expr: &ExpressionType) -> bool {
    match expr {
        ExpressionType::Call { args, function } => match &function.typ {
            ExpressionType::Member { object, property }
                if args.len() == 1 && Compiler::can_put_into_a(&args[0]) =>
            {
                matches!(&object.typ, ExpressionType::Identifier(name) if name == "colorscreen")
                    && &property.symbol == "color_of"
            }
            _ => false,
        },
        _ => false,
    }
}
