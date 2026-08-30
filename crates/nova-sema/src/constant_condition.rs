use crate::constant_int;
use crate::hir::{EnumId, Expression, ExpressionKind, FunctionId, MatchArm, RecordId, Type};
use nova_parser::ast::{BinaryOperator, UnaryOperator};

/// Evaluates only side-effect-free, closed bootstrap conditions whose value is
/// already determined by supported literal, identity, comparison, and Boolean proofs.
/// The HIR is never folded.
pub(crate) fn evaluate(expression: &Expression) -> Option<bool> {
    if expression.ty != Type::Bool {
        return None;
    }

    match &expression.kind {
        ExpressionKind::Boolean(value) => Some(*value),
        ExpressionKind::Unary {
            operator: UnaryOperator::Not,
            operand,
        } => evaluate(operand).map(|value| !value),
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => evaluate_binary(*operator, left, right),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate(condition)? {
            true if then_branch.statements.is_empty() => evaluate(then_branch.tail.as_deref()?),
            true => None,
            false => evaluate(else_branch),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => evaluate(selected_match_value(scrutinee, *enumeration, arms)?),
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => evaluate(selected_record_field_value(base, *record, *field_index)?),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            evaluate(block.tail.as_deref()?)
        }
        _ => None,
    }
}

fn evaluate_binary(
    operator: BinaryOperator,
    left: &Expression,
    right: &Expression,
) -> Option<bool> {
    match operator {
        BinaryOperator::And => match evaluate(left) {
            Some(false) => Some(false),
            Some(true) => evaluate(right),
            None => None,
        },
        BinaryOperator::Or => match evaluate(left) {
            Some(true) => Some(true),
            Some(false) => evaluate(right),
            None => None,
        },
        BinaryOperator::Equal | BinaryOperator::NotEqual => {
            let equal = match (&left.ty, &right.ty) {
                (Type::Int, Type::Int) => int_value(left)? == int_value(right)?,
                (Type::Bool, Type::Bool) => evaluate(left)? == evaluate(right)?,
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
        BinaryOperator::Less => Some(int_value(left)? < int_value(right)?),
        BinaryOperator::LessEqual => Some(int_value(left)? <= int_value(right)?),
        BinaryOperator::Greater => Some(int_value(left)? > int_value(right)?),
        BinaryOperator::GreaterEqual => Some(int_value(left)? >= int_value(right)?),
        _ => None,
    }
}

fn int_value(expression: &Expression) -> Option<i64> {
    constant_int::evaluate(expression)?.ok()
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
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } if payload.as_deref().is_none_or(is_closed_total_value) => {
            Some((*enumeration, *variant_index))
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate(condition)? {
            true if then_branch.statements.is_empty() => {
                match_variant_tag(then_branch.tail.as_deref()?)
            }
            true => None,
            false => match_variant_tag(else_branch),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => match_variant_tag(selected_match_value(scrutinee, *enumeration, arms)?),
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => match_variant_tag(selected_record_field_value(base, *record, *field_index)?),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            match_variant_tag(block.tail.as_deref()?)
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

pub(crate) fn selected_match_value<'a>(
    scrutinee: &Expression,
    enumeration: EnumId,
    arms: &'a [MatchArm],
) -> Option<&'a Expression> {
    let (scrutinee_enum, variant_index) = match_variant_tag(scrutinee)?;
    if scrutinee_enum != enumeration {
        return None;
    }

    let mut selected = arms.iter().filter(|arm| arm.variant_index == variant_index);
    let arm = selected.next()?;
    if selected.next().is_some() {
        return None;
    }
    Some(&arm.value)
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
