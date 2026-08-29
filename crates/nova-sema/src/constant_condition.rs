use crate::constant_int;
use crate::hir::{Expression, ExpressionKind, FunctionId, Type};
use nova_parser::ast::{BinaryOperator, UnaryOperator};

/// Evaluates only side-effect-free, closed bootstrap conditions whose value is
/// already determined by literal Bool/Int operations. The HIR is never folded.
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
                    enum_tag(left)? == enum_tag(right)?
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
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            match block.tail.as_deref() {
                Some(tail) => unit_value(tail),
                None => Some(()),
            }
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
        } if payload.is_none() => Some((*enumeration, *variant_index)),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            enum_tag(block.tail.as_deref()?)
        }
        _ => None,
    }
}

fn function_id(expression: &Expression) -> Option<FunctionId> {
    match &expression.kind {
        ExpressionKind::Function(function) => Some(*function),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            function_id(block.tail.as_deref()?)
        }
        _ => None,
    }
}
