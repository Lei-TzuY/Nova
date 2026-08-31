use crate::constant_condition::ClosedBinding;
use crate::hir::{
    Binding, BindingReference, Block, Expression, ExpressionKind, StatementKind, Type,
};
use nova_parser::ast::{BinaryOperator, UnaryOperator};

pub(crate) use nova_int_semantics::IntArithmeticError as ConstantIntError;

#[derive(Clone, Copy)]
struct ClosedIntBinding<'a> {
    binding: &'a Binding,
    value: &'a Expression,
}

pub(crate) fn evaluate_unary(
    operator: UnaryOperator,
    operand: &Expression,
) -> Option<Result<i64, ConstantIntError>> {
    evaluate_unary_with_context(operator, operand, &[], &[])
}

fn evaluate_unary_with_context<'a>(
    operator: UnaryOperator,
    operand: &'a Expression,
    bindings: &[ClosedBinding<'a>],
    int_bindings: &[ClosedIntBinding<'a>],
) -> Option<Result<i64, ConstantIntError>> {
    let operand = evaluate_with_context(operand, bindings, int_bindings)?;
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
    evaluate_binary_with_context(operator, left, right, &[], &[])
}

fn evaluate_binary_with_context<'a>(
    operator: BinaryOperator,
    left: &'a Expression,
    right: &'a Expression,
    bindings: &[ClosedBinding<'a>],
    int_bindings: &[ClosedIntBinding<'a>],
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

    let left = evaluate_with_context(left, bindings, int_bindings)?;
    let right = evaluate_with_context(right, bindings, int_bindings)?;
    Some(match (left, right) {
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(left), Ok(right)) => apply_binary(operator, left, right),
    })
}

pub(crate) fn evaluate_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<Result<i64, ConstantIntError>> {
    evaluate_with_context(expression, bindings, &[])
}

fn evaluate_with_context<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
    int_bindings: &[ClosedIntBinding<'a>],
) -> Option<Result<i64, ConstantIntError>> {
    match &expression.kind {
        ExpressionKind::Integer(value) => Some(Ok(*value)),
        ExpressionKind::Binding(reference) => {
            let value = closed_int_binding_value(reference, &expression.ty, int_bindings).or_else(
                || {
                    crate::constant_condition::closed_binding_value(
                        reference,
                        &expression.ty,
                        bindings,
                    )
                },
            )?;
            evaluate_with_context(value, bindings, int_bindings)
        }
        ExpressionKind::Unary { operator, operand } => {
            evaluate_unary_with_context(*operator, operand, bindings, int_bindings)
        }
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => evaluate_binary_with_context(*operator, left, right, bindings, int_bindings),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match crate::constant_condition::evaluate_with_bindings(condition, bindings)? {
            true => evaluate_block_with_context(then_branch, bindings, int_bindings),
            false => evaluate_with_context(else_branch, bindings, int_bindings),
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
            evaluate_with_context(value, &selected_bindings, int_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let (value, selected_bindings) =
                crate::constant_condition::selected_record_field_value_with_bindings(
                    base,
                    *record,
                    *field_index,
                    bindings,
                )?;
            evaluate_with_context(value, &selected_bindings, int_bindings)
        }
        ExpressionKind::Block(block) => evaluate_block_with_context(block, bindings, int_bindings),
        _ => None,
    }
}

fn evaluate_block_with_context<'a>(
    block: &'a Block,
    bindings: &[ClosedBinding<'a>],
    int_bindings: &[ClosedIntBinding<'a>],
) -> Option<Result<i64, ConstantIntError>> {
    let mut block_bindings = int_bindings.to_vec();
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding {
                binding,
                initializer,
            } if !binding.mutable && binding.ty == Type::Int && initializer.ty == Type::Int => {
                match evaluate_with_context(initializer, bindings, &block_bindings)? {
                    Ok(_) => block_bindings.push(ClosedIntBinding {
                        binding,
                        value: initializer,
                    }),
                    Err(error) => return Some(Err(error)),
                }
            }
            StatementKind::Expression(expression) if expression.ty == Type::Int => {
                if let Err(error) = evaluate_with_context(expression, bindings, &block_bindings)? {
                    return Some(Err(error));
                }
            }
            _ => return None,
        }
    }

    evaluate_with_context(block.tail.as_deref()?, bindings, &block_bindings)
}

fn closed_int_binding_value<'a>(
    reference: &BindingReference,
    ty: &Type,
    bindings: &[ClosedIntBinding<'a>],
) -> Option<&'a Expression> {
    let entry = bindings
        .iter()
        .rev()
        .find(|entry| entry.binding.id == reference.binding)?;
    if entry.binding.name != reference.binding_name
        || entry.binding.span != reference.declaration_span
        || &entry.binding.ty != ty
        || &entry.value.ty != ty
    {
        return None;
    }
    Some(entry.value)
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
