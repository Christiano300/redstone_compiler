pub mod compiler;
pub mod instruction;
mod module;

pub use compiler::{compile_program, compile_src};
pub use instruction::*;

use compiler::*;
use module::*;
