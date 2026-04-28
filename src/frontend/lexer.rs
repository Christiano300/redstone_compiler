use crate::error::Error;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::{borrow::Cow, char, fmt::Debug, iter::Peekable};
use z85::decode;

use super::{EqualityOperator as EqOp, Location, Operator, Range, eq_operator, operator};

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
    Data,
    DataString(Vec<u8>),
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
    NonNegNNum,
    Eof,
    InvalidChar(String),
    InvalidStringSigil(char),
    InvalidStringChar(char, &'static str),
    InvalidString(Cow<'static, str>),
}

impl crate::error::ErrorType for ErrorType {
    fn get_message(&self) -> Cow<'_, str> {
        match self {
            Self::InvalidNumber(n) => Cow::from(format!("Invalid number: {n}")),
            Self::NonNegNNum => Cow::from("Numbers in base 2 or 16 cannot be negated"),
            Self::Eof => Cow::from("Unexpected End of file"),
            Self::InvalidChar(c) => Cow::from(format!("Invalid character: {c}")),
            Self::InvalidStringSigil(c) => Cow::from(format!(
                "Invalid data string sigil {c}, allowed values types are a, x, h, b, z"
            )),
            Self::InvalidStringChar(c, format) => {
                Cow::from(format!("'{c}' is not allowed in {format} data strings"))
            }
            Self::InvalidString(msg) => Cow::from(format!("Invalid string: {msg}")),
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
        "data" => TokenType::Data,
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
                        let num = self.read_num(char, &mut src, true, &mut current_location)?;

                        tokens.push(T {
                            typ: Tt::Number(num),
                            location: Range(start, current_location),
                        });
                    } else if char.is_alphabetic() || char == '_' {
                        read_identifier(char, &mut src, &mut current_location, &mut tokens)?;
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
                        false,
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
        allow_n_num: bool,
        current_location: &mut Location,
    ) -> Result<i32, Error> {
        let mut c = src.peek();
        let start = *current_location;

        if first == '0' {
            match c {
                Some('b') => {
                    if !allow_n_num {
                        return err!(NonNegNNum, Range::single_char(*current_location));
                    }
                    return self.read_n_num(src, current_location, 2);
                }
                Some('x') => {
                    if !allow_n_num {
                        return err!(NonNegNNum, Range::single_char(*current_location));
                    }
                    return self.read_n_num(src, current_location, 16);
                }
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
        match self.target {
            LexerTarget::Redstone => num.parse::<i16>().map(|num| num as i32),
            LexerTarget::W4 => num.parse::<i32>(),
        }
        .or(err!(
            ErrorType::InvalidNumber(num),
            Range(start, *current_location)
        ))
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
        match self.target {
            LexerTarget::Redstone => u16::from_str_radix(&num, radix).map(|num| num as i32),
            LexerTarget::W4 => u32::from_str_radix(&num, radix).map(|num| num as i32),
        }
        .or(err!(
            ErrorType::InvalidNumber(num),
            Range(start, *current_location)
        ))
    }
}

fn read_identifier(
    char: char,
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
    tokens: &mut Vec<Token>,
) -> Result<(), Error> {
    let mut c = src.peek();
    if c == Some(&'"') {
        return read_string(char, src, current_location, tokens);
    }
    let start = *current_location;
    let mut identifier = String::new();
    identifier.push(char);

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
    Ok(())
}

fn read_string(
    char: char,
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
    tokens: &mut Vec<Token>,
) -> Result<(), Error> {
    let start = *current_location;
    // Consume the opening quote
    if src.peek() != Some(&'"') {
        return err!(
            ErrorType::InvalidString(Cow::from("Expected opening quote")),
            Range::single_char(*current_location)
        );
    }
    next(src, current_location); // consume opening "
    let data = match char {
        'a' => read_ascii_string(src, current_location)?,
        'x' | 'h' => read_hex_string(src, current_location)?,
        'b' => read_base64_string(src, current_location)?,
        'z' => read_z85_string(src, current_location)?,
        _ => {
            return err!(
                ErrorType::InvalidStringSigil(char),
                Range::single_char(start)
            );
        }
    };
    tokens.push(Token {
        typ: TokenType::DataString(data),
        location: Range(start, *current_location),
    });
    Ok(())
}

fn read_z85_string(
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
) -> Result<Vec<u8>, Error> {
    let mut buf = String::new();
    let start = *current_location;
    loop {
        let Some(c) = next(src, current_location) else {
            break;
        };
        if c == '"' {
            break;
        }
        buf.push(c);
    }
    decode(buf).map_err(|_| Error {
        typ: Box::new(ErrorType::InvalidString(Cow::from("Failed to parse Z85"))),
        location: Range(start, *current_location),
    })
}

fn read_base64_string(
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
) -> Result<Vec<u8>, Error> {
    let mut buf = String::new();
    let start = *current_location;
    loop {
        let Some(c) = next(src, current_location) else {
            break;
        };
        if c == '"' {
            break;
        }
        buf.push(c);
    }
    STANDARD.decode(buf).map_err(|_| Error {
        typ: Box::new(ErrorType::InvalidString(Cow::from(
            "Failed to parse Base 64",
        ))),
        location: Range(start, *current_location),
    })
}

fn read_hex_string(
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
) -> Result<Vec<u8>, Error> {
    let mut buf = String::new();
    let start = *current_location;
    loop {
        let Some(c) = next(src, current_location) else {
            break;
        };
        match c {
            '"' => break,
            c if c.is_ascii_hexdigit() => buf.push(c),
            _ => err!(
                ErrorType::InvalidStringChar(c, "hex"),
                Range::single_char(*current_location)
            )?,
        }
    }
    if buf.len() % 2 != 0 {
        return err!(
            ErrorType::InvalidString(Cow::from("Hex string must have an even number or nibbles")),
            Range(start, *current_location)
        );
    }
    Ok(buf
        .as_bytes()
        .chunks(2)
        .map(|s| str::from_utf8(s).unwrap())
        .map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect())
}

fn read_ascii_string(
    src: &mut Peekable<std::str::Chars<'_>>,
    current_location: &mut Location,
) -> Result<Vec<u8>, Error> {
    let mut buf = vec![];
    loop {
        let Some(c) = next(src, current_location) else {
            break;
        };
        match c {
            '"' => break,
            '\\' => {
                let Some(escaped) = next(src, current_location) else {
                    break;
                };
                let byte = match escaped {
                    't' => '\t' as u8,
                    'n' => '\n' as u8,
                    'r' => '\r' as u8,
                    '0' => '\0' as u8,
                    '\\' => '\\' as u8,
                    'x' => {
                        let Some(a) = next(src, current_location) else {
                            break;
                        };
                        if !a.is_ascii_hexdigit() {
                            return err!(
                                ErrorType::InvalidStringChar(a, "ascii"),
                                Range::single_char(*current_location)
                            );
                        }
                        let Some(b) = next(src, current_location) else {
                            break;
                        };
                        if !b.is_ascii_hexdigit() {
                            return err!(
                                ErrorType::InvalidStringChar(a, "ascii"),
                                Range::single_char(*current_location)
                            );
                        }
                        let mut s = String::with_capacity(2);
                        s.push(a);
                        s.push(b);
                        u8::from_str_radix(&s, 16).unwrap()
                    }
                    _ => err!(
                        ErrorType::InvalidString(Cow::from("Only Hex is allowed in hex excapes")),
                        Range::single_char(*current_location)
                    )?,
                };
                buf.push(byte);
            }
            c if c.is_ascii() => buf.push(c as u8),
            _ => err!(
                ErrorType::InvalidStringChar(c, "ascii"),
                Range::single_char(*current_location)
            )?,
        }
    }
    Ok(buf)
}

#[cfg(test)]
mod test {

    use std::iter::once;

    use crate::{
        Error,
        frontend::{EqualityOperator, Lexer, LexerTarget, Operator, TokenType},
    };

    fn token_types(code: &str, target: LexerTarget) -> Result<Vec<TokenType>, Error> {
        let lexer = Lexer::new(target);
        Ok(lexer.tokenize(code)?.into_iter().map(|t| t.typ).collect())
    }

    #[test]
    fn numbers_w4() {
        let code = "0  1  3 -17 0b1011 0xffff";
        let expected: Vec<_> = [0, 1, 3, -17, 11, 65535]
            .into_iter()
            .map(TokenType::Number)
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::W4).expect("Code to compile");
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
        let code = "0  1  3  -17  0b1011 0xffff";
        let expected: Vec<_> = [0, 1, 3, -17, 11, 65535]
            .into_iter()
            .map(TokenType::Number)
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::W4).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn ascii_data_string() {
        let code = "a\"hello\"";
        let expected: Vec<_> = [TokenType::DataString(vec![b'h', b'e', b'l', b'l', b'o'])]
            .into_iter()
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn ascii_data_string_with_escape() {
        let code = "a\"hello\\nworld\"";
        let expected: Vec<_> = [TokenType::DataString(vec![b'h', b'e', b'l', b'l', b'o', b'\n', b'w', b'o', b'r', b'l', b'd'])]
            .into_iter()
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn ascii_data_string_with_hex_escape() {
        let code = "a\"\\x41\\x42\\x43\"";
        let expected: Vec<_> = [TokenType::DataString(vec![0x41, 0x42, 0x43])]
            .into_iter()
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn hex_data_string() {
        let code = "x\"deadbeef\"";
        let expected: Vec<_> = [TokenType::DataString(vec![0xde, 0xad, 0xbe, 0xef])]
            .into_iter()
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn hex_data_string_invalid_char() {
        let code = "x\"deadzzzz\"";
        let result = token_types(code, LexerTarget::default());
        assert!(result.is_err());
    }

    #[test]
    fn hex_data_string_odd_length() {
        let code = "x\"deadbee\"";
        let result = token_types(code, LexerTarget::default());
        assert!(result.is_err());
    }

    #[test]
    fn base64_data_string() {
        let code = "b\"SGVsbG8=\"";
        let expected: Vec<_> = [TokenType::DataString(b"Hello".to_vec())]
            .into_iter()
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn z85_data_string() {
        let code = "z\"HelloWorld\"";
        let result = token_types(code, LexerTarget::default());
        assert!(result.is_ok());
        if let TokenType::DataString(data) = &result.unwrap()[0] {
            assert!(!data.is_empty());
        } else {
            panic!("Expected DataString token");
        }
    }

    #[test]
    fn empty_ascii_data_string() {
        let code = "a\"\"";
        let expected: Vec<_> = [TokenType::DataString(vec![])]
            .into_iter()
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn empty_hex_data_string() {
        let code = "x\"\"";
        let expected: Vec<_> = [TokenType::DataString(vec![])]
            .into_iter()
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn data_string_followed_by_token() {
        let code = "a\"hello\" world";
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(ast[0], TokenType::DataString(b"hello".to_vec()));
        assert_eq!(ast[1], TokenType::Identifier("world".to_string()));
    }

    #[test]
    fn ascii_string_null_escape() {
        let code = "a\"hello\\0world\"";
        let expected: Vec<_> = [TokenType::DataString(b"hello\0world".to_vec())]
            .into_iter()
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn ascii_string_tab_escape() {
        let code = "a\"hello\\tworld\"";
        let expected: Vec<_> = [TokenType::DataString(b"hello\tworld".to_vec())]
            .into_iter()
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }

    #[test]
    fn base64_invalid() {
        let code = "b\"invalid!!\"";
        let result = token_types(code, LexerTarget::default());
        assert!(result.is_err());
    }

    #[test]
    fn multiple_data_strings() {
        let code = "a\"hello\" x\"ff\"";
        let expected: Vec<_> = [
            TokenType::DataString(b"hello".to_vec()),
            TokenType::DataString(vec![0xff]),
        ]
        .into_iter()
        .chain(once(TokenType::Eof))
        .collect();
        let ast = token_types(code, LexerTarget::default()).expect("Code to compile");
        assert_eq!(expected, ast);
    }
}
