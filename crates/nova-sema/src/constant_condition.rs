use crate::constant_int;
use crate::hir::{
    Binding, BindingReference, Block, EnumId, Expression, ExpressionKind, FunctionId, MatchArm,
    RecordId, StatementKind, Type,
};
use nova_parser::ast::{BinaryOperator, UnaryOperator};
use nova_source::Span;

#[derive(Clone, Copy)]
pub(crate) struct ClosedBinding<'a> {
    binding: &'a Binding,
    value: &'a Expression,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ClosedConditionArithmeticFailure {
    pub(crate) error: constant_int::ConstantIntError,
    pub(crate) span: Span,
}

type ClosedBlockProof<'a> =
    Result<(Option<&'a Expression>, Vec<ClosedBinding<'a>>), ClosedConditionArithmeticFailure>;
type ClosedVariantProof<'a> = (
    EnumId,
    usize,
    Option<&'a Expression>,
    Vec<ClosedBinding<'a>>,
);
type ClosedSelectedValue<'a> = (&'a Expression, Vec<ClosedBinding<'a>>);

/// Evaluates only side-effect-free, closed bootstrap conditions whose value is
/// already determined by supported literal, identity, comparison, and Boolean proofs.
/// The HIR is never folded. Arithmetic failures are intentionally hidden from this
/// convenience API; semantic control-flow sites use `evaluate_checked` instead.
pub(crate) fn evaluate(expression: &Expression) -> Option<bool> {
    evaluate_checked(expression).ok().flatten()
}

pub(crate) fn evaluate_checked(
    expression: &Expression,
) -> Result<Option<bool>, ClosedConditionArithmeticFailure> {
    evaluate_checked_with_bindings(expression, &[])
}

pub(crate) fn evaluate_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Option<bool> {
    evaluate_checked_with_bindings(expression, bindings)
        .ok()
        .flatten()
}

pub(crate) fn evaluate_checked_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Result<Option<bool>, ClosedConditionArithmeticFailure> {
    if expression.ty != Type::Bool {
        return Ok(None);
    }

    match &expression.kind {
        ExpressionKind::Boolean(value) => Ok(Some(*value)),
        ExpressionKind::Binding(reference) => {
            let Some(value) = closed_binding_value(reference, &expression.ty, bindings) else {
                return Ok(None);
            };
            evaluate_checked_with_bindings(value, bindings)
        }
        ExpressionKind::Unary {
            operator: UnaryOperator::Not,
            operand,
        } => Ok(evaluate_checked_with_bindings(operand, bindings)?.map(|value| !value)),
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => evaluate_binary_checked(*operator, left, right, bindings),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_checked_with_bindings(condition, bindings)? {
            Some(true) => {
                let Some(proof) = closed_block_tail_with_bindings(then_branch, bindings) else {
                    return Ok(None);
                };
                let (tail, selected_bindings) = proof?;
                let Some(tail) = tail else {
                    return Ok(None);
                };
                evaluate_checked_with_bindings(tail, &selected_bindings)
            }
            Some(false) => evaluate_checked_with_bindings(else_branch, bindings),
            None => Ok(None),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let Some((value, selected_bindings)) = selected_match_value_checked_with_bindings(
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
                selected_record_field_value_checked_with_bindings(
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
        ExpressionKind::Block(block) => {
            let Some(proof) = closed_block_tail_with_bindings(block, bindings) else {
                return Ok(None);
            };
            let (tail, selected_bindings) = proof?;
            let Some(tail) = tail else {
                return Ok(None);
            };
            evaluate_checked_with_bindings(tail, &selected_bindings)
        }
        _ => Ok(None),
    }
}

fn evaluate_binary_checked<'a>(
    operator: BinaryOperator,
    left: &'a Expression,
    right: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Result<Option<bool>, ClosedConditionArithmeticFailure> {
    match operator {
        BinaryOperator::And => match evaluate_checked_with_bindings(left, bindings)? {
            Some(false) => Ok(Some(false)),
            Some(true) => evaluate_checked_with_bindings(right, bindings),
            None => Ok(None),
        },
        BinaryOperator::Or => match evaluate_checked_with_bindings(left, bindings)? {
            Some(true) => Ok(Some(true)),
            Some(false) => evaluate_checked_with_bindings(right, bindings),
            None => Ok(None),
        },
        BinaryOperator::Equal | BinaryOperator::NotEqual => {
            let equal = match (&left.ty, &right.ty) {
                (Type::Int, Type::Int) => {
                    let Some(left) = int_value_checked(left, bindings)? else {
                        return Ok(None);
                    };
                    let Some(right) = int_value_checked(right, bindings)? else {
                        return Ok(None);
                    };
                    left == right
                }
                (Type::Bool, Type::Bool) => {
                    let Some(left) = evaluate_checked_with_bindings(left, bindings)? else {
                        return Ok(None);
                    };
                    let Some(right) = evaluate_checked_with_bindings(right, bindings)? else {
                        return Ok(None);
                    };
                    left == right
                }
                (Type::Unit, Type::Unit) => {
                    let Some(()) = unit_value_checked_with_bindings(left, bindings)? else {
                        return Ok(None);
                    };
                    let Some(()) = unit_value_checked_with_bindings(right, bindings)? else {
                        return Ok(None);
                    };
                    true
                }
                (Type::Function(left_function), Type::Function(right_function))
                    if left_function == right_function =>
                {
                    let Some(left) = function_id_checked_with_bindings(left, bindings)? else {
                        return Ok(None);
                    };
                    let Some(right) = function_id_checked_with_bindings(right, bindings)? else {
                        return Ok(None);
                    };
                    left == right
                }
                (Type::Enum(left_enum), Type::Enum(right_enum))
                    if left_enum.id == right_enum.id =>
                {
                    let Some(left) = enum_identity_tag_checked_with_bindings(left, bindings)?
                    else {
                        return Ok(None);
                    };
                    let Some(right) = enum_identity_tag_checked_with_bindings(right, bindings)?
                    else {
                        return Ok(None);
                    };
                    left == right
                }
                _ => return Ok(None),
            };
            Ok(Some(if operator == BinaryOperator::Equal {
                equal
            } else {
                !equal
            }))
        }
        BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => {
            let Some(left) = int_value_checked(left, bindings)? else {
                return Ok(None);
            };
            let Some(right) = int_value_checked(right, bindings)? else {
                return Ok(None);
            };
            Ok(Some(match operator {
                BinaryOperator::Less => left < right,
                BinaryOperator::LessEqual => left <= right,
                BinaryOperator::Greater => left > right,
                BinaryOperator::GreaterEqual => left >= right,
                _ => unreachable!("comparison operator matched above"),
            }))
        }
        _ => Ok(None),
    }
}

fn int_value_checked<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Result<Option<i64>, ClosedConditionArithmeticFailure> {
    match constant_int::evaluate_checked_with_bindings(expression, bindings)? {
        Some(Ok(value)) => Ok(Some(value)),
        Some(Err(error)) => Err(ClosedConditionArithmeticFailure {
            error,
            span: expression.span,
        }),
        None => Ok(None),
    }
}

fn unit_value_checked_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Result<Option<()>, ClosedConditionArithmeticFailure> {
    if expression.ty != Type::Unit {
        return Ok(None);
    }

    match &expression.kind {
        ExpressionKind::Unit => Ok(Some(())),
        ExpressionKind::Binding(reference) => {
            let Some(value) = closed_binding_value(reference, &expression.ty, bindings) else {
                return Ok(None);
            };
            unit_value_checked_with_bindings(value, bindings)
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_checked_with_bindings(condition, bindings)? {
            Some(true) => {
                let Some(proof) = closed_block_tail_with_bindings(then_branch, bindings) else {
                    return Ok(None);
                };
                let (tail, selected_bindings) = proof?;
                match tail {
                    Some(tail) => unit_value_checked_with_bindings(tail, &selected_bindings),
                    None => Ok(Some(())),
                }
            }
            Some(false) => unit_value_checked_with_bindings(else_branch, bindings),
            None => Ok(None),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let Some((value, selected_bindings)) = selected_match_value_checked_with_bindings(
                scrutinee,
                *enumeration,
                arms,
                bindings,
            )?
            else {
                return Ok(None);
            };
            unit_value_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let Some((value, selected_bindings)) =
                selected_record_field_value_checked_with_bindings(
                    base,
                    *record,
                    *field_index,
                    bindings,
                )?
            else {
                return Ok(None);
            };
            unit_value_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) => {
            let Some(proof) = closed_block_tail_with_bindings(block, bindings) else {
                return Ok(None);
            };
            let (tail, selected_bindings) = proof?;
            match tail {
                Some(tail) => unit_value_checked_with_bindings(tail, &selected_bindings),
                None => Ok(Some(())),
            }
        }
        _ => Ok(None),
    }
}

fn enum_identity_tag_checked_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Result<Option<(EnumId, usize)>, ClosedConditionArithmeticFailure> {
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } if payload.is_none() => Ok(Some((*enumeration, *variant_index))),
        ExpressionKind::Binding(reference) => {
            let Some(value) = closed_binding_value(reference, &expression.ty, bindings) else {
                return Ok(None);
            };
            enum_identity_tag_checked_with_bindings(value, bindings)
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_checked_with_bindings(condition, bindings)? {
            Some(true) => {
                let Some(proof) = closed_block_tail_with_bindings(then_branch, bindings) else {
                    return Ok(None);
                };
                let (tail, selected_bindings) = proof?;
                let Some(tail) = tail else {
                    return Ok(None);
                };
                enum_identity_tag_checked_with_bindings(tail, &selected_bindings)
            }
            Some(false) => enum_identity_tag_checked_with_bindings(else_branch, bindings),
            None => Ok(None),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let Some((value, selected_bindings)) = selected_match_value_checked_with_bindings(
                scrutinee,
                *enumeration,
                arms,
                bindings,
            )?
            else {
                return Ok(None);
            };
            enum_identity_tag_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let Some((value, selected_bindings)) =
                selected_record_field_value_checked_with_bindings(
                    base,
                    *record,
                    *field_index,
                    bindings,
                )?
            else {
                return Ok(None);
            };
            enum_identity_tag_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) => {
            let Some(proof) = closed_block_tail_with_bindings(block, bindings) else {
                return Ok(None);
            };
            let (tail, selected_bindings) = proof?;
            let Some(tail) = tail else {
                return Ok(None);
            };
            enum_identity_tag_checked_with_bindings(tail, &selected_bindings)
        }
        _ => Ok(None),
    }
}

fn function_id_checked_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Result<Option<FunctionId>, ClosedConditionArithmeticFailure> {
    match &expression.kind {
        ExpressionKind::Function { function, .. } => Ok(Some(*function)),
        ExpressionKind::Binding(reference) => {
            let Some(value) = closed_binding_value(reference, &expression.ty, bindings) else {
                return Ok(None);
            };
            function_id_checked_with_bindings(value, bindings)
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_checked_with_bindings(condition, bindings)? {
            Some(true) => {
                let Some(proof) = closed_block_tail_with_bindings(then_branch, bindings) else {
                    return Ok(None);
                };
                let (tail, selected_bindings) = proof?;
                let Some(tail) = tail else {
                    return Ok(None);
                };
                function_id_checked_with_bindings(tail, &selected_bindings)
            }
            Some(false) => function_id_checked_with_bindings(else_branch, bindings),
            None => Ok(None),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let Some((value, selected_bindings)) = selected_match_value_checked_with_bindings(
                scrutinee,
                *enumeration,
                arms,
                bindings,
            )?
            else {
                return Ok(None);
            };
            function_id_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let Some((value, selected_bindings)) =
                selected_record_field_value_checked_with_bindings(
                    base,
                    *record,
                    *field_index,
                    bindings,
                )?
            else {
                return Ok(None);
            };
            function_id_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) => {
            let Some(proof) = closed_block_tail_with_bindings(block, bindings) else {
                return Ok(None);
            };
            let (tail, selected_bindings) = proof?;
            let Some(tail) = tail else {
                return Ok(None);
            };
            function_id_checked_with_bindings(tail, &selected_bindings)
        }
        _ => Ok(None),
    }
}

#[derive(Clone, Copy)]
struct StaticBinding<'a> {
    binding: &'a Binding,
    value: &'a Expression,
}

type StaticVariantProof<'a> = (
    EnumId,
    usize,
    Option<&'a Expression>,
    Vec<ClosedBinding<'a>>,
    Vec<StaticBinding<'a>>,
);

pub(crate) fn static_match_variant(expression: &Expression) -> Option<(EnumId, usize)> {
    let (enumeration, variant_index, _, _, _) = match_variant_for_static_tag(expression, &[], &[])?;
    Some((enumeration, variant_index))
}

fn match_variant_for_static_tag<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
    static_bindings: &[StaticBinding<'a>],
) -> Option<StaticVariantProof<'a>> {
    if expression.ty.is_error() || expression.ty.is_never() {
        return None;
    }
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } => match &expression.ty {
            Type::Enum(enum_type) if enum_type.id == *enumeration => Some((
                *enumeration,
                *variant_index,
                payload.as_deref(),
                bindings.to_vec(),
                static_bindings.to_vec(),
            )),
            _ => None,
        },
        ExpressionKind::Binding(reference) => {
            let value = closed_binding_value(reference, &expression.ty, bindings)
                .or_else(|| static_binding_value(reference, &expression.ty, static_bindings))?;
            match_variant_for_static_tag(value, bindings, static_bindings)
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_with_bindings(condition, bindings)? {
            true => {
                let (tail, selected_bindings, selected_static_bindings) =
                    static_block_tail_with_bindings(then_branch, bindings, static_bindings)?;
                match_variant_for_static_tag(tail, &selected_bindings, &selected_static_bindings)
            }
            false => match_variant_for_static_tag(else_branch, bindings, static_bindings),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let (value, selected_bindings, selected_static_bindings) =
                selected_match_value_for_static_tag(
                    scrutinee,
                    *enumeration,
                    arms,
                    bindings,
                    static_bindings,
                )?;
            match_variant_for_static_tag(value, &selected_bindings, &selected_static_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let (value, selected_bindings, selected_static_bindings) =
                selected_record_field_for_static_tag(
                    base,
                    *record,
                    *field_index,
                    bindings,
                    static_bindings,
                )?;
            match_variant_for_static_tag(value, &selected_bindings, &selected_static_bindings)
        }
        ExpressionKind::Block(block) => {
            let (tail, selected_bindings, selected_static_bindings) =
                static_block_tail_with_bindings(block, bindings, static_bindings)?;
            match_variant_for_static_tag(tail, &selected_bindings, &selected_static_bindings)
        }
        _ => None,
    }
}

fn static_block_tail_with_bindings<'a>(
    block: &'a Block,
    bindings: &[ClosedBinding<'a>],
    static_bindings: &[StaticBinding<'a>],
) -> Option<(
    &'a Expression,
    Vec<ClosedBinding<'a>>,
    Vec<StaticBinding<'a>>,
)> {
    match closed_block_tail_with_bindings(block, bindings) {
        Some(Ok((Some(tail), selected_bindings))) => {
            return Some((tail, selected_bindings, static_bindings.to_vec()));
        }
        Some(Ok((None, _))) | Some(Err(_)) => return None,
        None => {}
    }
    if block.ty.is_error() || block.ty.is_never() {
        return None;
    }

    let mut selected_bindings = bindings.to_vec();
    let mut selected_static_bindings = static_bindings.to_vec();
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding {
                binding,
                initializer,
            } if !binding.mutable
                && binding.ty == initializer.ty
                && !initializer.ty.is_error()
                && !initializer.ty.is_never() =>
            {
                if is_closed_total_value_with_bindings(initializer, &selected_bindings) {
                    selected_bindings.push(ClosedBinding {
                        binding,
                        value: initializer,
                    });
                }
                selected_static_bindings.push(StaticBinding {
                    binding,
                    value: initializer,
                });
            }
            StatementKind::Binding { .. } => {}
            StatementKind::Expression(expression)
                if expression.ty.is_error() || expression.ty.is_never() =>
            {
                return None;
            }
            StatementKind::Return(_) | StatementKind::Break | StatementKind::Continue => {
                return None;
            }
            StatementKind::UninitializedBinding(_)
            | StatementKind::Assignment { .. }
            | StatementKind::While { .. }
            | StatementKind::Expression(_) => {}
        }
    }

    Some((
        block.tail.as_deref()?,
        selected_bindings,
        selected_static_bindings,
    ))
}

fn selected_match_value_for_static_tag<'a>(
    scrutinee: &'a Expression,
    enumeration: EnumId,
    arms: &'a [MatchArm],
    bindings: &[ClosedBinding<'a>],
    static_bindings: &[StaticBinding<'a>],
) -> Option<(
    &'a Expression,
    Vec<ClosedBinding<'a>>,
    Vec<StaticBinding<'a>>,
)> {
    let (
        scrutinee_enum,
        variant_index,
        payload,
        mut selected_bindings,
        mut selected_static_bindings,
    ) = match_variant_for_static_tag(scrutinee, bindings, static_bindings)?;
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
        (Some(payload), Some(binding), false)
            if !binding.mutable
                && binding.ty == payload.ty
                && !payload.ty.is_error()
                && !payload.ty.is_never() =>
        {
            if is_closed_total_value_with_bindings(payload, &selected_bindings) {
                selected_bindings.push(ClosedBinding {
                    binding,
                    value: payload,
                });
            }
            selected_static_bindings.push(StaticBinding {
                binding,
                value: payload,
            });
        }
        _ => return None,
    }

    Some((&arm.value, selected_bindings, selected_static_bindings))
}

fn selected_record_field_for_static_tag<'a>(
    base: &'a Expression,
    record: RecordId,
    field_index: usize,
    bindings: &[ClosedBinding<'a>],
    static_bindings: &[StaticBinding<'a>],
) -> Option<(
    &'a Expression,
    Vec<ClosedBinding<'a>>,
    Vec<StaticBinding<'a>>,
)> {
    if base.ty.is_error() || base.ty.is_never() {
        return None;
    }
    match &base.kind {
        ExpressionKind::RecordLiteral {
            record: actual_record,
            fields,
        } if *actual_record == record
            && matches!(&base.ty, Type::Record(record_type) if record_type.id == record) =>
        {
            let mut selected = fields
                .iter()
                .filter(|field| field.field_index == field_index);
            let field = selected.next()?;
            if selected.next().is_some() {
                return None;
            }
            Some((&field.value, bindings.to_vec(), static_bindings.to_vec()))
        }
        ExpressionKind::Binding(reference) => {
            let value = closed_binding_value(reference, &base.ty, bindings)
                .or_else(|| static_binding_value(reference, &base.ty, static_bindings))?;
            selected_record_field_for_static_tag(
                value,
                record,
                field_index,
                bindings,
                static_bindings,
            )
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_with_bindings(condition, bindings)? {
            true => {
                let (tail, selected_bindings, selected_static_bindings) =
                    static_block_tail_with_bindings(then_branch, bindings, static_bindings)?;
                selected_record_field_for_static_tag(
                    tail,
                    record,
                    field_index,
                    &selected_bindings,
                    &selected_static_bindings,
                )
            }
            false => selected_record_field_for_static_tag(
                else_branch,
                record,
                field_index,
                bindings,
                static_bindings,
            ),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let (value, selected_bindings, selected_static_bindings) =
                selected_match_value_for_static_tag(
                    scrutinee,
                    *enumeration,
                    arms,
                    bindings,
                    static_bindings,
                )?;
            selected_record_field_for_static_tag(
                value,
                record,
                field_index,
                &selected_bindings,
                &selected_static_bindings,
            )
        }
        ExpressionKind::FieldAccess {
            base: outer_base,
            record: outer_record,
            field_index: outer_field_index,
            ..
        } => {
            let (outer_value, outer_bindings, outer_static_bindings) =
                selected_record_field_for_static_tag(
                    outer_base,
                    *outer_record,
                    *outer_field_index,
                    bindings,
                    static_bindings,
                )?;
            selected_record_field_for_static_tag(
                outer_value,
                record,
                field_index,
                &outer_bindings,
                &outer_static_bindings,
            )
        }
        ExpressionKind::Block(block) => {
            let (tail, selected_bindings, selected_static_bindings) =
                static_block_tail_with_bindings(block, bindings, static_bindings)?;
            selected_record_field_for_static_tag(
                tail,
                record,
                field_index,
                &selected_bindings,
                &selected_static_bindings,
            )
        }
        _ => None,
    }
}

fn static_binding_value<'a>(
    reference: &BindingReference,
    ty: &Type,
    bindings: &[StaticBinding<'a>],
) -> Option<&'a Expression> {
    let entry = bindings
        .iter()
        .rev()
        .find(|entry| entry.binding.id == reference.binding)?;
    if entry.binding.mutable
        || entry.binding.name != reference.binding_name
        || entry.binding.span != reference.declaration_span
        || &entry.binding.ty != ty
        || &entry.value.ty != ty
    {
        return None;
    }
    Some(entry.value)
}

pub(crate) fn closed_match_variant_checked(
    expression: &Expression,
) -> Result<Option<(EnumId, usize)>, ClosedConditionArithmeticFailure> {
    let Some((enumeration, variant_index, _, _)) =
        match_variant_checked_with_bindings(expression, &[])?
    else {
        return Ok(None);
    };
    Ok(Some((enumeration, variant_index)))
}

fn match_variant_checked_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Result<Option<ClosedVariantProof<'a>>, ClosedConditionArithmeticFailure> {
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } => {
            if let Some(payload) = payload.as_deref() {
                if !is_closed_total_value_checked_with_bindings(payload, bindings)? {
                    return Ok(None);
                }
            }
            Ok(Some((
                *enumeration,
                *variant_index,
                payload.as_deref(),
                bindings.to_vec(),
            )))
        }
        ExpressionKind::Binding(reference) => {
            let Some(value) = closed_binding_value(reference, &expression.ty, bindings) else {
                return Ok(None);
            };
            match_variant_checked_with_bindings(value, bindings)
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_checked_with_bindings(condition, bindings)? {
            Some(true) => {
                let Some(proof) = closed_block_tail_with_bindings(then_branch, bindings) else {
                    return Ok(None);
                };
                let (tail, selected_bindings) = proof?;
                let Some(tail) = tail else {
                    return Ok(None);
                };
                match_variant_checked_with_bindings(tail, &selected_bindings)
            }
            Some(false) => match_variant_checked_with_bindings(else_branch, bindings),
            None => Ok(None),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let Some((value, selected_bindings)) = selected_match_value_checked_with_bindings(
                scrutinee,
                *enumeration,
                arms,
                bindings,
            )?
            else {
                return Ok(None);
            };
            match_variant_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let Some((value, selected_bindings)) =
                selected_record_field_value_checked_with_bindings(
                    base,
                    *record,
                    *field_index,
                    bindings,
                )?
            else {
                return Ok(None);
            };
            match_variant_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) => {
            let Some(proof) = closed_block_tail_with_bindings(block, bindings) else {
                return Ok(None);
            };
            let (tail, selected_bindings) = proof?;
            let Some(tail) = tail else {
                return Ok(None);
            };
            match_variant_checked_with_bindings(tail, &selected_bindings)
        }
        _ => Ok(None),
    }
}

pub(crate) fn closed_value_arithmetic_failures(
    expression: &Expression,
) -> Vec<ClosedConditionArithmeticFailure> {
    let mut failures = Vec::new();
    let _ =
        collect_closed_value_arithmetic_failures_with_bindings(expression, &[], &[], &mut failures);
    failures
}

fn retain_reachable_break(expression: &Expression, reaches_break: bool) -> bool {
    reaches_break && !expression.ty.is_error()
}

fn collect_closed_value_arithmetic_failures_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
    static_bindings: &[StaticBinding<'a>],
    failures: &mut Vec<ClosedConditionArithmeticFailure>,
) -> bool {
    match &expression.kind {
        ExpressionKind::Block(block) => {
            let reaches_break = collect_closed_block_arithmetic_failures(
                block,
                bindings,
                static_bindings,
                failures,
            );
            retain_reachable_break(expression, reaches_break)
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let mut reaches_break = collect_closed_value_arithmetic_failures_with_bindings(
                condition,
                bindings,
                static_bindings,
                failures,
            );
            if condition.ty.is_never() {
                return retain_reachable_break(expression, reaches_break);
            }
            match evaluate_checked_with_bindings(condition, bindings) {
                Ok(Some(true)) => {
                    reaches_break |= collect_closed_block_arithmetic_failures(
                        then_branch,
                        bindings,
                        static_bindings,
                        failures,
                    );
                }
                Ok(Some(false)) => {
                    reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                        else_branch,
                        bindings,
                        static_bindings,
                        failures,
                    );
                }
                Ok(None) if condition.ty == Type::Bool => {
                    reaches_break |= collect_closed_block_arithmetic_failures(
                        then_branch,
                        bindings,
                        static_bindings,
                        failures,
                    );
                    reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                        else_branch,
                        bindings,
                        static_bindings,
                        failures,
                    );
                }
                Ok(None) => {}
                Err(failure) => failures.push(failure),
            }
            retain_reachable_break(expression, reaches_break)
        }
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let mut reaches_break = collect_closed_value_arithmetic_failures_with_bindings(
                scrutinee,
                bindings,
                static_bindings,
                failures,
            );
            if scrutinee.ty.is_never() {
                return retain_reachable_break(expression, reaches_break);
            }
            match selected_match_value_checked_with_bindings(
                scrutinee,
                *enumeration,
                arms,
                bindings,
            ) {
                Ok(Some((value, selected_bindings))) => {
                    reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                        value,
                        &selected_bindings,
                        static_bindings,
                        failures,
                    );
                }
                Ok(None) => {
                    if let Some((value, selected_bindings, selected_static_bindings)) =
                        selected_match_value_for_static_tag(
                            scrutinee,
                            *enumeration,
                            arms,
                            bindings,
                            static_bindings,
                        )
                    {
                        reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                            value,
                            &selected_bindings,
                            &selected_static_bindings,
                            failures,
                        );
                    } else if matches!(
                        &scrutinee.ty,
                        Type::Enum(scrutinee_enum) if scrutinee_enum.id == *enumeration
                    ) {
                        for arm in arms {
                            reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                                &arm.value,
                                bindings,
                                static_bindings,
                                failures,
                            );
                        }
                    }
                }
                Err(failure) => failures.push(failure),
            }
            retain_reachable_break(expression, reaches_break)
        }
        ExpressionKind::Binding(reference) => {
            let mut reaches_break = false;
            if let Some(value) = closed_binding_value(reference, &expression.ty, bindings) {
                reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                    value,
                    bindings,
                    static_bindings,
                    failures,
                );
            } else if let Err(failure) =
                is_closed_total_value_checked_with_bindings(expression, bindings)
            {
                failures.push(failure);
            }
            retain_reachable_break(expression, reaches_break)
        }
        ExpressionKind::Unary { operand, .. } => {
            let before = failures.len();
            let reaches_break = collect_closed_value_arithmetic_failures_with_bindings(
                operand,
                bindings,
                static_bindings,
                failures,
            );
            if !operand.ty.is_never() && failures.len() == before {
                if let Err(failure) =
                    is_closed_total_value_checked_with_bindings(expression, bindings)
                {
                    failures.push(failure);
                }
            }
            retain_reachable_break(expression, reaches_break)
        }
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => {
            let before = failures.len();
            let mut reaches_break = collect_closed_value_arithmetic_failures_with_bindings(
                left,
                bindings,
                static_bindings,
                failures,
            );
            if left.ty.is_never() {
                return retain_reachable_break(expression, reaches_break);
            }

            let skips_right = matches!(
                (*operator, evaluate_checked_with_bindings(left, bindings)),
                (BinaryOperator::And, Ok(Some(false))) | (BinaryOperator::Or, Ok(Some(true)))
            );
            if !skips_right {
                reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                    right,
                    bindings,
                    static_bindings,
                    failures,
                );
                if right.ty.is_never() {
                    return retain_reachable_break(expression, reaches_break);
                }
            }

            if failures.len() == before {
                if let Err(failure) =
                    is_closed_total_value_checked_with_bindings(expression, bindings)
                {
                    failures.push(failure);
                }
            }
            retain_reachable_break(expression, reaches_break)
        }
        ExpressionKind::Call { callee, arguments } => {
            let mut reaches_break = collect_closed_value_arithmetic_failures_with_bindings(
                callee,
                bindings,
                static_bindings,
                failures,
            );
            if callee.ty.is_never() {
                return retain_reachable_break(expression, reaches_break);
            }
            for argument in arguments {
                reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                    argument,
                    bindings,
                    static_bindings,
                    failures,
                );
                if argument.ty.is_never() {
                    return retain_reachable_break(expression, reaches_break);
                }
            }
            retain_reachable_break(expression, reaches_break)
        }
        ExpressionKind::RecordLiteral { fields, .. } => {
            let mut reaches_break = false;
            for field in fields {
                reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                    &field.value,
                    bindings,
                    static_bindings,
                    failures,
                );
                if field.value.ty.is_never() {
                    return retain_reachable_break(expression, reaches_break);
                }
            }
            retain_reachable_break(expression, reaches_break)
        }
        ExpressionKind::EnumConstructor { payload, .. } => {
            if let Some(payload) = payload.as_deref() {
                let reaches_break = collect_closed_value_arithmetic_failures_with_bindings(
                    payload,
                    bindings,
                    static_bindings,
                    failures,
                );
                retain_reachable_break(expression, reaches_break)
            } else {
                false
            }
        }
        ExpressionKind::FieldAccess { base, .. } => {
            let before = failures.len();
            let reaches_break = collect_closed_value_arithmetic_failures_with_bindings(
                base,
                bindings,
                static_bindings,
                failures,
            );
            if !base.ty.is_never() && failures.len() == before {
                if let Err(failure) =
                    is_closed_total_value_checked_with_bindings(expression, bindings)
                {
                    failures.push(failure);
                }
            }
            retain_reachable_break(expression, reaches_break)
        }
        _ => {
            if let Err(failure) = is_closed_total_value_checked_with_bindings(expression, bindings)
            {
                failures.push(failure);
            }
            false
        }
    }
}

fn collect_closed_block_arithmetic_failures<'a>(
    block: &'a Block,
    bindings: &[ClosedBinding<'a>],
    static_bindings: &[StaticBinding<'a>],
    failures: &mut Vec<ClosedConditionArithmeticFailure>,
) -> bool {
    let mut block_bindings = bindings.to_vec();
    let mut block_static_bindings = static_bindings.to_vec();
    let mut reaches_break = false;

    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding {
                binding,
                initializer,
            } => {
                reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                    initializer,
                    &block_bindings,
                    &block_static_bindings,
                    failures,
                );

                if initializer.ty.is_never() {
                    return reaches_break;
                }

                let retains_static_facts = !binding.mutable
                    && binding.ty == initializer.ty
                    && !initializer.ty.is_error()
                    && !initializer.ty.is_never();
                if retains_static_facts {
                    if matches!(
                        is_closed_total_value_checked_with_bindings(initializer, &block_bindings),
                        Ok(true)
                    ) {
                        block_bindings.push(ClosedBinding {
                            binding,
                            value: initializer,
                        });
                    }
                    block_static_bindings.push(StaticBinding {
                        binding,
                        value: initializer,
                    });
                }
            }
            StatementKind::Expression(expression) => {
                reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                    expression,
                    &block_bindings,
                    &block_static_bindings,
                    failures,
                );
                if expression.ty.is_never() {
                    return reaches_break;
                }
            }
            StatementKind::While { condition, body } => {
                let before = failures.len();
                let condition_reaches_break =
                    collect_closed_value_arithmetic_failures_with_bindings(
                        condition,
                        &block_bindings,
                        &block_static_bindings,
                        failures,
                    );
                if failures.len() != before
                    || (condition.ty != Type::Bool && !condition.ty.is_never())
                {
                    return reaches_break;
                }
                reaches_break |= condition_reaches_break;
                if condition.ty.is_never() {
                    return reaches_break;
                }

                match evaluate_checked_with_bindings(condition, &block_bindings) {
                    Ok(Some(false)) => {}
                    Ok(Some(true)) => {
                        let body_reaches_break = collect_closed_block_arithmetic_failures(
                            body,
                            &block_bindings,
                            &block_static_bindings,
                            failures,
                        );
                        if !body_reaches_break {
                            return reaches_break;
                        }
                    }
                    Ok(None) => {
                        let _ = collect_closed_block_arithmetic_failures(
                            body,
                            &block_bindings,
                            &block_static_bindings,
                            failures,
                        );
                    }
                    Err(failure) => {
                        failures.push(failure);
                        return reaches_break;
                    }
                }
            }
            StatementKind::UninitializedBinding(_) => {}
            StatementKind::Assignment { value, .. } => {
                reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                    value,
                    &block_bindings,
                    &block_static_bindings,
                    failures,
                );
                if value.ty.is_never() {
                    return reaches_break;
                }
            }
            StatementKind::Break => return true,
            StatementKind::Continue => return reaches_break,
            StatementKind::Return(value) => {
                if let Some(value) = value.as_ref() {
                    reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
                        value,
                        &block_bindings,
                        &block_static_bindings,
                        failures,
                    );
                }
                return reaches_break;
            }
        }
    }

    if let Some(tail) = block.tail.as_deref() {
        reaches_break |= collect_closed_value_arithmetic_failures_with_bindings(
            tail,
            &block_bindings,
            &block_static_bindings,
            failures,
        );
    }
    reaches_break
}

fn is_closed_total_value_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> bool {
    is_closed_total_value_checked_with_bindings(expression, bindings).unwrap_or(false)
}

fn is_closed_total_value_checked_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Result<bool, ClosedConditionArithmeticFailure> {
    match &expression.ty {
        Type::Int => match constant_int::evaluate_checked_with_bindings(expression, bindings)? {
            Some(Ok(_)) => Ok(true),
            Some(Err(error)) => Err(ClosedConditionArithmeticFailure {
                error,
                span: expression.span,
            }),
            None => Ok(false),
        },
        Type::Bool => Ok(evaluate_checked_with_bindings(expression, bindings)?.is_some()),
        Type::Unit => Ok(unit_value_checked_with_bindings(expression, bindings)?.is_some()),
        Type::Function(_) => Ok(function_id_checked_with_bindings(expression, bindings)?.is_some()),
        Type::Enum(_) => Ok(match_variant_checked_with_bindings(expression, bindings)?.is_some()),
        Type::Record(_) => record_value_is_closed_checked_with_bindings(expression, bindings),
        Type::Never | Type::Error => Ok(false),
    }
}

fn record_value_is_closed_checked_with_bindings<'a>(
    expression: &'a Expression,
    bindings: &[ClosedBinding<'a>],
) -> Result<bool, ClosedConditionArithmeticFailure> {
    match &expression.kind {
        ExpressionKind::RecordLiteral { fields, .. } => {
            let mut all_closed = true;
            for field in fields {
                let field_closed =
                    is_closed_total_value_checked_with_bindings(&field.value, bindings)?;
                all_closed &= field_closed;
            }
            Ok(all_closed)
        }
        ExpressionKind::Binding(reference) => {
            let Some(value) = closed_binding_value(reference, &expression.ty, bindings) else {
                return Ok(false);
            };
            record_value_is_closed_checked_with_bindings(value, bindings)
        }
        ExpressionKind::FieldAccess {
            base,
            record,
            field_index,
            ..
        } => {
            let Some((value, selected_bindings)) =
                selected_record_field_value_checked_with_bindings(
                    base,
                    *record,
                    *field_index,
                    bindings,
                )?
            else {
                return Ok(false);
            };
            is_closed_total_value_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_checked_with_bindings(condition, bindings)? {
            Some(true) => {
                let Some(proof) = closed_block_tail_with_bindings(then_branch, bindings) else {
                    return Ok(false);
                };
                let (tail, selected_bindings) = proof?;
                let Some(tail) = tail else {
                    return Ok(false);
                };
                is_closed_total_value_checked_with_bindings(tail, &selected_bindings)
            }
            Some(false) => is_closed_total_value_checked_with_bindings(else_branch, bindings),
            None => Ok(false),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let Some((value, selected_bindings)) = selected_match_value_checked_with_bindings(
                scrutinee,
                *enumeration,
                arms,
                bindings,
            )?
            else {
                return Ok(false);
            };
            is_closed_total_value_checked_with_bindings(value, &selected_bindings)
        }
        ExpressionKind::Block(block) => {
            let Some(proof) = closed_block_tail_with_bindings(block, bindings) else {
                return Ok(false);
            };
            let (tail, selected_bindings) = proof?;
            let Some(tail) = tail else {
                return Ok(false);
            };
            is_closed_total_value_checked_with_bindings(tail, &selected_bindings)
        }
        _ => Ok(false),
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

pub(crate) fn closed_block_tail_with_bindings<'a>(
    block: &'a Block,
    bindings: &[ClosedBinding<'a>],
) -> Option<ClosedBlockProof<'a>> {
    let mut block_bindings = bindings.to_vec();
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding {
                binding,
                initializer,
            } if !binding.mutable && binding.ty == initializer.ty => {
                if initializer.ty == Type::Int {
                    let value = match constant_int::evaluate_checked_with_bindings(
                        initializer,
                        &block_bindings,
                    ) {
                        Ok(Some(value)) => value,
                        Ok(None) => return None,
                        Err(failure) => return Some(Err(failure)),
                    };
                    if let Err(error) = value {
                        return Some(Err(ClosedConditionArithmeticFailure {
                            error,
                            span: initializer.span,
                        }));
                    }
                } else {
                    match is_closed_total_value_checked_with_bindings(initializer, &block_bindings)
                    {
                        Ok(true) => {}
                        Ok(false) => return None,
                        Err(failure) => return Some(Err(failure)),
                    }
                }
                block_bindings.push(ClosedBinding {
                    binding,
                    value: initializer,
                });
            }
            StatementKind::Expression(expression) => {
                if expression.ty == Type::Int {
                    let value = match constant_int::evaluate_checked_with_bindings(
                        expression,
                        &block_bindings,
                    ) {
                        Ok(Some(value)) => value,
                        Ok(None) => return None,
                        Err(failure) => return Some(Err(failure)),
                    };
                    if let Err(error) = value {
                        return Some(Err(ClosedConditionArithmeticFailure {
                            error,
                            span: expression.span,
                        }));
                    }
                } else {
                    match is_closed_total_value_checked_with_bindings(expression, &block_bindings) {
                        Ok(true) => {}
                        Ok(false) => return None,
                        Err(failure) => return Some(Err(failure)),
                    }
                }
            }
            _ => return None,
        }
    }

    Some(Ok((block.tail.as_deref(), block_bindings)))
}

pub(crate) fn selected_match_value_checked_with_bindings<'a>(
    scrutinee: &'a Expression,
    enumeration: EnumId,
    arms: &'a [MatchArm],
    bindings: &[ClosedBinding<'a>],
) -> Result<Option<ClosedSelectedValue<'a>>, ClosedConditionArithmeticFailure> {
    let Some((scrutinee_enum, variant_index, payload, mut selected_bindings)) =
        match_variant_checked_with_bindings(scrutinee, bindings)?
    else {
        return Ok(None);
    };
    if scrutinee_enum != enumeration {
        return Ok(None);
    }

    let mut selected = arms.iter().filter(|arm| arm.variant_index == variant_index);
    let Some(arm) = selected.next() else {
        return Ok(None);
    };
    if selected.next().is_some() {
        return Ok(None);
    }

    match (payload, arm.binding.as_ref(), arm.payload_discarded) {
        (None, None, false) | (Some(_), None, true) => {}
        (Some(payload), Some(binding), false) if binding.ty == payload.ty => {
            selected_bindings.push(ClosedBinding {
                binding,
                value: payload,
            });
        }
        _ => return Ok(None),
    }

    Ok(Some((&arm.value, selected_bindings)))
}

pub(crate) fn selected_record_field_value_checked_with_bindings<'a>(
    base: &'a Expression,
    record: RecordId,
    field_index: usize,
    bindings: &[ClosedBinding<'a>],
) -> Result<Option<ClosedSelectedValue<'a>>, ClosedConditionArithmeticFailure> {
    match &base.kind {
        ExpressionKind::RecordLiteral {
            record: actual_record,
            fields,
        } if *actual_record == record => {
            let mut all_closed = true;
            for field in fields {
                let field_closed =
                    is_closed_total_value_checked_with_bindings(&field.value, bindings)?;
                all_closed &= field_closed;
            }
            if !all_closed {
                return Ok(None);
            }
            let mut selected = fields
                .iter()
                .filter(|field| field.field_index == field_index);
            let Some(field) = selected.next() else {
                return Ok(None);
            };
            if selected.next().is_some() {
                return Ok(None);
            }
            Ok(Some((&field.value, bindings.to_vec())))
        }
        ExpressionKind::Binding(reference) => {
            let Some(value) = closed_binding_value(reference, &base.ty, bindings) else {
                return Ok(None);
            };
            selected_record_field_value_checked_with_bindings(value, record, field_index, bindings)
        }
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_checked_with_bindings(condition, bindings)? {
            Some(true) => {
                let Some(proof) = closed_block_tail_with_bindings(then_branch, bindings) else {
                    return Ok(None);
                };
                let (tail, selected_bindings) = proof?;
                let Some(tail) = tail else {
                    return Ok(None);
                };
                selected_record_field_value_checked_with_bindings(
                    tail,
                    record,
                    field_index,
                    &selected_bindings,
                )
            }
            Some(false) => selected_record_field_value_checked_with_bindings(
                else_branch,
                record,
                field_index,
                bindings,
            ),
            None => Ok(None),
        },
        ExpressionKind::Match {
            scrutinee,
            enumeration,
            arms,
        } => {
            let Some((value, selected_bindings)) = selected_match_value_checked_with_bindings(
                scrutinee,
                *enumeration,
                arms,
                bindings,
            )?
            else {
                return Ok(None);
            };
            selected_record_field_value_checked_with_bindings(
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
            let Some((outer_value, outer_bindings)) =
                selected_record_field_value_checked_with_bindings(
                    outer_base,
                    *outer_record,
                    *outer_field_index,
                    bindings,
                )?
            else {
                return Ok(None);
            };
            selected_record_field_value_checked_with_bindings(
                outer_value,
                record,
                field_index,
                &outer_bindings,
            )
        }
        ExpressionKind::Block(block) => {
            let Some(proof) = closed_block_tail_with_bindings(block, bindings) else {
                return Ok(None);
            };
            let (tail, selected_bindings) = proof?;
            let Some(tail) = tail else {
                return Ok(None);
            };
            selected_record_field_value_checked_with_bindings(
                tail,
                record,
                field_index,
                &selected_bindings,
            )
        }
        _ => Ok(None),
    }
}
