use crate::hir::{
    self, BinaryOperator, BuiltinType, Mutability, Symbol, SymbolId, SymbolKind, UnaryOperator,
};
use nova_diagnostics::{Diagnostic, LabelStyle};
use nova_parser::ast;
use nova_source::Span;
use std::collections::BTreeMap;

const MAX_LOWERING_DEPTH: usize = 256;

/// Result of AST-to-HIR lowering and deterministic name resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LowerOutput {
    /// Complete resolved HIR, present only when no semantic error occurred.
    pub program: Option<hir::Program>,
    /// Semantic diagnostics ordered by their primary source position.
    pub diagnostics: Vec<Diagnostic>,
}

impl LowerOutput {
    /// Reports whether a complete resolved HIR was produced.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.program.is_some() && self.diagnostics.is_empty()
    }
}

/// Lowers a parsed program into HIR and resolves the implemented name subset.
///
/// Top-level functions are collected before bodies, so recursion and forward
/// calls are valid. Local resolution is lexical. A binding becomes visible
/// only after its initializer, and only duplicate declarations in the same
/// lexical scope are rejected.
#[must_use]
pub fn lower(program: &ast::Program) -> LowerOutput {
    let mut lowerer = Lowerer::new();
    let lowered = lowerer.lower_program(program);
    lowerer.diagnostics.sort_by_key(diagnostic_position);

    let program = lowerer.diagnostics.is_empty().then_some(lowered);
    LowerOutput {
        program,
        diagnostics: lowerer.diagnostics,
    }
}

#[derive(Clone, Copy)]
struct Declaration {
    symbol: SymbolId,
    span: Span,
}

struct Lowerer {
    symbols: Vec<Symbol>,
    globals: BTreeMap<String, Declaration>,
    scopes: Vec<BTreeMap<String, Declaration>>,
    diagnostics: Vec<Diagnostic>,
    lowering_depth: usize,
    depth_diagnostic_emitted: bool,
}

impl Lowerer {
    fn new() -> Self {
        Self {
            symbols: Vec::new(),
            globals: BTreeMap::new(),
            scopes: Vec::new(),
            diagnostics: Vec::new(),
            lowering_depth: 0,
            depth_diagnostic_emitted: false,
        }
    }

    fn lower_program(&mut self, program: &ast::Program) -> hir::Program {
        let mut function_symbols = Vec::with_capacity(program.functions.len());
        for function in &program.functions {
            function_symbols.push(self.declare_global_function(&function.name));
        }

        let mut functions = Vec::with_capacity(program.functions.len());
        for (function, symbol) in program.functions.iter().zip(function_symbols) {
            if let Some(symbol) = symbol {
                if let Some(function) = self.lower_function(function, symbol) {
                    functions.push(function);
                }
            }
        }

        hir::Program {
            functions,
            symbols: std::mem::take(&mut self.symbols),
            span: program.span,
        }
    }

    fn declare_global_function(&mut self, name: &ast::Name) -> Option<SymbolId> {
        if let Some(previous) = self.globals.get(&name.text).copied() {
            self.emit_duplicate(name, previous.span, "top-level function");
            return None;
        }

        let symbol = self.allocate_symbol(name, SymbolKind::Function)?;
        self.globals.insert(
            name.text.clone(),
            Declaration {
                symbol,
                span: name.span,
            },
        );
        Some(symbol)
    }

    fn lower_function(
        &mut self,
        function: &ast::Function,
        symbol: SymbolId,
    ) -> Option<hir::Function> {
        self.scopes.push(BTreeMap::new());
        self.lowering_depth = 0;
        self.depth_diagnostic_emitted = false;
        let mut complete = true;

        let mut parameters = Vec::with_capacity(function.parameters.len());
        for parameter in &function.parameters {
            let ty = self.lower_type_ref(&parameter.ty);
            let parameter_symbol = self.declare_local(&parameter.name, SymbolKind::Parameter);
            match (parameter_symbol, ty) {
                (Some(symbol), Some(ty)) => parameters.push(hir::Parameter {
                    symbol,
                    ty,
                    span: parameter.span,
                }),
                _ => complete = false,
            }
        }

        let return_type = self.lower_type_ref(&function.return_type);
        let body = self.lower_block(&function.body, false);
        self.scopes.pop();

        match (complete, return_type, body) {
            (true, Some(return_type), Some(body)) => Some(hir::Function {
                symbol,
                parameters,
                return_type,
                body,
                span: function.span,
            }),
            _ => None,
        }
    }

    fn lower_type_ref(&mut self, ty: &ast::TypeRef) -> Option<hir::TypeRef> {
        let kind = match ty.name.text.as_str() {
            "Int" => BuiltinType::Int,
            "Bool" => BuiltinType::Bool,
            _ => {
                self.diagnostics.push(
                    Diagnostic::error("N3003", format!("unknown type name `{}`", ty.name.text))
                        .with_primary(ty.name.span, "this type is not declared")
                        .with_note("the current semantic subset defines only `Int` and `Bool`"),
                );
                return None;
            }
        };
        Some(hir::TypeRef {
            kind,
            span: ty.span,
        })
    }

    fn lower_block(&mut self, block: &ast::Block, introduce_scope: bool) -> Option<hir::Block> {
        if introduce_scope {
            self.scopes.push(BTreeMap::new());
        }

        let mut complete = true;
        let mut statements = Vec::with_capacity(block.statements.len());
        for statement in &block.statements {
            if let Some(statement) = self.lower_statement(statement) {
                statements.push(statement);
            } else {
                complete = false;
            }
        }

        let tail = match &block.tail {
            Some(expression) => match self.lower_expression(expression) {
                Some(expression) => Some(Box::new(expression)),
                None => {
                    complete = false;
                    None
                }
            },
            None => None,
        };

        if introduce_scope {
            self.scopes.pop();
        }

        complete.then_some(hir::Block {
            statements,
            tail,
            span: block.span,
        })
    }

    fn lower_statement(&mut self, statement: &ast::Statement) -> Option<hir::Statement> {
        let kind = match &statement.kind {
            ast::StatementKind::Binding {
                mutable,
                name,
                annotation,
                initializer,
            } => {
                let annotation = match annotation {
                    Some(annotation) => self.lower_type_ref(annotation).map(Some),
                    None => Some(None),
                };
                let initializer = self.lower_expression(initializer);
                let mutability = if *mutable {
                    Mutability::Mutable
                } else {
                    Mutability::Immutable
                };

                // The declaration is deliberately installed after resolving
                // its initializer, while still being installed after an error
                // so later references do not produce a misleading cascade.
                let symbol = self.declare_local(name, SymbolKind::Local(mutability));
                match (symbol, annotation, initializer) {
                    (Some(symbol), Some(annotation), Some(initializer)) => {
                        hir::StatementKind::Binding {
                            symbol,
                            mutability,
                            annotation,
                            initializer,
                        }
                    }
                    _ => return None,
                }
            }
            ast::StatementKind::Return(expression) => {
                hir::StatementKind::Return(self.lower_expression(expression)?)
            }
            ast::StatementKind::Expression(expression) => {
                hir::StatementKind::Expression(self.lower_expression(expression)?)
            }
        };

        Some(hir::Statement {
            kind,
            span: statement.span,
        })
    }

    fn lower_expression(&mut self, expression: &ast::Expression) -> Option<hir::Expression> {
        if self.lowering_depth >= MAX_LOWERING_DEPTH {
            if !self.depth_diagnostic_emitted {
                self.diagnostics.push(
                    Diagnostic::error("N3004", "HIR lowering nesting limit exceeded")
                        .with_primary(
                            expression.span,
                            format!(
                                "the bootstrap semantic pass accepts at most {MAX_LOWERING_DEPTH} nested expression frames"
                            ),
                        )
                        .with_note(
                            "this guarded implementation limit may change in later Nova versions",
                        ),
                );
                self.depth_diagnostic_emitted = true;
            }
            return None;
        }

        self.lowering_depth += 1;
        let kind = match &expression.kind {
            ast::ExpressionKind::Integer(value) => Some(hir::ExpressionKind::Integer(*value)),
            ast::ExpressionKind::Boolean(value) => Some(hir::ExpressionKind::Boolean(*value)),
            ast::ExpressionKind::Name(name) => {
                self.resolve_value(name).map(hir::ExpressionKind::Name)
            }
            ast::ExpressionKind::Unary { operator, operand } => {
                self.lower_expression(operand)
                    .map(|operand| hir::ExpressionKind::Unary {
                        operator: lower_unary_operator(*operator),
                        operand: Box::new(operand),
                    })
            }
            ast::ExpressionKind::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.lower_expression(left);
                let right = self.lower_expression(right);
                match (left, right) {
                    (Some(left), Some(right)) => Some(hir::ExpressionKind::Binary {
                        operator: lower_binary_operator(*operator),
                        left: Box::new(left),
                        right: Box::new(right),
                    }),
                    _ => None,
                }
            }
            ast::ExpressionKind::Call { callee, arguments } => {
                let callee = self.lower_expression(callee);
                let mut complete = true;
                let mut lowered_arguments = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    if let Some(argument) = self.lower_expression(argument) {
                        lowered_arguments.push(argument);
                    } else {
                        complete = false;
                    }
                }
                match (complete, callee) {
                    (true, Some(callee)) => Some(hir::ExpressionKind::Call {
                        callee: Box::new(callee),
                        arguments: lowered_arguments,
                    }),
                    _ => None,
                }
            }
            ast::ExpressionKind::Block(block) => self
                .lower_block(block, true)
                .map(hir::ExpressionKind::Block),
            ast::ExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.lower_expression(condition);
                let then_branch = self.lower_block(then_branch, true);
                let else_branch = self.lower_expression(else_branch);
                match (condition, then_branch, else_branch) {
                    (Some(condition), Some(then_branch), Some(else_branch)) => {
                        Some(hir::ExpressionKind::If {
                            condition: Box::new(condition),
                            then_branch,
                            else_branch: Box::new(else_branch),
                        })
                    }
                    _ => None,
                }
            }
        };
        self.lowering_depth -= 1;

        kind.map(|kind| hir::Expression {
            kind,
            span: expression.span,
        })
    }

    fn declare_local(&mut self, name: &ast::Name, kind: SymbolKind) -> Option<SymbolId> {
        let Some(scope) = self.scopes.last() else {
            self.diagnostics.push(
                Diagnostic::error("N3099", "name-resolution scope invariant failed")
                    .with_primary(name.span, "no lexical scope exists for this declaration"),
            );
            return None;
        };

        if let Some(previous) = scope.get(&name.text).copied() {
            self.emit_duplicate(name, previous.span, "value");
            return None;
        }

        let symbol = self.allocate_symbol(name, kind)?;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name.text.clone(),
                Declaration {
                    symbol,
                    span: name.span,
                },
            );
            Some(symbol)
        } else {
            self.diagnostics.push(
                Diagnostic::error("N3099", "name-resolution scope invariant failed").with_primary(
                    name.span,
                    "lexical scope disappeared while declaring this name",
                ),
            );
            None
        }
    }

    fn resolve_value(&mut self, name: &ast::Name) -> Option<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(declaration) = scope.get(&name.text) {
                return Some(declaration.symbol);
            }
        }
        if let Some(declaration) = self.globals.get(&name.text) {
            return Some(declaration.symbol);
        }

        self.diagnostics.push(
            Diagnostic::error("N3002", format!("unknown value name `{}`", name.text))
                .with_primary(name.span, "no visible declaration has this name")
                .with_note(
                    "top-level functions are visible throughout the file; local bindings are visible only after their initializer",
                ),
        );
        None
    }

    fn allocate_symbol(&mut self, name: &ast::Name, kind: SymbolKind) -> Option<SymbolId> {
        let Ok(raw) = u32::try_from(self.symbols.len()) else {
            self.diagnostics.push(
                Diagnostic::error("N3005", "too many declarations in one source file")
                    .with_primary(
                        name.span,
                        "the bootstrap symbol identity space is exhausted",
                    ),
            );
            return None;
        };
        let id = SymbolId::new(raw);
        self.symbols.push(Symbol {
            id,
            name: name.text.clone(),
            kind,
            span: name.span,
        });
        Some(id)
    }

    fn emit_duplicate(&mut self, name: &ast::Name, previous: Span, category: &str) {
        self.diagnostics.push(
            Diagnostic::error(
                "N3001",
                format!("duplicate {category} declaration `{}`", name.text),
            )
            .with_primary(name.span, "declared again in the same lexical scope")
            .with_secondary(previous, "the first declaration is here")
            .with_note("nested blocks may shadow outer value declarations"),
        );
    }
}

fn diagnostic_position(diagnostic: &Diagnostic) -> (u32, usize) {
    diagnostic
        .labels
        .iter()
        .find(|label| label.style == LabelStyle::Primary)
        .or_else(|| diagnostic.labels.first())
        .map_or((u32::MAX, usize::MAX), |label| {
            (label.span.source().raw(), label.span.start())
        })
}

const fn lower_unary_operator(operator: ast::UnaryOperator) -> UnaryOperator {
    match operator {
        ast::UnaryOperator::Negate => UnaryOperator::Negate,
        ast::UnaryOperator::Not => UnaryOperator::Not,
    }
}

const fn lower_binary_operator(operator: ast::BinaryOperator) -> BinaryOperator {
    match operator {
        ast::BinaryOperator::Or => BinaryOperator::Or,
        ast::BinaryOperator::And => BinaryOperator::And,
        ast::BinaryOperator::Equal => BinaryOperator::Equal,
        ast::BinaryOperator::NotEqual => BinaryOperator::NotEqual,
        ast::BinaryOperator::Less => BinaryOperator::Less,
        ast::BinaryOperator::LessEqual => BinaryOperator::LessEqual,
        ast::BinaryOperator::Greater => BinaryOperator::Greater,
        ast::BinaryOperator::GreaterEqual => BinaryOperator::GreaterEqual,
        ast::BinaryOperator::Add => BinaryOperator::Add,
        ast::BinaryOperator::Subtract => BinaryOperator::Subtract,
        ast::BinaryOperator::Multiply => BinaryOperator::Multiply,
        ast::BinaryOperator::Divide => BinaryOperator::Divide,
        ast::BinaryOperator::Remainder => BinaryOperator::Remainder,
    }
}

#[cfg(test)]
mod tests {
    use super::lower;
    use crate::hir::{ExpressionKind, Mutability, StatementKind, SymbolKind};
    use nova_lexer::lex;
    use nova_parser::{ast, parse};
    use nova_source::{SourceFile, SourceId};

    fn lower_text(text: &str) -> super::LowerOutput {
        let source = SourceFile::new(SourceId::new(0), "test.nv", text);
        let lexed = lex(&source);
        assert!(lexed.diagnostics.is_empty(), "{:?}", lexed.diagnostics);
        let parsed = parse(&source, &lexed.tokens);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        lower(&parsed.program)
    }

    #[test]
    fn resolves_forward_calls_recursion_parameters_and_locals_deterministically() {
        let text = r#"
fn first(value: Int) -> Int {
    let local = second(value);
    first(local)
}
fn second(input: Int) -> Int { input }
"#;
        let first = lower_text(text);
        let second = lower_text(text);
        assert_eq!(first, second);
        assert!(first.is_success(), "{:?}", first.diagnostics);

        let program = first.program.expect("valid program produces HIR");
        let symbols = program
            .symbols
            .iter()
            .map(|symbol| (symbol.id.raw(), symbol.name.as_str(), symbol.kind))
            .collect::<Vec<_>>();
        assert_eq!(
            symbols,
            vec![
                (0, "first", SymbolKind::Function),
                (1, "second", SymbolKind::Function),
                (2, "value", SymbolKind::Parameter),
                (3, "local", SymbolKind::Local(Mutability::Immutable)),
                (4, "input", SymbolKind::Parameter),
            ]
        );
        assert_eq!(
            program
                .symbol(program.functions[0].symbol)
                .map(|symbol| symbol.name.as_str()),
            Some("first")
        );
        assert!(program.symbol(crate::hir::SymbolId::new(99)).is_none());

        let StatementKind::Binding { initializer, .. } =
            &program.functions[0].body.statements[0].kind
        else {
            panic!("expected local binding");
        };
        let ExpressionKind::Call { callee, arguments } = &initializer.kind else {
            panic!("expected initializer call");
        };
        assert!(matches!(callee.kind, ExpressionKind::Name(id) if id.raw() == 1));
        assert!(matches!(arguments[0].kind, ExpressionKind::Name(id) if id.raw() == 2));

        let tail = program.functions[0]
            .body
            .tail
            .as_deref()
            .expect("first has a tail");
        let ExpressionKind::Call { callee, arguments } = &tail.kind else {
            panic!("expected recursive call");
        };
        assert!(matches!(callee.kind, ExpressionKind::Name(id) if id.raw() == 0));
        assert!(matches!(arguments[0].kind, ExpressionKind::Name(id) if id.raw() == 3));
    }

    #[test]
    fn nested_shadowing_is_lexical_and_initializer_sees_the_outer_name() {
        let output = lower_text("fn f(value: Int) -> Int { { let value = value; value } }");
        assert!(output.is_success(), "{:?}", output.diagnostics);
        let program = output.program.expect("valid program produces HIR");
        let outer = program.symbols[1].id;
        let inner = program.symbols[2].id;

        let nested = program.functions[0]
            .body
            .tail
            .as_deref()
            .expect("outer block has a tail");
        let ExpressionKind::Block(block) = &nested.kind else {
            panic!("expected nested block");
        };
        let StatementKind::Binding { initializer, .. } = &block.statements[0].kind else {
            panic!("expected binding");
        };
        assert!(matches!(initializer.kind, ExpressionKind::Name(id) if id == outer));
        assert!(matches!(
            block.tail.as_deref().map(|tail| &tail.kind),
            Some(ExpressionKind::Name(id)) if *id == inner
        ));
    }

    #[test]
    fn rejects_duplicate_names_in_one_scope_with_both_spans() {
        for text in [
            "fn f(value: Int, value: Int) -> Int { value }",
            "fn f(value: Int) -> Int { let value = 1; value }",
            "fn f() -> Int { let value = 1; let value = 2; value }",
            "fn f() -> Int { 1 } fn f() -> Int { 2 }",
        ] {
            let output = lower_text(text);
            assert!(!output.is_success());
            assert!(output.program.is_none());
            let diagnostic = output
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == "N3001")
                .expect("duplicate diagnostic");
            assert_eq!(diagnostic.labels.len(), 2);
            assert_eq!(
                diagnostic.labels[0].style,
                nova_diagnostics::LabelStyle::Primary
            );
            assert_eq!(
                diagnostic.labels[1].style,
                nova_diagnostics::LabelStyle::Secondary
            );
            assert!(diagnostic.labels[1].span.start() < diagnostic.labels[0].span.start());
        }
    }

    #[test]
    fn nested_bindings_do_not_leak_into_the_parent_scope() {
        let text = "fn f() -> Int { { let hidden = 1; hidden }; hidden }";
        let output = lower_text(text);

        assert!(!output.is_success());
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "N3002")
            .expect("unknown-name diagnostic");
        assert_eq!(diagnostic.labels.len(), 1);
        assert_eq!(
            diagnostic.labels[0].span.start(),
            text.rfind("hidden").unwrap()
        );
    }

    #[test]
    fn a_local_value_may_shadow_a_top_level_function() {
        let output =
            lower_text("fn target() -> Int { 1 } fn f() -> Int { let target = 2; target }");

        assert!(output.is_success(), "{:?}", output.diagnostics);
        let program = output.program.expect("valid program produces HIR");
        let local = program.symbols[2].id;
        let tail = program.functions[1]
            .body
            .tail
            .as_deref()
            .expect("f has a tail");
        assert!(matches!(tail.kind, ExpressionKind::Name(id) if id == local));
    }

    #[test]
    fn a_binding_is_not_visible_in_its_own_initializer() {
        let output = lower_text("fn f() -> Int { let value = value; value }");

        assert!(!output.is_success());
        assert_eq!(
            output
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "N3002")
                .count(),
            1
        );
    }

    #[test]
    fn reports_unknown_values_and_types_in_source_order() {
        let text = "fn f(arg: Missing) -> Other { let x: Third = absent; later; 0 }";
        let output = lower_text(text);

        assert!(!output.is_success());
        let codes = output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();
        assert_eq!(codes, vec!["N3003", "N3003", "N3003", "N3002", "N3002"]);
        let starts = output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.labels[0].span.start())
            .collect::<Vec<_>>();
        assert!(starts.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn rejects_excessively_deep_manually_constructed_ast_without_panicking() {
        let source = SourceFile::new(SourceId::new(0), "test.nv", "fn f() -> Int { 0 }");
        let lexed = lex(&source);
        let mut parsed = parse(&source, &lexed.tokens);
        assert!(parsed.is_success());
        let mut expression = parsed.program.functions[0]
            .body
            .tail
            .take()
            .expect("tail expression");
        for _ in 0..300 {
            let span = expression.span;
            expression = Box::new(ast::Expression {
                kind: ast::ExpressionKind::Unary {
                    operator: ast::UnaryOperator::Negate,
                    operand: expression,
                },
                span,
            });
        }
        parsed.program.functions[0].body.tail = Some(expression);

        let output = lower(&parsed.program);
        assert!(!output.is_success());
        assert_eq!(
            output
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "N3004")
                .count(),
            1
        );
    }

    #[test]
    fn lowers_an_empty_source_file() {
        let output = lower_text("");
        assert!(output.is_success());
        let program = output.program.expect("empty program has valid HIR");
        assert!(program.functions.is_empty());
        assert!(program.symbols.is_empty());
    }
}
