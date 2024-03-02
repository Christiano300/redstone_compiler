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
            Self::EmptyBlock => "Cannot have empty block. Use 'pass'".to_string(),
            Self::MissingEnd => "Missing end keyword".to_string(),
            Self::InvalidModuleName => "Invalid module name".to_string(),
            Self::InvalidDeclartion => "Expected identifier".to_string(),
            Self::InvalidAssignment => "Can only assign to identifiers".to_string(),
            Self::MissingEquals => "Expected equals following identifier".to_string(),
            Self::FunctionChaining => {
                "You can't chain functions, what do you think this is, Python?".to_string()
            }
            Self::MissingOpenParen => "Expected '(' after function call".to_string(),
            Self::MissingClosingParen => "Missing ')'".to_string(),
            Self::InvalidDot => "Cannot use . on this".to_string(),
            Self::Eof => "Unexpected EOF while parsing".to_string(),
            Self::UnexpectedOther => "Unexpected token found".to_string(),
            Self::ExpectedParen => "Unexpected token, expected ')'".to_string(),
        }
    }
}
