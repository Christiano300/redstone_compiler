use core::panic;
use std::collections::VecDeque;
use std::fmt::Debug;

use crate::frontend::*;

#[derive(Default)]
pub struct Parser {
    tokens: VecDeque<Token>,
}

macro_rules! match_fn {
    ($pattern:pat $(if $guard:expr)? $(,)?) => {
        |value| match value {
            $pattern $(if $guard)? => true,
            _ => false
        }
    };
}

impl Parser {
    pub fn new() -> Self {
        Default::default()
    }

    fn eat(&mut self) -> Token {
        self.tokens.pop_front().unwrap()
    }

    fn at(&self) -> &Token {
        self.tokens.front().unwrap()
    }

    fn eat_if<F, E>(&mut self, validator: F, err: E) -> Token
    where
        F: Fn(&Token) -> bool,
        E: Debug,
    {
        let token = self.eat();
        if !validator(&token) {
            panic!("{:?}", err);
        }
        token
    }

    pub fn produce_ast(&mut self, source_code: String) -> Code {
        let tokens = tokenize(source_code);
        self.tokens = VecDeque::from(tokens);

        let mut body = vec![];

        while !self.tokens.is_empty() && *self.at() != Token::Eof {
            body.push(self.parse_statement());
        }
        Code::Stmt(Statement::Program { body })
    }

    fn parse_statement(&mut self) -> Code {
        match self.at() {
            Token::Inline => Code::Stmt(self.parse_inline_declaration()),
            Token::If => Code::Stmt(self.parse_conditional()),
            _ => Code::Expr(self.parse_expression()),
        }
    }

    fn parse_conditional(&mut self) -> Statement {
        self.eat();
        let (condition, body) = self.parse_conditional_branch();
        // self.at is now elif, else or end

        match self.at() {
            Token::Elif => (),
            Token::Else => (),
            Token::End => (),
            _ => unreachable!(),
        };
    }

    fn parse_conditional_branch(&mut self) -> (Expression, Statement) {
        let condition = self.parse_expression();
        let mut body = vec![];
        while !matches!(self.at(), Token::Elif | Token::Else | Token::End) {
            body.push(self.parse_statement());
        }
        (condition, Statement::Program { body })
    }

    fn parse_inline_declaration(&mut self) -> Statement {
        self.eat();
        let Token::Identifier(identifier) = self.eat_if(
            match_fn!(Token::Identifier { .. }),
            "'inline' can only be followed by an identifier",
        ) else { unreachable!("this should not happen") };

        self.eat_if(
            match_fn!(Token::Equals),
            "expected equals following identifier in inline declaration",
        );

        Statement::InlineDeclaration {
            symbol: identifier,
            value: self.parse_expression(),
        }
    }

    fn parse_expression(&mut self) -> Expression {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Expression {
        let left = self.parse_additive();

        if matches!(self.at(), Token::Equals) {
            if !matches!(left, Expression::Identifier(..)) {
                panic!("can only assign to identifiers")
            }
            let Expression::Identifier(name) = left else {unreachable!()};
            self.eat();
            let value = self.parse_assignment();
            return Expression::Assignment {
                symbol: name,
                value: Box::new(value),
            };
        }

        left
    }

    fn parse_additive(&mut self) -> Expression {
        let mut left = self.parse_multiplicative();

        let mut operator = Operator::Plus; // default, gets overwritten

        while {
            match self.at() {
                Token::BinaryOperator(op) => {
                    operator = *op;
                    *op == Operator::Plus || *op == Operator::Minus
                }
                _ => false,
            }
        } {
            self.eat();
            let right = self.parse_multiplicative();
            left = Expression::BinaryExpr {
                left: Box::from(left),
                right: Box::from(right),
                operator,
            };
        }

        left
    }

    fn parse_multiplicative(&mut self) -> Expression {
        let mut left = self.parse_call_member();

        let mut operator = Operator::Plus; // default, gets overwritten

        while {
            match self.at() {
                Token::BinaryOperator(op) => {
                    operator = *op;
                    *op == Operator::Mult
                }
                _ => false,
            }
        } {
            self.eat();
            let right = self.parse_call_member();
            left = Expression::BinaryExpr {
                left: Box::from(left),
                right: Box::from(right),
                operator,
            };
        }

        left
    }

    fn parse_call_member(&mut self) -> Expression {
        let member = self.parse_member();

        if matches!(self.at(), Token::OpenParen) {
            return self.parse_call(member);
        }
        member
    }

    fn parse_call(&mut self, caller: Expression) -> Expression {
        let args = self.parse_args();

        if matches!(self.at(), Token::OpenParen) {
            panic!("no function chaining");
        }

        Expression::Call {
            args,
            function: Box::new(caller),
        }
    }

    fn parse_args(&mut self) -> Vec<Expression> {
        self.eat_if(match_fn!(Token::OpenParen), "Expected '('");

        let args = if matches!(self.at(), Token::CloseParen) {
            vec![]
        } else {
            self.parse_arguments_list()
        };

        self.eat_if(match_fn!(Token::CloseParen), "Missing closing_paren");

        args
    }

    fn parse_arguments_list(&mut self) -> Vec<Expression> {
        let mut args = vec![self.parse_expression()];

        while matches!(self.at(), Token::Comma) {
            self.eat();
            args.push(self.parse_expression());
        }
        args
    }

    fn parse_member(&mut self) -> Expression {
        let mut object = self.parse_primary();

        while matches!(self.at(), Token::Dot) {
            self.eat();
            let property = self.parse_primary();

            if !matches!(property, Expression::Identifier(..)) {
                panic!("Cannot use dot operator on whatever you typed");
            }
            let Expression::Identifier(property) = property else {unreachable!()};

            object = Expression::Member {
                object: Box::new(object),
                property,
            }
        }

        object
    }

    fn parse_primary(&mut self) -> Expression {
        let token = self.eat();

        match token {
            Token::Identifier(name) => Expression::Identifier(name),
            Token::Number(value) => Expression::NumericLiteral(value),
            Token::OpenParen => {
                let value = self.parse_expression();
                self.eat_if(
                    match_fn!(Token::CloseParen),
                    "unexpected token (expected closing paren)",
                );
                value
            }
            Token::Eof => panic!("Unexpected EOF while parsing"),
            _ => {
                panic!("Unexpected token found while parsing! {:?}", self.at())
            }
        }
    }
}
