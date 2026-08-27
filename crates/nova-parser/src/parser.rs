use crate::ast::{
    BinaryOperator, Block, Expression, ExpressionKind, Function, Name, Parameter, Program,
    Statement, StatementKind, TypeRef, UnaryOperator,
};
use nova_diagnostics::Diagnostic;
use nova_lexer::{Token, TokenKind};
use nova_source::{SourceFile, Span};

const MAX_EXPRESSION_DEPTH: usize = 256;
const OR_BINDING_POWER: (u8, u8) = (1, 2);
const AND_BINDING_POWER: (u8, u8) = (3, 4);
const EQUALITY_BINDING_POWER: (u8, u8) = (5, 6);
const COMPARISON_BINDING_POWER: (u8, u8) = (7, 8);
const ADDITIVE_BINDING_POWER: (u8, u8) = (9, 10);
const MULTIPLICATIVE_BINDING_POWER: (u8, u8) = (11, 12);
const PREFIX_BINDING_POWER: u8 = 13;
const CALL_BINDING_POWER: u8 = 15;

/// Complete deterministic result of parsing one token stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseOutput {
    /// Parsed functions, including successfully recovered later declarations.
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
        let mut functions = Vec::new();

        while !self.at(TokenKind::Eof) {
            let before = self.position;
            if self.at(TokenKind::Fn) {
                if let Some(function) = self.parse_function() {
                    functions.push(function);
                } else {
                    self.recover_top_level();
                }
            } else {
                let token = self.current();
                self.diagnostics.push(
                    Diagnostic::error("N2003", "expected a top-level function declaration")
                        .with_primary(
                            token.span,
                            format!(
                                "found {}; top-level statements are not supported",
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
            functions,
            span: self
                .source
                .span(0, self.source.len())
                .unwrap_or(self.source.eof_span()),
        }
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
        let name = self.parse_name(context)?;
        Some(TypeRef {
            span: name.span,
            name,
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
            if self.at(TokenKind::Fn) {
                break;
            }
            let before = self.position;
            let statement = if self.at(TokenKind::Let) || self.at(TokenKind::Var) {
                self.parse_binding_statement()
            } else if self.at(TokenKind::Return) {
                self.parse_return_statement()
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
                && !self.at(TokenKind::Fn)
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
        self.expect(TokenKind::Equal, "before the binding initializer")?;
        let initializer = self.parse_expression()?;
        let semicolon = self.expect(TokenKind::Semicolon, "after the binding initializer")?;
        Some(Statement {
            span: self.cover(keyword.span, semicolon.span),
            kind: StatementKind::Binding {
                mutable,
                name,
                annotation,
                initializer,
            },
        })
    }

    fn parse_return_statement(&mut self) -> Option<Statement> {
        let keyword = self.expect(TokenKind::Return, "to start a return statement")?;
        let expression = self.parse_expression()?;
        let semicolon = self.expect(TokenKind::Semicolon, "after the returned expression")?;
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
                if CALL_BINDING_POWER < minimum {
                    break;
                }
                left = self.parse_call_expression(left)?;
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
            TokenKind::True | TokenKind::False => {
                self.bump();
                Some(Expression {
                    kind: ExpressionKind::Boolean(matches!(token.kind, TokenKind::True)),
                    span: token.span,
                })
            }
            TokenKind::Identifier => {
                self.bump();
                let name = Name {
                    text: self.source.slice(token.span).unwrap_or("").to_owned(),
                    span: token.span,
                };
                Some(Expression {
                    kind: ExpressionKind::Name(name),
                    span: token.span,
                })
            }
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
        while !self.at(TokenKind::Fn) && !self.at(TokenKind::Eof) {
            self.bump();
        }
    }

    fn recover_statement(&mut self) {
        while !self.at(TokenKind::Semicolon)
            && !self.at(TokenKind::RightBrace)
            && !self.at(TokenKind::Fn)
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
        let actual = self.current().kind;
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
    use crate::ast::{BinaryOperator, ExpressionKind, StatementKind};
    use nova_lexer::lex;
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

    #[test]
    fn parses_functions_bindings_calls_blocks_and_if_expressions() {
        let text = r#"
fn choose(flag: Bool, a: Int, b: Int) -> Int {
    var copy: Int = a;
    let selected = if flag { copy } else { b };
    return selected + call(1, 2,);
}
"#;
        let (_, parsed) = parse_text(text);

        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.program.functions.len(), 1);
        let function = &parsed.program.functions[0];
        assert_eq!(function.name.text, "choose");
        assert_eq!(function.parameters.len(), 3);
        assert_eq!(function.return_type.name.text, "Int");
        assert_eq!(function.body.statements.len(), 3);
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
            StatementKind::Return(_)
        ));
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
    fn recovers_to_a_later_top_level_function() {
        let (_, parsed) = parse_text("fn broken() { 1 } fn good() -> Int { 2 }");

        assert!(!parsed.diagnostics.is_empty());
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
