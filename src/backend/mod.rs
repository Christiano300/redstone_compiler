pub mod compiler;
pub mod instruction;
mod module;

#[allow(unused)]
pub use compiler::compile_program;
pub use instruction::{Instruction, InstructionVariant};

use compiler::{Compiler, ComputerState, Error, ModuleCall};
