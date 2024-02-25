mod lexer_tests {

    use std::iter::once;

    use crate::frontend::{tokenize, EqualityOperator, Operator, TokenType};

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
        let ast: Vec<TokenType> = tokenize(code).unwrap().into_iter().map(|t| t.typ).collect();
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
        let ast: Vec<TokenType> = tokenize(code).unwrap().into_iter().map(|t| t.typ).collect();
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
        let ast: Vec<TokenType> = tokenize(code).unwrap().into_iter().map(|t| t.typ).collect();
        assert_eq!(expected, ast);
    }
}
