pub mod compiler;
pub mod instruction;
mod module;

pub use compiler::compile_program;
pub use instruction::{Instruction, InstructionVariant};

use compiler::{Compiler, ComputerState, Error, ModuleCall};
