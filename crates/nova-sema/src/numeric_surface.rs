use nova_parser::ast::{self, BinaryOperator, Block, ExpressionKind, StatementKind, UnaryOperator};

/// Canonicalizes the bootstrap numeric surface before ordinary name/type
/// resolution.
///
/// The parser intentionally keeps `Qualifier::Member` syntax generic. `Int` and
/// `Bool` are already reserved primitive types, so the built-in spellings handled
/// here cannot conflict with a user-defined enum. Boundary constants lower to the
/// same literal HIR used by source integers, while explicit conversions and numeric
/// predicates lower to ordinary typed expressions so operands are evaluated exactly
/// once and normal type checking remains authoritative.
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
            ("MAX", None) => Some(NumericBuiltin::IntBoundary(IntBoundary::Max)),
            ("MIN", None) => Some(NumericBuiltin::IntBoundary(IntBoundary::Min)),
            ("from", Some(payload)) => Some(NumericBuiltin::IntFromBool((**payload).clone())),
            ("abs", Some(payload)) => Some(NumericBuiltin::IntAbs((**payload).clone())),
            ("is_negative", Some(payload)) => Some(NumericBuiltin::IntPredicate(
                IntPredicate::Negative,
                (**payload).clone(),
            )),
            ("is_zero", Some(payload)) => Some(NumericBuiltin::IntPredicate(
                IntPredicate::Zero,
                (**payload).clone(),
            )),
            ("is_positive", Some(payload)) => Some(NumericBuiltin::IntPredicate(
                IntPredicate::Positive,
                (**payload).clone(),
            )),
            ("is_even", Some(payload)) => Some(NumericBuiltin::IntParityPredicate(
                IntParityPredicate::Even,
                (**payload).clone(),
            )),
            ("is_odd", Some(payload)) => Some(NumericBuiltin::IntParityPredicate(
                IntParityPredicate::Odd,
                (**payload).clone(),
            )),
            _ => None,
        },
        ExpressionKind::EnumConstructor {
            enumeration,
            variant,
            payload: Some(payload),
        } if enumeration.text == "Bool" && variant.text == "from" => {
            Some(NumericBuiltin::BoolFromInt((**payload).clone()))
        }
        _ => None,
    };

    if let Some(builtin) = builtin {
        expression.kind = match builtin {
            NumericBuiltin::IntBoundary(IntBoundary::Max) => {
                ExpressionKind::Integer(i64::MAX as u64)
            }
            NumericBuiltin::IntBoundary(IntBoundary::Min) => ExpressionKind::Unary {
                operator: UnaryOperator::Negate,
                operand: Box::new(ast::Expression {
                    kind: ExpressionKind::Integer(1_u64 << 63),
                    span: expression.span,
                }),
            },
            NumericBuiltin::IntFromBool(mut condition) => {
                rewrite_expression(&mut condition);
                ExpressionKind::If {
                    condition: Box::new(condition),
                    then_branch: int_literal_block(1, expression.span),
                    else_branch: Box::new(int_literal(0, expression.span)),
                }
            }
            NumericBuiltin::BoolFromInt(mut operand) => {
                rewrite_expression(&mut operand);
                ExpressionKind::Binary {
                    operator: BinaryOperator::NotEqual,
                    left: Box::new(operand),
                    right: Box::new(int_literal(0, expression.span)),
                }
            }
            NumericBuiltin::IntAbs(mut operand) => {
                rewrite_expression(&mut operand);
                int_abs_expression(operand, expression.span).kind
            }
            NumericBuiltin::IntPredicate(predicate, mut operand) => {
                rewrite_expression(&mut operand);
                ExpressionKind::Binary {
                    operator: match predicate {
                        IntPredicate::Negative => BinaryOperator::Less,
                        IntPredicate::Zero => BinaryOperator::Equal,
                        IntPredicate::Positive => BinaryOperator::Greater,
                    },
                    left: Box::new(operand),
                    right: Box::new(int_literal(0, expression.span)),
                }
            }
            NumericBuiltin::IntParityPredicate(predicate, mut operand) => {
                rewrite_expression(&mut operand);
                let remainder = ast::Expression {
                    kind: ExpressionKind::Binary {
                        operator: BinaryOperator::Remainder,
                        left: Box::new(operand),
                        right: Box::new(int_literal(2, expression.span)),
                    },
                    span: expression.span,
                };
                ExpressionKind::Binary {
                    operator: match predicate {
                        IntParityPredicate::Even => BinaryOperator::Equal,
                        IntParityPredicate::Odd => BinaryOperator::NotEqual,
                    },
                    left: Box::new(remainder),
                    right: Box::new(int_literal(0, expression.span)),
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

fn int_abs_expression(operand: ast::Expression, span: nova_source::Span) -> ast::Expression {
    let temporary = ast::Name {
        text: "__nova_int_abs_operand".to_owned(),
        span,
    };
    let temporary_expression = || ast::Expression {
        kind: ExpressionKind::Name(temporary.clone()),
        span,
    };
    ast::Expression {
        kind: ExpressionKind::Block(Block {
            statements: vec![ast::Statement {
                kind: StatementKind::Binding {
                    mutable: false,
                    name: temporary.clone(),
                    annotation: None,
                    initializer: operand,
                },
                span,
            }],
            tail: Some(Box::new(ast::Expression {
                kind: ExpressionKind::If {
                    condition: Box::new(ast::Expression {
                        kind: ExpressionKind::Binary {
                            operator: BinaryOperator::Less,
                            left: Box::new(temporary_expression()),
                            right: Box::new(int_literal(0, span)),
                        },
                        span,
                    }),
                    then_branch: Block {
                        statements: Vec::new(),
                        tail: Some(Box::new(ast::Expression {
                            kind: ExpressionKind::Unary {
                                operator: UnaryOperator::Negate,
                                operand: Box::new(temporary_expression()),
                            },
                            span,
                        })),
                        span,
                    },
                    else_branch: Box::new(temporary_expression()),
                },
                span,
            })),
            span,
        }),
        span,
    }
}

fn int_literal(value: u64, span: nova_source::Span) -> ast::Expression {
    ast::Expression {
        kind: ExpressionKind::Integer(value),
        span,
    }
}

fn int_literal_block(value: u64, span: nova_source::Span) -> Block {
    Block {
        statements: Vec::new(),
        tail: Some(Box::new(int_literal(value, span))),
        span,
    }
}

#[derive(Clone)]
enum NumericBuiltin {
    IntBoundary(IntBoundary),
    IntFromBool(ast::Expression),
    BoolFromInt(ast::Expression),
    IntAbs(ast::Expression),
    IntPredicate(IntPredicate, ast::Expression),
    IntParityPredicate(IntParityPredicate, ast::Expression),
}

#[derive(Clone, Copy)]
enum IntBoundary {
    Min,
    Max,
}

#[derive(Clone, Copy)]
enum IntPredicate {
    Negative,
    Zero,
    Positive,
}

#[derive(Clone, Copy)]
enum IntParityPredicate {
    Even,
    Odd,
}
