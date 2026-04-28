use std::collections::VecDeque;

use crate::{
    error::Error,
    frontend::{DataValue, ErrorType, Fragment, Range, Statement, Stmt},
};

use super::{EqualityOperator, Expr, Expression, Ident, Operator, Token, TokenType};

#[derive(Default)]
pub struct Parser {
    tokens: VecDeque<Token>,
}

type Res<T = Expression, E = Error> = Result<T, E>;

type ResStmt<E = Error> = Res<Statement, E>;

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

    fn eat(&mut self) -> Token {
        self.tokens.pop_front().expect("Eof before Stream ends")
    }

    fn at(&self) -> &Token {
        self.tokens.front().expect("Eof before Stream ends")
    }

    fn eat_if_or<F>(&mut self, validator: F, err: ErrorType, location: Range) -> Res<Token>
    where
        F: Fn(&TokenType) -> bool,
    {
        let token = self.eat();
        if !validator(&token.typ) {
            return Err(Error {
                typ: Box::new(err),
                location,
            });
        }
        Ok(token)
    }

    fn eat_if<F>(&mut self, validator: F, err: ErrorType) -> Res<Token>
    where
        F: Fn(&TokenType) -> bool,
    {
        let token = self.eat();
        if !validator(&token.typ) {
            return Err(Error {
                typ: Box::new(err),
                location: token.location,
            });
        }
        Ok(token)
    }

    /// Transform source code into an AST
    ///
    /// # Errors
    ///
    /// when any error occurs
    pub fn produce_ast(&mut self, tokens: Vec<Token>) -> Res<Vec<Statement>, Vec<Error>> {
        self.tokens = VecDeque::from(tokens);

        let mut body = vec![];
        let mut errors = vec![];

        while !self.tokens.is_empty() && self.at().typ != TokenType::Eof {
            match self.parse_statement() {
                Ok(expr) => body.push(expr),
                Err(err) => errors.push(err),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(body)
    }

    fn parse_statement(&mut self) -> ResStmt {
        let current = self.at();
        Ok(match current.typ {
            TokenType::Inline => self.parse_inline_declaration()?,
            TokenType::Data => self.parse_data_declaration()?,
            TokenType::If => self.parse_conditional()?,
            TokenType::Pass => {
                let token = self.eat();
                Statement {
                    typ: Stmt::Pass,
                    location: token.location,
                }
            }
            TokenType::Use => self.parse_use_statement()?,
            TokenType::Var => self.parse_var_declaration()?,
            TokenType::Forever => self.parse_endless()?,
            TokenType::While => self.parse_while()?,
            TokenType::Fun => self.parse_function_declaration()?,
            _ => {
                let expr = self.parse_expression()?;
                Statement {
                    typ: Stmt::Expr(expr.typ),
                    location: expr.location,
                }
            }
        })
    }

    fn parse_conditional(&mut self) -> ResStmt {
        let start = self.eat().location;
        let (condition, body) = self.parse_conditional_branch()?;
        // self.at is now elif, else or end
        let mut paths = vec![];

        while matches!(self.at().typ, TokenType::Elif | TokenType::Eof) {
            self.eat();
            paths.push(self.parse_conditional_branch()?);
        }

        let alternate = if matches!(self.at().typ, TokenType::Else) {
            Some({
                self.eat();
                let mut body = vec![];
                while !matches!(self.at().typ, TokenType::End | TokenType::Eof) {
                    body.push(self.parse_statement()?);
                }
                if body.is_empty() {
                    return err!(EmptyBlock, start + self.at().location);
                }
                body
            })
        } else {
            None
        };

        let end = self
            .eat_if_or(match_fn!(TokenType::End), ErrorType::MissingEnd, start)?
            .location;
        Ok(Statement {
            typ: Stmt::Conditional {
                condition: Box::new(condition),
                body,
                paths,
                alternate,
            },
            location: start + end,
        })
    }

    fn parse_conditional_branch(&mut self) -> Res<(Expression, Fragment)> {
        let condition = self.parse_expression()?;
        let start = self.at().location;
        let mut body = vec![];
        while !matches!(
            self.at().typ,
            TokenType::Elif | TokenType::Else | TokenType::End | TokenType::Eof
        ) {
            body.push(self.parse_statement()?);
        }
        if body.is_empty() {
            return err!(EmptyBlock, start + self.at().location);
        }
        Ok((condition, body))
    }

    fn parse_endless(&mut self) -> ResStmt {
        use TokenType as T;
        let start = self.eat().location;
        let mut body = vec![];
        while !matches!(self.at().typ, T::End | T::Eof) {
            body.push(self.parse_statement()?);
        }
        let end = self
            .eat_if_or(match_fn!(TokenType::End), ErrorType::MissingEnd, start)?
            .location;
        if body.is_empty() {
            return err!(EmptyBlock, start + self.at().location);
        }
        Ok(Statement {
            typ: Stmt::EndlessLoop { body },
            location: start + end,
        })
    }

    fn parse_while(&mut self) -> ResStmt {
        use TokenType as T;
        let start = self.eat().location;
        let condition = self.parse_expression()?;
        let mut body = vec![];
        while !matches!(self.at().typ, T::End | T::Eof) {
            body.push(self.parse_statement()?);
        }
        let end = self.eat_if_or(match_fn!(T::End), ErrorType::MissingEnd, start)?;
        if body.is_empty() {
            return err!(EmptyBlock, start + self.at().location);
        }
        Ok(Statement {
            typ: Stmt::WhileLoop {
                condition: Box::from(condition),
                body,
            },
            location: start + end.location,
        })
    }

    fn parse_use_statement(&mut self) -> ResStmt {
        use TokenType as T;
        let start = self.eat().location;
        let token = self.eat();
        let mut imports = vec1::vec1!(match token.typ {
            T::Identifier(symbol) => Ident {
                symbol,
                location: token.location,
            },
            _ => return err!(InvalidModuleName, token.location),
        });
        while matches!(self.at().typ, T::Comma) {
            self.eat();
            let token = self.eat();
            match token.typ {
                T::Identifier(symbol) => imports.push(Ident {
                    symbol,
                    location: token.location,
                }),
                _ => return err!(InvalidModuleName, token.location),
            }
        }
        Ok(Statement {
            location: start + imports.last().location,
            typ: Stmt::Use(imports),
        })
    }

    fn parse_var_declaration(&mut self) -> ResStmt {
        use TokenType as T;
        let start = self.eat().location;
        let token = self.eat();
        match token.typ {
            T::Identifier(symbol) => Ok(Statement {
                typ: Stmt::VarDeclaration {
                    ident: Ident {
                        symbol,
                        location: token.location,
                    },
                },
                location: start + token.location,
            }),
            _ => err!(InvalidDeclartion, token.location),
        }
    }

    fn parse_inline_declaration(&mut self) -> ResStmt {
        let start = self.eat().location;
        let token = self.eat();
        let TokenType::Identifier(ident) = token.typ else {
            return err!(InvalidAssignment, token.location);
        };

        self.eat_if(match_fn!(TokenType::Equals), ErrorType::MissingEquals)?;

        let value = self.parse_expression()?;
        let end = value.location;
        Ok(Statement {
            typ: Stmt::InlineDeclaration {
                ident: Ident {
                    symbol: ident,
                    location: token.location,
                },
                value: Box::new(value),
            },
            location: start + end,
        })
    }

    fn parse_data_declaration(&mut self) -> ResStmt {
        let start = self.eat().location;
        let token = self.eat();
        let TokenType::Identifier(ident) = token.typ else {
            return err!(InvalidAssignment, token.location);
        };

        self.eat_if(match_fn!(TokenType::Equals), ErrorType::MissingEquals)?;

        let next_token = self.at();
        let (data, end) = if let TokenType::DataString(..) = next_token.typ {
            let token = self.eat();
            let TokenType::DataString(bytes) = token.typ else {
                unreachable!()
            };
            (DataValue::Bytes(bytes), token.location)
        } else {
            let expr = self.parse_expression()?;
            let end = expr.location;
            (DataValue::Expr(expr), end)
        };
        Ok(Statement {
            typ: Stmt::DataDeclaration {
                ident: Ident {
                    symbol: ident,
                    location: token.location,
                },
                value: data,
            },
            location: start + end,
        })
    }

    fn parse_function_declaration(&mut self) -> ResStmt {
        let start = self.eat().location;
        let token = self.eat();
        let TokenType::Identifier(symbol) = token.typ else {
            return err!(InvalidFunctionName, token.location);
        };
        let mut args = vec![];
        if matches!(
            self.at().typ,
            TokenType::OpenParen | TokenType::OpenFuncParen
        ) {
            self.eat();
            loop {
                let token = self.eat();
                if let TokenType::Identifier(name) = token.typ {
                    args.push(Ident {
                        symbol: name,
                        location: token.location,
                    });
                } else {
                    return err!(InvalidParam, token.location);
                }
                let next = self.eat();
                match next.typ {
                    TokenType::Comma => continue,
                    TokenType::CloseParen => break,
                    _ => return err!(ExpectedParen, next.location),
                }
            }
        }

        let mut body = vec![];
        while !matches!(self.at().typ, TokenType::End | TokenType::Eof) {
            body.push(self.parse_statement()?);
        }
        let end = self.eat_if_or(match_fn!(TokenType::End), ErrorType::MissingEnd, start)?;
        if body.is_empty() {
            return err!(EmptyBlock, start + self.at().location);
        }

        Ok(Statement {
            typ: Stmt::FunctionDeclaration {
                ident: Ident {
                    symbol,
                    location: token.location,
                },
                args,
                body,
            },
            location: start + end.location,
        })
    }

    fn parse_expression(&mut self) -> Res {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Res {
        let left = self.parse_i_assignment()?;

        if matches!(self.at().typ, TokenType::Equals) {
            let Expr::Identifier(name) = left.typ else {
                return err!(InvalidAssignment, self.at().location);
            };
            self.eat();
            let value = self.parse_assignment()?;
            let end = value.location;
            return Ok(Expression {
                typ: Expr::Assignment {
                    ident: Ident {
                        symbol: name,
                        location: left.location,
                    },
                    value: Box::new(value),
                },
                location: left.location + end,
            });
        }

        Ok(left)
    }

    fn parse_i_assignment(&mut self) -> Res {
        let left = self.parse_eq_expression()?;

        if let TokenType::IOperator(operator) = self.at().typ {
            let Expr::Identifier(ref name) = left.typ else {
                return err!(InvalidAssignment, left.location);
            };
            self.eat();
            let value = self.parse_i_assignment()?;
            let location = left.location + value.location;
            return Ok(Expression {
                typ: Expr::IAssignment {
                    ident: Ident {
                        symbol: name.clone(),
                        location: left.location,
                    },
                    value: Box::new(value),
                    operator,
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
            match self.at().typ {
                TokenType::EqOperator(op) => {
                    operator = op;
                    true
                }
                _ => false,
            }
        } {
            self.eat();
            let right = self.parse_additive()?;
            let location = left.location + right.location;
            left = Expression {
                typ: Expr::EqExpr {
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
            match self.at().typ {
                TokenType::BinaryOperator(op) => {
                    operator = op;
                    op == Operator::Plus || op == Operator::Minus
                }
                _ => false,
            }
        } {
            self.eat();
            let right = self.parse_multiplicative()?;
            let location = left.location + right.location;
            left = Expression {
                typ: Expr::BinaryExpr {
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
        let mut left = self.parse_bitop()?;

        let mut operator = Operator::Plus; // default, gets overwritten

        while {
            match self.at().typ {
                TokenType::BinaryOperator(op) => {
                    operator = op;
                    op == Operator::Mult
                }
                _ => false,
            }
        } {
            self.eat();
            let right = self.parse_bitop()?;
            let location = left.location + right.location;
            left = Expression {
                typ: Expr::BinaryExpr {
                    left: Box::from(left),
                    right: Box::from(right),
                    operator,
                },
                location,
            };
        }

        Ok(left)
    }

    fn parse_bitop(&mut self) -> Res {
        let mut left = self.parse_call_member()?;

        let mut operator = Operator::Plus; // default, gets overwritten

        while {
            match self.at().typ {
                TokenType::BinaryOperator(op) => {
                    operator = op;
                    matches!(op, Operator::And | Operator::Or | Operator::Xor)
                }
                _ => false,
            }
        } {
            self.eat();
            let right = self.parse_call_member()?;
            let location = left.location + right.location;
            left = Expression {
                typ: Expr::BinaryExpr {
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

        if matches!(self.at().typ, TokenType::OpenFuncParen) {
            return self.parse_call(member);
        }
        Ok(member)
    }

    fn parse_call(&mut self, caller: Expression) -> Res {
        let (args, end) = self.parse_args()?;

        if matches!(self.at().typ, TokenType::OpenFuncParen) {
            return err!(FunctionChaining, self.at().location);
        }

        let location = caller.location + end;
        Ok(Expression {
            typ: Expr::Call {
                args,
                function: Box::new(caller),
            },
            location,
        })
    }

    fn parse_args(&mut self) -> Result<(Vec<Expression>, Range), Error> {
        let start = self
            .eat_if(
                match_fn!(TokenType::OpenFuncParen),
                ErrorType::MissingOpenParen,
            )?
            .location;

        let args = if matches!(self.at().typ, TokenType::CloseParen) {
            vec![]
        } else {
            self.parse_arguments_list()?
        };

        let end = self
            .eat_if(
                match_fn!(TokenType::CloseParen),
                ErrorType::MissingClosingParen,
            )?
            .location;

        Ok((args, start + end))
    }

    fn parse_arguments_list(&mut self) -> Result<Vec<Expression>, Error> {
        let mut args = vec![self.parse_expression()?];

        while matches!(self.at().typ, TokenType::Comma) {
            self.eat();
            args.push(self.parse_expression()?);
        }
        Ok(args)
    }

    fn parse_member(&mut self) -> Res {
        let mut object = self.parse_primary()?;

        while matches!(self.at().typ, TokenType::Dot) {
            let dot = self.eat().location;
            let property = self.parse_primary()?;

            let Expr::Identifier(name) = property.typ else {
                return err!(InvalidDot, dot);
            };

            let location = object.location + property.location;
            object = Expression {
                typ: Expr::Member {
                    object: Box::new(object),
                    property: Ident {
                        symbol: name,
                        location: property.location,
                    },
                },
                location,
            }
        }

        Ok(object)
    }

    fn parse_primary(&mut self) -> Res {
        let token = self.eat();

        Ok(match token.typ {
            TokenType::Identifier(name) => Expression {
                typ: Expr::Identifier(name),
                location: token.location,
            },
            TokenType::Number(value) => Expression {
                typ: Expr::NumericLiteral(value),
                location: token.location,
            },
            TokenType::Debug => Expression {
                typ: Expr::Debug,
                location: token.location,
            },
            TokenType::OpenParen => {
                let value = self.parse_expression()?;
                self.eat_if(match_fn!(TokenType::CloseParen), ErrorType::ExpectedParen)?;
                value
            }
            TokenType::DataString(..) => return err!(UnexpectedData, token.location),
            TokenType::Eof => return err!(Eof, token.location),
            _ => return err!(UnexpectedOther, token.location),
        })
    }
}
