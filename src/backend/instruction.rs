use core::panic;
use std::fmt;
use std::u8;

#[derive(Debug)]
pub struct InstructionVariant {
    id: u8,
    disc_jump: bool,
    jump: bool,
    instant: bool,
    name: &'static str,
}

impl InstructionVariant {
    const fn new(
        name: &'static str,
        disc_jump: bool,
        jump: bool,
        id: u8,
        instant: bool,
    ) -> InstructionVariant {
        InstructionVariant {
            name,
            id,
            instant,
            jump,
            disc_jump,
        }
    }

    fn to_byte(&self) -> u8 {
        self.instant as u8 | self.id << 1
    }

    pub fn disc_jump(&self) -> InstructionVariant {
        if self.disc_jump || !self.jump || matches!(self.id, 0..=6) {
            panic!("{self:?} is not a valid jump command");
        }
        match &self.id {
            0 => Self::JMD,
            1 => Self::JDE,
            2 => Self::JDN,
            3 => Self::JDG,
            4 => Self::JDGE,
            5 => Self::JDL,
            6 => Self::JDLE,
            _ => panic!(),
        }
    }

    pub const STOP: InstructionVariant = Self::new("STP", false, false, 0, false);

    pub const NON: InstructionVariant = Self::new("NON", false, false, 0, true);
    pub const LA: InstructionVariant = Self::new("LA", false, false, 1, true);
    pub const LB: InstructionVariant = Self::new("LB", false, false, 2, true);
    pub const LAL: InstructionVariant = Self::new("LAL", false, false, 3, true);
    pub const LAH: InstructionVariant = Self::new("LAH", false, false, 4, true);
    pub const LBL: InstructionVariant = Self::new("LBL", false, false, 5, true);
    pub const LBH: InstructionVariant = Self::new("LBH", false, false, 6, true);

    pub const SVA: InstructionVariant = Self::new("SVA", false, false, 7, false);

    pub const ADD: InstructionVariant = Self::new("ADD", false, false, 8, true);
    pub const SUB: InstructionVariant = Self::new("SUB", false, false, 9, true);
    pub const AND: InstructionVariant = Self::new("AND", false, false, 10, true);
    pub const OR: InstructionVariant = Self::new("OR", false, false, 11, true);
    pub const XOR: InstructionVariant = Self::new("XOR", false, false, 12, true);

    pub const SUP: InstructionVariant = Self::new("SUP", false, false, 13, false);
    pub const SDN: InstructionVariant = Self::new("SDN", false, false, 14, false);
    pub const MUL: InstructionVariant = Self::new("MUL", false, false, 15, false);

    pub const RW: InstructionVariant = Self::new("RW", false, false, 16, true);
    pub const RR: InstructionVariant = Self::new("RR", false, false, 17, true);
    pub const RC: InstructionVariant = Self::new("RC", false, false, 18, false);

    pub const INB: InstructionVariant = Self::new("INB", false, false, 19, false);
    pub const LCL: InstructionVariant = Self::new("LCL", false, false, 20, true);

    pub const JMP: InstructionVariant = Self::new("JMP", false, true, 0, true);
    pub const JE: InstructionVariant = Self::new("JE", false, true, 1, true);
    pub const JNE: InstructionVariant = Self::new("JNE", false, true, 2, true);
    pub const JG: InstructionVariant = Self::new("JG", false, true, 3, true);
    pub const JGE: InstructionVariant = Self::new("JGE", false, true, 4, true);
    pub const JL: InstructionVariant = Self::new("JL", false, true, 5, true);
    pub const JLE: InstructionVariant = Self::new("JLE", false, true, 6, true);

    pub const JMD: InstructionVariant = Self::new("JMD", true, true, 0, true);
    pub const JDE: InstructionVariant = Self::new("JDE", true, true, 1, true);
    pub const JDN: InstructionVariant = Self::new("JDN", true, true, 2, true);
    pub const JDG: InstructionVariant = Self::new("JDG", true, true, 3, true);
    pub const JDGE: InstructionVariant = Self::new("JDGE", true, true, 4, true);
    pub const JDL: InstructionVariant = Self::new("JDL", true, true, 5, true);
    pub const JDLE: InstructionVariant = Self::new("JDLE", true, true, 6, true);
}

pub struct Instruction {
    variant: InstructionVariant,
    arg: Option<u8>,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.arg {
            None => write!(f, "{}", self.variant.name),
            Some(arg) => write!(f, "{} {}", self.variant.name, arg),
        }
    }
}

impl Instruction {
    pub fn to_bin(&self) -> u16 {
        (self.variant.to_byte() as u16) << 8 | self.arg.unwrap_or(0) as u16
    }
}
