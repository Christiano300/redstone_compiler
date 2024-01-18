use std::io::{self, Write};

use frontend::Parser;

use crate::backend::compile_program;

#[allow(dead_code, unused)]
mod backend;
mod frontend;
#[allow(dead_code)]
mod runtime;

fn main() -> io::Result<()> {
    let mut parser = Parser::new();
    println!("Repl v -0.1");

    loop {
        let mut line = String::new();

        print!("> ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut line)?;
        line.replace_range(line.len() - 1.., "");

        if line == "exit" {
            return io::Result::Ok(());
        }

        let ast = parser.produce_ast(line).unwrap();
        println!("{:#?}", ast);

        let code = compile_program(ast);
        println!("{:#?}", code)
    }
}
