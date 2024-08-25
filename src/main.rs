use std::{
    collections::VecDeque,
    env,
    fs::{self, create_dir_all, File},
    io::{self, Read, Write},
};

use colored::{Colorize, CustomColor};
use redstone_compiler::frontend::{tokenize, Parser};

use redstone_compiler::backend::compile_program;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const fn color_from_hex(n: i32) -> CustomColor {
    CustomColor {
        r: (n >> 16 & 0xff) as u8,
        g: (n >> 8 & 0xff) as u8,
        b: (n & 0xff) as u8,
    }
}

const REDSTONE: [CustomColor; 8] = [
    color_from_hex(0x00EE_0F00),
    color_from_hex(0x00E2_0F00),
    color_from_hex(0x00CF_1000),
    color_from_hex(0x00BC_1000),
    color_from_hex(0x00AA_1100),
    color_from_hex(0x0094_1100),
    color_from_hex(0x007F_1200),
    color_from_hex(0x0069_1200),
];

fn redstone_color_print(str: &str) {
    for char in str.chars() {
        print!(
            "{}",
            char.to_string()
                .custom_color(fastrand::choice(REDSTONE).unwrap())
        );
    }
}

fn has_arg(args: &mut VecDeque<String>, arg: &'static str) -> bool {
    if args.contains(&arg.to_string()) {
        args.retain(|a| a != arg);
        true
    } else {
        false
    }
}

fn main() -> io::Result<()> {
    redstone_color_print(format!("RedC v{VERSION}\n").as_str());
    let mut args: VecDeque<_> = env::args().collect();
    args.pop_front();

    let debug = has_arg(&mut args, "--dbg");

    let program = match args.pop_front() {
        None => input("Enter program or leave empty for repl: ")?,
        Some(p) => p,
    };

    if program.is_empty() {
        return repl();
    }

    let dir = if fs::metadata(format!("{program}/{program}.🖥️")).is_ok()
        || matches!(env::current_dir(), Ok(p) if p.ends_with("programs"))
    {
        program.clone()
    } else {
        format!("programs/{program}")
    };
    let path = format!("{dir}/{program}.🖥️");
    let Ok(mut file) = File::open(path.clone()) else {
        if input("Program doesn't exist, create? [Y/n]: ")?.as_str() == "n" {
            return Ok(());
        }
        create_dir_all(dir).expect("something went wrong with creating the directory");
        fs::write(path, "").expect("something went wrong with writing the program");
        return Ok(());
    };

    let mut code = String::new();
    file.read_to_string(&mut code)?;

    let tokens = match tokenize(code.as_str()) {
        Ok(tokens) => tokens,
        Err(err) => {
            err.pretty_print(code.as_str(), path.as_str());
            return Ok(());
        }
    };
    if debug {
        println!("{tokens:#?}");
    }

    let mut parser = Parser::new();
    let ast = match parser.produce_ast(tokens) {
        Ok(ast) => ast,
        Err(errs) => {
            errs.into_iter()
                .for_each(|err| err.pretty_print(code.as_str(), path.as_str()));
            return Ok(());
        }
    };
    if debug {
        println!("{ast:#?}");
    }

    let assembly = match compile_program(ast) {
        Ok(assembly) => assembly,
        Err(errs) => {
            for err in errs {
                err.pretty_print(code.as_str(), path.as_str());
            }
            return Ok(());
        }
    };

    let mut asm_string = String::new();
    assembly
        .iter()
        .map(|instr| format!("{instr}\n"))
        .for_each(|line| asm_string.push_str(line.as_str()));

    fs::write(format!("{dir}/{program}.asm"), asm_string)?;

    let mut bin_string = String::new();
    assembly
        .iter()
        .map(|instr| format!("{:016b}\n", instr.to_bin()))
        .for_each(|line| bin_string.push_str(line.as_str()));

    fs::write(format!("{dir}/{program}.bin"), bin_string)?;

    if has_arg(&mut args, "--loc") {
        let mut locations = String::new();
        let mut last = None;
        for instr in &assembly {
            let line_s = (instr.orig_location.0 .0, instr.orig_location.1 .0);
            if last != Some(line_s) {
                locations.push_str(&if line_s.0 == line_s.1 {
                    format!("{}:\n", line_s.0 + 1)
                } else {
                    format!("{}-{}:\n", line_s.0 + 1, line_s.1 + 1)
                });
                last = Some(line_s);
            }
            locations.push_str(&format!("\t{instr}\n"));
        }
        fs::write(format!("{dir}/{program}.loc"), locations)?;
    }

    println!(
        "{}\n{} {}",
        "Compilation finished successful".bright_green(),
        "Saved assembly to".truecolor(19, 161, 14),
        format!("{dir}/{program}.asm").truecolor(222, 222, 222)
    );

    Ok(())
}

fn input(prompt: &str) -> Result<String, io::Error> {
    let mut contents = String::new();
    print!("{prompt}");
    io::stdout().flush()?;
    io::stdin().read_line(&mut contents)?;
    Ok(contents.trim().to_owned())
}

fn repl() -> io::Result<()> {
    let mut parser = Parser::new();
    println!("Repl v{VERSION}");
    loop {
        let line = input("> ")?;
        if line.as_str() == "exit" {
            return io::Result::Ok(());
        }

        let tokens = tokenize(line.as_str());
        let tokens = match tokens {
            Ok(tokens) => tokens,
            Err(err) => {
                err.pretty_print(&line, "Repl");
                continue;
            }
        };
        println!("{tokens:#?}");

        let parser_result = parser.produce_ast(tokens);

        let ast = match parser_result {
            Ok(ast) => ast,
            Err(errs) => {
                errs.into_iter()
                    .for_each(|err| err.pretty_print(&line, "Repl"));
                continue;
            }
        };
        println!("{ast:#?}");

        let code = compile_program(ast);
        match code {
            Ok(code) => println!("{code:#?}"),
            Err(err) => err.into_iter().for_each(|err| {
                err.pretty_print(&line, "Repl");
            }),
        }
    }
}
