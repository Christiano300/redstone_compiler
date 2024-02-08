use super::{eq_operator, operator, EqualityOperator as EqOp, Operator};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Number(i16),
    Identifier(String),
    Equals,
    OpenParen,
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
    Pass,
    Use,
    Eof,
}

fn keyword(string: String) -> Token {
    match string.as_str() {
        "inline" => Token::Inline,
        "if" => Token::If,
        "elif" | "elseif" => Token::Elif,
        "else" => Token::Else,
        "end" => Token::End,
        "pass" => Token::Pass,
        "use" => Token::Use,
        _ => Token::Identifier(string),
    }
}

const fn is_skippable(src: char) -> bool {
    matches!(src, ' ' | '\n' | '\t' | '\r' | ';')
}

use Token as T;

pub fn tokenize(source_code: &str) -> Result<Vec<Token>, String> {
    let mut tokens = vec![];

    let mut src = source_code.chars().peekable();

    loop {
        let char = src.next();
        if char.is_none() {
            break;
        }
        let Some(char) = char else { unreachable!() };

        match char {
            '(' => tokens.push(T::OpenParen),
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
    }
    tokens.push(T::Eof);

    Ok(tokens)
}
