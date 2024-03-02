use colored::{Colorize, CustomColor};
use std::fmt::{Debug, Display};

use crate::frontend::Range;

#[macro_export]
macro_rules! err {
    ($type:ident, $loc:expr) => {
        Err($crate::error::Error {
            typ: Box::new(ErrorType::$type),
            location: $loc,
        })
    };
}

#[allow(clippy::module_name_repetitions)]
pub trait ErrorType {
    fn get_message(&self) -> String;
}

pub struct Error {
    pub typ: Box<dyn ErrorType>,
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

const RED: CustomColor = CustomColor {
    r: 197,
    g: 15,
    b: 31,
};

const BRIGHT_RED: CustomColor = CustomColor {
    r: 231,
    g: 72,
    b: 86,
};

const BRIGHT_BLUE: CustomColor = CustomColor {
    r: 59,
    g: 120,
    b: 255,
};

impl Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {:?}", self.typ.get_message(), self.location)?;
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
            "Error:".custom_color(RED),
            self.typ.get_message().custom_color(BRIGHT_RED),
            self.location
        );

        let line_number = format!("{} | ", self.location.0 .0 + 1);
        let len = line_number.len() - 3;

        println!("{} {} ", " ".repeat(len), "|".custom_color(BRIGHT_BLUE));
        print!("{}", line_number.as_str().custom_color(BRIGHT_BLUE));
        println!("{line}");
        print!("{} {} ", " ".repeat(len), "|".custom_color(BRIGHT_BLUE));
        println!(
            "{}{}\n",
            " ".repeat(self.location.0 .1 as usize - 1),
            "^".repeat((self.location.1 .1 - self.location.0 .1) as usize + 1)
                .custom_color(BRIGHT_RED)
        );
    }
}
