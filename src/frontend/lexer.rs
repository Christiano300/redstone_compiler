use std::iter::Peekable;

use super::{eq_operator, operator, EqualityOperator as EqOp, Location, Operator, Range};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenType {
    Number(i16),
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
    Use,
    Var,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub typ: TokenType,
    pub location: Range,
}

impl Token {
    fn from_char(typ: TokenType, location: Location) -> Self {
        Self {
            typ,
            location: Range::single_char(location),
        }
    }

    fn with_len(typ: TokenType, location: Location, len: u16) -> Self {
        Self {
            typ,
            location: Range(location, Location(location.0, location.1 + len - 1)),
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
            '\n' => *location = Location(location.0 + 1, 0),
            '\r' => {}
            _ => location.1 += 1,
        }
    }
    n
}

use Token as T;
use TokenType as Tt;

/// Transform source code into Tokens
///
/// # Errors
///
/// This function will return an error if there is an invalid character
pub fn tokenize(source_code: &str) -> Result<Vec<Token>, String> {
    let mut tokens: Vec<Token> = vec![];
    let mut src = source_code.chars().peekable();
    let mut current_location = Location(0, 0);
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
            '-' => tokens.push(match src.peek() {
                None => T::from_char(Tt::BinaryOperator(Operator::Minus), current_location),
                Some(c) => match c {
                    '=' => {
                        let t = T::with_len(Tt::IOperator(Operator::Minus), current_location, 2);
                        next(&mut src, &mut current_location);
                        t
                    }
                    '0'..='9' => {
                        let start = current_location;
                        let num = -read_num(next(&mut src, &mut current_location).ok_or("Unexpected end")?, &mut src, &mut current_location)?;
                        T {
                            typ: Tt::Number(num),
                            location: Range(start, current_location),
                        }
                    }
                    _ => T::from_char(Tt::BinaryOperator(Operator::Minus), current_location),
                },
            }),
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
            '#' => while next(&mut src, &mut current_location) != Some('\n') {},
            '\t' => return Err(format!("Please indent using spaces, tabs break the errors, found tab at {current_location:?}")),
            _ => {
                if char.is_ascii_digit() {
                    let start = current_location;
                    let num = read_num(char, &mut src, &mut current_location)?;
                    tokens.push(T {
                        typ: Tt::Number(num),
                        location: Range(start, current_location),
                    });
                } else if char.is_alphabetic() {
                    read_identifier(char, &mut src, &mut current_location, &mut tokens);
                } else if !is_skippable(char) {
                    return Err(format!("Unrecognized Character found: {char:?}"));
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

fn read_num(
    first: char,
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
) -> Result<i16, String> {
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

fn read_n_num(
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
    radix: u32,
) -> Result<i16, String> {
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
    u16::from_str_radix(num.as_str(), radix).map_or_else(
        |_| Err(format!("Invalid hex number at {current_location:?}")),
        |u| Ok(u as i16),
    )
}
