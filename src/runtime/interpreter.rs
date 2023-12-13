use crate::frontend::*;
use crate::runtime::*;

pub fn evaluate(ast_node: Code, env: &mut Environment) -> ValueType {
    match ast_node {
        Code::Expr(expr) => evaluate_expression(expr, env),
        Code::Stmt(stmt) => evalueate_statement(stmt, env),
    }
}

// TODO: maybe rewrite using &mut ValueType
fn evaluate_expression(expression: Expression, env: &mut Environment) -> ValueType {
    match expression {
        Expression::NumericLiteral(value) => ValueType::Number(value),
        Expression::BinaryExpr {
            left,
            right,
            operator,
        } => ValueType::Number({
            let ValueType::Number(l) = evaluate_expression(*left, env);
            let ValueType::Number(r) = evaluate_expression(*right, env);
            match operator {
                Operator::Plus => l + r,
                Operator::Minus => l - r,
                Operator::Mult => l * r,
                _ => unimplemented!(),
            }
        }),
        Expression::Identifier(name) => *env.lookup(&name),
        Expression::Assignment { symbol, value } => {
            let value = evaluate_expression(*value, env);
            env.assign(symbol, value)
        }
        _ => unimplemented!(),
    }
}

fn evalueate_statement(statement: Statement, env: &mut Environment) -> ValueType {
    match statement {
        Statement::Program { body } => {
            let mut return_value = ValueType::Number(0);
            for ele in body {
                return_value = evaluate(ele, env);
            }
            return_value
        }
        Statement::InlineDeclaration { symbol, value } => {
            // treat inline like normal in interpreter
            let value = evaluate_expression(value, env);
            env.assign(symbol, value)
        }
        _ => unimplemented!(),
    }
}
