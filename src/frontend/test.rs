mod lexer_tests {

    use std::iter::once;

    use crate::frontend::{tokenize, EqualityOperator, Operator, Token};

    #[test]
    fn operators() {
        let code = "+-*&^| + - * &^|";
        use Operator::*;
        let ops = vec![Plus, Minus, Mult, And, Xor, Or];
        let len = ops.len();
        let expected: Vec<_> = ops
            .into_iter()
            .cycle()
            .take(len * 2)
            .map(Token::BinaryOperator)
            .chain(once(Token::Eof))
            .collect();
        assert_eq!(Ok(expected), tokenize(code.to_string()))
    }

    #[test]
    fn equals() {
        let code = "= == != >= <= > <";
        use EqualityOperator::*;
        let ops = vec![EqualTo, NotEqual, GreaterEq, LessEq, Greater, Less];
        let expected: Vec<_> = once(Token::Equals)
            .chain(ops.into_iter().map(Token::EqOperator))
            .chain(once(Token::Eof))
            .collect();
        assert_eq!(Ok(expected), tokenize(code.to_string()))
    }

    #[test]
    fn iop() {
        let code = "+= -= *= &= ^= |=";
        use Operator::*;
        let ops = vec![Plus, Minus, Mult, And, Xor, Or];
        let expected: Vec<_> = ops
            .into_iter()
            .map(Token::IOperator)
            .chain(once(Token::Eof))
            .collect();
        assert_eq!(Ok(expected), tokenize(code.to_string()))
    }
}
