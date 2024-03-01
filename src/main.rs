use std::{
    collections::VecDeque,
    env,
    fs::{self, create_dir_all, File},
    io::{self, Read, Write},
};

use redstone_compiler::frontend::{tokenize, Parser};

use redstone_compiler::backend::compile_program;

fn has_arg(args: &mut VecDeque<String>, arg: &'static str) -> bool {
    if args.contains(&arg.to_string()) {
        args.retain(|a| a != arg);
        true
    } else {
        false
    }
}

fn main() -> io::Result<()> {
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

    let dir = match env::current_dir() {
        Ok(p) if p.ends_with("programs") => program.clone(),
        _ => format!("programs/{program}"),
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
            println!("{err:#?}");
            return Ok(());
        }
    };
    if debug {
        println!("{tokens:#?}");
    }

    let mut parser = Parser::new();
    let ast = match parser.produce_ast(tokens) {
        Ok(ast) => ast,
        Err(err) => {
            println!("{err:#?}");
            return Ok(());
        }
    };
    if debug {
        println!("{ast:?}");
    }

    let assembly = match compile_program(ast) {
        Ok(assembly) => assembly,
        Err(err) => {
            err.pretty_print(code.as_str(), path.as_str());
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
        .map(|instr| format!("{:b}", instr.to_bin()))
        .for_each(|line| bin_string.push_str(line.as_str()));

    fs::write(format!("{dir}/{program}.bin"), bin_string)?;

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
    println!("Repl v -0.1");
    loop {
        let line = input("> ")?;
        if line.as_str() == "exit" {
            return io::Result::Ok(());
        }

        let tokens = tokenize(line.as_str());
        let Ok(tokens) = tokens else {
            println!("{tokens:#?}");
            continue;
        };

        let parser_result = parser.produce_ast(tokens);

        let Ok(ast) = parser_result else {
            println!("{parser_result:#?}");
            continue;
        };
        println!("{ast:#?}");

        let code = compile_program(ast);
        println!("{code:#?}");
    }
}
