#[derive(Debug)]
pub enum Expression {
    Program(Vec<Expression>),
    InlineDeclaration {
        symbol: String,
        value: Box<Expression>,
    },
    Use(String),
    Conditional {
        condition: Box<Expression>,
        body: Vec<Expression>,
        paths: Vec<(Expression, Vec<Expression>)>,
        alternate: Option<Vec<Expression>>,
    },
    Pass,
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
    use Operator as O;
    let mut map = HashMap::new();
    map.insert('+', O::Plus);
    map.insert('-', O::Minus);
    map.insert('*', O::Mult);
    map.insert('&', O::And);
    map.insert('|', O::Or);
    map.insert('^', O::Xor);
    map
});

pub static EQ_OPERATORS: Lazy<HashMap<(char, bool), EqualityOperator>> = Lazy::new(|| {
    use EqualityOperator as EO;
    let mut map = HashMap::new();
    map.insert(('>', true), EO::GreaterEq);
    map.insert(('>', false), EO::Greater);
    map.insert(('<', true), EO::LessEq);
    map.insert(('<', false), EO::Less);
    map.insert(('!', true), EO::NotEqual);
    map
});
