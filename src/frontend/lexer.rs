use super::{eq_operator, operator, EqualityOperator as EqOp, Operator};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
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

fn keyword(string: String) -> Token {
    match string.as_str() {
        "inline" => Token::Inline,
        "if" => Token::If,
        "elif" | "elseif" => Token::Elif,
        "else" => Token::Else,
        "end" => Token::End,
        "forever" => Token::Forever,
        "while" => Token::While,
        "pass" => Token::Pass,
        "use" => Token::Use,
        "var" => Token::Var,
        "debug" => Token::Debug,
        _ => Token::Identifier(string),
    }
}

const fn is_skippable(c: char) -> bool {
    matches!(c, ' ' | '\n' | '\t' | '\r' | ';')
}

use Token as T;

/// Transform source code into Tokens
///
/// # Errors
///
/// This function will return an error if there is an invalid character
pub fn tokenize(source_code: &str) -> Result<Vec<Token>, String> {
    let mut tokens = vec![];

    let mut src = source_code.chars().peekable();

    let Some(mut char) = src.next() else {
        return Ok(vec![]);
    };
    let mut prev = ' ';
    loop {
        match char {
            '(' => tokens.push(if prev.is_whitespace() | is_skippable(prev) {
                T::OpenParen
            } else {
                T::OpenFuncParen
            }),
            ')' => tokens.push(T::CloseParen),
            '+' | '-' | '*' | '&' | '|' | '^' => {
                let equals_after = matches!(src.peek(), Some('='));

                if let Some(operator) = operator(char) {
                    tokens.push(if equals_after {
                        T::IOperator(operator)
                    } else {
                        T::BinaryOperator(operator)
                    });
                }

                if equals_after {
                    src.next();
                }
            }
            ',' => tokens.push(T::Comma),
            '.' => tokens.push(T::Dot),

            '=' => match src.peek() {
                Some('=') => {
                    src.next();
                    tokens.push(T::EqOperator(EqOp::EqualTo));
                }
                _ => tokens.push(T::Equals),
            },
            '>' | '<' | '!' => {
                let equals_after = matches!(src.peek(), Some('='));

                if let Some(token) = eq_operator(char, equals_after) {
                    tokens.push(T::EqOperator(token));
                    src.next();
                }
            }
            '#' => while src.next() != Some('\n') {},
            _ => {
                if char.is_ascii_digit() {
                    let mut num = String::new();
                    num.push(char);
                    let mut c = src.peek();

                    loop {
                        let Some(n) = c else {
                            break;
                        };
                        if !n.is_ascii_digit() {
                            break;
                        }
                        num.push(*n);
                        src.next();
                        c = src.peek();
                    }
                    tokens.push(T::Number(num.parse::<i16>().unwrap_or(0)));
                } else if char.is_alphabetic() {
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
                        src.next();
                        c = src.peek();
                    }

                    tokens.push(keyword(identifier));
                } else if !is_skippable(char) {
                    return Err(format!("Unrecognized Character found: {char:?}"));
                }
            }
        }
        prev = char;
        char = match src.next() {
            Some(c) => c,
            None => break,
        };
    }
    tokens.push(T::Eof);

    Ok(tokens)
}
