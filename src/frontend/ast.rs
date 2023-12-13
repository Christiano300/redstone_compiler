#[derive(Debug)]
pub enum Statement {
    Program {
        body: Vec<Code>,
    },
    InlineDeclaration {
        symbol: String,
        value: Expression,
    },
    Conditional {
        condition: Expression,
        body: Box<Code>,
        paths: Vec<(Expression, Box<Code>)>,
        alternate: Option<Box<Code>>,
    },
}

#[derive(Debug)]
pub enum Expression {
    BinaryExpr {
        left: Box<Expression>,
        right: Box<Expression>,
        operator: Operator,
    },
    Identifier(String),
    NumericLiteral(i16),
    Assignment {
        symbol: String,
        value: Box<Expression>,
    },
    Member {
        object: Box<Expression>,
        property: String,
    },
    Call {
        args: Vec<Expression>,
        function: Box<Expression>,
    },
}

#[derive(Debug)]
pub enum Code {
    Expr(Expression),
    Stmt(Statement),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Operator {
    Plus,
    Minus,
    Mult,
    And,
    Or,
    Xor,
}

use std::collections::HashMap;

use once_cell::sync::Lazy;

pub static OPERATORS: Lazy<HashMap<char, Operator>> = Lazy::new(|| {
    use Operator::*;
    let mut map = HashMap::new();
    map.insert('+', Plus);
    map.insert('-', Minus);
    map.insert('*', Mult);
    map.insert('&', And);
    map.insert('|', Or);
    map.insert('^', Xor);
    map
});
