use std::collections::VecDeque;

use crate::frontend::Range;

use super::{EqualityOperator, Expression, ExpressionType, Operator, Token, TokenType};

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
        F: Fn(&TokenType) -> bool,
    {
        let token = self.eat()?;
        if !validator(&token.typ) {
            return Err(err.to_string());
        }
        Ok(token)
    }

    /// Transform source code into an AST
    ///
    /// # Errors
    ///
    /// when any error occurs
    pub fn produce_ast(&mut self, tokens: Vec<Token>) -> Res<Vec<Expression>> {
        self.tokens = VecDeque::from(tokens);

        let mut body = vec![];

        while !self.tokens.is_empty() && self.at()?.typ != TokenType::Eof {
            body.push(self.parse_statement()?);
        }
        Ok(body)
    }

    fn parse_statement(&mut self) -> Res {
        let current = self.at()?;
        Ok(match current.typ {
            TokenType::Inline => self.parse_inline_declaration()?,
            TokenType::If => self.parse_conditional()?,
            TokenType::Pass => {
                let token = self.eat()?;
                Expression {
                    typ: ExpressionType::Pass,
                    location: token.location,
                }
            }
            TokenType::Use => self.parse_use_statement()?,
            TokenType::Var => self.parse_var_declaration()?,
            TokenType::Forever => self.parse_endless()?,
            TokenType::While => self.parse_while()?,
            _ => self.parse_expression()?,
        })
    }

    fn parse_conditional(&mut self) -> Res {
        let start = self.eat()?.location;
        let (condition, body) = self.parse_conditional_branch()?;
        // self.at is now elif, else or end
        let mut paths = vec![];

        while matches!(self.at()?.typ, TokenType::Elif) {
            self.eat()?;
            paths.push(self.parse_conditional_branch()?);
        }

        let alternate = if matches!(self.at()?.typ, TokenType::Else) {
            Some({
                self.eat()?;
                let mut body = vec![];
                while !matches!(self.at()?.typ, TokenType::End) {
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

        let end = self
            .eat_if(
                match_fn!(TokenType::End),
                "you need to end with an end keyword",
            )?
            .location;
        Ok(Expression {
            typ: ExpressionType::Conditional {
                condition: Box::new(condition),
                body,
                paths,
                alternate,
            },
            location: start + end,
        })
    }

    fn parse_conditional_branch(&mut self) -> Res<(Expression, Vec<Expression>)> {
        let condition = self.parse_expression()?;
        let mut body = vec![];
        while !matches!(
            self.at()?.typ,
            TokenType::Elif | TokenType::Else | TokenType::End
        ) {
            body.push(self.parse_statement()?);
        }
        if body.is_empty() {
            return Err("Cannot have empty block. Use 'pass'".to_string());
        }
        Ok((condition, body))
    }

    fn parse_endless(&mut self) -> Res {
        use TokenType as T;
        let start = self.eat()?.location;
        let mut body = vec![];
        while !matches!(self.at()?.typ, T::End) {
            body.push(self.parse_statement()?);
        }
        let end = self
            .eat_if(
                match_fn!(TokenType::End),
                "you need to end with an end keyword",
            )?
            .location;
        if body.is_empty() {
            return Err("Cannot have empty block. Use 'pass'".to_string());
        }
        Ok(Expression {
            typ: ExpressionType::EndlessLoop { body },
            location: start + end,
        })
    }

    fn parse_while(&mut self) -> Res {
        use TokenType as T;
        let start = self.eat()?.location;
        let condition = self.parse_expression()?;
        let mut body = vec![];
        while !matches!(self.at()?.typ, T::End) {
            body.push(self.parse_statement()?);
        }
        let end = self.eat_if(match_fn!(T::End), "you need to end with an end keyword")?;
        if body.is_empty() {
            return Err("Cannot have empty block. Use 'pass'".to_string());
        }
        Ok(Expression {
            typ: ExpressionType::WhileLoop {
                condition: Box::from(condition),
                body,
            },
            location: start + end.location,
        })
    }

    fn parse_use_statement(&mut self) -> Res {
        use TokenType as T;
        let start = self.eat()?.location;
        let token = self.eat()?;
        match token.typ {
            T::Identifier(symbol) => Ok(Expression {
                typ: ExpressionType::Use(symbol),
                location: start + token.location,
            }),
            T::Number(value) => {
                if value == 17 {
                    Ok(Expression {
                        typ: ExpressionType::Use(" ".to_string()),
                        location: start + token.location,
                    })
                } else {
                    Err("Invalid module".to_string())
                }
            }
            _ => Err("Invalid module".to_string()),
        }
    }

    fn parse_var_declaration(&mut self) -> Res {
        use TokenType as T;
        let start = self.eat()?.location;
        let token = self.eat()?;
        match token.typ {
            T::Identifier(symbol) => Ok(Expression {
                typ: ExpressionType::VarDeclaration { symbol },
                location: start + token.location,
            }),
            _ => Err("Invalid variable declaration".to_string()),
        }
    }

    fn parse_inline_declaration(&mut self) -> Res {
        let start = self.eat()?.location;
        let token = self.eat()?;
        let TokenType::Identifier(ident) = token.typ else {
            return Err("'inline' must be followed by an identifier".to_string());
        };

        self.eat_if(
            match_fn!(TokenType::Equals),
            "expected equals following identifier in inline declaration",
        )?;

        let value = self.parse_expression()?;
        let end = value.location;
        Ok(Expression {
            typ: ExpressionType::InlineDeclaration {
                symbol: ident,
                value: Box::new(value),
            },
            location: start + end,
        })
    }

    fn parse_expression(&mut self) -> Res {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Res {
        let left = self.parse_i_assignment()?;

        if matches!(self.at()?.typ, TokenType::Equals) {
            let ExpressionType::Identifier(name) = left.typ else {
                return Err("can only assign to identifiers".to_string());
            };
            self.eat()?;
            let value = self.parse_assignment()?;
            let end = value.location;
            return Ok(Expression {
                typ: ExpressionType::Assignment {
                    symbol: name,
                    value: Box::new(value),
                },
                location: left.location + end,
            });
        }

        Ok(left)
    }

    fn parse_i_assignment(&mut self) -> Res {
        let left = self.parse_eq_expression()?;

        if let TokenType::IOperator(operator) = self.at()?.typ {
            let ExpressionType::Identifier(ref name) = left.typ else {
                return Err("can only assign to identifiers".to_string());
            };
            self.eat()?;
            let value = self.parse_i_assignment()?;
            let location = left.location + value.location;
            return Ok(Expression {
                typ: ExpressionType::Assignment {
                    symbol: name.clone(),
                    value: Box::new(Expression {
                        typ: ExpressionType::BinaryExpr {
                            left: Box::new(left),
                            right: Box::new(value),
                            operator,
                        },
                        location,
                    }),
                },
                location,
            });
        }

        Ok(left)
    }

    fn parse_eq_expression(&mut self) -> Res {
        let mut left = self.parse_additive()?;

        let mut operator = EqualityOperator::EqualTo; // default, gets overwritten

        while {
            match self.at()?.typ {
                TokenType::EqOperator(op) => {
                    operator = op;
                    true
                }
                _ => false,
            }
        } {
            self.eat()?;
            let right = self.parse_additive()?;
            let location = left.location + right.location;
            left = Expression {
                typ: ExpressionType::EqExpr {
                    left: Box::from(left),
                    right: Box::from(right),
                    operator,
                },
                location,
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Res {
        let mut left = self.parse_multiplicative()?;

        let mut operator = Operator::Plus; // default, gets overwritten

        while {
            match self.at()?.typ {
                TokenType::BinaryOperator(op) => {
                    operator = op;
                    op == Operator::Plus || op == Operator::Minus
                }
                _ => false,
            }
        } {
            self.eat()?;
            let right = self.parse_multiplicative()?;
            let location = left.location + right.location;
            left = Expression {
                typ: ExpressionType::BinaryExpr {
                    left: Box::from(left),
                    right: Box::from(right),
                    operator,
                },
                location,
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Res {
        let mut left = self.parse_call_member()?;

        let mut operator = Operator::Plus; // default, gets overwritten

        while {
            match self.at()?.typ {
                TokenType::BinaryOperator(op) => {
                    operator = op;
                    op == Operator::Mult
                }
                _ => false,
            }
        } {
            self.eat()?;
            let right = self.parse_call_member()?;
            let location = left.location + right.location;
            left = Expression {
                typ: ExpressionType::BinaryExpr {
                    left: Box::from(left),
                    right: Box::from(right),
                    operator,
                },
                location,
            };
        }

        Ok(left)
    }

    fn parse_call_member(&mut self) -> Res {
        let member = self.parse_member()?;

        if matches!(self.at()?.typ, TokenType::OpenFuncParen) {
            return self.parse_call(member);
        }
        Ok(member)
    }

    fn parse_call(&mut self, caller: Expression) -> Res {
        let (args, end) = self.parse_args()?;

        if matches!(self.at()?.typ, TokenType::OpenFuncParen) {
            return Err("no function chaining".to_string());
        }

        let location = caller.location + end;
        Ok(Expression {
            typ: ExpressionType::Call {
                args,
                function: Box::new(caller),
            },
            location,
        })
    }

    fn parse_args(&mut self) -> Result<(Vec<Expression>, Range), String> {
        let start = self
            .eat_if(match_fn!(TokenType::OpenFuncParen), "Expected '('")?
            .location;

        let args = if matches!(self.at()?.typ, TokenType::CloseParen) {
            vec![]
        } else {
            self.parse_arguments_list()?
        };

        let end = self
            .eat_if(match_fn!(TokenType::CloseParen), "Missing closing_paren")?
            .location;

        Ok((args, start + end))
    }

    fn parse_arguments_list(&mut self) -> Result<Vec<Expression>, String> {
        let mut args = vec![self.parse_expression()?];

        while matches!(self.at()?.typ, TokenType::Comma) {
            self.eat()?;
            args.push(self.parse_expression()?);
        }
        Ok(args)
    }

    fn parse_member(&mut self) -> Res {
        let mut object = self.parse_primary()?;

        while matches!(self.at()?.typ, TokenType::Dot) {
            self.eat()?;
            let property = self.parse_primary()?;

            let ExpressionType::Identifier(name) = property.typ else {
                return Err("Cannot use dot operator on whatever you typed".to_string());
            };

            let location = object.location + property.location;
            object = Expression {
                typ: ExpressionType::Member {
                    object: Box::new(object),
                    property: name,
                },
                location,
            }
        }

        Ok(object)
    }

    fn parse_primary(&mut self) -> Res {
        let token = self.eat()?;

        Ok(match token.typ {
            TokenType::Identifier(name) => Expression {
                typ: ExpressionType::Identifier(name),
                location: token.location,
            },
            TokenType::Number(value) => Expression {
                typ: ExpressionType::NumericLiteral(value),
                location: token.location,
            },
            TokenType::Debug => Expression {
                typ: ExpressionType::Debug,
                location: token.location,
            },
            TokenType::OpenParen => {
                let value = self.parse_expression()?;
                self.eat_if(
                    match_fn!(TokenType::CloseParen),
                    "unexpected token (expected closing paren)",
                )?;
                value
            }
            TokenType::Eof => return Err("Unexpected EOF while parsing".to_string()),
            _ => return Err(format!("Unexpected token found while parsing: {token:?}")),
        })
    }
}
