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
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match crate::constant_condition::evaluate(condition)? {
            true if then_branch.statements.is_empty() => evaluate(then_branch.tail.as_deref()?),
            true => None,
            false => evaluate(else_branch),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let (scrutinee_enum, variant_index) = enum_tag(scrutinee)?;
            if scrutinee_enum != *enumeration {
                return None;
            }

            let mut selected = arms.iter().filter(|arm| arm.variant_index == variant_index);
            let arm = selected.next()?;
            if selected.next().is_some() {
                return None;
            }
            evaluate(&arm.value)
        }
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            evaluate(block.tail.as_deref()?)
        }
        _ => None,
    }
}

fn enum_tag(expression: &Expression) -> Option<(crate::hir::EnumId, usize)> {
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } if payload.is_none() => Some((*enumeration, *variant_index)),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            enum_tag(block.tail.as_deref()?)
        }
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
