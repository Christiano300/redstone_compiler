mod lexer_tests {

    fn token_types(code: &str) -> Result<Vec<TokenType>, String> {
        Ok(tokenize(code)?.into_iter().map(|t| t.typ).collect())
    }
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
        let ast = token_types(code);
        assert_eq!(Ok(expected), ast);
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
        let ast = token_types(code);
        assert_eq!(Ok(expected), ast);
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
        let ast = token_types(code);
        assert_eq!(Ok(expected), ast);
    }

    #[test]
    fn numbers() {
        let code = "0  1  3  -17  0b1011 0xffff -0b101";
        let expected: Vec<_> = [0, 1, 3, -17, 11, -1, -5]
            .into_iter()
            .map(TokenType::Number)
            .chain(once(TokenType::Eof))
            .collect();
        let ast = token_types(code);
        assert_eq!(Ok(expected), ast);
    }
}
