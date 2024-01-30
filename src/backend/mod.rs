pub mod compiler;
pub mod instruction;
mod module;

#[allow(unused)]
pub use compiler::compile_program;
pub use instruction::*;

use compiler::{Compiler, ComputerState, Error, ModuleCall};
