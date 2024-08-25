use super::Range;

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Ident {
    pub symbol: String,
    pub location: Range,
}

#[derive(Debug, Default)]
pub enum ExpressionType {
    InlineDeclaration {
        ident: Ident,
        value: Box<Expression>,
    },
    Use(String),
    Conditional {
        condition: Box<Expression>,
        body: Vec<Expression>,
        paths: Vec<(Expression, Vec<Expression>)>,
        alternate: Option<Vec<Expression>>,
    },
    EndlessLoop {
        body: Vec<Expression>,
    },
    WhileLoop {
        condition: Box<Expression>,
        body: Vec<Expression>,
    },
    #[default]
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
        ident: Ident,
        value: Box<Expression>,
    },
    IAssignment {
        ident: Ident,
        value: Box<Expression>,
        operator: Operator,
    },
    VarDeclaration {
        ident: Ident,
    },
    Member {
        object: Box<Expression>,
        property: Ident,
    },
    Call {
        args: Vec<Expression>,
        function: Box<Expression>,
    },
    Debug,
}

pub struct Expression {
    pub typ: ExpressionType,
    pub location: Range,
}

impl std::fmt::Debug for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{:#?} at {:?}", self.typ, self.location)
        } else {
            write!(f, "{:?} at {:?}", self.typ, self.location)
        }
    }
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

impl Operator {
    #[inline]
    #[must_use]
    pub const fn is_commutative(self) -> bool {
        !matches!(self, Self::Minus)
    }
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

impl EqualityOperator {
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::EqualTo => Self::NotEqual,
            Self::NotEqual => Self::EqualTo,
            Self::Greater => Self::LessEq,
            Self::GreaterEq => Self::Less,
            Self::Less => Self::GreaterEq,
            Self::LessEq => Self::Greater,
        }
    }

    #[must_use]
    pub const fn turnaround(self) -> Self {
        match self {
            Self::EqualTo => Self::EqualTo,
            Self::NotEqual => Self::NotEqual,
            Self::Greater => Self::Less,
            Self::GreaterEq => Self::LessEq,
            Self::Less => Self::Greater,
            Self::LessEq => Self::GreaterEq,
        }
    }
}

#[must_use]
pub const fn operator(symbol: char) -> Option<Operator> {
    use Operator as O;
    match symbol {
        '+' => Some(O::Plus),
        '-' => Some(O::Minus),
        '*' => Some(O::Mult),
        '&' => Some(O::And),
        '|' => Some(O::Or),
        '^' => Some(O::Xor),
        _ => None,
    }
}

#[must_use]
pub const fn eq_operator(symbol: char, eq_after: bool) -> Option<EqualityOperator> {
    use EqualityOperator as EO;
    match (symbol, eq_after) {
        ('>', true) => Some(EO::GreaterEq),
        ('>', false) => Some(EO::Greater),
        ('<', true) => Some(EO::LessEq),
        ('<', false) => Some(EO::Less),
        ('!', true) => Some(EO::NotEqual),
        _ => None,
    }
}
