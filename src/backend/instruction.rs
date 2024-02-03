use std::{
    fmt::{self, Debug},
    u8,
};

use table_enum::table_enum;

table_enum! {
    #[derive(Debug)]
    #[allow(unused)]
    pub enum InstructionVariant(
    name: &'static str,
    disc_jump: bool,
    jump: bool,
    id: u8,
    instant: bool,
) {
    STOP("STP", false, false, 0, false),

    NON("NON", false, false, 0, true),
    LA("LA", false, false, 1, true),
    LB("LB", false, false, 2, true),
    LC("LC", false, false, 3, true),

    SVA("SVA", false, false, 4, false),

    LAL("LAL", false, false, 5, true),
    LAH("LAH", false, false, 6, true),
    LBL("LBL", false, false, 7, true),
    LBH("LBH", false, false, 8, true),
    LCL("LCL", false, false, 9, true),

    ADD("ADD", false, false, 10, true),
    SUB("SUB", false, false, 11, true),
    AND("AND", false, false, 12, true),
    OR("OR", false, false, 13, true),
    XOR("XOR", false, false, 14, true),

    SUP("SUP", false, false, 15, false),
    SDN("SDN", false, false, 16, false),
    MUL("MUL", false, false, 17, false),

    RW("RW", false, false, 18, true),
    RR("RR", false, false, 19, true),
    RC("RC", false, false, 20, false),

    INB("INB", false, false, 21, false),

    JMP("JMP", false, true, 0, true),
    JE("JE", false, true, 1, true),
    JNE("JNE", false, true, 2, true),
    JG("JG", false, true, 3, true),
    JGE("JGE", false, true, 4, true),
    JL("JL", false, true, 5, true),
    JLE("JLE", false, true, 6, true),

    JMD("JMD", true, true, 0, true),
    JDE("JDE", true, true, 1, true),
    JDN("JDN", true, true, 2, true),
    JDG("JDG", true, true, 3, true),
    JDGE("JDGE", true, true, 4, true),
    JDL("JDL", true, true, 5, true),
    JDLE("JDLE", true, true, 6, true),
}}

impl InstructionVariant {
    fn to_byte(&self) -> u8 {
        u8::from(self.instant()) | self.id() << 1
    }

    pub fn to_disc_jump(&self) -> Self {
        assert!(
            !self.disc_jump() && self.jump() && !matches!(self.id(), 0..=6),
            "{self:?} is not a valid jump command"
        );

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
}

pub struct Instruction {
    pub variant: &'static InstructionVariant,
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

use super::{compiler::RegisterContents, ComputerState};

impl Instruction {
    pub fn to_bin(&self) -> u16 {
        u16::from(self.variant.to_byte()) << 8 | u16::from(self.arg.unwrap_or(0))
    }

    pub fn to_string(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.arg {
            None => write!(f, "{}", self.variant.name()),
            Some(arg) => write!(f, "{} {}", self.variant.name(), arg),
        }
    }

    pub fn execute(&self, on: &mut ComputerState) {
        use InstructionVariant as IV;
        match self.variant {
            IV::LA => on.a = RegisterContents::Variable(self.arg.unwrap_or(17)),
            IV::LB => on.b = RegisterContents::Variable(self.arg.unwrap_or(17)),
            IV::LAL | IV::LBL | IV::LAH | IV::LBH => on.a = RegisterContents::Number(0),
            IV::ADD | IV::SUB | IV::MUL | IV::AND | IV::OR | IV::XOR | IV::SUP | IV::SDN => {
                on.a = RegisterContents::Result(0);
            }
            IV::LCL => on.c = self.arg.unwrap_or(0),
            IV::SVA => on.a = RegisterContents::Variable(self.arg.unwrap_or(21)),
            _ => {}
        }
    }
}
