use once_cell::sync::Lazy;

use super::{EqualityOperator as EqOp, Operator, EQ_OPERATORS, OPERATORS};
use std::collections::HashMap;

static KEYWORDS: Lazy<HashMap<String, Token>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("inline".to_string(), Token::Inline);
    map.insert("if".to_string(), Token::If);
    map.insert("elif".to_string(), Token::Elif);
    map.insert("elseif".to_string(), Token::Elif);
    map.insert("else".to_string(), Token::Else);
    map.insert("end".to_string(), Token::End);
    map.insert("pass".to_string(), Token::Pass);
    map.insert("use".to_string(), Token::Use);
    map
});

#[derive(Clone, Debug, PartialEq)]
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

fn is_skippable(src: char) -> bool {
    matches!(src, ' ' | '\n' | '\t' | '\r' | ';')
}

use Token as T;

pub fn tokenize(source_code: String) -> Result<Vec<Token>, String> {
    let mut tokens = vec![];

    let mut src = source_code.chars().peekable();

    loop {
        let char = src.next();
        if char.is_none() {
            break;
        }
        let char = char.unwrap();

        match char {
            '(' => tokens.push(T::OpenParen),
            ')' => tokens.push(T::CloseParen),
            '+' | '-' | '*' | '&' | '|' | '^' => {
                let equals_after = matches!(src.peek(), Some('='));

                let operator = *OPERATORS.get(&char).unwrap();
                tokens.push(if equals_after {
                    T::IOperator(operator)
                } else {
                    T::BinaryOperator(operator)
                });

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

                if let Some(token) = EQ_OPERATORS.get(&(char, equals_after)) {
                    tokens.push(T::EqOperator(*token));
                    src.next();
                }
            }
            _ => {
                if char.is_ascii_digit() {
                    let mut num = String::new();
                    num.push(char);
                    let mut c = src.peek();

                    while c.is_some() && c.unwrap().is_ascii_digit() {
                        num.push(*c.unwrap());
                        src.next();
                        c = src.peek();
                    }
                    tokens.push(T::Number(num.parse::<i16>().unwrap_or(0)));
                } else if char.is_alphabetic() {
                    let mut identifier = String::new();
                    identifier.push(char);
                    let mut c = src.peek();

                    while c.is_some() && (c.unwrap().is_alphanumeric() || *c.unwrap() == '_') {
                        identifier.push(*c.unwrap());
                        src.next();
                        c = src.peek();
                    }

                    tokens.push(if KEYWORDS.contains_key(&identifier) {
                        KEYWORDS.get(&identifier).unwrap().clone()
                    } else {
                        T::Identifier(identifier)
                    });
                } else if !is_skippable(char) {
                    return Err(format!("Unrecognized Character found: {:?}", char));
                }
            }
        }
    }
    tokens.push(T::Eof);

    Ok(tokens)
}
