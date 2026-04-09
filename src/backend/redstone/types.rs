use std::collections::HashMap;

use super::instruction::Instruction;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
#[allow(unused)]
pub enum RegisterContents {
    Variable(u8),
    Number(i16),
    RamAddress(i32),
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RamPage {
    ThisOne(u8),
    Unknown,
}

impl Default for RamPage {
    fn default() -> Self {
        Self::ThisOne(0)
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct ComputerState {
    pub a: RegisterContents,
    pub b: RegisterContents,
    pub c: RegisterContents,
    pub ram_page: RamPage,
}

#[derive(Debug)]
pub enum Instr {
    Code(Instruction),
    Scope(Vec<Self>),
}

#[derive(Debug, Default)]
pub struct Scope {
    pub state: ComputerState,
    pub variables: HashMap<String, u8>,
    pub inline_variables: HashMap<String, i16>,
    pub instructions: Vec<Instr>,
}

impl Scope {
    pub fn with_state(state: ComputerState) -> Self {
        Self {
            state,
            ..Self::default()
        }
    }
}
