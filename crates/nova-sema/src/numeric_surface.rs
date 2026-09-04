use nova_parser::ast::{self, Block, ExpressionKind, StatementKind, UnaryOperator};

/// Canonicalizes the bootstrap numeric surface before ordinary name/type
/// resolution.
///
/// The parser intentionally keeps `Qualifier::Member` syntax generic. `Int` is
/// already a reserved primitive type, so the built-in spellings handled here
/// cannot conflict with a user-defined enum. Boundary constants lower to the
/// same literal HIR used by source integers, while `Int::from(Bool)` lowers to a
/// normal conditional expression so the operand is evaluated exactly once and
/// ordinary Bool type checking remains authoritative.
pub(crate) fn canonicalize_int_constants(program: &ast::Program) -> ast::Program {
    let mut program = program.clone();
    for function in &mut program.functions {
        rewrite_block(&mut function.body);
    }
    program
}

fn rewrite_block(block: &mut ast::Block) {
    for statement in &mut block.statements {
        match &mut statement.kind {
            StatementKind::Binding { initializer, .. } => rewrite_expression(initializer),
            StatementKind::UninitializedBinding { .. }
            | StatementKind::Break
            | StatementKind::Continue => {}
            StatementKind::Assignment { value, .. } => rewrite_expression(value),
            StatementKind::While { condition, body } => {
                rewrite_expression(condition);
                rewrite_block(body);
            }
            StatementKind::Return(value) => {
                if let Some(value) = value {
                    rewrite_expression(value);
                }
            }
            StatementKind::Expression(expression) => rewrite_expression(expression),
        }
    }

    if let Some(tail) = &mut block.tail {
        rewrite_expression(tail);
    }
}

fn rewrite_expression(expression: &mut ast::Expression) {
    let builtin = match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant,
            payload,
        } if enumeration.text == "Int" => match (variant.text.as_str(), payload) {
            ("MAX", None) => Some(IntBuiltin::Boundary(IntBoundary::Max)),
            ("MIN", None) => Some(IntBuiltin::Boundary(IntBoundary::Min)),
            ("from", Some(payload)) => Some(IntBuiltin::FromBool((**payload).clone())),
            _ => None,
        },
        _ => None,
    };

    if let Some(builtin) = builtin {
        expression.kind = match builtin {
            IntBuiltin::Boundary(IntBoundary::Max) => ExpressionKind::Integer(i64::MAX as u64),
            IntBuiltin::Boundary(IntBoundary::Min) => ExpressionKind::Unary {
                operator: UnaryOperator::Negate,
                operand: Box::new(ast::Expression {
                    kind: ExpressionKind::Integer(1_u64 << 63),
                    span: expression.span,
                }),
            },
            IntBuiltin::FromBool(mut condition) => {
                rewrite_expression(&mut condition);
                ExpressionKind::If {
                    condition: Box::new(condition),
                    then_branch: int_literal_block(1, expression.span),
                    else_branch: Box::new(ast::Expression {
                        kind: ExpressionKind::Integer(0),
                        span: expression.span,
                    }),
                }
            }
        };
        return;
    }

    match &mut expression.kind {
        ExpressionKind::Integer(_)
        | ExpressionKind::String(_)
        | ExpressionKind::Boolean(_)
        | ExpressionKind::Unit
        | ExpressionKind::Name(_) => {}
        ExpressionKind::Lambda { body, .. } => rewrite_block(body),
        ExpressionKind::RecordLiteral { fields, .. } => {
            for field in fields {
                rewrite_expression(&mut field.value);
            }
        }
        ExpressionKind::EnumConstructor { payload, .. } => {
            if let Some(payload) = payload {
                rewrite_expression(payload);
            }
        }
        ExpressionKind::FieldAccess { base, .. } => rewrite_expression(base),
        ExpressionKind::Unary { operand, .. } => rewrite_expression(operand),
        ExpressionKind::Binary { left, right, .. } => {
            rewrite_expression(left);
            rewrite_expression(right);
        }
        ExpressionKind::Call { callee, arguments } => {
            rewrite_expression(callee);
            for argument in arguments {
                rewrite_expression(argument);
            }
        }
        ExpressionKind::Block(block) => rewrite_block(block),
        ExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            rewrite_expression(condition);
            rewrite_block(then_branch);
            rewrite_expression(else_branch);
        }
        ExpressionKind::Match {
            scrutinee, arms, ..
        } => {
            rewrite_expression(scrutinee);
            for arm in arms {
                rewrite_expression(&mut arm.value);
            }
        }
    }
}

fn int_literal_block(value: u64, span: nova_source::Span) -> Block {
    Block {
        statements: Vec::new(),
        tail: Some(Box::new(ast::Expression {
            kind: ExpressionKind::Integer(value),
            span,
        })),
        span,
    }
}

#[derive(Clone)]
enum IntBuiltin {
    Boundary(IntBoundary),
    FromBool(ast::Expression),
}

#[derive(Clone, Copy)]
enum IntBoundary {
    Min,
    Max,
}
