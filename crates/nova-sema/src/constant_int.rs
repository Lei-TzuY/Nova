use crate::constant_condition::ClosedBinding;
use crate::hir::{Expression, ExpressionKind};
use nova_parser::ast::{BinaryOperator, UnaryOperator};

pub(crate) use nova_int_semantics::IntArithmeticError as ConstantIntError;

pub(crate) fn evaluate_unary(
    operator: UnaryOperator,
    operand: &Expression,
) -> Option<Result<i64, ConstantIntError>> {
    evaluate_unary_with_bindings(operator, operand, &[])
}

fn evaluate_unary_with_bindings<'a>(
    operator: UnaryOperator,
    operand: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<Result<i64, ConstantIntError>> {
    let operand = evaluate_with_bindings(operand, bindings)?;
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
    evaluate_binary_with_bindings(operator, left, right, &[])
}

fn evaluate_binary_with_bindings<'a>(
    operator: BinaryOperator,
    left: &'a Expression,
    right: &'a Expression,
    bindings: &[ClosedBinding<'a>],
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

    let left = evaluate_with_bindings(left, bindings)?;
    let right = evaluate_with_bindings(right, bindings)?;
    Some(match (left, right) {
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(left), Ok(right)) => apply_binary(operator, left, right),
    })
}

pub(crate) fn evaluate(expression: &Expression) -> Option<Result<i64, ConstantIntError>> {
    evaluate_with_bindings(expression, &[])
}

pub(crate) fn evaluate_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<Result<i64, ConstantIntError>> {
    match &expression.kind {
        ExpressionKind::Integer(value) => Some(Ok(*value)),
        ExpressionKind::Binding(reference) => evaluate_with_bindings(
            crate::constant_condition::closed_binding_value(reference, &expression.ty, bindings)?,
            bindings,
        ),
        ExpressionKind::Unary { operator, operand } => {
            evaluate_unary_with_bindings(*operator, operand, bindings)
        }
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => evaluate_binary_with_bindings(*operator, left, right, bindings),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match crate::constant_condition::evaluate_with_bindings(condition, bindings)? {
            true if then_branch.statements.is_empty() => {
                evaluate_with_bindings(then_branch.tail.as_deref()?, bindings)
            }
            true => None,
            false => evaluate_with_bindings(else_branch, bindings),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let (value, selected_bindings) =
                crate::constant_condition::selected_match_value_with_bindings(
                    scrutinee,
                    *enumeration,
                    arms,
                    bindings,
                )?;
            evaluate_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => evaluate_with_bindings(
            crate::constant_condition::selected_record_field_value(base, *record, *field_index)?,
            bindings,
        ),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            evaluate_with_bindings(block.tail.as_deref()?, bindings)
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
