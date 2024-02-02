use std::{
    fs::{self, create_dir_all, File},
    io::{self, Read, Write},
};

use frontend::Parser;

use backend::compile_program;

#[allow(dead_code)]
mod backend;
mod frontend;

fn main() -> io::Result<()> {
    let mut parser = Parser::new();
    let program = input("Enter program or leave empty for repl: ")?;

    if program.is_empty() {
        repl(&mut parser)?;
        return Ok(());
    }

    let path = format!("programs/{program}/{program}.🖥️");
    let Ok(mut file) = File::open(path.clone()) else {
        if input("Program doesn't exist, create? [Y/n]: ")?.as_str() == "n" {
            return Ok(());
        }
        create_dir_all(format!("programs/{program}"))?;
        return fs::write(path, "");
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let ast = match parser.produce_ast(contents.as_str()) {
        Ok(ast) => ast,
        Err(err) => {
            println!("{err:#?}");
            return Ok(());
        }
    };

    let assembly = match compile_program(ast) {
        Ok(assembly) => assembly,
        Err(err) => {
            println!("{err:#?}");
            return Ok(());
        }
    };

    let mut asm_string = String::new();
    assembly
        .into_iter()
        .map(|instr| format!("{instr}\n"))
        .for_each(|line| asm_string.push_str(line.as_str()));

    fs::write(format!("programs/{program}/{program}.asm"), asm_string)?;

    Ok(())
}

fn input(prompt: &str) -> Result<String, io::Error> {
    let mut contents = String::new();
    print!("{prompt}");
    io::stdout().flush()?;
    io::stdin().read_line(&mut contents)?;
    Ok(contents.trim().to_owned())
}

fn repl(parser: &mut Parser) -> io::Result<()> {
    println!("Repl v -0.1");
    loop {
        let line = input("> ")?;
        if line.as_str() == "exit" {
            return io::Result::Ok(());
        }

        let parser_result = parser.produce_ast(line.as_str());

        let Ok(ast) = parser_result else {
            println!("{parser_result:#?}");
            continue;
        };
        println!("{ast:#?}");

        let code = compile_program(ast);
        println!("{code:#?}");
    }
}
