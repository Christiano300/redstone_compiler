use core::panic;
use std::{
    fmt::{self, Debug},
    u8,
};

use table_enum::table_enum;

table_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[allow(unused)]
    pub enum InstructionVariant(
    name: &'static str,
    disc_jump: bool,
    jump: bool,
    id: u8,
    instant: bool,
    alu: bool,
    has_arg: bool,
) {
    STOP("STP", false, false, 0, false, false, false),

    NON("NON", false, false, 0, true, false, false),
    LA("LA", false, false, 1, true, false, true),
    LB("LB", false, false, 2, true, false, true),
    LC("LC", false, false, 3, true, false, true),

    SVA("SVA", false, false, 4, true, false, true),

    LAL("LAL", false, false, 5, true, false, true),
    LAH("LAH", false, false, 6, true, false, true),
    LBL("LBL", false, false, 7, true, false, true),
    LBH("LBH", false, false, 8, true, false, true),
    LCL("LCL", false, false, 9, true, false, true),

    ADD("ADD", false, false, 1, true, true, false),
    SUB("SUB", false, false, 2, true, true, false),
    AND("AND", false, false, 3, true, true, false),
    OR("OR", false, false, 4, true, true, false),
    XOR("XOR", false, false, 5, true, true, false),

    SUP("SUP", false, false, 6, false, true, true),
    SDN("SDN", false, false, 7, false, true, true),
    MUL("MUL", false, false, 8, false, true, false),

    RW("RW", false, false, 255, true, false, false),
    RR("RR", false, false, 255, true, false, false),
    RC("RC", false, false, 255, false, false, false),

    INB("INB", false, false, 255, false, false, false),

    JMP("JMP", false, true, 0, true, true, true),
    JE("JE", false, true, 1, true, true, true),
    JNE("JNE", false, true, 2, true, true, true),
    JG("JG", false, true, 3, true, true, true),
    JGE("JGE", false, true, 4, true, true, true),
    JL("JL", false, true, 5, true, true, true),
    JLE("JLE", false, true, 6, true, true, true),

    JMD("JMD", true, true, 0, true, true, true),
    JDE("JDE", true, true, 1, true, true, true),
    JDN("JDN", true, true, 2, true, true, true),
    JDG("JDG", true, true, 3, true, true, true),
    JDGE("JDGE", true, true, 4, true, true, true),
    JDL("JDL", true, true, 5, true, true, true),
    JDLE("JDLE", true, true, 6, true, true, true),

    SMP("SMP", false, true, 0, true, true, true),
    SE("SE", false, true, 1, true, true, true),
    SNE("SNE", false, true, 2, true, true, true),
    SG("SG", false, true, 3, true, true, true),
    SGE("SGE", false, true, 4, true, true, true),
    SL("SL", false, true, 5, true, true, true),
    SLE("SLE", false, true, 6, true, true, true),

    SMD("SMD", true, true, 0, true, true, true),
    SDE("SDE", true, true, 1, true, true, true),
    SDNE("SDNE", true, true, 2, true, true, true),
    SDG("SDG", true, true, 3, true, true, true),
    SDGE("SDGE", true, true, 4, true, true, true),
    SDL("SDL", true, true, 5, true, true, true),
    SDLE("SDLE", true, true, 6, true, true, true),

}}

impl InstructionVariant {
    /// Converts a normal jump into a disc jump
    ///
    /// # Panics
    ///
    /// Panics if instruction is not a valid jump
    #[must_use]
    pub fn to_disc_jump(self) -> Self {
        assert!(self.jump(), "{self:?} is not a valid jump command");
        assert!(!self.disc_jump(), "{self:?} is a disc-jump");

        match self.id() {
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

    #[must_use]
    pub const fn is_jump(self) -> bool {
        self.jump()
    }

    #[must_use]
    pub const fn from_op(op: EqualityOperator) -> Self {
        match op {
            EqualityOperator::EqualTo => Self::JE,
            EqualityOperator::NotEqual => Self::JNE,
            EqualityOperator::Greater => Self::JG,
            EqualityOperator::GreaterEq => Self::JGE,
            EqualityOperator::Less => Self::JL,
            EqualityOperator::LessEq => Self::JLE,
        }
    }

    #[must_use]
    pub const fn to_byte(&self) -> u8 {
        (self.jump() as u8) << 6
            | (self.disc_jump() as u8) << 5
            | self.id() << 1
            | (self.instant() as u8)
    }
}

#[derive(PartialEq, Eq)]
pub struct Instruction {
    pub variant: InstructionVariant,
    pub arg: Option<u8>,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_string(f)
    }
}

impl Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_string(f)
    }
}

use crate::frontend::EqualityOperator;

use super::{ComputerState, RamPage, RegisterContents};

impl Instruction {
    /// Creates a new [`Instruction`].
    ///
    /// # Panics
    ///
    /// Panics if an invalid number of args is supplied
    #[must_use]
    pub const fn new(variant: InstructionVariant, arg: Option<u8>) -> Self {
        assert!(variant.has_arg() == arg.is_some(),);
        Self { variant, arg }
    }

    #[must_use]
    pub fn to_bin(&self) -> u16 {
        u16::from(self.arg.unwrap_or(0)) | u16::from(self.variant.to_byte()) << 8
    }

    /// Used by Debug and Display
    ///
    /// # Errors
    ///
    /// This function will return an error if something goes wrong, apparently
    pub fn to_string(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.arg {
            None => write!(f, "{}", self.variant.name()),
            Some(arg) => write!(f, "{} {}", self.variant.name(), arg),
        }
    }

    pub fn execute(&self, on: &mut ComputerState) {
        use InstructionVariant as IV;
        use RegisterContents as RC;
        match self.variant {
            IV::LA | IV::SVA => on.a = RC::Variable(self.arg.unwrap_or(0)),
            IV::LB => on.b = RC::Variable(self.arg.unwrap_or(0)),
            IV::LAL => on.a = RC::Number(self.arg.unwrap_or(0).into()),
            IV::LAH => {
                on.a = match on.a {
                    RC::Number(value) => {
                        RC::Number(value + (i16::from(self.arg.unwrap_or(0)) << 8))
                    }
                    _ => RC::Unknown,
                }
            }
            IV::LBL => on.b = RC::Number(self.arg.unwrap_or(0).into()),
            IV::LBH => {
                on.b = match on.b {
                    RC::Number(value) => {
                        RC::Number(value + (i16::from(self.arg.unwrap_or(0)) << 8))
                    }
                    _ => RC::Unknown,
                }
            }
            IV::ADD | IV::SUB | IV::MUL | IV::AND | IV::OR | IV::XOR | IV::SUP | IV::SDN => {
                on.a = match (on.a, on.b) {
                    (RC::Number(a), RC::Number(b)) => RC::Number(match self.variant {
                        IV::ADD => a + b,
                        IV::SUB => a - b,
                        IV::AND => a & b,
                        IV::OR => a | b,
                        IV::XOR => a ^ b,
                        IV::SUP => a << b,
                        IV::SDN => a >> b,
                        IV::MUL => a * b,
                        _ => unreachable!(),
                    }),
                    _ => RC::Unknown,
                }
            }
            IV::LCL => on.c = RC::Number(self.arg.unwrap_or(0).into()),
            IV::LC => on.c = RC::Variable(self.arg.unwrap_or(0)),
            IV::RR => on.a = RC::Unknown,
            IV::INB => {
                on.b = match on.b {
                    RC::Number(value) => RC::Number(value + 1),
                    _ => RC::Unknown,
                }
            }
            IV::RC => {
                on.ram_page = match on.b {
                    RC::Number(address) => RamPage::ThisOne((address / 16).try_into().unwrap_or(0)),
                    _ => RamPage::Unknown,
                }
            }
            _ => {}
        }
    }
}
