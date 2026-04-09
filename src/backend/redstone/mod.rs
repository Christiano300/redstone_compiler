#[macro_use]
pub mod compiler;
pub mod error;
pub mod instruction;
#[macro_use]
pub mod module;
pub mod types;

pub use compiler::Compiler;
pub use error::Type as ErrorType;
pub use instruction::{Instruction, InstructionVariant};
pub use types::{ComputerState, Instr, RamPage, RegisterContents, Scope};
