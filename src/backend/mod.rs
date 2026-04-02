#[macro_use]
mod compiler;
mod error;
pub mod instruction;
#[macro_use]
mod module;
mod target;
mod types;

pub use compiler::Compiler;
pub use instruction::{Instruction, InstructionVariant};
pub use target::{Output, Target};

use error::Type as ErrorType;
use types::{ComputerState, Instr, RamPage, RegisterContents, Scope};
