use nova_parser::ast::{self, ExpressionKind, StatementKind, UnaryOperator};

/// Canonicalizes the bootstrap numeric associated constants before ordinary
/// name/type resolution.
///
/// The parser intentionally keeps `Qualifier::Member` syntax generic. `Int` is
/// already a reserved primitive type, so these two payload-free paths cannot
/// conflict with a user-defined enum and can be lowered to the same literal HIR
/// used by every other signed-64 boundary value.
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
        } if enumeration.text == "Int" && payload.is_none() => match variant.text.as_str() {
            "MAX" => Some(IntBoundary::Max),
            "MIN" => Some(IntBoundary::Min),
            _ => None,
        },
        _ => None,
    };

    if let Some(boundary) = builtin {
        expression.kind = match boundary {
            IntBoundary::Max => ExpressionKind::Integer(i64::MAX as u64),
            IntBoundary::Min => ExpressionKind::Unary {
                operator: UnaryOperator::Negate,
                operand: Box::new(ast::Expression {
                    kind: ExpressionKind::Integer(1_u64 << 63),
                    span: expression.span,
                }),
            },
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

#[derive(Clone, Copy)]
enum IntBoundary {
    Min,
    Max,
}

#[cfg(test)]
mod tests {
    use super::canonicalize_int_constants;
    use nova_lexer::lex;
    use nova_parser::{ast::ExpressionKind, parse};
    use nova_source::{SourceFile, SourceId};

    #[test]
    fn canonicalizes_int_boundaries_without_claiming_other_members() {
        let source = SourceFile::new(
            SourceId::new(0),
            "test.nv",
            "fn main() -> Int { if true { Int::MAX } else { Int::MIN } }",
        );
        let lexed = lex(&source);
        assert!(lexed.diagnostics.is_empty());
        let parsed = parse(&source, &lexed.tokens);
        assert!(parsed.diagnostics.is_empty());

        let program = canonicalize_int_constants(&parsed.program);
        let tail = program.functions[0].body.tail.as_ref().expect("function tail");
        let ExpressionKind::If {
            then_branch,
            else_branch,
            ..
        } = &tail.kind
        else {
            panic!("expected if tail");
        };
        assert!(matches!(
            then_branch.tail.as_ref().map(|expression| &expression.kind),
            Some(ExpressionKind::Integer(value)) if *value == i64::MAX as u64
        ));
        assert!(matches!(
            &else_branch.kind,
            ExpressionKind::Block(block)
                if matches!(
                    block.tail.as_ref().map(|expression| &expression.kind),
                    Some(ExpressionKind::Unary { operator: nova_parser::ast::UnaryOperator::Negate, operand })
                        if matches!(operand.kind, ExpressionKind::Integer(value) if value == (1_u64 << 63))
                )
        ));
    }
}
