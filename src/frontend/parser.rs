use std::collections::VecDeque;

use super::{tokenize, EqualityOperator, Expression, Operator, Token};

#[derive(Default)]
pub struct Parser {
    tokens: VecDeque<Token>,
}

type Res<T = Expression> = Result<T, String>;

macro_rules! match_fn {
    ($pattern:pat $(if $guard:expr)? $(,)?) => {
        |value| match value {
            $pattern $(if $guard)? => true,
            _ => false
        }
    };
}

impl Parser {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn eat(&mut self) -> Res<Token> {
        self.tokens
            .pop_front()
            .ok_or_else(|| "Ran out of tokens".to_string())
    }

    fn at(&self) -> Res<&Token> {
        self.tokens
            .front()
            .ok_or_else(|| "Ran out of tokens".to_string())
    }

    fn eat_if<F>(&mut self, validator: F, err: &str) -> Res<Token>
    where
        F: Fn(&Token) -> bool,
    {
        let token = self.eat()?;
        if !validator(&token) {
            return Err(err.to_string());
        }
        Ok(token)
    }

    /// Transform source code into an AST
    ///
    /// # Errors
    ///
    /// when any error occurs
    pub fn produce_ast(&mut self, source_code: &str) -> Res {
        let tokens = tokenize(source_code)?;
        self.tokens = VecDeque::from(tokens);

        let mut body = vec![];

        while !self.tokens.is_empty() && *self.at()? != Token::Eof {
            body.push(self.parse_statement()?);
        }
        Ok(Expression::Program(body))
    }

    fn parse_statement(&mut self) -> Res {
        Ok(match self.at()? {
            Token::Inline => self.parse_inline_declaration()?,
            Token::If => self.parse_conditional()?,
            Token::Pass => {
                self.eat()?;
                Expression::Pass
            }
            Token::Use => self.parse_use_statement()?,
            Token::Var => self.parse_var_declaration()?,
            Token::Forever => self.parse_endless()?,
            Token::While => self.parse_while()?,
            _ => self.parse_expression()?,
        })
    }

    fn parse_conditional(&mut self) -> Res {
        self.eat()?;
        let (condition, body) = self.parse_conditional_branch()?;
        // self.at is now elif, else or end
        let mut paths = vec![];

        while matches!(self.at()?, Token::Elif) {
            self.eat()?;
            paths.push(self.parse_conditional_branch()?);
        }

        let alternate = if matches!(self.at()?, Token::Else) {
            Some({
                self.eat()?;
                let mut body = vec![];
                while !matches!(self.at()?, Token::End) {
                    body.push(self.parse_statement()?);
                }
                if body.is_empty() {
                    return Err("Cannot have empty block. Use 'pass'".to_string());
                }
                body
            })
        } else {
            None
        };

        self.eat_if(match_fn!(Token::End), "you need to end with an end keyword")?;
        Ok(Expression::Conditional {
            condition: Box::new(condition),
            body,
            paths,
            alternate,
        })
    }

    fn parse_conditional_branch(&mut self) -> Res<(Expression, Vec<Expression>)> {
        let condition = self.parse_expression()?;
        let mut body = vec![];
        while !matches!(self.at()?, Token::Elif | Token::Else | Token::End) {
            body.push(self.parse_statement()?);
        }
        if body.is_empty() {
            return Err("Cannot have empty block. Use 'pass'".to_string());
        }
        Ok((condition, body))
    }

    fn parse_endless(&mut self) -> Res {
        use Token as T;
        self.eat()?;
        let mut body = vec![];
        while !matches!(self.at()?, T::End) {
            body.push(self.parse_statement()?);
        }
        self.eat()?;
        if body.is_empty() {
            return Err("Cannot have empty block. Use 'pass'".to_string());
        }
        Ok(Expression::EndlessLoop { body })
    }

    fn parse_while(&mut self) -> Res {
        use Token as T;
        self.eat()?;
        let condition = self.parse_expression()?;
        let mut body = vec![];
        while !matches!(self.at()?, T::End) {
            body.push(self.parse_statement()?);
        }
        self.eat()?;
        if body.is_empty() {
            return Err("Cannot have empty block. Use 'pass'".to_string());
        }
        Ok(Expression::WhileLoop {
            condition: Box::from(condition),
            body,
        })
    }

    fn parse_use_statement(&mut self) -> Res {
        use Token as T;
        self.eat()?;
        match self.eat()? {
            T::Identifier(symbol) => Ok(Expression::Use(symbol)),
            T::Number(value) => {
                if value == 17 {
                    return Ok(Expression::Use(" ".to_string()));
                }
                Err("Invalid module".to_string())
            }
            _ => Err("Invalid module".to_string()),
        }
    }

    fn parse_var_declaration(&mut self) -> Res {
        use Token as T;
        self.eat()?;
        match self.eat()? {
            T::Identifier(symbol) => Ok(Expression::VarDeclaration { symbol }),
            _ => Err("Invalid variable declaration".to_string()),
        }
    }

    fn parse_inline_declaration(&mut self) -> Res {
        self.eat()?;
        let Token::Identifier(identifier) = self.eat_if(
            match_fn!(Token::Identifier { .. }),
            "'inline' can only be followed by an identifier",
        )?
        else {
            unreachable!()
        };

        self.eat_if(
            match_fn!(Token::Equals),
            "expected equals following identifier in inline declaration",
        )?;

        Ok(Expression::InlineDeclaration {
            symbol: identifier,
            value: Box::new(self.parse_expression()?),
        })
    }

    fn parse_expression(&mut self) -> Res {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Res {
        let left = self.parse_i_assignment()?;

        if matches!(self.at()?, Token::Equals) {
            if !matches!(left, Expression::Identifier(..)) {
                return Err("can only assign to identifiers".to_string());
            }
            let Expression::Identifier(name) = left else {
                unreachable!()
            };
            self.eat()?;
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                symbol: name,
                value: Box::new(value),
            });
        }

        Ok(left)
    }

    fn parse_i_assignment(&mut self) -> Res {
        let left = self.parse_additive()?;

        if matches!(self.at()?, Token::IOperator(_)) {
            if !matches!(left, Expression::Identifier(..)) {
                return Err("can only assign to identifiers".to_string());
            }
            let Expression::Identifier(name) = left else {
                return Err("can only assign to identifiers".to_string());
            };
            let Token::IOperator(operator) = self.eat()? else {
                unreachable!()
            };
            let value = self.parse_i_assignment()?;
            return Ok(Expression::Assignment {
                symbol: name.clone(),
                value: Box::new(Expression::BinaryExpr {
                    left: Box::new(Expression::Identifier(name)),
                    right: Box::new(value),
                    operator,
                }),
            });
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Res {
        let mut left = self.parse_multiplicative()?;

        let mut operator = Operator::Plus; // default, gets overwritten

        while {
            match self.at()? {
                Token::BinaryOperator(op) => {
                    operator = *op;
                    *op == Operator::Plus || *op == Operator::Minus
                }
                _ => false,
            }
        } {
            self.eat()?;
            let right = self.parse_multiplicative()?;
            left = Expression::BinaryExpr {
                left: Box::from(left),
                right: Box::from(right),
                operator,
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Res {
        let mut left = self.parse_eq_expression()?;

        let mut operator = Operator::Plus; // default, gets overwritten

        while {
            match self.at()? {
                Token::BinaryOperator(op) => {
                    operator = *op;
                    *op == Operator::Mult
                }
                _ => false,
            }
        } {
            self.eat()?;
            let right = self.parse_eq_expression()?;
            left = Expression::BinaryExpr {
                left: Box::from(left),
                right: Box::from(right),
                operator,
            };
        }

        Ok(left)
    }

    fn parse_eq_expression(&mut self) -> Res {
        let mut left = self.parse_call_member()?;

        let mut operator = EqualityOperator::EqualTo; // default, gets overwritten

        while {
            match self.at()? {
                Token::EqOperator(op) => {
                    operator = *op;
                    true
                }
                _ => false,
            }
        } {
            self.eat()?;
            let right = self.parse_call_member()?;
            left = Expression::EqExpr {
                left: Box::from(left),
                right: Box::from(right),
                operator,
            };
        }

        Ok(left)
    }

    fn parse_call_member(&mut self) -> Res {
        let member = self.parse_member()?;

        if matches!(self.at()?, Token::OpenFuncParen) {
            return self.parse_call(member);
        }
        Ok(member)
    }

    fn parse_call(&mut self, caller: Expression) -> Res {
        let args = self.parse_args()?;

        if matches!(self.at()?, Token::OpenFuncParen) {
            return Err("no function chaining".to_string());
        }

        Ok(Expression::Call {
            args,
            function: Box::new(caller),
        })
    }

    fn parse_args(&mut self) -> Result<Vec<Expression>, String> {
        self.eat_if(match_fn!(Token::OpenParen), "Expected '('")?;

        let args = if matches!(self.at()?, Token::CloseParen) {
            vec![]
        } else {
            self.parse_arguments_list()?
        };

        self.eat_if(match_fn!(Token::CloseParen), "Missing closing_paren")?;

        Ok(args)
    }

    fn parse_arguments_list(&mut self) -> Result<Vec<Expression>, String> {
        let mut args = vec![self.parse_expression()?];

        while matches!(self.at()?, Token::Comma) {
            self.eat()?;
            args.push(self.parse_expression()?);
        }
        Ok(args)
    }

    fn parse_member(&mut self) -> Res {
        let mut object = self.parse_primary()?;

        while matches!(self.at()?, Token::Dot) {
            self.eat()?;
            let property = self.parse_primary()?;

            if !matches!(property, Expression::Identifier(..)) {
                return Err("Cannot use dot operator on whatever you typed".to_string());
            }
            let Expression::Identifier(property) = property else {
                unreachable!()
            };

            object = Expression::Member {
                object: Box::new(object),
                property,
            }
        }

        Ok(object)
    }

    fn parse_primary(&mut self) -> Res {
        let token = self.eat()?;

        Ok(match token {
            Token::Identifier(name) => Expression::Identifier(name),
            Token::Number(value) => Expression::NumericLiteral(value),
            Token::Debug => Expression::Debug,
            Token::OpenParen => {
                let value = self.parse_expression()?;
                self.eat_if(
                    match_fn!(Token::CloseParen),
                    "unexpected token (expected closing paren)",
                )?;
                value
            }
            Token::Eof => return Err("Unexpected EOF while parsing".to_string()),
            _ => panic!(
                "{}",
                format!("Unexpected token found while parsing: {token:?}")
            ),
        })
    }
}
