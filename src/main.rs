use std::{
    env,
    fs::{self, File, create_dir_all},
    io::{self, Read, Write},
};

use clap::Parser as CLIParser;
use colored::{Colorize, CustomColor};
use redstone_compiler::{
    backend::{OptLevel, Output, Target as BackendTarget},
    frontend::{Lexer, LexerTarget, Parser},
};

#[cfg(feature = "redstone")]
use redstone_compiler::backend::Compiler;

#[cfg(feature = "w4")]
use redstone_compiler::backend::w4::W4Compiler;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const fn color_from_hex(n: i32) -> CustomColor {
    CustomColor {
        r: (n >> 16 & 0xff) as u8,
        g: (n >> 8 & 0xff) as u8,
        b: (n & 0xff) as u8,
    }
}

#[allow(clippy::unreadable_literal)] // I think they are quite readable
const REDSTONE: [CustomColor; 8] = [
    color_from_hex(0xEE0F00),
    color_from_hex(0xE20F00),
    color_from_hex(0xCF1000),
    color_from_hex(0xBC1000),
    color_from_hex(0xAA1100),
    color_from_hex(0x941100),
    color_from_hex(0x7F1200),
    color_from_hex(0x691200),
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

fn parse_opt_level(opt: &str) -> OptLevel {
    match opt.to_lowercase().as_str() {
        "basic" => OptLevel::Basic,
        "full" => OptLevel::Full,
        _ => OptLevel::None,
    }
}

#[derive(CLIParser, Debug)]
#[command(name = "RedC")]
#[command(version = VERSION)]
struct Args {
    /// Program name or path
    #[arg(value_name = "PROGRAM")]
    program: Option<String>,

    /// Enable debug output
    #[arg(short, long)]
    dbg: bool,

    /// Generate location mapping file
    #[arg(short, long)]
    loc: bool,

    /// Target architecture
    #[arg(short, long, value_name = "TARGET")]
    target: Option<String>,

    /// Optimization level (none, basic, full)
    #[arg(short, long, value_name = "LEVEL", default_value = "none")]
    opt: String,
}

fn main() -> io::Result<()> {
    redstone_color_print(format!("Redstone Compiler v{VERSION}\n").as_str());

    let args = Args::parse();

    let program = match args.program {
        None => input("Enter program or leave empty for repl: ")?,
        Some(p) => p,
    };

    if program.is_empty() {
        let opt_level = parse_opt_level(&args.opt);
        let target_str = args.target.as_deref().map(|t| t.to_lowercase());
        let lexer_target = match target_str.as_deref() {
            Some("w4") => LexerTarget::W4,
            _ => LexerTarget::Redstone,
        };
        return match target_str.as_deref() {
            #[cfg(feature = "redstone")]
            Some("mcn-16") | None => repl(Compiler::with_opt_level(opt_level), lexer_target),
            #[cfg(feature = "w4")]
            Some("w4") => {
                let mut compiler = W4Compiler::default();
                compiler.opt_level = opt_level;
                repl(compiler, lexer_target)
            }
            Some(other) => {
                eprintln!("Unknown target: {other}");
                return Ok(());
            }
        };
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

    let opt_level = parse_opt_level(&args.opt);

    match args.target.map(|t| t.to_lowercase()).as_deref() {
        #[cfg(feature = "redstone")]
        Some("mcn-16") | None => {
            run_compiler(
                Compiler::with_opt_level(opt_level),
                &code,
                &path,
                &dir,
                &program,
                args.dbg,
                args.loc,
                LexerTarget::Redstone,
            )
        }
        #[cfg(feature = "w4")]
        Some("w4") => {
            let mut compiler = W4Compiler::default();
            compiler.opt_level = opt_level;
            run_compiler(
                compiler,
                &code,
                &path,
                &dir,
                &program,
                args.dbg,
                args.loc,
                LexerTarget::W4,
            )
        }
        Some(other) => {
            eprintln!("Unknown target: {other}");
            Ok(())
        }
    }
}

fn run_compiler<T: BackendTarget>(
    mut target: T,
    code: &str,
    path: &str,
    dir: &str,
    program: &str,
    dbg: bool,
    loc: bool,
    lexer_target: LexerTarget,
) -> io::Result<()> {
    let lexer = Lexer::new(lexer_target);
    let tokens = match lexer.tokenize(code) {
        Ok(tokens) => tokens,
        Err(err) => {
            err.pretty_print(code, path);
            return Ok(());
        }
    };
    if dbg {
        println!("{tokens:#?}");
    }

    let mut parser = Parser::new();
    let ast = match parser.produce_ast(tokens) {
        Ok(ast) => ast,
        Err(errs) => {
            for err in errs {
                err.pretty_print(code, path);
            }
            return Ok(());
        }
    };
    if dbg {
        println!("{ast:#?}");
    }

    let assembly = match target.compile_program(ast) {
        Ok(assembly) => assembly,
        Err(errs) => {
            for err in errs {
                err.pretty_print(code, path);
            }
            return Ok(());
        }
    };

    let asm_string = assembly.repr();

    fs::write(format!("{dir}/{program}.asm"), asm_string)?;

    if let Some(bin_string) = assembly.repr_bin() {
        fs::write(format!("{dir}/{program}.bin"), bin_string)?;
    }

    if loc && let Some(locations) = assembly.repr_loc() {
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

fn repl<T: BackendTarget>(mut target: T, lexer_target: LexerTarget) -> io::Result<()> {
    let mut parser = Parser::new();
    let lexer = Lexer::new(lexer_target);
    println!("Repl v{VERSION}");
    loop {
        let line = input("> ")?;
        if line.as_str() == "exit" {
            return io::Result::Ok(());
        }

        let tokens = lexer.tokenize(line.as_str());
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
                for err in errs {
                    err.pretty_print(&line, "Repl");
                }
                continue;
            }
        };
        println!("{ast:#?}");

        let code = target.compile_program(ast);
        target.reset();
        match code {
            Ok(code) => println!("{}", code.repr()),
            Err(err) => err.into_iter().for_each(|err| {
                err.pretty_print(&line, "Repl");
            }),
        }
    }
}
