use std::collections::VecDeque;

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

    fn eat_if<F>(&mut self, validator: F, err: &str) -> Result<Token, String>
    where
        F: Fn(&Token) -> bool,
    {
        let token = self.eat();
        if !validator(&token) {
            return Err(err.to_string());
        }
        Ok(token)
    }

    pub fn produce_ast(&mut self, source_code: String) -> Result<Code, String> {
        let tokens = tokenize(source_code)?;
        self.tokens = VecDeque::from(tokens);

        let mut body = vec![];

        while !self.tokens.is_empty() && *self.at() != Token::Eof {
            body.push(self.parse_statement()?);
        }
        Ok(Code::Stmt(Statement::Program { body }))
    }

    fn parse_statement(&mut self) -> Result<Code, String> {
        Ok(match self.at() {
            Token::Inline => Code::Stmt(self.parse_inline_declaration()?),
            Token::If => Code::Stmt(self.parse_conditional()?),
            Token::Pass => {
                self.eat();
                Code::Stmt(Statement::Pass)
            }
            _ => Code::Expr(self.parse_expression()?),
        })
    }

    fn parse_conditional(&mut self) -> Result<Statement, String> {
        self.eat();
        let (condition, body) = self.parse_conditional_branch()?;
        // self.at is now elif, else or end
        let mut paths = vec![];

        while matches!(self.at(), Token::Elif) {
            self.eat();
            paths.push(self.parse_conditional_branch()?);
        }

        let alternate = if matches!(self.at(), Token::Else) {
            Some(Box::new({
                self.eat();
                let mut body = vec![];
                while !matches!(self.at(), Token::End) {
                    body.push(self.parse_statement()?);
                }
                if body.is_empty() {
                    return Err("Cannot have empty block. Use 'pass'".to_string());
                }
                Statement::Program { body }
            }))
        } else {
            None
        };

        self.eat_if(match_fn!(Token::End), "you need to end with an end keyword")?;
        Ok(Statement::Conditional {
            condition,
            body,
            paths,
            alternate,
        })
    }

    fn parse_conditional_branch(&mut self) -> Result<(Expression, Box<Statement>), String> {
        let condition = self.parse_expression()?;
        let mut body = vec![];
        while !matches!(self.at(), Token::Elif | Token::Else | Token::End) {
            body.push(self.parse_statement()?);
        }
        if body.is_empty() {
            return Err("Cannot have empty block. Use 'pass'".to_string());
        }
        Ok((condition, Box::new(Statement::Program { body })))
    }

    fn parse_inline_declaration(&mut self) -> Result<Statement, String> {
        self.eat();
        let Token::Identifier(identifier) = self.eat_if(
            match_fn!(Token::Identifier { .. }),
            "'inline' can only be followed by an identifier",
        )? else { unreachable!() };

        self.eat_if(
            match_fn!(Token::Equals),
            "expected equals following identifier in inline declaration",
        )?;

        Ok(Statement::InlineDeclaration {
            symbol: identifier,
            value: self.parse_expression()?,
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expression, String> {
        let left = self.parse_additive()?;

        if matches!(self.at(), Token::Equals) {
            if !matches!(left, Expression::Identifier(..)) {
                return Err("can only assign to identifiers".to_string());
            }
            let Expression::Identifier(name) = left else {unreachable!()};
            self.eat();
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                symbol: name,
                value: Box::new(value),
            });
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative()?;

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
            let right = self.parse_multiplicative()?;
            left = Expression::BinaryExpr {
                left: Box::from(left),
                right: Box::from(right),
                operator,
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_eq_expression()?;

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
            let right = self.parse_eq_expression()?;
            left = Expression::BinaryExpr {
                left: Box::from(left),
                right: Box::from(right),
                operator,
            };
        }

        Ok(left)
    }

    fn parse_eq_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_call_member()?;

        let mut operator = EqualityOperator::EqualTo; // default, gets overwritten

        while {
            match self.at() {
                Token::EqOperator(op) => {
                    operator = *op;
                    true
                }
                _ => false,
            }
        } {
            self.eat();
            let right = self.parse_call_member()?;
            left = Expression::EqExpr {
                left: Box::from(left),
                right: Box::from(right),
                operator,
            };
        }

        Ok(left)
    }
    fn parse_call_member(&mut self) -> Result<Expression, String> {
        let member = self.parse_member()?;

        if matches!(self.at(), Token::OpenParen) {
            return self.parse_call(member);
        }
        Ok(member)
    }

    fn parse_call(&mut self, caller: Expression) -> Result<Expression, String> {
        let args = self.parse_args()?;

        if matches!(self.at(), Token::OpenParen) {
            return Err("no function chaining".to_string());
        }

        Ok(Expression::Call {
            args,
            function: Box::new(caller),
        })
    }

    fn parse_args(&mut self) -> Result<Vec<Expression>, String> {
        self.eat_if(match_fn!(Token::OpenParen), "Expected '('")?;

        let args = if matches!(self.at(), Token::CloseParen) {
            vec![]
        } else {
            self.parse_arguments_list()?
        };

        self.eat_if(match_fn!(Token::CloseParen), "Missing closing_paren")?;

        Ok(args)
    }

    fn parse_arguments_list(&mut self) -> Result<Vec<Expression>, String> {
        let mut args = vec![self.parse_expression()?];

        while matches!(self.at(), Token::Comma) {
            self.eat();
            args.push(self.parse_expression()?);
        }
        Ok(args)
    }

    fn parse_member(&mut self) -> Result<Expression, String> {
        let mut object = self.parse_primary()?;

        while matches!(self.at(), Token::Dot) {
            self.eat();
            let property = self.parse_primary()?;

            if !matches!(property, Expression::Identifier(..)) {
                return Err("Cannot use dot operator on whatever you typed".to_string());
            }
            let Expression::Identifier(property) = property else {unreachable!()};

            object = Expression::Member {
                object: Box::new(object),
                property,
            }
        }

        Ok(object)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let token = self.eat();

        Ok(match token {
            Token::Identifier(name) => Expression::Identifier(name),
            Token::Number(value) => Expression::NumericLiteral(value),
            Token::OpenParen => {
                let value = self.parse_expression()?;
                self.eat_if(
                    match_fn!(Token::CloseParen),
                    "unexpected token (expected closing paren)",
                )?;
                value
            }
            Token::Eof => return Err("Unexpected EOF while parsing".to_string()),
            _ => {
                return Err(format!(
                    "Unexpected token found while parsing! {:?}",
                    self.at()
                ))
            }
        })
    }
}
