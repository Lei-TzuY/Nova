use crate::hir::{Expression, ExpressionKind};
use nova_parser::ast::{BinaryOperator, UnaryOperator};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConstantIntError {
    Overflow,
    ZeroDivisor,
}

pub(crate) fn evaluate_unary(
    operator: UnaryOperator,
    operand: &Expression,
) -> Option<Result<i64, ConstantIntError>> {
    let operand = evaluate(operand)?;
    match operator {
        UnaryOperator::Negate => Some(operand.and_then(|value| {
            value.checked_neg().ok_or(ConstantIntError::Overflow)
        })),
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

fn evaluate(expression: &Expression) -> Option<Result<i64, ConstantIntError>> {
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

fn apply_binary(
    operator: BinaryOperator,
    left: i64,
    right: i64,
) -> Result<i64, ConstantIntError> {
    match operator {
        BinaryOperator::Add => left.checked_add(right).ok_or(ConstantIntError::Overflow),
        BinaryOperator::Subtract => left.checked_sub(right).ok_or(ConstantIntError::Overflow),
        BinaryOperator::Multiply => left.checked_mul(right).ok_or(ConstantIntError::Overflow),
        BinaryOperator::Divide => {
            classify_divisor(left, right)?;
            Ok(left / right)
        }
        BinaryOperator::Remainder => {
            classify_divisor(left, right)?;
            Ok(left % right)
        }
        _ => unreachable!("constant Int evaluator only applies arithmetic operators"),
    }
}

fn classify_divisor(left: i64, right: i64) -> Result<(), ConstantIntError> {
    if right == 0 {
        return Err(ConstantIntError::ZeroDivisor);
    }
    if left == i64::MIN && right == -1 {
        return Err(ConstantIntError::Overflow);
    }
    Ok(())
}
