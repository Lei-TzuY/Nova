use crate::constant_condition::{ClosedBinding, ClosedConditionArithmeticFailure};
use crate::hir::{Block, Expression, ExpressionKind};
use nova_parser::ast::{BinaryOperator, UnaryOperator};

pub(crate) use nova_int_semantics::IntArithmeticError as ConstantIntError;

pub(crate) type CheckedConstantIntProof =
    Result<Option<Result<i64, ConstantIntError>>, ClosedConditionArithmeticFailure>;

pub(crate) fn evaluate_unary_checked(
    operator: UnaryOperator,
    operand: &Expression,
) -> CheckedConstantIntProof {
    evaluate_unary_checked_with_bindings(operator, operand, &[])
}

fn evaluate_unary_checked_with_bindings<'a>(
    operator: UnaryOperator,
    operand: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> CheckedConstantIntProof {
    let Some(operand) = evaluate_checked_with_bindings(operand, bindings)? else {
        return Ok(None);
    };
    Ok(match operator {
        UnaryOperator::Negate => Some(operand.and_then(nova_int_semantics::negate)),
        UnaryOperator::Not => None,
    })
}

pub(crate) fn evaluate_binary_checked(
    operator: BinaryOperator,
    left: &Expression,
    right: &Expression,
) -> CheckedConstantIntProof {
    evaluate_binary_checked_with_bindings(operator, left, right, &[])
}

fn evaluate_binary_checked_with_bindings<'a>(
    operator: BinaryOperator,
    left: &'a Expression,
    right: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> CheckedConstantIntProof {
    if !matches!(
        operator,
        BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder
    ) {
        return Ok(None);
    }

    let Some(left) = evaluate_checked_with_bindings(left, bindings)? else {
        return Ok(None);
    };
    let Some(right) = evaluate_checked_with_bindings(right, bindings)? else {
        return Ok(None);
    };
    Ok(Some(match (left, right) {
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(left), Ok(right)) => apply_binary(operator, left, right),
    }))
}

pub(crate) fn evaluate_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<Result<i64, ConstantIntError>> {
    evaluate_checked_with_bindings(expression, bindings)
        .ok()
        .flatten()
}

pub(crate) fn evaluate_checked_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> CheckedConstantIntProof {
    match &expression.kind {
        ExpressionKind::Integer(value) => Ok(Some(Ok(*value))),
        ExpressionKind::Binding(reference) => {
            let Some(value) = crate::constant_condition::closed_binding_value(
                reference,
                &expression.ty,
                bindings,
            ) else {
                return Ok(None);
            };
            evaluate_checked_with_bindings(value, bindings)
        }
        ExpressionKind::Unary { operator, operand } => {
            evaluate_unary_checked_with_bindings(*operator, operand, bindings)
        }
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => evaluate_binary_checked_with_bindings(*operator, left, right, bindings),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            match crate::constant_condition::evaluate_checked_with_bindings(condition, bindings)? {
                Some(true) => evaluate_block_checked_with_bindings(then_branch, bindings),
                Some(false) => evaluate_checked_with_bindings(else_branch, bindings),
                None => Ok(None),
            }
        }
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let Some((value, selected_bindings)) =
                crate::constant_condition::selected_match_value_checked_with_bindings(
                    scrutinee,
                    *enumeration,
                    arms,
                    bindings,
                )?
            else {
                return Ok(None);
            };
            evaluate_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let Some((value, selected_bindings)) =
                crate::constant_condition::selected_record_field_value_checked_with_bindings(
                    base,
                    *record,
                    *field_index,
                    bindings,
                )?
            else {
                return Ok(None);
            };
            evaluate_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) => evaluate_block_checked_with_bindings(block, bindings),
        _ => Ok(None),
    }
}

fn evaluate_block_checked_with_bindings<'a>(
    block: &'a Block,
    bindings: &[ClosedBinding<'a>],
) -> CheckedConstantIntProof {
    let Some(proof) = crate::constant_condition::closed_block_tail_with_bindings(block, bindings)
    else {
        return Ok(None);
    };
    match proof {
        Ok((Some(tail), selected_bindings)) => {
            evaluate_checked_with_bindings(tail, &selected_bindings)
        }
        Ok((None, _)) => Ok(None),
        Err(failure) => Err(failure),
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
