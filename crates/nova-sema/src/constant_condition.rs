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
        } => {
            let (value, selected_bindings) =
                selected_record_field_value_with_bindings(base, *record, *field_index, bindings)?;
            evaluate_with_bindings(value, &selected_bindings)
        }
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
                (Type::Int, Type::Int) => int_value(left, bindings)? == int_value(right, bindings)?,
                (Type::Bool, Type::Bool) => {
                    evaluate_with_bindings(left, bindings)?
                        == evaluate_with_bindings(right, bindings)?
                }
                (Type::Unit, Type::Unit) => {
                    unit_value_with_bindings(left, bindings)?;
                    unit_value_with_bindings(right, bindings)?;
                    true
                }
                (Type::Function(left_function), Type::Function(right_function))
                    if left_function == right_function =>
                {
                    function_id_with_bindings(left, bindings)?
                        == function_id_with_bindings(right, bindings)?
                }
                (Type::Enum(left_enum), Type::Enum(right_enum))
                    if left_enum.id == right_enum.id =>
                {
                    enum_identity_tag_with_bindings(left, bindings)?
                        == enum_identity_tag_with_bindings(right, bindings)?
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
    unit_value_with_bindings(expression, &[])
}

fn unit_value_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<()> {
    if expression.ty != Type::Unit {
        return None;
    }

    match &expression.kind {
        ExpressionKind::Unit => Some(()),
        ExpressionKind::Binding(reference) => unit_value_with_bindings(
            closed_binding_value(reference, &expression.ty, bindings)?,
            bindings,
        ),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_with_bindings(condition, bindings)? {
            true if then_branch.statements.is_empty() => match then_branch.tail.as_deref() {
                Some(tail) => unit_value_with_bindings(tail, bindings),
                None => Some(()),
            },
            true => None,
            false => unit_value_with_bindings(else_branch, bindings),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let (value, selected_bindings) =
                selected_match_value_with_bindings(scrutinee, *enumeration, arms, bindings)?;
            unit_value_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let (value, selected_bindings) =
                selected_record_field_value_with_bindings(base, *record, *field_index, bindings)?;
            unit_value_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            match block.tail.as_deref() {
                Some(tail) => unit_value_with_bindings(tail, bindings),
                None => Some(()),
            }
        }
        _ => None,
    }
}

fn enum_identity_tag(expression: &Expression) -> Option<(EnumId, usize)> {
    enum_identity_tag_with_bindings(expression, &[])
}

fn enum_identity_tag_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<(EnumId, usize)> {
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } if payload.is_none() => Some((*enumeration, *variant_index)),
        ExpressionKind::Binding(reference) => enum_identity_tag_with_bindings(
            closed_binding_value(reference, &expression.ty, bindings)?,
            bindings,
        ),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_with_bindings(condition, bindings)? {
            true if then_branch.statements.is_empty() => {
                enum_identity_tag_with_bindings(then_branch.tail.as_deref()?, bindings)
            }
            true => None,
            false => enum_identity_tag_with_bindings(else_branch, bindings),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let (value, selected_bindings) =
                selected_match_value_with_bindings(scrutinee, *enumeration, arms, bindings)?;
            enum_identity_tag_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let (value, selected_bindings) =
                selected_record_field_value_with_bindings(base, *record, *field_index, bindings)?;
            enum_identity_tag_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            enum_identity_tag_with_bindings(block.tail.as_deref()?, bindings)
        }
        _ => None,
    }
}

fn function_id(expression: &Expression) -> Option<FunctionId> {
    function_id_with_bindings(expression, &[])
}

fn function_id_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<FunctionId> {
    match &expression.kind {
        ExpressionKind::Function { function, .. } => Some(*function),
        ExpressionKind::Binding(reference) => function_id_with_bindings(
            closed_binding_value(reference, &expression.ty, bindings)?,
            bindings,
        ),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_with_bindings(condition, bindings)? {
            true if then_branch.statements.is_empty() => {
                function_id_with_bindings(then_branch.tail.as_deref()?, bindings)
            }
            true => None,
            false => function_id_with_bindings(else_branch, bindings),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let (value, selected_bindings) =
                selected_match_value_with_bindings(scrutinee, *enumeration, arms, bindings)?;
            function_id_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let (value, selected_bindings) =
                selected_record_field_value_with_bindings(base, *record, *field_index, bindings)?;
            function_id_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            function_id_with_bindings(block.tail.as_deref()?, bindings)
        }
        _ => None,
    }
}

fn match_variant_tag(expression: &Expression) -> Option<(EnumId, usize)> {
    let (enumeration, variant_index, _, _) = match_variant_with_bindings(expression, &[])?;
    Some((enumeration, variant_index))
}

fn match_variant_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<(EnumId, usize, Option<&'a Expression>, Vec<ClosedBinding<'a>>)> {
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } if payload
            .as_deref()
            .is_none_or(|payload| is_closed_total_value_with_bindings(payload, bindings)) =>
        {
            Some((
                *enumeration,
                *variant_index,
                payload.as_deref(),
                bindings.to_vec(),
            ))
        }
        ExpressionKind::Binding(reference) => match_variant_with_bindings(
            closed_binding_value(reference, &expression.ty, bindings)?,
            bindings,
        ),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_with_bindings(condition, bindings)? {
            true if then_branch.statements.is_empty() => {
                match_variant_with_bindings(then_branch.tail.as_deref()?, bindings)
            }
            true => None,
            false => match_variant_with_bindings(else_branch, bindings),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let (value, selected_bindings) =
                selected_match_value_with_bindings(scrutinee, *enumeration, arms, bindings)?;
            match_variant_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let (value, selected_bindings) =
                selected_record_field_value_with_bindings(base, *record, *field_index, bindings)?;
            match_variant_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            match_variant_with_bindings(block.tail.as_deref()?, bindings)
        }
        _ => None,
    }
}

fn is_closed_total_value(expression: &Expression) -> bool {
    is_closed_total_value_with_bindings(expression, &[])
}

fn is_closed_total_value_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> bool {
    match &expression.ty {
        Type::Int => constant_int::evaluate_with_bindings(expression, bindings)
            .is_some_and(|value| value.is_ok()),
        Type::Bool => evaluate_with_bindings(expression, bindings).is_some(),
        Type::Unit => unit_value_with_bindings(expression, bindings).is_some(),
        Type::Function(_) => function_id_with_bindings(expression, bindings).is_some(),
        Type::Enum(_) => match_variant_with_bindings(expression, bindings).is_some(),
        Type::Record(_) => record_value_is_closed_with_bindings(expression, bindings),
        Type::Never | Type::Error => false,
    }
}

fn record_value_is_closed(expression: &Expression) -> bool {
    record_value_is_closed_with_bindings(expression, &[])
}

fn record_value_is_closed_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> bool {
    match &expression.kind {
        ExpressionKind::RecordLiteral { fields, .. } => fields
            .iter()
            .all(|field| is_closed_total_value_with_bindings(&field.value, bindings)),
        ExpressionKind::Binding(reference) => closed_binding_value(reference, &expression.ty, bindings)
            .is_some_and(|value| record_value_is_closed_with_bindings(value, bindings)),
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => selected_record_field_value_with_bindings(base, *record, *field_index, bindings)
            .is_some_and(|(value, selected_bindings)| {
                is_closed_total_value_with_bindings(value, &selected_bindings)
            }),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_with_bindings(condition, bindings) {
            Some(true) if then_branch.statements.is_empty() => then_branch
                .tail
                .as_deref()
                .is_some_and(|tail| is_closed_total_value_with_bindings(tail, bindings)),
            Some(true) => false,
            Some(false) => is_closed_total_value_with_bindings(else_branch, bindings),
            None => false,
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => selected_match_value_with_bindings(scrutinee, *enumeration, arms, bindings)
            .is_some_and(|(value, selected_bindings)| {
                is_closed_total_value_with_bindings(value, &selected_bindings)
            }),
        ExpressionKind::Block(block) if block.statements.is_empty() => block
            .tail
            .as_deref()
            .is_some_and(|tail| is_closed_total_value_with_bindings(tail, bindings)),
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
    let (scrutinee_enum, variant_index, payload, mut selected_bindings) =
        match_variant_with_bindings(scrutinee, bindings)?;
    if scrutinee_enum != enumeration {
        return None;
    }

    let mut selected = arms.iter().filter(|arm| arm.variant_index == variant_index);
    let arm = selected.next()?;
    if selected.next().is_some() {
        return None;
    }

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
    let (value, _) = selected_record_field_value_with_bindings(base, record, field_index, &[])?;
    Some(value)
}

pub(crate) fn selected_record_field_value_with_bindings<'a>(
    base: &'a Expression,
    record: RecordId,
    field_index: usize,
    bindings: &[ClosedBinding<'a>],
) -> Option<(&'a Expression, Vec<ClosedBinding<'a>>)> {
    match &base.kind {
        ExpressionKind::RecordLiteral {
            record: actual_record,
            fields,
        } if *actual_record == record
            && fields
                .iter()
                .all(|field| is_closed_total_value_with_bindings(&field.value, bindings)) =>
        {
            let mut selected = fields
                .iter()
                .filter(|field| field.field_index == field_index);
            let field = selected.next()?;
            if selected.next().is_some() {
                return None;
            }
            Some((&field.value, bindings.to_vec()))
        }
        ExpressionKind::Binding(reference) => selected_record_field_value_with_bindings(
            closed_binding_value(reference, &base.ty, bindings)?,
            record,
            field_index,
            bindings,
        ),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_with_bindings(condition, bindings)? {
            true if then_branch.statements.is_empty() => selected_record_field_value_with_bindings(
                then_branch.tail.as_deref()?,
                record,
                field_index,
                bindings,
            ),
            true => None,
            false => selected_record_field_value_with_bindings(
                else_branch,
                record,
                field_index,
                bindings,
            ),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let (value, selected_bindings) =
                selected_match_value_with_bindings(scrutinee, *enumeration, arms, bindings)?;
            selected_record_field_value_with_bindings(
                value,
                record,
                field_index,
                &selected_bindings,
            )
        }
        ExpressionKind::FieldAccess {
            base: outer_base,
            record: outer_record,
            field_index: outer_field_index,
            ..
        } => {
            let (outer_value, outer_bindings) = selected_record_field_value_with_bindings(
                outer_base,
                *outer_record,
                *outer_field_index,
                bindings,
            )?;
            selected_record_field_value_with_bindings(
                outer_value,
                record,
                field_index,
                &outer_bindings,
            )
        }
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            selected_record_field_value_with_bindings(
                block.tail.as_deref()?,
                record,
                field_index,
                bindings,
            )
        }
        _ => None,
    }
}
