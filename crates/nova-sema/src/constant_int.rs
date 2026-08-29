use crate::hir::{Expression, ExpressionKind};
use nova_parser::ast::{BinaryOperator, UnaryOperator};

pub(crate) use nova_int_semantics::IntArithmeticError as ConstantIntError;

pub(crate) fn evaluate_unary(
    operator: UnaryOperator,
    operand: &Expression,
) -> Option<Result<i64, ConstantIntError>> {
    let operand = evaluate(operand)?;
    match operator {
        UnaryOperator::Negate => Some(operand.and_then(nova_int_semantics::negate)),
        UnaryOperator::Not => None,
    }
}

pub(crate) fn evaluate_binary(
    operator: BinaryOperator,
    left: &Expression,
    right: &Expression,
) -> Option<Result<i64, ConstantIntError>> {
    if !matches!(
        operator,
        BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder
    ) {
        return None;
    }

    let left = evaluate(left)?;
    let right = evaluate(right)?;
    Some(match (left, right) {
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(left), Ok(right)) => apply_binary(operator, left, right),
    })
}

pub(crate) fn evaluate(expression: &Expression) -> Option<Result<i64, ConstantIntError>> {
    match &expression.kind {
        ExpressionKind::Integer(value) => Some(Ok(*value)),
        ExpressionKind::Unary { operator, operand } => evaluate_unary(*operator, operand),
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => evaluate_binary(*operator, left, right),
        _ => None,
    }
}

fn apply_binary(operator: BinaryOperator, left: i64, right: i64) -> Result<i64, ConstantIntError> {
    match operator {
        BinaryOperator::Add => nova_int_semantics::add(left, right),
        BinaryOperator::Subtract => nova_int_semantics::subtract(left, right),
        BinaryOperator::Multiply => nova_int_semantics::multiply(left, right),
        BinaryOperator::Divide => nova_int_semantics::divide(left, right),
        BinaryOperator::Remainder => nova_int_semantics::remainder(left, right),
        _ => unreachable!("constant Int evaluator only dispatches arithmetic operators"),
    }
}
