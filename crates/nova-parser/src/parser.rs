use crate::ast::{
    BinaryOperator, Block, Enum, EnumPattern, EnumVariant, Expression, ExpressionKind, Function,
    MatchArm, Name, Parameter, Program, Record, RecordField, RecordLiteralField, Statement,
    StatementKind, TypeRef, TypeRefKind, UnaryOperator,
};
use nova_diagnostics::Diagnostic;
use nova_lexer::{Token, TokenKind, decode_string_literal};
use nova_source::{SourceFile, Span};

const MAX_EXPRESSION_DEPTH: usize = 256;
const MAX_TYPE_DEPTH: usize = 128;
const OR_BINDING_POWER: (u8, u8) = (1, 2);
const AND_BINDING_POWER: (u8, u8) = (3, 4);
const EQUALITY_BINDING_POWER: (u8, u8) = (5, 6);
const COMPARISON_BINDING_POWER: (u8, u8) = (7, 8);
const ADDITIVE_BINDING_POWER: (u8, u8) = (9, 10);
const MULTIPLICATIVE_BINDING_POWER: (u8, u8) = (11, 12);
const PREFIX_BINDING_POWER: u8 = 13;
const POSTFIX_BINDING_POWER: u8 = 15;

/// Complete deterministic result of parsing one token stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseOutput {
    /// Parsed declarations, including successfully recovered later declarations.
    pub program: Program,
    /// Syntax diagnostics in source order.
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseOutput {
    /// Reports whether the token stream matched the implemented grammar.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Parses tokens produced for `source`.
///
/// The parser normalizes a missing EOF marker, bounds expression recursion, and
/// ensures every recovery loop either consumes a token or terminates.
#[must_use]
pub fn parse(source: &SourceFile, tokens: &[Token]) -> ParseOutput {
    let mut parser = Parser::new(source, tokens);
    let program = parser.parse_program();
    ParseOutput {
        program,
        diagnostics: parser.diagnostics,
    }
}

struct Parser<'source> {
    source: &'source SourceFile,
    tokens: Vec<Token>,
    position: usize,
    diagnostics: Vec<Diagnostic>,
    expression_depth: usize,
    depth_diagnostic_emitted: bool,
}

impl<'source> Parser<'source> {
    fn new(source: &'source SourceFile, tokens: &[Token]) -> Self {
        let mut normalized = Vec::with_capacity(tokens.len().saturating_add(1));
        for token in tokens {
            normalized.push(*token);
            if matches!(token.kind, TokenKind::Eof) {
                break;
            }
        }
        if !matches!(
            normalized.last().map(|token| token.kind),
            Some(TokenKind::Eof)
        ) {
            normalized.push(Token {
                kind: TokenKind::Eof,
                span: source.eof_span(),
            });
        }

        Self {
            source,
            tokens: normalized,
            position: 0,
            diagnostics: Vec::new(),
            expression_depth: 0,
            depth_diagnostic_emitted: false,
        }
    }

    fn parse_program(&mut self) -> Program {
        let mut records = Vec::new();
        let mut enums = Vec::new();
        let mut functions = Vec::new();

        while !self.at(TokenKind::Eof) {
            let before = self.position;
            if self.at(TokenKind::Record) {
                if let Some(record) = self.parse_record() {
                    records.push(record);
                } else {
                    self.recover_top_level();
                }
            } else if self.at(TokenKind::Enum) {
                if let Some(enumeration) = self.parse_enum() {
                    enums.push(enumeration);
                } else {
                    self.recover_top_level();
                }
            } else if self.at(TokenKind::Fn) {
                if let Some(function) = self.parse_function() {
                    functions.push(function);
                } else {
                    self.recover_top_level();
                }
            } else {
                let token = self.current();
                self.diagnostics.push(
                    Diagnostic::error("N2003", "expected a top-level declaration").with_primary(
                        token.span,
                        format!(
                            "found {}; only `record`, `enum`, and `fn` declarations are supported",
                            token.kind
                        ),
                    ),
                );
                self.recover_top_level();
            }

            if self.position == before && !self.at(TokenKind::Eof) {
                self.bump();
            }
        }

        Program {
            records,
            enums,
            functions,
            span: self
                .source
                .span(0, self.source.len())
                .unwrap_or(self.source.eof_span()),
        }
    }

    fn parse_enum(&mut self) -> Option<Enum> {
        let start = self
            .expect(TokenKind::Enum, "to start an enum declaration")?
            .span;
        let name = self.parse_name("after `enum`")?;
        self.expect(TokenKind::LeftBrace, "after the enum name")?;
        let mut variants = Vec::new();

        if self.at(TokenKind::RightBrace) {
            self.diagnostics.push(
                Diagnostic::error("N2001", "expected an enum variant").with_primary(
                    self.current().span,
                    "an enum must declare at least one variant",
                ),
            );
        } else {
            loop {
                let variant_name = self.parse_name("as an enum variant name")?;
                let (payload, end) = if self.consume(TokenKind::LeftParen).is_some() {
                    let payload = self.parse_type_ref("as the enum variant payload type")?;
                    let closing =
                        self.expect(TokenKind::RightParen, "after the enum variant payload type")?;
                    (Some(payload), closing.span)
                } else {
                    (None, variant_name.span)
                };
                let span = self.cover(variant_name.span, end);
                variants.push(EnumVariant {
                    name: variant_name,
                    payload,
                    span,
                });

                if self.consume(TokenKind::Comma).is_none() {
                    break;
                }
                if self.at(TokenKind::RightBrace) {
                    break;
                }
            }
        }

        let closing = self.expect(TokenKind::RightBrace, "to close the enum declaration")?;
        Some(Enum {
            name,
            variants,
            span: self.cover(start, closing.span),
        })
    }

    fn parse_record(&mut self) -> Option<Record> {
        let start = self
            .expect(TokenKind::Record, "to start a record declaration")?
            .span;
        let name = self.parse_name("after `record`")?;
        self.expect(TokenKind::LeftBrace, "after the record name")?;
        let mut fields = Vec::new();

        if !self.at(TokenKind::RightBrace) {
            loop {
                let field_name = self.parse_name("as a record field name")?;
                self.expect(TokenKind::Colon, "after the record field name")?;
                let ty = self.parse_type_ref("after `:` in the record field")?;
                let span = self.cover(field_name.span, ty.span);
                fields.push(RecordField {
                    name: field_name,
                    ty,
                    span,
                });

                if self.consume(TokenKind::Comma).is_none() {
                    break;
                }
                if self.at(TokenKind::RightBrace) {
                    break;
                }
            }
        }

        let closing = self.expect(TokenKind::RightBrace, "to close the record declaration")?;
        Some(Record {
            name,
            fields,
            span: self.cover(start, closing.span),
        })
    }

    fn parse_function(&mut self) -> Option<Function> {
        let start = self.expect(TokenKind::Fn, "to start a function")?.span;
        let name = self.parse_name("after `fn`")?;
        self.expect(TokenKind::LeftParen, "after the function name")?;
        let parameters = self.parse_parameters()?;
        self.expect(TokenKind::RightParen, "after the parameter list")?;
        self.expect(TokenKind::Arrow, "before the explicit return type")?;
        let return_type = self.parse_type_ref("after `->`")?;
        let body = self.parse_block()?;
        let span = self.cover(start, body.span);

        Some(Function {
            name,
            parameters,
            return_type,
            body,
            span,
        })
    }

    fn parse_parameters(&mut self) -> Option<Vec<Parameter>> {
        let mut parameters = Vec::new();
        if self.at(TokenKind::RightParen) {
            return Some(parameters);
        }

        loop {
            let name = self.parse_name("in the parameter list")?;
            self.expect(TokenKind::Colon, "after a parameter name")?;
            let ty = self.parse_type_ref("after `:`")?;
            let span = self.cover(name.span, ty.span);
            parameters.push(Parameter { name, ty, span });

            if self.consume(TokenKind::Comma).is_none() {
                break;
            }
            if self.at(TokenKind::RightParen) {
                break;
            }
        }
        Some(parameters)
    }

    fn parse_type_ref(&mut self, context: &str) -> Option<TypeRef> {
        self.parse_type_ref_with_depth(context, 0)
    }

    fn parse_type_ref_with_depth(&mut self, context: &str, depth: usize) -> Option<TypeRef> {
        if depth >= MAX_TYPE_DEPTH {
            let token = self.current();
            self.diagnostics.push(
                Diagnostic::error("N2009", "type nesting limit exceeded").with_primary(
                    token.span,
                    format!(
                        "the bootstrap parser accepts at most {MAX_TYPE_DEPTH} nested type frames"
                    ),
                ),
            );
            return None;
        }

        if let Some(bang) = self.consume(TokenKind::Bang) {
            return Some(TypeRef {
                kind: TypeRefKind::Never,
                span: bang.span,
            });
        }

        if let Some(keyword) = self.consume(TokenKind::Fn) {
            self.expect(TokenKind::LeftParen, "after `fn` in a function type")?;
            let mut parameters = Vec::new();
            if !self.at(TokenKind::RightParen) {
                loop {
                    parameters.push(
                        self.parse_type_ref_with_depth("as a function-type parameter", depth + 1)?,
                    );
                    if self.consume(TokenKind::Comma).is_none() {
                        break;
                    }
                    if self.at(TokenKind::RightParen) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RightParen, "after function-type parameters")?;
            self.expect(TokenKind::Arrow, "before a function-type return type")?;
            let return_type =
                self.parse_type_ref_with_depth("after `->` in a function type", depth + 1)?;
            let span = self.cover(keyword.span, return_type.span);
            return Some(TypeRef {
                kind: TypeRefKind::Function {
                    parameters,
                    return_type: Box::new(return_type),
                },
                span,
            });
        }

        let name = self.parse_name(context)?;
        Some(TypeRef {
            span: name.span,
            kind: TypeRefKind::Named(name),
        })
    }

    fn parse_name(&mut self, context: &str) -> Option<Name> {
        let token = self.expect(TokenKind::Identifier, context)?;
        Some(Name {
            text: self.source.slice(token.span).unwrap_or("").to_owned(),
            span: token.span,
        })
    }

    fn parse_block(&mut self) -> Option<Block> {
        let opening = self.expect(TokenKind::LeftBrace, "to start a block")?.span;
        let mut statements = Vec::new();
        let mut tail = None;

        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            if (self.at(TokenKind::Fn) && self.at_offset(1, TokenKind::Identifier))
                || self.at(TokenKind::Record)
                || self.at(TokenKind::Enum)
            {
                break;
            }
            let before = self.position;
            let statement = if self.at(TokenKind::Let) || self.at(TokenKind::Var) {
                self.parse_binding_statement()
            } else if self.at(TokenKind::While) {
                self.parse_while_statement()
            } else if self.at(TokenKind::Break) {
                self.parse_break_statement()
            } else if self.at(TokenKind::Continue) {
                self.parse_continue_statement()
            } else if self.at(TokenKind::Return) {
                self.parse_return_statement()
            } else if self.at(TokenKind::Identifier) && self.at_offset(1, TokenKind::Equal) {
                self.parse_assignment_statement()
            } else {
                match self.parse_expression() {
                    Some(expression) => {
                        if let Some(semicolon) = self.consume(TokenKind::Semicolon) {
                            let span = self.cover(expression.span, semicolon.span);
                            Some(Statement {
                                kind: StatementKind::Expression(expression),
                                span,
                            })
                        } else if self.at(TokenKind::RightBrace) {
                            tail = Some(Box::new(expression));
                            break;
                        } else {
                            let token = self.current();
                            self.diagnostics.push(
                                Diagnostic::error("N2004", "expected `;` or `}` after expression")
                                    .with_primary(
                                        token.span,
                                        format!(
                                            "found {} immediately after this expression",
                                            token.kind
                                        ),
                                    ),
                            );
                            None
                        }
                    }
                    None => None,
                }
            };

            if let Some(statement) = statement {
                statements.push(statement);
            } else {
                self.recover_statement();
            }

            if self.position == before
                && !self.at(TokenKind::RightBrace)
                && !self.at(TokenKind::Eof)
                && !(self.at(TokenKind::Fn) && self.at_offset(1, TokenKind::Identifier))
                && !self.at(TokenKind::Record)
                && !self.at(TokenKind::Enum)
            {
                self.bump();
            }
        }

        let closing = self.expect(TokenKind::RightBrace, "to close the block");
        let end = closing.map_or_else(|| self.current().span, |token| token.span);
        Some(Block {
            statements,
            tail,
            span: self.cover(opening, end),
        })
    }

    fn parse_binding_statement(&mut self) -> Option<Statement> {
        let keyword = self.bump();
        let mutable = matches!(keyword.kind, TokenKind::Var);
        let name = self.parse_name("after the binding keyword")?;
        let annotation = if self.consume(TokenKind::Colon).is_some() {
            Some(self.parse_type_ref("after `:`")?)
        } else {
            None
        };

        if self.consume(TokenKind::Equal).is_some() {
            let initializer = self.parse_expression()?;
            let semicolon = self.expect(TokenKind::Semicolon, "after the binding initializer")?;
            return Some(Statement {
                span: self.cover(keyword.span, semicolon.span),
                kind: StatementKind::Binding {
                    mutable,
                    name,
                    annotation,
                    initializer,
                },
            });
        }

        if mutable {
            if let Some(annotation) = annotation {
                let semicolon = self.expect(
                    TokenKind::Semicolon,
                    "after the uninitialized mutable binding",
                )?;
                return Some(Statement {
                    span: self.cover(keyword.span, semicolon.span),
                    kind: StatementKind::UninitializedBinding { name, annotation },
                });
            }
        }

        self.expect(TokenKind::Equal, "before the binding initializer")?;
        None
    }

    fn parse_assignment_statement(&mut self) -> Option<Statement> {
        let target = self.parse_name("as the assignment target")?;
        self.expect(TokenKind::Equal, "after the assignment target")?;
        let value = self.parse_expression()?;
        let semicolon = self.expect(TokenKind::Semicolon, "after the assigned value")?;
        Some(Statement {
            span: self.cover(target.span, semicolon.span),
            kind: StatementKind::Assignment { target, value },
        })
    }

    fn parse_while_statement(&mut self) -> Option<Statement> {
        let keyword = self.expect(TokenKind::While, "to start a while statement")?;
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        Some(Statement {
            span: self.cover(keyword.span, body.span),
            kind: StatementKind::While { condition, body },
        })
    }

    fn parse_break_statement(&mut self) -> Option<Statement> {
        let keyword = self.expect(TokenKind::Break, "to start a break statement")?;
        let semicolon = self.expect(TokenKind::Semicolon, "after `break`")?;
        Some(Statement {
            span: self.cover(keyword.span, semicolon.span),
            kind: StatementKind::Break,
        })
    }

    fn parse_continue_statement(&mut self) -> Option<Statement> {
        let keyword = self.expect(TokenKind::Continue, "to start a continue statement")?;
        let semicolon = self.expect(TokenKind::Semicolon, "after `continue`")?;
        Some(Statement {
            span: self.cover(keyword.span, semicolon.span),
            kind: StatementKind::Continue,
        })
    }

    fn parse_return_statement(&mut self) -> Option<Statement> {
        let keyword = self.expect(TokenKind::Return, "to start a return statement")?;
        let expression = if self.at(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        let semicolon = self.expect(TokenKind::Semicolon, "after `return`")?;
        Some(Statement {
            span: self.cover(keyword.span, semicolon.span),
            kind: StatementKind::Return(expression),
        })
    }

    fn parse_expression(&mut self) -> Option<Expression> {
        if self.expression_depth == 0 {
            self.depth_diagnostic_emitted = false;
        }
        self.parse_expression_with_binding_power(0)
    }

    fn parse_expression_with_binding_power(&mut self, minimum: u8) -> Option<Expression> {
        if self.expression_depth >= MAX_EXPRESSION_DEPTH {
            if !self.depth_diagnostic_emitted {
                let token = self.current();
                self.diagnostics.push(
                    Diagnostic::error("N2008", "expression nesting limit exceeded")
                        .with_primary(
                            token.span,
                            format!(
                                "the bootstrap parser accepts at most {MAX_EXPRESSION_DEPTH} nested expression frames"
                            ),
                        )
                        .with_note("this guarded implementation limit may change in later Nova versions"),
                );
                self.depth_diagnostic_emitted = true;
            }
            self.recover_deep_expression();
            return None;
        }

        self.expression_depth += 1;
        let result = self.parse_expression_inner(minimum);
        self.expression_depth -= 1;
        result
    }

    fn parse_expression_inner(&mut self, minimum: u8) -> Option<Expression> {
        let mut left = self.parse_prefix_expression()?;

        loop {
            if self.at(TokenKind::LeftParen) {
                if POSTFIX_BINDING_POWER < minimum {
                    break;
                }
                left = self.parse_call_expression(left)?;
                continue;
            }
            if self.at(TokenKind::Dot) {
                if POSTFIX_BINDING_POWER < minimum {
                    break;
                }
                left = self.parse_field_access_expression(left)?;
                continue;
            }

            let Some((operator, left_power, right_power)) = self.current_binary_operator() else {
                break;
            };
            if left_power < minimum {
                break;
            }
            self.bump();
            let right = self.parse_expression_with_binding_power(right_power)?;
            let span = self.cover(left.span, right.span);
            left = Expression {
                kind: ExpressionKind::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }

        Some(left)
    }

    fn parse_prefix_expression(&mut self) -> Option<Expression> {
        let token = self.current();
        match token.kind {
            TokenKind::Integer(value) => {
                self.bump();
                Some(Expression {
                    kind: ExpressionKind::Integer(value),
                    span: token.span,
                })
            }
            TokenKind::String => {
                self.bump();
                let Some(value) = decode_string_literal(self.source, token.span) else {
                    self.diagnostics.push(
                        Diagnostic::error("N2010", "invalid string literal token")
                            .with_primary(
                                token.span,
                                "the token span does not contain a valid Nova string literal",
                            )
                            .with_note(
                                "the parser rejects malformed synthetic token streams instead of manufacturing a string value",
                            ),
                    );
                    return None;
                };
                Some(Expression {
                    kind: ExpressionKind::String(value),
                    span: token.span,
                })
            }
            TokenKind::True | TokenKind::False => {
                self.bump();
                Some(Expression {
                    kind: ExpressionKind::Boolean(matches!(token.kind, TokenKind::True)),
                    span: token.span,
                })
            }
            TokenKind::Identifier if self.at_offset(1, TokenKind::ColonColon) => {
                self.parse_enum_constructor_expression()
            }
            TokenKind::Identifier => self.parse_name("as an expression").map(|name| Expression {
                span: name.span,
                kind: ExpressionKind::Name(name),
            }),
            TokenKind::New => self.parse_record_literal_expression(),
            TokenKind::Fn => self.parse_lambda_expression(),
            TokenKind::Minus | TokenKind::Bang => {
                self.bump();
                let operator = if matches!(token.kind, TokenKind::Minus) {
                    UnaryOperator::Negate
                } else {
                    UnaryOperator::Not
                };
                let operand = self.parse_expression_with_binding_power(PREFIX_BINDING_POWER)?;
                Some(Expression {
                    span: self.cover(token.span, operand.span),
                    kind: ExpressionKind::Unary {
                        operator,
                        operand: Box::new(operand),
                    },
                })
            }
            TokenKind::LeftParen => {
                let opening = self.bump();
                if let Some(closing) = self.consume(TokenKind::RightParen) {
                    return Some(Expression {
                        kind: ExpressionKind::Unit,
                        span: self.cover(opening.span, closing.span),
                    });
                }
                let mut expression = self.parse_expression_with_binding_power(0)?;
                let closing = self.expect(TokenKind::RightParen, "to close the expression")?;
                expression.span = self.cover(opening.span, closing.span);
                Some(expression)
            }
            TokenKind::LeftBrace => self.parse_block().map(|block| Expression {
                span: block.span,
                kind: ExpressionKind::Block(block),
            }),
            TokenKind::If => self.parse_if_expression(),
            TokenKind::Match => self.parse_match_expression(),
            _ => {
                self.diagnostics.push(
                    Diagnostic::error("N2002", "expected an expression").with_primary(
                        token.span,
                        format!("{} cannot start an expression", token.kind),
                    ),
                );
                None
            }
        }
    }

    fn parse_lambda_expression(&mut self) -> Option<Expression> {
        let keyword = self
            .expect(TokenKind::Fn, "to start an anonymous function")?
            .span;
        self.expect(TokenKind::LeftParen, "after `fn` in an anonymous function")?;
        let parameters = self.parse_parameters()?;
        self.expect(
            TokenKind::RightParen,
            "after the anonymous-function parameter list",
        )?;
        self.expect(
            TokenKind::Arrow,
            "before the anonymous function's explicit return type",
        )?;
        let return_type = self.parse_type_ref("after `->` in an anonymous function")?;
        let body = self.parse_block()?;
        Some(Expression {
            span: self.cover(keyword, body.span),
            kind: ExpressionKind::Lambda {
                parameters,
                return_type,
                body,
            },
        })
    }

    fn parse_enum_constructor_expression(&mut self) -> Option<Expression> {
        let enumeration = self.parse_name("as an enum constructor qualifier")?;
        self.expect(TokenKind::ColonColon, "after the enum type name")?;
        let variant = self.parse_name("after `::`")?;
        let (payload, end) = if self.consume(TokenKind::LeftParen).is_some() {
            let payload = self.parse_expression_with_binding_power(0)?;
            self.consume(TokenKind::Comma);
            let closing = self.expect(TokenKind::RightParen, "after the enum variant payload")?;
            (Some(Box::new(payload)), closing.span)
        } else {
            (None, variant.span)
        };

        Some(Expression {
            span: self.cover(enumeration.span, end),
            kind: ExpressionKind::EnumConstructor {
                enumeration,
                variant,
                payload,
            },
        })
    }

    fn parse_record_literal_expression(&mut self) -> Option<Expression> {
        let keyword = self.expect(TokenKind::New, "to start a record construction")?;
        let name = self.parse_name("after `new`")?;
        self.expect(TokenKind::LeftBrace, "after the record type name")?;
        let mut fields = Vec::new();

        if !self.at(TokenKind::RightBrace) {
            loop {
                let field_name = self.parse_name("as a record initializer field")?;
                self.expect(TokenKind::Colon, "after the record initializer field")?;
                let value = self.parse_expression()?;
                let span = self.cover(field_name.span, value.span);
                fields.push(RecordLiteralField {
                    name: field_name,
                    value,
                    span,
                });

                if self.consume(TokenKind::Comma).is_none() {
                    break;
                }
                if self.at(TokenKind::RightBrace) {
                    break;
                }
            }
        }

        let closing = self.expect(TokenKind::RightBrace, "to close the record construction")?;
        Some(Expression {
            span: self.cover(keyword.span, closing.span),
            kind: ExpressionKind::RecordLiteral { name, fields },
        })
    }

    fn parse_call_expression(&mut self, callee: Expression) -> Option<Expression> {
        self.expect(TokenKind::LeftParen, "to start the argument list")?;
        let mut arguments = Vec::new();

        if !self.at(TokenKind::RightParen) {
            loop {
                let before = self.position;
                if let Some(argument) = self.parse_expression() {
                    arguments.push(argument);
                } else {
                    self.recover_argument();
                }
                if self.position == before && !self.at(TokenKind::Eof) {
                    self.bump();
                }

                if self.consume(TokenKind::Comma).is_none() {
                    break;
                }
                if self.at(TokenKind::RightParen) {
                    break;
                }
            }
        }

        let closing = self.expect(TokenKind::RightParen, "after the argument list")?;
        Some(Expression {
            span: self.cover(callee.span, closing.span),
            kind: ExpressionKind::Call {
                callee: Box::new(callee),
                arguments,
            },
        })
    }

    fn parse_field_access_expression(&mut self, base: Expression) -> Option<Expression> {
        self.expect(TokenKind::Dot, "before the field name")?;
        let field = self.parse_name("after `.`")?;
        Some(Expression {
            span: self.cover(base.span, field.span),
            kind: ExpressionKind::FieldAccess {
                base: Box::new(base),
                field,
            },
        })
    }

    fn parse_if_expression(&mut self) -> Option<Expression> {
        let keyword = self.expect(TokenKind::If, "to start an if expression")?;
        let condition = self.parse_expression_with_binding_power(0)?;
        let then_branch = self.parse_block()?;

        if self.consume(TokenKind::Else).is_none() {
            let token = self.current();
            self.diagnostics.push(
                Diagnostic::error("N2006", "an if expression requires an else branch")
                    .with_primary(token.span, "expected `else` here")
                    .with_note("every `if` is value-producing in the implemented subset"),
            );
            return None;
        }

        let else_branch = if self.at(TokenKind::LeftBrace) {
            let block = self.parse_block()?;
            Expression {
                span: block.span,
                kind: ExpressionKind::Block(block),
            }
        } else if self.at(TokenKind::If) {
            self.parse_nested_if_expression()?
        } else {
            let token = self.current();
            self.diagnostics.push(
                Diagnostic::error("N2001", "expected a block or `if` after `else`")
                    .with_primary(token.span, format!("found {}", token.kind)),
            );
            return None;
        };

        Some(Expression {
            span: self.cover(keyword.span, else_branch.span),
            kind: ExpressionKind::If {
                condition: Box::new(condition),
                then_branch,
                else_branch: Box::new(else_branch),
            },
        })
    }

    fn parse_match_expression(&mut self) -> Option<Expression> {
        let keyword = self.expect(TokenKind::Match, "to start a match expression")?;
        let scrutinee = self.parse_expression_with_binding_power(0)?;
        self.expect(TokenKind::LeftBrace, "after the match scrutinee")?;
        let mut arms = Vec::new();

        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let pattern = self.parse_enum_pattern()?;
            self.expect(TokenKind::FatArrow, "after the match pattern")?;
            let value = self.parse_expression_with_binding_power(0)?;
            let span = self.cover(pattern.span, value.span);
            arms.push(MatchArm {
                pattern,
                value,
                span,
            });

            if self.consume(TokenKind::Comma).is_none() {
                break;
            }
        }

        let closing = self.expect(TokenKind::RightBrace, "to close the match expression")?;
        Some(Expression {
            span: self.cover(keyword.span, closing.span),
            kind: ExpressionKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
        })
    }

    fn parse_enum_pattern(&mut self) -> Option<EnumPattern> {
        let enumeration = self.parse_name("as a match pattern qualifier")?;
        self.expect(
            TokenKind::ColonColon,
            "after the enum type name in a pattern",
        )?;
        let variant = self.parse_name("after `::` in a pattern")?;
        let (binding, payload_discarded, end) = if self.consume(TokenKind::LeftParen).is_some() {
            let payload = self.parse_name("as the variant payload binding or `_`")?;
            let payload_discarded = payload.text == "_";
            let binding = if payload_discarded {
                None
            } else {
                Some(payload)
            };
            let closing = self.expect(TokenKind::RightParen, "after the payload pattern")?;
            (binding, payload_discarded, closing.span)
        } else {
            (None, false, variant.span)
        };
        Some(EnumPattern {
            span: self.cover(enumeration.span, end),
            enumeration,
            variant,
            binding,
            payload_discarded,
        })
    }

    fn parse_nested_if_expression(&mut self) -> Option<Expression> {
        if self.expression_depth >= MAX_EXPRESSION_DEPTH {
            if !self.depth_diagnostic_emitted {
                let token = self.current();
                self.diagnostics.push(
                    Diagnostic::error("N2008", "expression nesting limit exceeded").with_primary(
                        token.span,
                        "nested `else if` chain exceeds the parser budget",
                    ),
                );
                self.depth_diagnostic_emitted = true;
            }
            self.recover_deep_expression();
            return None;
        }

        self.expression_depth += 1;
        let result = self.parse_if_expression();
        self.expression_depth -= 1;
        result
    }

    fn current_binary_operator(&self) -> Option<(BinaryOperator, u8, u8)> {
        let (operator, powers) = match self.current().kind {
            TokenKind::OrOr => (BinaryOperator::Or, OR_BINDING_POWER),
            TokenKind::AndAnd => (BinaryOperator::And, AND_BINDING_POWER),
            TokenKind::EqualEqual => (BinaryOperator::Equal, EQUALITY_BINDING_POWER),
            TokenKind::BangEqual => (BinaryOperator::NotEqual, EQUALITY_BINDING_POWER),
            TokenKind::Less => (BinaryOperator::Less, COMPARISON_BINDING_POWER),
            TokenKind::LessEqual => (BinaryOperator::LessEqual, COMPARISON_BINDING_POWER),
            TokenKind::Greater => (BinaryOperator::Greater, COMPARISON_BINDING_POWER),
            TokenKind::GreaterEqual => (BinaryOperator::GreaterEqual, COMPARISON_BINDING_POWER),
            TokenKind::Plus => (BinaryOperator::Add, ADDITIVE_BINDING_POWER),
            TokenKind::Minus => (BinaryOperator::Subtract, ADDITIVE_BINDING_POWER),
            TokenKind::Star => (BinaryOperator::Multiply, MULTIPLICATIVE_BINDING_POWER),
            TokenKind::Slash => (BinaryOperator::Divide, MULTIPLICATIVE_BINDING_POWER),
            TokenKind::Percent => (BinaryOperator::Remainder, MULTIPLICATIVE_BINDING_POWER),
            _ => return None,
        };
        Some((operator, powers.0, powers.1))
    }

    fn recover_top_level(&mut self) {
        while !self.at(TokenKind::Fn)
            && !self.at(TokenKind::Record)
            && !self.at(TokenKind::Enum)
            && !self.at(TokenKind::Eof)
        {
            self.bump();
        }
    }

    fn recover_statement(&mut self) {
        while !self.at(TokenKind::Semicolon)
            && !self.at(TokenKind::RightBrace)
            && !self.at(TokenKind::Fn)
            && !self.at(TokenKind::Record)
            && !self.at(TokenKind::Enum)
            && !self.at(TokenKind::Eof)
        {
            self.bump();
        }
        self.consume(TokenKind::Semicolon);
    }

    fn recover_argument(&mut self) {
        while !self.at(TokenKind::Comma)
            && !self.at(TokenKind::RightParen)
            && !self.at(TokenKind::Eof)
        {
            self.bump();
        }
    }

    fn recover_deep_expression(&mut self) {
        let mut parentheses = 0_usize;
        let mut braces = 0_usize;

        loop {
            let kind = self.current().kind;
            if matches!(kind, TokenKind::Eof) {
                return;
            }
            if parentheses == 0
                && braces == 0
                && matches!(
                    kind,
                    TokenKind::Semicolon
                        | TokenKind::Comma
                        | TokenKind::RightParen
                        | TokenKind::RightBrace
                )
            {
                return;
            }

            match kind {
                TokenKind::LeftParen => parentheses += 1,
                TokenKind::LeftBrace => braces += 1,
                TokenKind::RightParen if parentheses > 0 => parentheses -= 1,
                TokenKind::RightBrace if braces > 0 => braces -= 1,
                _ => {}
            }
            self.bump();

            if parentheses == 0
                && braces == 0
                && matches!(kind, TokenKind::RightParen | TokenKind::RightBrace)
            {
                return;
            }
        }
    }

    fn current(&self) -> Token {
        self.tokens.get(self.position).copied().unwrap_or(Token {
            kind: TokenKind::Eof,
            span: self.source.eof_span(),
        })
    }

    fn at(&self, expected: TokenKind) -> bool {
        Self::token_kind_matches(self.current().kind, expected)
    }

    fn at_offset(&self, offset: usize, expected: TokenKind) -> bool {
        let actual = self
            .tokens
            .get(self.position.saturating_add(offset))
            .map_or(TokenKind::Eof, |token| token.kind);
        Self::token_kind_matches(actual, expected)
    }

    fn token_kind_matches(actual: TokenKind, expected: TokenKind) -> bool {
        match (actual, expected) {
            (TokenKind::Integer(_), TokenKind::Integer(_)) => true,
            _ => actual == expected,
        }
    }

    fn bump(&mut self) -> Token {
        let token = self.current();
        if !matches!(token.kind, TokenKind::Eof) {
            self.position = self.position.saturating_add(1);
        }
        token
    }

    fn consume(&mut self, expected: TokenKind) -> Option<Token> {
        self.at(expected).then(|| self.bump())
    }

    fn expect(&mut self, expected: TokenKind, context: &str) -> Option<Token> {
        if self.at(expected) {
            return Some(self.bump());
        }

        let token = self.current();
        self.diagnostics.push(
            Diagnostic::error(
                "N2001",
                format!("expected {} {context}", expected.description()),
            )
            .with_primary(token.span, format!("found {}", token.kind)),
        );
        None
    }

    fn cover(&self, first: Span, second: Span) -> Span {
        first.covering(second).unwrap_or(first)
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::ast::{BinaryOperator, ExpressionKind, StatementKind, TypeRef, TypeRefKind};
    use nova_lexer::{TokenKind, lex};
    use nova_source::{SourceFile, SourceId};

    fn parse_text(text: &str) -> (SourceFile, super::ParseOutput) {
        let source = SourceFile::new(SourceId::new(0), "test.nv", text);
        let lexed = lex(&source);
        assert!(
            lexed.diagnostics.is_empty(),
            "parser test source must lex successfully: {:?}",
            lexed.diagnostics
        );
        let parsed = parse(&source, &lexed.tokens);
        (source, parsed)
    }

    fn named_type_text(reference: &TypeRef) -> &str {
        let TypeRefKind::Named(name) = &reference.kind else {
            panic!("expected a named type reference, got {:?}", reference.kind);
        };
        &name.text
    }

    #[test]
    fn parses_functions_bindings_assignments_calls_blocks_and_if_expressions() {
        let text = r#"
fn choose(flag: Bool, a: Int, b: Int) -> Int {
    var copy: Int = a;
    let selected = if flag { copy } else { b };
    copy = selected;
    return copy + call(1, 2,);
}
"#;
        let (_, parsed) = parse_text(text);

        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.program.functions.len(), 1);
        let function = &parsed.program.functions[0];
        assert_eq!(function.name.text, "choose");
        assert_eq!(function.parameters.len(), 3);
        assert_eq!(named_type_text(&function.return_type), "Int");
        assert_eq!(function.body.statements.len(), 4);
        assert!(matches!(
            &function.body.statements[0].kind,
            StatementKind::Binding { mutable: true, .. }
        ));
        assert!(matches!(
            &function.body.statements[1].kind,
            StatementKind::Binding { mutable: false, .. }
        ));
        assert!(matches!(
            &function.body.statements[2].kind,
            StatementKind::Assignment { target, .. } if target.text == "copy"
        ));
        assert!(matches!(
            &function.body.statements[3].kind,
            StatementKind::Return(_)
        ));
    }

    #[test]
    fn parses_record_declaration_construction_and_projection() {
        let text = r#"
record Pair { left: Int, right: Bool, }
fn f() -> Int {
    let pair = new Pair { right: true, left: 7, };
    pair.left
}
"#;
        let (_, parsed) = parse_text(text);
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.program.records.len(), 1);
        assert_eq!(parsed.program.records[0].name.text, "Pair");
        assert_eq!(parsed.program.records[0].fields.len(), 2);

        let function = &parsed.program.functions[0];
        let StatementKind::Binding { initializer, .. } = &function.body.statements[0].kind else {
            panic!("expected record binding");
        };
        assert!(matches!(
            &initializer.kind,
            ExpressionKind::RecordLiteral { name, fields }
                if name.text == "Pair" && fields.len() == 2
        ));
        assert!(matches!(
            &function.body.tail.as_deref().expect("tail").kind,
            ExpressionKind::FieldAccess { field, .. } if field.text == "left"
        ));
    }

    #[test]
    fn parses_decoded_string_literals_with_exact_source_spans() {
        let text = r#"fn greet() -> String { "hello 🦀\n" }"#;
        let (source, parsed) = parse_text(text);

        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        let function = &parsed.program.functions[0];
        assert_eq!(named_type_text(&function.return_type), "String");
        let expression = function.body.tail.as_deref().expect("string tail");
        assert!(matches!(
            &expression.kind,
            ExpressionKind::String(value) if value == "hello 🦀\n"
        ));
        assert_eq!(source.slice(expression.span), Some(r#""hello 🦀\n""#));
    }

    #[test]
    fn rejects_a_synthetic_string_token_with_invalid_source_contents() {
        let source = SourceFile::new(
            SourceId::new(0),
            "synthetic.nv",
            "fn main() -> String { nope }",
        );
        let mut tokens = lex(&source).tokens;
        let token = tokens
            .iter_mut()
            .find(|token| source.slice(token.span) == Some("nope"))
            .expect("literal placeholder token");
        token.kind = TokenKind::String;

        let parsed = parse(&source, &tokens);
        assert!(parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "N2010" && source.slice(diagnostic.labels[0].span) == Some("nope")
        }));
        assert_eq!(parsed.program.functions.len(), 1);
        assert!(parsed.program.functions[0].body.tail.is_none());
    }

    #[test]
    fn parses_enum_construction_and_match_with_exact_pattern_spans() {
        let text = "enum Maybe { None, Some(Int), }\n\
                    fn main() -> Int {\n\
                        let value = Maybe::Some(42);\n\
                        match value { Maybe::None => 0, Maybe::Some(inner) => inner, }\n\
                    }";
        let (source, parsed) = parse_text(text);
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.program.enums.len(), 1);
        assert_eq!(parsed.program.enums[0].name.text, "Maybe");
        assert_eq!(parsed.program.enums[0].variants.len(), 2);
        assert!(parsed.program.enums[0].variants[0].payload.is_none());
        assert_eq!(
            parsed.program.enums[0].variants[1]
                .payload
                .as_ref()
                .map(named_type_text),
            Some("Int")
        );

        let function = &parsed.program.functions[0];
        let StatementKind::Binding { initializer, .. } = &function.body.statements[0].kind else {
            panic!("expected enum constructor binding");
        };
        assert!(matches!(
            &initializer.kind,
            ExpressionKind::EnumConstructor {
                enumeration,
                variant,
                payload: Some(_),
            } if enumeration.text == "Maybe" && variant.text == "Some"
        ));

        let tail = function.body.tail.as_deref().expect("match tail");
        let ExpressionKind::Match { arms, .. } = &tail.kind else {
            panic!("expected match expression");
        };
        assert_eq!(arms.len(), 2);
        assert_eq!(source.slice(arms[0].pattern.span), Some("Maybe::None"));
        assert_eq!(
            source.slice(arms[1].pattern.span),
            Some("Maybe::Some(inner)")
        );
        assert_eq!(
            arms[1]
                .pattern
                .binding
                .as_ref()
                .map(|binding| binding.text.as_str()),
            Some("inner")
        );
    }

    #[test]
    fn rejects_empty_enums_and_empty_payload_parentheses() {
        let (_, empty) = parse_text("enum Empty {} fn main() -> Int { 0 }");
        assert!(empty.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "N2001" && diagnostic.message == "expected an enum variant"
        }));

        let (_, empty_payload) =
            parse_text("enum Maybe { None } fn main() -> Maybe { Maybe::None() }");
        assert!(
            empty_payload
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "N2002"),
            "{:?}",
            empty_payload.diagnostics
        );
    }

    #[test]
    fn malformed_match_recovery_consumes_input_and_reaches_later_declarations() {
        let (_, parsed) = parse_text(
            "enum Maybe { None, Some(Int) }\n\
             fn broken(value: Maybe) -> Int {\n\
                 match value { Maybe::Some( => 1, Maybe::None => 0, }\n\
             }\n\
             fn good() -> Int { 2 }",
        );
        assert!(!parsed.is_success());
        assert!(parsed.diagnostics.len() < 10, "{:?}", parsed.diagnostics);
        assert!(
            parsed
                .program
                .functions
                .iter()
                .any(|function| function.name.text == "good"),
            "later declaration was lost: {:?}",
            parsed.program
        );
    }

    #[test]
    fn parses_surface_unit_type_and_literal() {
        let (_, parsed) = parse_text("fn noop() -> Unit { () } fn empty() -> Unit {}");
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        assert_eq!(
            named_type_text(&parsed.program.functions[0].return_type),
            "Unit"
        );
        assert!(matches!(
            parsed.program.functions[0]
                .body
                .tail
                .as_deref()
                .map(|expression| &expression.kind),
            Some(ExpressionKind::Unit)
        ));
        assert!(parsed.program.functions[1].body.tail.is_none());
    }

    #[test]
    fn parses_typed_uninitialized_var() {
        let (_, parsed) = parse_text("fn f() -> Int { var value: Int; value = 1; value }");
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        assert!(matches!(
            &parsed.program.functions[0].body.statements[0].kind,
            StatementKind::UninitializedBinding { name, annotation }
                if name.text == "value" && named_type_text(annotation) == "Int"
        ));
    }

    #[test]
    fn parses_while_statement_without_trailing_semicolon() {
        let (_, parsed) = parse_text(
            "fn f() -> Int { var value = 0; while value < 3 { value = value + 1; } value }",
        );
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        let statement = &parsed.program.functions[0].body.statements[1];
        assert!(matches!(
            &statement.kind,
            StatementKind::While { condition, body }
                if matches!(condition.kind, ExpressionKind::Binary { operator: BinaryOperator::Less, .. })
                    && body.statements.len() == 1
        ));
    }

    #[test]
    fn parses_break_and_continue_as_statement_only_loop_control() {
        let (_, parsed) = parse_text("fn f() -> Int { while true { continue; break; } 0 }");
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        let StatementKind::While { body, .. } =
            &parsed.program.functions[0].body.statements[0].kind
        else {
            panic!("expected while statement");
        };
        assert!(matches!(body.statements[0].kind, StatementKind::Continue));
        assert!(matches!(body.statements[1].kind, StatementKind::Break));
    }

    #[test]
    fn rejects_uninitialized_let_and_untyped_var() {
        for text in [
            "fn f() -> Int { let value: Int; 0 }",
            "fn f() -> Int { var value; 0 }",
        ] {
            let (_, parsed) = parse_text(text);
            assert!(!parsed.is_success(), "{text}");
            assert!(
                parsed
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "N2001"),
                "{:?}",
                parsed.diagnostics
            );
        }
    }

    #[test]
    fn applies_documented_precedence_and_left_associativity() {
        let (_, parsed) = parse_text("fn f() -> Bool { 1 + 2 * 3 == 7 || false && true }");
        let tail = parsed.program.functions[0]
            .body
            .tail
            .as_deref()
            .expect("function has a tail expression");

        let ExpressionKind::Binary {
            operator: BinaryOperator::Or,
            left,
            right,
        } = &tail.kind
        else {
            panic!("expected outer logical-or expression: {tail:?}");
        };
        assert!(matches!(
            &left.kind,
            ExpressionKind::Binary {
                operator: BinaryOperator::Equal,
                ..
            }
        ));
        assert!(matches!(
            &right.kind,
            ExpressionKind::Binary {
                operator: BinaryOperator::And,
                ..
            }
        ));

        let ExpressionKind::Binary { left, .. } = &left.kind else {
            panic!("expected equality");
        };
        assert!(matches!(
            &left.kind,
            ExpressionKind::Binary {
                operator: BinaryOperator::Add,
                ..
            }
        ));

        let (_, parsed) = parse_text("fn f() -> Int { 10 - 3 - 2 }");
        let tail = parsed.program.functions[0]
            .body
            .tail
            .as_deref()
            .expect("function has a tail expression");
        let ExpressionKind::Binary {
            operator: BinaryOperator::Subtract,
            left,
            right,
        } = &tail.kind
        else {
            panic!("expected outer subtraction: {tail:?}");
        };
        assert!(matches!(&right.kind, ExpressionKind::Integer(2)));
        assert!(matches!(
            &left.kind,
            ExpressionKind::Binary {
                operator: BinaryOperator::Subtract,
                ..
            }
        ));
    }

    #[test]
    fn field_access_binds_as_postfix_before_binary_operators() {
        let (_, parsed) =
            parse_text("record Pair { left: Int } fn f(pair: Pair) -> Int { pair.left + 1 }");
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        let tail = parsed.program.functions[0]
            .body
            .tail
            .as_deref()
            .expect("tail");
        let ExpressionKind::Binary {
            operator: BinaryOperator::Add,
            left,
            ..
        } = &tail.kind
        else {
            panic!("expected addition");
        };
        assert!(matches!(left.kind, ExpressionKind::FieldAccess { .. }));
    }

    #[test]
    fn preserves_token_and_construct_spans_including_parentheses() {
        let text = "fn id(value: Int) -> Int { (value) }";
        let (source, parsed) = parse_text(text);
        let function = &parsed.program.functions[0];
        let tail = function.body.tail.as_deref().expect("tail expression");

        assert_eq!(source.slice(function.name.span), Some("id"));
        assert_eq!(
            source.slice(function.parameters[0].span),
            Some("value: Int")
        );
        assert_eq!(source.slice(tail.span), Some("(value)"));
        assert_eq!(source.slice(function.span), Some(text));
    }

    #[test]
    fn recovers_to_later_top_level_declarations() {
        let (_, parsed) =
            parse_text("fn broken() { 1 } record Good { value: Int } fn good() -> Int { 2 }");

        assert!(!parsed.diagnostics.is_empty());
        assert_eq!(parsed.program.records.len(), 1);
        assert_eq!(parsed.program.records[0].name.text, "Good");
        assert_eq!(parsed.program.functions.len(), 1);
        assert_eq!(parsed.program.functions[0].name.text, "good");
    }

    #[test]
    fn requires_else_for_every_if_expression() {
        let (_, parsed) = parse_text("fn f() -> Int { if true { 1 } }");

        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "N2006")
        );
    }

    #[test]
    fn bounds_recursive_expression_parsing() {
        let nested = format!(
            "fn f() -> Int {{ {}1{} }}",
            "(".repeat(300),
            ")".repeat(300)
        );
        let (_, parsed) = parse_text(&nested);

        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "N2008")
        );
        assert!(
            parsed.diagnostics.len() < 20,
            "recovery diagnostic cascade: {:?}",
            parsed.diagnostics
        );
    }

    #[test]
    fn normalizes_empty_or_truncated_token_streams_without_panicking() {
        let source = SourceFile::new(SourceId::new(0), "empty.nv", "");
        let empty = parse(&source, &[]);
        assert!(empty.is_success());

        let source = SourceFile::new(SourceId::new(0), "partial.nv", "fn");
        let lexed = lex(&source);
        let without_eof = &lexed.tokens[..lexed.tokens.len().saturating_sub(1)];
        let partial = parse(&source, without_eof);
        assert!(!partial.is_success());
    }
}
