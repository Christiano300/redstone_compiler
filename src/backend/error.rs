use colored::Colorize;

use crate::frontend::Range;
use std::fmt::{Debug, Display};

#[derive(Debug, PartialEq, Eq)]
pub enum Type {
    NonexistentVar(String),
    NonexistentInlineVar(String),
    TooManyVars,
    ForbiddenInline,
    UnknownModule(String),
    UnknownMethod(String),
    InvalidArgs(String),
    CompileTimeArg(String),
    SomethingElseWentWrong(String),
    ModuleInitTwice(String),
    EqInNormalExpr,
    NormalInEqExpr,
    UseOutsideGlobalScope,
}

#[derive(PartialEq, Eq)]
pub struct Error {
    pub typ: Type,
    pub location: Range,
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt(f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt(f)
    }
}

impl Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {:?}", self.get_message(), self.location)?;
        Ok(())
    }

    pub fn pretty_print(&self, code: &str, file: &str) {
        if self.location.0 .0 != self.location.1 .0 {
            println!("Multi-line errors don't support nice error messages yet\n{self}");
            return;
        }
        let Some(line) = code.split('\n').nth(self.location.0 .0 as usize) else {
            println!("Compiler crashed, line does not exist in file, apparently\n{self}");
            return;
        };

        println!(
            "{} {}\nat {file}:{:?}",
            "Error:".red().dimmed(),
            self.get_message().bright_red(),
            self.location
        );

        let line_number = format!("{} | ", self.location.0 .0 + 1);
        let len = line_number.len() - 3;

        println!("{} {} ", " ".repeat(len), "|".bright_blue());
        print!("{}", line_number.as_str().bright_blue());
        println!("{line}");
        print!("{} {} ", " ".repeat(len), "|".bright_blue());
        println!(
            "{}{}\n",
            " ".repeat(self.location.0 .1 as usize - 1),
            "^".repeat((self.location.1 .1 - self.location.0 .1) as usize + 1)
                .bright_red()
        );
    }

    fn get_message(&self) -> String {
        match &self.typ {
            Type::NonexistentVar(name) => {
                format!("Varialble {name} is not defined")
            }
            Type::NonexistentInlineVar(name) => {
                format!("Inline variable {name} is not defined")
            }
            Type::TooManyVars => "There are too many variales".to_string(),
            Type::ForbiddenInline => {
                "This expression cannot be used in an inline expression".to_string()
            }
            Type::UnknownModule(name) => {
                format!("The module {name} is either not loaded or doesn't exist")
            }
            Type::UnknownMethod(name) => {
                format!("The method {name} doesn't exist")
            }
            Type::InvalidArgs(args) => {
                format!("The arguments {args} are invalid")
            }
            Type::SomethingElseWentWrong(e) => {
                format!("Something else has gone wrong: {e}. Please report this to the developer")
            }
            Type::ModuleInitTwice(name) => {
                format!("The module {name} was initialilzed twice")
            }
            Type::EqInNormalExpr => {
                "You can't use an Equality Expression in a Normal Expression".to_string()
            }
            Type::NormalInEqExpr => "You can't use a normal Expression here".to_string(),
            Type::UseOutsideGlobalScope => "You can only use 'use' in the global scope".to_string(),
            Type::CompileTimeArg(name) => {
                format!("{name} has to be known at compile-time")
            }
        }
    }
}
