pub mod target;

#[cfg(feature = "redstone")]
pub mod redstone;

#[cfg(feature = "w4")]
pub mod w4;

#[cfg(feature = "redstone")]
pub use redstone::{Compiler, ErrorType, Instruction, InstructionVariant};

pub use target::{Output, Target};
