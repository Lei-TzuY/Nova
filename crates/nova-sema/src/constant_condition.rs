use crate::constant_int;
use crate::hir::{
    Binding, BindingReference, EnumId, Expression, ExpressionKind, FunctionId, MatchArm, RecordId,
    Type,
};
use nova_parser::ast::{BinaryOperator, UnaryOperator};

#[derive(Clone, Copy)]
pub(crate) struct ClosedBinding<'a> {
    binding: &'a Binding,
    value: &'a Expression,
}

/// Evaluates only side-effect-free, closed bootstrap conditions whose value is
/// already determined by supported literal, identity, comparison, and Boolean proofs.
/// The HIR is never folded.
pub(crate) fn evaluate(expression: &Expression) -> Option<bool> {
    evaluate_with_bindings(expression, &[])
}

pub(crate) fn evaluate_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<bool> {
    if expression.ty != Type::Bool {
        return None;
    }

    match &expression.kind {
        ExpressionKind::Boolean(value) => Some(*value),
        ExpressionKind::Binding(reference) => evaluate_with_bindings(
            closed_binding_value(reference, &expression.ty, bindings)?,
            bindings,
        ),
        ExpressionKind::Unary {
            operator: UnaryOperator::Not,
            operand,
        } => evaluate_with_bindings(operand, bindings).map(|value| !value),
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => evaluate_binary(*operator, left, right, bindings),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_with_bindings(condition, bindings)? {
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
                selected_match_value_with_bindings(scrutinee, *enumeration, arms, bindings)?;
            evaluate_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => evaluate_with_bindings(
            selected_record_field_value(base, *record, *field_index)?,
            bindings,
        ),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            evaluate_with_bindings(block.tail.as_deref()?, bindings)
        }
        _ => None,
    }
}

fn evaluate_binary<'a>(
    operator: BinaryOperator,
    left: &'a Expression,
    right: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<bool> {
    match operator {
        BinaryOperator::And => match evaluate_with_bindings(left, bindings) {
            Some(false) => Some(false),
            Some(true) => evaluate_with_bindings(right, bindings),
            None => None,
        },
        BinaryOperator::Or => match evaluate_with_bindings(left, bindings) {
            Some(true) => Some(true),
            Some(false) => evaluate_with_bindings(right, bindings),
            None => None,
        },
        BinaryOperator::Equal | BinaryOperator::NotEqual => {
            let equal = match (&left.ty, &right.ty) {
                (Type::Int, Type::Int) => {
                    int_value(left, bindings)? == int_value(right, bindings)?
                }
                (Type::Bool, Type::Bool) => {
                    evaluate_with_bindings(left, bindings)?
                        == evaluate_with_bindings(right, bindings)?
                }
                (Type::Unit, Type::Unit) => {
                    unit_value(left)?;
                    unit_value(right)?;
                    true
                }
                (Type::Function(left_function), Type::Function(right_function))
                    if left_function == right_function =>
                {
                    function_id(left)? == function_id(right)?
                }
                (Type::Enum(left_enum), Type::Enum(right_enum))
                    if left_enum.id == right_enum.id =>
                {
                    enum_identity_tag(left)? == enum_identity_tag(right)?
                }
                _ => return None,
            };
            Some(if operator == BinaryOperator::Equal {
                equal
            } else {
                !equal
            })
        }
        BinaryOperator::Less => Some(int_value(left, bindings)? < int_value(right, bindings)?),
        BinaryOperator::LessEqual => {
            Some(int_value(left, bindings)? <= int_value(right, bindings)?)
        }
        BinaryOperator::Greater => Some(int_value(left, bindings)? > int_value(right, bindings)?),
        BinaryOperator::GreaterEqual => {
            Some(int_value(left, bindings)? >= int_value(right, bindings)?)
        }
        _ => None,
    }
}

fn int_value<'a>(expression: &'a Expression, bindings: &[ClosedBinding<'a>]) -> Option<i64> {
    constant_int::evaluate_with_bindings(expression, bindings)?.ok()
}

fn unit_value(expression: &Expression) -> Option<()> {
    if expression.ty != Type::Unit {
        return None;
    }

    match &expression.kind {
        ExpressionKind::Unit => Some(()),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate(condition)? {
            true if then_branch.statements.is_empty() => match then_branch.tail.as_deref() {
                Some(tail) => unit_value(tail),
                None => Some(()),
            },
            true => None,
            false => unit_value(else_branch),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => unit_value(selected_match_value(scrutinee, *enumeration, arms)?),
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => unit_value(selected_record_field_value(base, *record, *field_index)?),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            match block.tail.as_deref() {
                Some(tail) => unit_value(tail),
                None => Some(()),
            }
        }
        _ => None,
    }
}

fn enum_identity_tag(expression: &Expression) -> Option<(EnumId, usize)> {
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } if payload.is_none() => Some((*enumeration, *variant_index)),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate(condition)? {
            true if then_branch.statements.is_empty() => {
                enum_identity_tag(then_branch.tail.as_deref()?)
            }
            true => None,
            false => enum_identity_tag(else_branch),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => enum_identity_tag(selected_match_value(scrutinee, *enumeration, arms)?),
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => enum_identity_tag(selected_record_field_value(base, *record, *field_index)?),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            enum_identity_tag(block.tail.as_deref()?)
        }
        _ => None,
    }
}

fn function_id(expression: &Expression) -> Option<FunctionId> {
    match &expression.kind {
        ExpressionKind::Function { function, .. } => Some(*function),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate(condition)? {
            true if then_branch.statements.is_empty() => function_id(then_branch.tail.as_deref()?),
            true => None,
            false => function_id(else_branch),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => function_id(selected_match_value(scrutinee, *enumeration, arms)?),
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => function_id(selected_record_field_value(base, *record, *field_index)?),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            function_id(block.tail.as_deref()?)
        }
        _ => None,
    }
}

fn match_variant_tag(expression: &Expression) -> Option<(EnumId, usize)> {
    let (enumeration, variant_index, _) = match_variant(expression)?;
    Some((enumeration, variant_index))
}

fn match_variant(expression: &Expression) -> Option<(EnumId, usize, Option<&Expression>)> {
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } if payload.as_deref().is_none_or(is_closed_total_value) => {
            Some((*enumeration, *variant_index, payload.as_deref()))
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate(condition)? {
            true if then_branch.statements.is_empty() => {
                match_variant(then_branch.tail.as_deref()?)
            }
            true => None,
            false => match_variant(else_branch),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => match_variant(selected_match_value(scrutinee, *enumeration, arms)?),
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => match_variant(selected_record_field_value(base, *record, *field_index)?),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            match_variant(block.tail.as_deref()?)
        }
        _ => None,
    }
}

fn is_closed_total_value(expression: &Expression) -> bool {
    match &expression.ty {
        Type::Int => constant_int::evaluate(expression).is_some_and(|value| value.is_ok()),
        Type::Bool => evaluate(expression).is_some(),
        Type::Unit => unit_value(expression).is_some(),
        Type::Function(_) => function_id(expression).is_some(),
        Type::Enum(_) => match_variant_tag(expression).is_some(),
        Type::Record(_) => record_value_is_closed(expression),
        Type::Never | Type::Error => false,
    }
}

fn record_value_is_closed(expression: &Expression) -> bool {
    match &expression.kind {
        ExpressionKind::RecordLiteral { fields, .. } => fields
            .iter()
            .all(|field| is_closed_total_value(&field.value)),
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => selected_record_field_value(base, *record, *field_index)
            .is_some_and(is_closed_total_value),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate(condition) {
            Some(true) if then_branch.statements.is_empty() => then_branch
                .tail
                .as_deref()
                .is_some_and(is_closed_total_value),
            Some(true) => false,
            Some(false) => is_closed_total_value(else_branch),
            None => false,
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => selected_match_value(scrutinee, *enumeration, arms).is_some_and(is_closed_total_value),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            block.tail.as_deref().is_some_and(is_closed_total_value)
        }
        _ => false,
    }
}

pub(crate) fn closed_binding_value<'a>(
    reference: &BindingReference,
    ty: &Type,
    bindings: &[ClosedBinding<'a>],
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

pub(crate) fn selected_match_value<'a>(
    scrutinee: &'a Expression,
    enumeration: EnumId,
    arms: &'a [MatchArm],
) -> Option<&'a Expression> {
    let (value, _) = selected_match_value_with_bindings(scrutinee, enumeration, arms, &[])?;
    Some(value)
}

pub(crate) fn selected_match_value_with_bindings<'a>(
    scrutinee: &'a Expression,
    enumeration: EnumId,
    arms: &'a [MatchArm],
    bindings: &[ClosedBinding<'a>],
) -> Option<(&'a Expression, Vec<ClosedBinding<'a>>)> {
    let (scrutinee_enum, variant_index, payload) = match_variant(scrutinee)?;
    if scrutinee_enum != enumeration {
        return None;
    }

    let mut selected = arms.iter().filter(|arm| arm.variant_index == variant_index);
    let arm = selected.next()?;
    if selected.next().is_some() {
        return None;
    }

    let mut selected_bindings = bindings.to_vec();
    match (payload, arm.binding.as_ref(), arm.payload_discarded) {
        (None, None, false) | (Some(_), None, true) => {}
        (Some(payload), Some(binding), false) if binding.ty == payload.ty => {
            selected_bindings.push(ClosedBinding {
                binding,
                value: payload,
            });
        }
        _ => return None,
    }

    Some((&arm.value, selected_bindings))
}

pub(crate) fn selected_record_field_value(
    base: &Expression,
    record: RecordId,
    field_index: usize,
) -> Option<&Expression> {
    match &base.kind {
        ExpressionKind::RecordLiteral {
            record: actual_record,
            fields,
        } if *actual_record == record
            && fields
                .iter()
                .all(|field| is_closed_total_value(&field.value)) =>
        {
            let mut selected = fields
                .iter()
                .filter(|field| field.field_index == field_index);
            let field = selected.next()?;
            if selected.next().is_some() {
                return None;
            }
            Some(&field.value)
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate(condition)? {
            true if then_branch.statements.is_empty() => {
                selected_record_field_value(then_branch.tail.as_deref()?, record, field_index)
            }
            true => None,
            false => selected_record_field_value(else_branch, record, field_index),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => selected_record_field_value(
            selected_match_value(scrutinee, *enumeration, arms)?,
            record,
            field_index,
        ),
        ExpressionKind::FieldAccess {
            base: outer_base,
            record: outer_record,
            field_index: outer_field_index,
            ..
        } => selected_record_field_value(
            selected_record_field_value(outer_base, *outer_record, *outer_field_index)?,
            record,
            field_index,
        ),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            selected_record_field_value(block.tail.as_deref()?, record, field_index)
        }
        _ => None,
    }
}
