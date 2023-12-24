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
        body: Box<Statement>,
        paths: Vec<(Expression, Box<Statement>)>,
        alternate: Option<Box<Statement>>,
    },
    Pass,
}

#[derive(Debug)]
pub enum Expression {
    BinaryExpr {
        left: Box<Expression>,
        right: Box<Expression>,
        operator: Operator,
    },
    EqExpr {
        left: Box<Expression>,
        right: Box<Expression>,
        operator: EqualityOperator,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EqualityOperator {
    EqualTo,
    NotEqual,
    Greater,
    GreaterEq,
    Less,
    LessEq,
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

pub static EQ_OPERATORS: Lazy<HashMap<(char, bool), EqualityOperator>> = Lazy::new(|| {
    use EqualityOperator::*;
    let mut map = HashMap::new();
    map.insert(('>', true), GreaterEq);
    map.insert(('>', false), Greater);
    map.insert(('<', true), LessEq);
    map.insert(('<', false), Less);
    map.insert(('!', true), NotEqual);
    map
});
