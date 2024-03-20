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

use core::panic;

use crate::{
    backend::compiler::Compiler,
    frontend::{Expression, ExpressionType},
    instr,
};

use super::{arg_parse, Arg, Call, Error, ErrorType, Res};

pub fn module(compiler: &mut Compiler, call: &Call) -> Res {
    match call.method_name.as_str() {
        "set" => set(compiler, call),
        "flip" => flip(compiler, call),
        "fill" => fill(compiler, call),
        "color_of" => color_of(compiler, call),
        _ => Err(Error {
            typ: Box::new(ErrorType::UnknownMethod(call.method_name.clone())),
            location: call.location,
        }),
    }
}

fn flip(compiler: &mut Compiler, call: &Call) -> Res {
    let _ = arg_parse(compiler, [], call.args, call.location)?;
    compiler.put_a_number(FLIP);
    compiler.save_to_out(SCREENOP_REG);
    Ok(())
}

fn set(compiler: &mut Compiler, call: &Call) -> Res {
    let [position, color] = arg_parse(
        compiler,
        [Arg::Number("position"), Arg::Number("color")],
        call.args,
        call.location,
    )?;

    load_position_color(compiler, position, color, call)?;
    compiler.save_to_out(SCREENPOS1_REG);
    compiler.save_to_out(SCREENPOS2_REG);
    compiler.put_a_number(PAINT);
    compiler.save_to_out(SCREENOP_REG);
    Ok(())
}

fn fill(compiler: &mut Compiler, call: &Call) -> Res {
    let [from, to, color] = arg_parse(
        compiler,
        [Arg::Number("from"), Arg::Number("to"), Arg::Number("color")],
        call.args,
        call.location,
    )?;
    load_position_color(compiler, from, color, call)?;
    compiler.save_to_out(SCREENPOS1_REG);
    compiler.eval_expr(to)?;
    compiler.save_to_out(SCREENPOS2_REG);
    compiler.put_a_number(PAINT);
    compiler.save_to_out(SCREENOP_REG);
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
            compiler.save_to(temp);
            compiler.eval_expr(position)?;
            instr!(compiler, LB, temp);
            compiler.cleanup_temp_var(temp);
            instr!(compiler, OR);
        }
        (None, Some(color)) => {
            let color = get_color(color);
            compiler.eval_expr(position)?;
            compiler.put_b_number(color);
            instr!(compiler, OR);
        }
        (Some(pos), None) => {
            compiler.eval_expr(color)?;
            compiler.put_b_number(pos);
            instr!(compiler, OR);
        }
        (Some(pos), Some(color)) => compiler.put_a_number(pos | get_color(color)),
    }
    Ok(())
}

fn color_of(compiler: &mut Compiler, call: &Call) -> Res {
    let color = arg_parse(compiler, [Arg::Number("color")], call.args, call.location)?[0];

    match compiler.try_get_constant(color) {
        Some(number) => compiler.put_a_number(number),
        None => compiler.eval_expr(color)?,
    }
    Ok(())
}

fn get_color(color: &str) -> i16 {
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
        _ => panic!("Invalid color name"),
    }
}

fn is_color_name(name: &str) -> bool {
    matches!(
        name,
        "white"
            | "orange"
            | "magenta"
            | "light_blue"
            | "yellow"
            | "lime"
            | "pink"
            | "gray"
            | "light_gray"
            | "cyan"
            | "purple"
            | "blue"
            | "brown"
            | "green"
            | "red"
            | "black"
    )
}

fn is_const_color(expr: &Expression) -> Option<&String> {
    match &expr.typ {
        ExpressionType::Member { object, property } => {
            if is_color_name(property)
                && matches!(&object.typ, ExpressionType::Identifier(name) if name == "colorscreen")
            {
                Some(property)
            } else {
                None
            }
        }
        _ => None,
    }
}
