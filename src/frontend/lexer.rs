use std::{borrow::Cow, fmt::Debug, iter::Peekable};

use crate::error::Error;

use super::{eq_operator, operator, EqualityOperator as EqOp, Location, Operator, Range};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LexerTarget {
    #[default]
    Redstone,
    W4,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenType {
    Number(i32),
    Identifier(String),
    Equals,
    OpenParen,
    OpenFuncParen,
    CloseParen,
    Comma,
    Dot,
    BinaryOperator(Operator),
    IOperator(Operator),
    EqOperator(EqOp),
    Inline,
    If,
    Elif,
    Else,
    End,
    Forever,
    While,
    Pass,
    Debug,
    Fun,
    Use,
    Var,
    Eof,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Token {
    pub typ: TokenType,
    pub location: Range,
}

impl Token {
    const fn from_char(typ: TokenType, location: Location) -> Self {
        Self {
            typ,
            location: Range::single_char(location),
        }
    }

    const fn with_len(typ: TokenType, location: Location, len: u16) -> Self {
        Self {
            typ,
            location: Range(
                location,
                Location {
                    line: location.line,
                    column: location.column + len - 1,
                },
            ),
        }
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} at {:?}", self.typ, self.location)
    }
}

enum ErrorType {
    InvalidNumber(String),
    Eof,
    InvalidChar(String),
}

impl crate::error::ErrorType for ErrorType {
    fn get_message(&self) -> Cow<'_, str> {
        match self {
            Self::InvalidNumber(n) => Cow::from(format!("Invalid number: {n}")),
            Self::Eof => Cow::from("Unexpected End of file"),
            Self::InvalidChar(c) => Cow::from(format!("Invalid character: {c}")),
        }
    }
}

fn keyword(string: String) -> TokenType {
    match string.as_str() {
        "inline" => TokenType::Inline,
        "if" => TokenType::If,
        "elif" | "elseif" => TokenType::Elif,
        "else" => TokenType::Else,
        "end" => TokenType::End,
        "forever" => TokenType::Forever,
        "while" => TokenType::While,
        "pass" => TokenType::Pass,
        "use" => TokenType::Use,
        "var" => TokenType::Var,
        "debug" => TokenType::Debug,
        "fun" => TokenType::Fun,
        _ => TokenType::Identifier(string),
    }
}

const fn is_skippable(c: char) -> bool {
    matches!(c, ' ' | '\n' | '\t' | '\r' | ';')
}

fn next(iter: &mut impl Iterator<Item = char>, location: &mut Location) -> Option<char> {
    let n = iter.next();
    if let Some(char) = n {
        match char {
            '\n' => {
                *location = Location {
                    line: location.line + 1,
                    column: 0,
                }
            }
            '\r' => {}
            _ => location.column += 1,
        }
    }
    n
}

use Token as T;
use TokenType as Tt;

#[derive(Debug, Clone, Default)]
pub struct Lexer {
    target: LexerTarget,
}

impl Lexer {
    pub fn new(target: LexerTarget) -> Self {
        Self { target }
    }

    pub fn with_target_redstone(self) -> Self {
        Self {
            target: LexerTarget::Redstone,
            ..self
        }
    }

    pub fn with_target_w4(self) -> Self {
        Self {
            target: LexerTarget::W4,
            ..self
        }
    }

    /// Transform source code into Tokens
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an invalid character
    pub fn tokenize(&self, source_code: &str) -> Result<Vec<Token>, Error> {
        let mut tokens: Vec<Token> = vec![];
        let mut src = source_code.chars().peekable();
        let mut current_location = Location { line: 0, column: 0 };
        let Some(mut char) = next(&mut src, &mut current_location) else {
            return Ok(vec![]);
        };
        let mut prev = ' ';
        loop {
            match char {
                '(' => tokens.push(if prev.is_whitespace() | is_skippable(prev) {
                    T::from_char(Tt::OpenParen, current_location)
                } else {
                    T::from_char(Tt::OpenFuncParen, current_location)
                }),
                ')' => tokens.push(T::from_char(Tt::CloseParen, current_location)),
                '+' | '*' | '&' | '|' | '^' => {
                    let equals_after = matches!(src.peek(), Some('='));

                    if let Some(operator) = operator(char) {
                        tokens.push(if equals_after {
                            T::with_len(Tt::IOperator(operator), current_location, 2)
                        } else {
                            T::from_char(Tt::BinaryOperator(operator), current_location)
                        });
                    }

                    if equals_after {
                        next(&mut src, &mut current_location);
                    }
                }
                '-' => tokens.push(self.read_hyphen(&mut src, &mut current_location)?),
                ',' => tokens.push(T::from_char(Tt::Comma, current_location)),
                '.' => tokens.push(T::from_char(Tt::Dot, current_location)),

                '=' => match src.peek() {
                    Some('=') => {
                        next(&mut src, &mut current_location);
                        tokens.push(T::from_char(
                            Tt::EqOperator(EqOp::EqualTo),
                            current_location,
                        ));
                    }
                    _ => tokens.push(T::from_char(Tt::Equals, current_location)),
                },
                '>' | '<' | '!' => {
                    let equals_after = matches!(src.peek(), Some('='));

                    if let Some(token) = eq_operator(char, equals_after) {
                        tokens.push(T::with_len(
                            Tt::EqOperator(token),
                            current_location,
                            if equals_after { 2 } else { 1 },
                        ));
                        next(&mut src, &mut current_location);
                    }
                }
                '#' => while !matches!(next(&mut src, &mut current_location), Some('\n') | None) {},
                _ => {
                    if char.is_ascii_digit() {
                        let start = current_location;
                        let num = self.read_num(char, &mut src, &mut current_location)?;

                        tokens.push(T {
                            typ: Tt::Number(num),
                            location: Range(start, current_location),
                        });
                    } else if char.is_alphabetic() {
                        read_identifier(char, &mut src, &mut current_location, &mut tokens);
                    } else if !is_skippable(char) {
                        return err!(
                            ErrorType::InvalidChar(char.to_string()),
                            Range(current_location, current_location)
                        );
                    }
                }
            }
            prev = char;
            char = match next(&mut src, &mut current_location) {
                Some(c) => c,
                None => break,
            };
        }
        tokens.push(T::from_char(Tt::Eof, current_location));

        Ok(tokens)
    }

    fn read_hyphen(
        &self,
        src: &mut Peekable<std::str::Chars<'_>>,
        current_location: &mut Location,
    ) -> Result<Token, Error> {
        Ok(match src.peek() {
            None => T::from_char(Tt::BinaryOperator(Operator::Minus), *current_location),
            Some(c) => match c {
                '=' => {
                    let t = T::with_len(Tt::IOperator(Operator::Minus), *current_location, 2);
                    next(src, current_location);
                    t
                }
                '0'..='9' => {
                    let start = *current_location;
                    let num = -self.read_num(
                        next(src, current_location).ok_or(<Result<i16, Error>>::unwrap_err(
                            err!(ErrorType::Eof, Range(start, *current_location)),
                        ))?,
                        src,
                        current_location,
                    )?;
                    T {
                        typ: Tt::Number(num),
                        location: Range(start, *current_location),
                    }
                }
                _ => T::from_char(Tt::BinaryOperator(Operator::Minus), *current_location),
            },
        })
    }

    fn read_num(
        &self,
        first: char,
        src: &mut Peekable<std::str::Chars<'_>>,
        current_location: &mut Location,
    ) -> Result<i32, Error> {
        let mut c = src.peek();

        if first == '0' {
            match c {
                Some('b') => return self.read_n_num(src, current_location, 2),
                Some('x') => return self.read_n_num(src, current_location, 16),
                _ => {}
            }
        }

        let mut num = String::new();
        num.push(first);

        loop {
            let Some(n) = c else {
                break;
            };
            if !n.is_ascii_digit() {
                break;
            }
            num.push(*n);
            next(src, current_location);
            c = src.peek();
        }
        let value: i32 = match num.parse() {
            Ok(v) => v,
            Err(_) => {
                return err!(
                    ErrorType::InvalidNumber(num),
                    Range(*current_location, *current_location)
                );
            }
        };
        self.validate_number(value, *current_location)?;
        Ok(value)
    }

    fn read_n_num(
        &self,
        src: &mut Peekable<std::str::Chars<'_>>,
        current_location: &mut Location,
        radix: u32,
    ) -> Result<i32, Error> {
        let start = *current_location;
        next(src, current_location);
        let mut c = src.peek();
        let mut num = String::new();

        loop {
            let Some(n) = c else {
                break;
            };
            if !n.is_ascii_hexdigit() {
                break;
            }
            num.push(*n);
            next(src, current_location);
            c = src.peek();
        }
        let value = u32::from_str_radix(num.as_str(), radix).map_or_else(
            |_| {
                err!(
                    ErrorType::InvalidNumber(num),
                    Range(start, *current_location)
                )
            },
            |u| {
                let value = u as i32;
                self.validate_number(value, start)?;
                Ok(value)
            },
        )?;
        Ok(value)
    }

    fn validate_number(&self, value: i32, location: Location) -> Result<(), Error> {
        match self.target {
            LexerTarget::Redstone => {
                if value > u16::MAX as i32 {
                    return err!(
                        ErrorType::InvalidNumber(format!(
                            "{} is out of range for Redstone (must be between 0 and {})",
                            value,
                            u16::MAX
                        )),
                        Range(location, location)
                    );
                }
                if value < i16::MIN as i32 {
                    return err!(
                        ErrorType::InvalidNumber(format!(
                            "{} is out of range for Redstone (must be between {} and {})",
                            value,
                            i16::MIN,
                            i16::MAX
                        )),
                        Range(location, location)
                    );
                }
            }
            LexerTarget::W4 => {
                if value > i32::MAX {
                    return err!(
                        ErrorType::InvalidNumber(format!(
                            "{} is out of range for W4 (must be between {} and {})",
                            value,
                            i32::MIN,
                            i32::MAX
                        )),
                        Range(location, location)
                    );
                }
                if value < i32::MIN {
                    return err!(
                        ErrorType::InvalidNumber(format!(
                            "{} is out of range for W4 (must be between {} and {})",
                            value,
                            i32::MIN,
                            i32::MAX
                        )),
                        Range(location, location)
                    );
                }
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
fn read_num(
    first: char,
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
) -> Result<i32, Error> {
    let mut c = src.peek();

    if first == '0' {
        match c {
            Some('b') => return read_n_num(src, current_location, 2),
            Some('x') => return read_n_num(src, current_location, 16),
            _ => {}
        }
    }

    let mut num = String::new();
    num.push(first);

    loop {
        let Some(n) = c else {
            break;
        };
        if !n.is_ascii_digit() {
            break;
        }
        num.push(*n);
        next(src, current_location);
        c = src.peek();
    }
    Ok(num.parse().unwrap())
}

fn read_identifier(
    char: char,
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
    tokens: &mut Vec<Token>,
) {
    let start = *current_location;
    let mut identifier = String::new();
    identifier.push(char);
    let mut c = src.peek();

    loop {
        let Some(a) = c else {
            break;
        };
        if !a.is_alphanumeric() && *a != '_' {
            break;
        }
        identifier.push(*a);
        next(src, current_location);
        c = src.peek();
    }
    let len = identifier.len() as u16;
    tokens.push(T::with_len(keyword(identifier), start, len));
}

#[allow(dead_code)]
fn read_n_num(
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
    radix: u32,
) -> Result<i32, Error> {
    let start = *current_location;
    next(src, current_location);
    let mut c = src.peek();
    let mut num = String::new();

    loop {
        let Some(n) = c else {
            break;
        };
        if !n.is_ascii_hexdigit() {
            break;
        }
        num.push(*n);
        next(src, current_location);
        c = src.peek();
    }
    u32::from_str_radix(num.as_str(), radix).map_or_else(
        |_| {
            err!(
                ErrorType::InvalidNumber(num),
                Range(start, *current_location)
            )
        },
        |u| Ok(u as i32),
    )
}

#[cfg(test)]
mod test {

    use std::iter::once;

    use crate::{
        frontend::{EqualityOperator, Lexer, LexerTarget, Operator, TokenType},
        Error,
    };

    fn token_types(code: &str, target: LexerTarget) -> Result<Vec<TokenType>, Error> {
        let lexer = Lexer::new(target);
        Ok(lexer.tokenize(code)?.into_iter().map(|t| t.typ).collect())
    }

    #[test]
    fn numbers_w4() {
        let code = "0  1  3  -17  0b1011 0xffff -0b101";
        let expected: Vec<_> = [0, 1, 3, -17, 11, 65535, -5]
            .into_iter()
            .map(TokenType::Number)
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::W4).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn numbers_redstone_clamped() {
        let code = "0  1  3  -17  0b1011 0xffff -0b101";
        let expected: Vec<_> = [0, 1, 3, -17, 11, 65535, -5]
            .into_iter()
            .map(TokenType::Number)
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::Redstone).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn numbers_redstone_out_of_range() {
        let code = "65536";
        let lexer = Lexer::new(LexerTarget::Redstone);
        let result = lexer.tokenize(code);
        assert!(result.is_err());
    }

    #[test]
    fn operators() {
        use Operator::*;
        let code = "+-*&^| + - * &^|";
        let ops = vec![Plus, Minus, Mult, And, Xor, Or];
        let len = ops.len();
        let expected: Vec<_> = ops
            .into_iter()
            .cycle()
            .take(len * 2)
            .map(TokenType::BinaryOperator)
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn equals() {
        use EqualityOperator::*;
        let code = "= == != >= <= > <";
        let ops = vec![EqualTo, NotEqual, GreaterEq, LessEq, Greater, Less];
        let expected: Vec<_> = once(TokenType::Equals)
            .chain(ops.into_iter().map(TokenType::EqOperator))
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn iop() {
        use Operator::*;
        let code = "+= -= *= &= ^= |=";
        let ops = vec![Plus, Minus, Mult, And, Xor, Or];
        let expected: Vec<_> = ops
            .into_iter()
            .map(TokenType::IOperator)
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn numbers() {
        let code = "0  1  3  -17  0b1011 0xffff -0b101";
        let expected: Vec<_> = [0, 1, 3, -17, 11, 65535, -5]
            .into_iter()
            .map(TokenType::Number)
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::W4).expect("Code to compile");
        assert_eq!(expected, ast);
    }
}
