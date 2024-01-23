pub mod compiler;
pub mod instruction;
mod module;

#[allow(unused)]
pub use compiler::{compile_program, compile_src};
pub use instruction::*;

use compiler::*;
