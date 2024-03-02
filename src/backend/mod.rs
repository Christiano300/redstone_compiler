mod compiler;
mod error;
pub mod instruction;
mod module;
mod types;

pub use compiler::compile_program;
pub use instruction::{Instruction, InstructionVariant};

use compiler::Compiler;
use error::Type as ErrorType;
use types::{ComputerState, Instr, RamPage, RegisterContents, Scope};
