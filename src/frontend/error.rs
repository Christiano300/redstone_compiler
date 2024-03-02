use crate::error::ErrorType;

pub enum Type {
    EmptyBlock,
    MissingEnd,
    InvalidModuleName,
    InvalidDeclartion,
    InvalidAssignment,
    MissingEquals,
    FunctionChaining,
    MissingOpenParen,
    MissingClosingParen,
    InvalidDot,
    Eof,
    UnexpectedOther,
    ExpectedParen,
}

impl ErrorType for Type {
    fn get_message(&self) -> String {
        match self {
            Self::EmptyBlock => "Cannot have empty block. Use 'pass'",
            Self::MissingEnd => "Missing end keyword",
            Self::InvalidModuleName => "Invalid module name",
            Self::InvalidDeclartion => "Expected identifier",
            Self::InvalidAssignment => "Can only assign to identifiers",
            Self::MissingEquals => "Expected equals following identifier",
            Self::FunctionChaining => {
                "You can't chain functions, what do you think this is, Python?"
            }
            Self::MissingOpenParen => "Expected '(' after function call",
            Self::MissingClosingParen => "Missing ')'",
            Self::InvalidDot => "Cannot use . on this",
            Self::Eof => "Unexpected EOF while parsing",
            Self::UnexpectedOther => "Unexpected token found",
            Self::ExpectedParen => "Unexpected token, expected ')'",
        }
        .to_string()
    }
}
