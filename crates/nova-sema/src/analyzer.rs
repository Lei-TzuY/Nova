use crate::hir::{
    self, BindingId, EnumId, EnumType, ExpressionKind, FunctionId, FunctionType, MatchArm,
    RecordFieldValue, RecordId, RecordType, StatementKind, Type,
};
use nova_diagnostics::{Diagnostic, LabelStyle};
use nova_parser::ast::{self, BinaryOperator, UnaryOperator};
use nova_source::Span;
use std::collections::BTreeMap;

/// Complete deterministic result of semantic analysis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisOutput {
    /// Resolved and typed HIR, including error-recovery nodes when diagnostics exist.
    pub program: hir::Program,
    /// Semantic diagnostics in source order.
    pub diagnostics: Vec<Diagnostic>,
}

impl AnalysisOutput {
    /// Reports whether semantic analysis accepted the program.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Lowers a parsed program to HIR while resolving names and checking bootstrap types.
#[must_use]
pub fn analyze(program: &ast::Program) -> AnalysisOutput {
    let mut analyzer = Analyzer::new();
    analyzer.collect_type_definitions(program);
    analyzer.collect_function_signatures(program);

    let records = analyzer
        .record_definitions
        .iter()
        .map(RecordDefinition::to_hir)
        .collect();
    let enums = analyzer
        .enum_definitions
        .iter()
        .map(EnumDefinition::to_hir)
        .collect();
    let functions = program
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| analyzer.lower_function(FunctionId::new(index), function))
        .collect();

    analyzer.diagnostics.sort_by_key(diagnostic_sort_key);
    AnalysisOutput {
        program: hir::Program {
            records,
            enums,
            functions,
            span: program.span,
        },
        diagnostics: analyzer.diagnostics,
    }
}

fn diagnostic_sort_key(diagnostic: &Diagnostic) -> (u32, usize, usize) {
    let span = diagnostic
        .labels
        .iter()
        .find(|label| label.style == LabelStyle::Primary)
        .or_else(|| diagnostic.labels.first())
        .map(|label| label.span);
    match span {
        Some(span) => (span.source().raw(), span.start(), span.end()),
        None => (u32::MAX, usize::MAX, usize::MAX),
    }
}

#[derive(Clone, Debug)]
struct RecordDefinition {
    id: RecordId,
    name: String,
    fields: Vec<hir::RecordField>,
    span: Span,
}

impl RecordDefinition {
    fn record_type(&self) -> RecordType {
        RecordType {
            id: self.id,
            name: self.name.clone(),
        }
    }

    fn to_hir(&self) -> hir::Record {
        hir::Record {
            id: self.id,
            name: self.name.clone(),
            fields: self.fields.clone(),
            span: self.span,
        }
    }
}

#[derive(Clone, Debug)]
struct EnumDefinition {
    id: EnumId,
    name: String,
    variants: Vec<hir::EnumVariant>,
    span: Span,
}

impl EnumDefinition {
    fn enum_type(&self) -> EnumType {
        EnumType {
            id: self.id,
            name: self.name.clone(),
        }
    }

    fn to_hir(&self) -> hir::Enum {
        hir::Enum {
            id: self.id,
            name: self.name.clone(),
            variants: self.variants.clone(),
            span: self.span,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum TypeDefinition {
    Record(RecordId),
    Enum(EnumId),
}

#[derive(Clone, Copy, Debug)]
struct TypeSymbol {
    definition: TypeDefinition,
    span: Span,
}

#[derive(Clone, Debug)]
struct SignatureRecord {
    parameters: Vec<Type>,
    return_type: Type,
}

impl SignatureRecord {
    fn function_type(&self) -> FunctionType {
        FunctionType {
            parameters: self.parameters.clone(),
            return_type: Box::new(self.return_type.clone()),
        }
    }
}

#[derive(Clone, Debug)]
struct FunctionSymbol {
    id: FunctionId,
    signature: FunctionType,
    span: Span,
}

#[derive(Clone, Debug)]
struct LocalSymbol {
    id: BindingId,
    ty: Type,
    mutable: bool,
    initialized: bool,
    span: Span,
}

type Scope = BTreeMap<String, LocalSymbol>;
type ScopeState = Vec<Scope>;

#[derive(Clone, Debug)]
struct LoopContext {
    visible_scope_count: usize,
    break_states: Vec<ScopeState>,
}

#[derive(Clone, Debug)]
struct ReachableState {
    scopes: ScopeState,
    loop_stack: Vec<LoopContext>,
}

struct Analyzer {
    diagnostics: Vec<Diagnostic>,
    record_definitions: Vec<RecordDefinition>,
    enum_definitions: Vec<EnumDefinition>,
    types: BTreeMap<String, TypeSymbol>,
    signatures: Vec<SignatureRecord>,
    functions: BTreeMap<String, FunctionSymbol>,
    scopes: ScopeState,
    next_binding: usize,
    loop_stack: Vec<LoopContext>,
}

impl Analyzer {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            record_definitions: Vec::new(),
            enum_definitions: Vec::new(),
            types: BTreeMap::new(),
            signatures: Vec::new(),
            functions: BTreeMap::new(),
            scopes: Vec::new(),
            next_binding: 0,
            loop_stack: Vec::new(),
        }
    }

    fn capture_reachable_state(&self) -> ReachableState {
        ReachableState {
            scopes: self.scopes.clone(),
            loop_stack: self.loop_stack.clone(),
        }
    }

    fn restore_reachable_state(&mut self, state: ReachableState) {
        self.scopes = state.scopes;
        self.loop_stack = state.loop_stack;
    }

    fn collect_type_definitions(&mut self, program: &ast::Program) {
        self.record_definitions = program
            .records
            .iter()
            .enumerate()
            .map(|(index, record)| RecordDefinition {
                id: RecordId::new(index),
                name: record.name.text.clone(),
                fields: Vec::new(),
                span: record.span,
            })
            .collect();

        self.enum_definitions = program
            .enums
            .iter()
            .enumerate()
            .map(|(index, enumeration)| EnumDefinition {
                id: EnumId::new(index),
                name: enumeration.name.text.clone(),
                variants: Vec::new(),
                span: enumeration.span,
            })
            .collect();

        let mut declarations = program
            .records
            .iter()
            .enumerate()
            .map(|(index, record)| {
                (
                    record.name.span.start(),
                    &record.name,
                    TypeDefinition::Record(RecordId::new(index)),
                )
            })
            .chain(
                program
                    .enums
                    .iter()
                    .enumerate()
                    .map(|(index, enumeration)| {
                        (
                            enumeration.name.span.start(),
                            &enumeration.name,
                            TypeDefinition::Enum(EnumId::new(index)),
                        )
                    }),
            )
            .collect::<Vec<_>>();
        declarations.sort_by_key(|(start, _, _)| *start);

        for (_, name, definition) in declarations {
            if matches!(name.text.as_str(), "Int" | "Bool") {
                self.diagnostics.push(
                    Diagnostic::error("N3002", "duplicate type definition").with_primary(
                        name.span,
                        format!("`{}` is a built-in type name", name.text),
                    ),
                );
                continue;
            }
            if let Some(previous) = self.types.get(&name.text).copied() {
                self.diagnostics.push(
                    Diagnostic::error("N3002", "duplicate type definition")
                        .with_primary(
                            name.span,
                            format!("type `{}` is defined more than once", name.text),
                        )
                        .with_secondary(previous.span, "first type definition is here"),
                );
            } else {
                self.types.insert(
                    name.text.clone(),
                    TypeSymbol {
                        definition,
                        span: name.span,
                    },
                );
            }
        }

        for (index, record) in program.records.iter().enumerate() {
            let mut seen_fields = BTreeMap::<String, Span>::new();
            let mut fields = Vec::with_capacity(record.fields.len());
            for field in &record.fields {
                let ty = self.resolve_type_ref(&field.ty);
                if let Some(previous) = seen_fields.get(&field.name.text).copied() {
                    self.diagnostics.push(
                        Diagnostic::error("N3010", "duplicate record field")
                            .with_primary(
                                field.name.span,
                                format!("field `{}` is declared more than once", field.name.text),
                            )
                            .with_secondary(previous, "first field declaration is here"),
                    );
                } else {
                    seen_fields.insert(field.name.text.clone(), field.name.span);
                }
                fields.push(hir::RecordField {
                    name: field.name.text.clone(),
                    ty,
                    span: field.span,
                });
            }
            self.record_definitions[index].fields = fields;
        }

        for (index, enumeration) in program.enums.iter().enumerate() {
            let mut seen_variants = BTreeMap::<String, Span>::new();
            let mut variants = Vec::with_capacity(enumeration.variants.len());
            for variant in &enumeration.variants {
                let payload = variant
                    .payload
                    .as_ref()
                    .map(|reference| self.resolve_type_ref(reference));
                if let Some(previous) = seen_variants.get(&variant.name.text).copied() {
                    self.diagnostics.push(
                        Diagnostic::error("N3020", "duplicate enum variant")
                            .with_primary(
                                variant.name.span,
                                format!(
                                    "variant `{}` is declared more than once",
                                    variant.name.text
                                ),
                            )
                            .with_secondary(previous, "first variant declaration is here"),
                    );
                } else {
                    seen_variants.insert(variant.name.text.clone(), variant.name.span);
                }
                variants.push(hir::EnumVariant {
                    name: variant.name.text.clone(),
                    payload,
                    span: variant.span,
                });
            }
            self.enum_definitions[index].variants = variants;
        }
    }

    fn collect_function_signatures(&mut self, program: &ast::Program) {
        for (index, function) in program.functions.iter().enumerate() {
            let id = FunctionId::new(index);
            let parameters = function
                .parameters
                .iter()
                .map(|parameter| self.resolve_type_ref(&parameter.ty))
                .collect::<Vec<_>>();
            let return_type = self.resolve_type_ref(&function.return_type);
            let record = SignatureRecord {
                parameters,
                return_type,
            };

            if let Some(previous) = self.functions.get(&function.name.text) {
                self.diagnostics.push(
                    Diagnostic::error("N3002", "duplicate definition")
                        .with_primary(
                            function.name.span,
                            format!("`{}` is defined more than once", function.name.text),
                        )
                        .with_secondary(previous.span, "first definition is here"),
                );
            } else {
                self.functions.insert(
                    function.name.text.clone(),
                    FunctionSymbol {
                        id,
                        signature: record.function_type(),
                        span: function.name.span,
                    },
                );
            }
            self.signatures.push(record);
        }
    }

    fn resolve_type_ref(&mut self, reference: &ast::TypeRef) -> Type {
        match reference.name.text.as_str() {
            "Int" => Type::Int,
            "Bool" => Type::Bool,
            unknown => {
                if let Some(symbol) = self.types.get(unknown).copied() {
                    return match symbol.definition {
                        TypeDefinition::Record(id) => Type::Record(RecordType {
                            id,
                            name: unknown.to_owned(),
                        }),
                        TypeDefinition::Enum(id) => Type::Enum(EnumType {
                            id,
                            name: unknown.to_owned(),
                        }),
                    };
                }
                self.diagnostics.push(
                    Diagnostic::error("N3001", "unknown type")
                        .with_primary(reference.span, format!("unknown type `{unknown}`"))
                        .with_note(
                            "the bootstrap semantic core recognizes Int, Bool, and declared record or enum names",
                        ),
                );
                Type::Error
            }
        }
    }

    fn lower_function(&mut self, id: FunctionId, function: &ast::Function) -> hir::Function {
        let signature = self.signatures[id.index()].clone();
        self.scopes.clear();
        self.scopes.push(BTreeMap::new());
        self.loop_stack.clear();

        let mut parameters = Vec::with_capacity(function.parameters.len());
        for (parameter, ty) in function.parameters.iter().zip(&signature.parameters) {
            let binding = self.new_binding(&parameter.name, ty.clone(), false);
            self.insert_local(&binding, true);
            parameters.push(binding);
        }

        let body = self.lower_block(&function.body, &signature.return_type, false);
        debug_assert!(self.loop_stack.is_empty());
        if !body.ty.is_never() && function.body.tail.is_none() {
            self.diagnostics.push(
                Diagnostic::error("N3007", "function can complete without returning a value")
                    .with_primary(
                        function.body.span,
                        format!(
                            "`{}` must return {} on every path",
                            function.name.text, signature.return_type
                        ),
                    ),
            );
        } else if function.body.tail.is_some() {
            self.require_type(
                &body.ty,
                &signature.return_type,
                function
                    .body
                    .tail
                    .as_ref()
                    .map_or(function.body.span, |tail| tail.span),
                "function tail expression",
            );
        }

        self.scopes.clear();
        hir::Function {
            id,
            name: function.name.text.clone(),
            parameters,
            return_type: signature.return_type,
            body,
            span: function.span,
        }
    }

    fn lower_block(
        &mut self,
        block: &ast::Block,
        return_type: &Type,
        push_scope: bool,
    ) -> hir::Block {
        if push_scope {
            self.scopes.push(BTreeMap::new());
        }

        let mut terminated = false;
        let mut statements = Vec::with_capacity(block.statements.len());
        for statement in &block.statements {
            let reachable_state = self.capture_reachable_state();
            let (statement, diverges) = self.lower_statement(statement, return_type);
            statements.push(statement);
            if terminated {
                self.restore_reachable_state(reachable_state);
            } else if diverges {
                terminated = true;
            }
        }

        let tail = block.tail.as_deref().map(|expression| {
            let reachable_state = self.capture_reachable_state();
            let expression = Box::new(self.lower_expression(expression, return_type));
            if terminated {
                self.restore_reachable_state(reachable_state);
            }
            expression
        });
        let ty = if terminated {
            Type::Never
        } else {
            tail.as_ref()
                .map_or(Type::Unit, |expression| expression.ty.clone())
        };

        if push_scope {
            let popped = self.scopes.pop();
            debug_assert!(popped.is_some());
        }

        hir::Block {
            statements,
            tail,
            ty,
            span: block.span,
        }
    }

    fn lower_statement(
        &mut self,
        statement: &ast::Statement,
        return_type: &Type,
    ) -> (hir::Statement, bool) {
        let (kind, diverges) = match &statement.kind {
            ast::StatementKind::Binding {
                mutable,
                name,
                annotation,
                initializer,
            } => {
                let initializer = self.lower_expression(initializer, return_type);
                let annotation_type = annotation
                    .as_ref()
                    .map(|reference| self.resolve_type_ref(reference));
                if let Some(expected) = &annotation_type {
                    self.require_type(
                        &initializer.ty,
                        expected,
                        initializer.span,
                        "binding initializer",
                    );
                }
                let binding_type = annotation_type.unwrap_or_else(|| initializer.ty.clone());
                let binding = self.new_binding(name, binding_type, *mutable);
                self.insert_local(&binding, true);
                let diverges = initializer.ty.is_never();
                (
                    StatementKind::Binding {
                        binding,
                        initializer,
                    },
                    diverges,
                )
            }
            ast::StatementKind::UninitializedBinding { name, annotation } => {
                let ty = self.resolve_type_ref(annotation);
                let binding = self.new_binding(name, ty, true);
                self.insert_local(&binding, false);
                (StatementKind::UninitializedBinding(binding), false)
            }
            ast::StatementKind::Assignment { target, value } => {
                let local = self.find_local(&target.text);
                let function_span = self.functions.get(&target.text).map(|symbol| symbol.span);
                let value = self.lower_expression(value, return_type);
                let target_id = if let Some(symbol) = local {
                    if !symbol.mutable {
                        self.diagnostics.push(
                            Diagnostic::error("N3008", "cannot assign to immutable binding")
                                .with_primary(
                                    target.span,
                                    format!("`{}` is not mutable", target.text),
                                )
                                .with_secondary(symbol.span, "binding declared here"),
                        );
                    }
                    self.require_type(&value.ty, &symbol.ty, value.span, "assigned value");
                    if symbol.mutable
                        && !value.ty.is_error()
                        && !value.ty.is_never()
                        && types_compatible(&value.ty, &symbol.ty)
                    {
                        self.mark_initialized(&target.text);
                    }
                    Some(symbol.id)
                } else if let Some(span) = function_span {
                    self.diagnostics.push(
                        Diagnostic::error("N3008", "invalid assignment target")
                            .with_primary(target.span, "functions cannot be assigned")
                            .with_secondary(span, "function declared here"),
                    );
                    None
                } else {
                    self.diagnostics.push(
                        Diagnostic::error("N3003", "unknown name")
                            .with_primary(target.span, format!("cannot resolve `{}`", target.text)),
                    );
                    None
                };
                let diverges = value.ty.is_never();
                (
                    StatementKind::Assignment {
                        target: target_id,
                        value,
                    },
                    diverges,
                )
            }
            ast::StatementKind::While { condition, body } => {
                let condition = self.lower_expression(condition, return_type);
                self.require_type(
                    &condition.ty,
                    &Type::Bool,
                    condition.span,
                    "while condition",
                );

                let post_condition_scopes = self.scopes.clone();
                let guaranteed_entry = matches!(&condition.kind, ExpressionKind::Boolean(true));
                self.loop_stack.push(LoopContext {
                    visible_scope_count: self.scopes.len(),
                    break_states: Vec::new(),
                });
                let body = self.lower_block(body, return_type, true);
                let loop_context = self
                    .loop_stack
                    .pop()
                    .expect("while lowering must own one loop context");

                let diverges = if condition.ty.is_never() {
                    self.scopes = post_condition_scopes;
                    true
                } else if guaranteed_entry {
                    if loop_context.break_states.is_empty() {
                        self.scopes = post_condition_scopes;
                        true
                    } else {
                        self.merge_loop_break_initialization(
                            &post_condition_scopes,
                            &loop_context.break_states,
                        );
                        false
                    }
                } else {
                    self.scopes = post_condition_scopes;
                    false
                };
                (StatementKind::While { condition, body }, diverges)
            }
            ast::StatementKind::Break => {
                let legal = !self.loop_stack.is_empty();
                if legal {
                    self.record_loop_break_exit();
                } else {
                    self.diagnostics.push(
                        Diagnostic::error("N3013", "loop control outside loop").with_primary(
                            statement.span,
                            "`break` requires a lexically enclosing `while` body",
                        ),
                    );
                }
                (StatementKind::Break, legal)
            }
            ast::StatementKind::Continue => {
                let legal = !self.loop_stack.is_empty();
                if !legal {
                    self.diagnostics.push(
                        Diagnostic::error("N3013", "loop control outside loop").with_primary(
                            statement.span,
                            "`continue` requires a lexically enclosing `while` body",
                        ),
                    );
                }
                (StatementKind::Continue, legal)
            }
            ast::StatementKind::Return(expression) => {
                let expression = self.lower_expression(expression, return_type);
                self.require_type(
                    &expression.ty,
                    return_type,
                    expression.span,
                    "return expression",
                );
                (StatementKind::Return(expression), true)
            }
            ast::StatementKind::Expression(expression) => {
                let expression = self.lower_expression(expression, return_type);
                let diverges = expression.ty.is_never();
                (StatementKind::Expression(expression), diverges)
            }
        };

        (
            hir::Statement {
                kind,
                span: statement.span,
            },
            diverges,
        )
    }

    fn lower_expression(
        &mut self,
        expression: &ast::Expression,
        return_type: &Type,
    ) -> hir::Expression {
        let (kind, ty) = match &expression.kind {
            ast::ExpressionKind::Integer(value) => (ExpressionKind::Integer(*value), Type::Int),
            ast::ExpressionKind::Boolean(value) => (ExpressionKind::Boolean(*value), Type::Bool),
            ast::ExpressionKind::Name(name) => self.lower_name(name),
            ast::ExpressionKind::RecordLiteral { name, fields } => {
                self.lower_record_literal(name, fields, return_type, expression.span)
            }
            ast::ExpressionKind::EnumConstructor {
                enumeration,
                variant,
                payload,
            } => self.lower_enum_constructor(enumeration, variant, payload.as_deref(), return_type),
            ast::ExpressionKind::FieldAccess { base, field } => {
                self.lower_field_access(base, field, return_type)
            }
            ast::ExpressionKind::Unary { operator, operand } => {
                let operand = self.lower_expression(operand, return_type);
                let ty = self.check_unary(*operator, &operand, expression.span);
                (
                    ExpressionKind::Unary {
                        operator: *operator,
                        operand: Box::new(operand),
                    },
                    ty,
                )
            }
            ast::ExpressionKind::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.lower_expression(left, return_type);
                let left_scopes = self.scopes.clone();
                let left_literal = match &left.kind {
                    ExpressionKind::Boolean(value) => Some(*value),
                    _ => None,
                };
                let skips_right = matches!(
                    (*operator, left_literal),
                    (BinaryOperator::And, Some(false)) | (BinaryOperator::Or, Some(true))
                );
                let forces_right = matches!(
                    (*operator, left_literal),
                    (BinaryOperator::And, Some(true)) | (BinaryOperator::Or, Some(false))
                );
                let short_circuit_operator =
                    matches!(operator, BinaryOperator::And | BinaryOperator::Or);

                let right = if left.ty.is_never() || (short_circuit_operator && skips_right) {
                    self.lower_expression_for_diagnostics(right, return_type)
                } else {
                    self.lower_expression(right, return_type)
                };

                if short_circuit_operator && !left.ty.is_never() && !skips_right && !forces_right {
                    let right_scopes = self.scopes.clone();
                    self.merge_optional_execution_initialization(
                        &left_scopes,
                        &right_scopes,
                        right.ty.is_never(),
                    );
                }

                let ty = self.check_binary(*operator, &left, &right, expression.span);
                (
                    ExpressionKind::Binary {
                        operator: *operator,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    ty,
                )
            }
            ast::ExpressionKind::Call { callee, arguments } => {
                let callee = self.lower_expression(callee, return_type);
                let mut can_continue = !callee.ty.is_never();
                let mut lowered_arguments = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    let argument = if can_continue {
                        self.lower_expression(argument, return_type)
                    } else {
                        self.lower_expression_for_diagnostics(argument, return_type)
                    };
                    if can_continue && argument.ty.is_never() {
                        can_continue = false;
                    }
                    lowered_arguments.push(argument);
                }
                let ty = self.check_call(&callee, &lowered_arguments, expression.span);
                (
                    ExpressionKind::Call {
                        callee: Box::new(callee),
                        arguments: lowered_arguments,
                    },
                    ty,
                )
            }
            ast::ExpressionKind::Block(block) => {
                let block = self.lower_block(block, return_type, true);
                let ty = block.ty.clone();
                (ExpressionKind::Block(block), ty)
            }
            ast::ExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.lower_expression(condition, return_type);
                self.require_type(&condition.ty, &Type::Bool, condition.span, "if condition");

                let condition_literal = match &condition.kind {
                    ExpressionKind::Boolean(value) => Some(*value),
                    _ => None,
                };
                let entry_scopes = self.scopes.clone();
                let post_condition_loop_stack = self.loop_stack.clone();

                let (then_branch, then_scopes, else_branch, else_scopes) = match condition_literal {
                    Some(true) => {
                        let then_branch = self.lower_block(then_branch, return_type, true);
                        let then_scopes = self.scopes.clone();
                        let then_loop_stack = self.loop_stack.clone();

                        self.scopes = entry_scopes.clone();
                        self.loop_stack = post_condition_loop_stack.clone();
                        let else_branch =
                            self.lower_expression_for_diagnostics(else_branch, return_type);
                        let else_scopes = self.scopes.clone();

                        self.scopes = then_scopes.clone();
                        self.loop_stack = then_loop_stack;
                        (then_branch, then_scopes, else_branch, else_scopes)
                    }
                    Some(false) => {
                        let then_branch =
                            self.lower_block_for_diagnostics(then_branch, return_type, true);
                        let then_scopes = self.scopes.clone();

                        self.scopes = entry_scopes.clone();
                        self.loop_stack = post_condition_loop_stack.clone();
                        let else_branch = self.lower_expression(else_branch, return_type);
                        let else_scopes = self.scopes.clone();
                        (then_branch, then_scopes, else_branch, else_scopes)
                    }
                    None => {
                        let then_branch = self.lower_block(then_branch, return_type, true);
                        let then_scopes = self.scopes.clone();

                        self.scopes = entry_scopes.clone();
                        let else_branch = self.lower_expression(else_branch, return_type);
                        let else_scopes = self.scopes.clone();
                        (then_branch, then_scopes, else_branch, else_scopes)
                    }
                };

                let joined_type = self.join_branch_types(
                    &then_branch.ty,
                    then_branch.span,
                    &else_branch.ty,
                    else_branch.span,
                );
                let ty = if condition.ty.is_never() {
                    self.scopes = entry_scopes;
                    self.loop_stack = post_condition_loop_stack;
                    Type::Never
                } else {
                    match condition_literal {
                        Some(true) => {
                            self.scopes = then_scopes;
                            if joined_type.is_error() {
                                Type::Error
                            } else {
                                then_branch.ty.clone()
                            }
                        }
                        Some(false) => {
                            self.scopes = else_scopes;
                            if joined_type.is_error() {
                                Type::Error
                            } else {
                                else_branch.ty.clone()
                            }
                        }
                        None => {
                            self.merge_branch_initialization(
                                &entry_scopes,
                                &then_scopes,
                                then_branch.ty.is_never(),
                                &else_scopes,
                                else_branch.ty.is_never(),
                            );
                            joined_type
                        }
                    }
                };
                (
                    ExpressionKind::If {
                        condition: Box::new(condition),
                        then_branch,
                        else_branch: Box::new(else_branch),
                    },
                    ty,
                )
            }
            ast::ExpressionKind::Match { scrutinee, arms } => {
                self.lower_match(scrutinee, arms, return_type, expression.span)
            }
        };

        hir::Expression {
            kind,
            ty,
            span: expression.span,
        }
    }

    fn lower_expression_for_diagnostics(
        &mut self,
        expression: &ast::Expression,
        return_type: &Type,
    ) -> hir::Expression {
        let reachable_state = self.capture_reachable_state();
        let lowered = self.lower_expression(expression, return_type);
        self.restore_reachable_state(reachable_state);
        lowered
    }

    fn lower_block_for_diagnostics(
        &mut self,
        block: &ast::Block,
        return_type: &Type,
        push_scope: bool,
    ) -> hir::Block {
        let reachable_state = self.capture_reachable_state();
        let lowered = self.lower_block(block, return_type, push_scope);
        self.restore_reachable_state(reachable_state);
        lowered
    }

    fn lower_record_literal(
        &mut self,
        name: &ast::Name,
        fields: &[ast::RecordLiteralField],
        return_type: &Type,
        _span: Span,
    ) -> (ExpressionKind, Type) {
        let Some(symbol) = self.types.get(&name.text).copied() else {
            let mut can_continue = true;
            for field in fields {
                let value = if can_continue {
                    self.lower_expression(&field.value, return_type)
                } else {
                    self.lower_expression_for_diagnostics(&field.value, return_type)
                };
                if can_continue && value.ty.is_never() {
                    can_continue = false;
                }
            }
            self.diagnostics.push(
                Diagnostic::error("N3001", "unknown type")
                    .with_primary(name.span, format!("unknown record type `{}`", name.text)),
            );
            return (ExpressionKind::Error, Type::Error);
        };
        let TypeDefinition::Record(record_id) = symbol.definition else {
            let mut can_continue = true;
            for field in fields {
                let value = if can_continue {
                    self.lower_expression(&field.value, return_type)
                } else {
                    self.lower_expression_for_diagnostics(&field.value, return_type)
                };
                if can_continue && value.ty.is_never() {
                    can_continue = false;
                }
            }
            self.diagnostics.push(
                Diagnostic::error("N3004", "type mismatch")
                    .with_primary(
                        name.span,
                        format!("`{}` is an enum, not a record", name.text),
                    )
                    .with_secondary(symbol.span, "enum declared here"),
            );
            return (ExpressionKind::Error, Type::Error);
        };
        let definition = self.record_definitions[record_id.index()].clone();
        let aggregate_entry_state = self.capture_reachable_state();
        let mut seen = BTreeMap::<String, Span>::new();
        let mut resolved = Vec::with_capacity(fields.len());
        let mut structural_error = false;
        let mut contains_error = false;
        let mut contains_never = false;
        let mut type_error = false;
        let mut can_continue = true;

        for field in fields {
            let value = if can_continue {
                self.lower_expression(&field.value, return_type)
            } else {
                self.lower_expression_for_diagnostics(&field.value, return_type)
            };
            contains_error |= value.ty.is_error();
            contains_never |= value.ty.is_never();
            if can_continue && value.ty.is_never() {
                can_continue = false;
            }

            let Some(field_index) = definition
                .fields
                .iter()
                .position(|declared| declared.name == field.name.text)
            else {
                self.diagnostics.push(
                    Diagnostic::error("N3011", "unknown record field")
                        .with_primary(
                            field.name.span,
                            format!(
                                "record `{}` has no field named `{}`",
                                definition.name, field.name.text
                            ),
                        )
                        .with_secondary(definition.span, "record declared here"),
                );
                structural_error = true;
                continue;
            };

            if let Some(previous) = seen.get(&field.name.text).copied() {
                self.diagnostics.push(
                    Diagnostic::error("N3010", "duplicate record field")
                        .with_primary(
                            field.name.span,
                            format!("field `{}` is initialized more than once", field.name.text),
                        )
                        .with_secondary(previous, "first initializer is here"),
                );
                structural_error = true;
                continue;
            }
            seen.insert(field.name.text.clone(), field.name.span);

            let expected = &definition.fields[field_index].ty;
            let type_matches = types_compatible(&value.ty, expected);
            self.require_type(&value.ty, expected, value.span, "record field initializer");
            type_error |= !type_matches;
            resolved.push(RecordFieldValue { field_index, value });
        }

        for declared in &definition.fields {
            if !seen.contains_key(&declared.name) {
                self.diagnostics.push(
                    Diagnostic::error("N3012", "missing record field")
                        .with_primary(
                            name.span,
                            format!(
                                "construction of `{}` is missing field `{}`",
                                definition.name, declared.name
                            ),
                        )
                        .with_secondary(declared.span, "field declared here"),
                );
                structural_error = true;
            }
        }

        let ty = if contains_never {
            Type::Never
        } else if structural_error || contains_error || type_error {
            Type::Error
        } else {
            Type::Record(definition.record_type())
        };
        let should_restore = ty.is_error();
        let result = if structural_error {
            (ExpressionKind::Error, ty)
        } else {
            (
                ExpressionKind::RecordLiteral {
                    record: record_id,
                    fields: resolved,
                },
                ty,
            )
        };
        if should_restore {
            self.restore_reachable_state(aggregate_entry_state);
        }
        result
    }

    fn lower_enum_constructor(
        &mut self,
        enumeration: &ast::Name,
        variant: &ast::Name,
        payload: Option<&ast::Expression>,
        return_type: &Type,
    ) -> (ExpressionKind, Type) {
        let aggregate_entry_state = self.capture_reachable_state();
        let payload =
            payload.map(|expression| Box::new(self.lower_expression(expression, return_type)));
        let payload_never = payload
            .as_deref()
            .is_some_and(|expression| expression.ty.is_never());
        let payload_error = payload
            .as_deref()
            .is_some_and(|expression| expression.ty.is_error());

        let Some(symbol) = self.types.get(&enumeration.text).copied() else {
            self.diagnostics.push(
                Diagnostic::error("N3021", "unknown enum")
                    .with_primary(
                        enumeration.span,
                        format!("cannot resolve enum `{}`", enumeration.text),
                    )
                    .with_note("enum constructors use `Enum::Variant` qualification"),
            );
            let ty = if payload_never {
                Type::Never
            } else {
                self.restore_reachable_state(aggregate_entry_state);
                Type::Error
            };
            return (ExpressionKind::Error, ty);
        };
        let TypeDefinition::Enum(enum_id) = symbol.definition else {
            self.diagnostics.push(
                Diagnostic::error("N3021", "invalid enum constructor")
                    .with_primary(
                        enumeration.span,
                        format!("`{}` is a record, not an enum", enumeration.text),
                    )
                    .with_secondary(symbol.span, "record declared here"),
            );
            let ty = if payload_never {
                Type::Never
            } else {
                self.restore_reachable_state(aggregate_entry_state);
                Type::Error
            };
            return (ExpressionKind::Error, ty);
        };
        let definition = self.enum_definitions[enum_id.index()].clone();
        let Some(variant_index) = definition
            .variants
            .iter()
            .position(|declared| declared.name == variant.text)
        else {
            self.diagnostics.push(
                Diagnostic::error("N3021", "unknown enum variant")
                    .with_primary(
                        variant.span,
                        format!(
                            "enum `{}` has no variant named `{}`",
                            definition.name, variant.text
                        ),
                    )
                    .with_secondary(definition.span, "enum declared here"),
            );
            let ty = if payload_never {
                Type::Never
            } else {
                self.restore_reachable_state(aggregate_entry_state);
                Type::Error
            };
            return (ExpressionKind::Error, ty);
        };

        let declared = &definition.variants[variant_index];
        let mut payload_type_error = false;
        let arity_matches = match (&declared.payload, payload.as_deref()) {
            (Some(expected), Some(actual)) => {
                let type_matches = types_compatible(&actual.ty, expected);
                self.require_type(&actual.ty, expected, actual.span, "enum variant payload");
                payload_type_error = !type_matches;
                true
            }
            (None, None) => true,
            (Some(_), None) => {
                self.diagnostics.push(
                    Diagnostic::error("N3022", "missing enum variant payload")
                        .with_primary(
                            variant.span,
                            format!("variant `{}` requires one payload", declared.name),
                        )
                        .with_secondary(declared.span, "variant declared with a payload here"),
                );
                false
            }
            (None, Some(actual)) => {
                self.diagnostics.push(
                    Diagnostic::error("N3022", "unexpected enum variant payload")
                        .with_primary(
                            actual.span,
                            format!("variant `{}` does not accept a payload", declared.name),
                        )
                        .with_secondary(declared.span, "payload-free variant declared here"),
                );
                false
            }
        };

        let ty = if payload_never {
            Type::Never
        } else if payload_error || payload_type_error || !arity_matches {
            Type::Error
        } else {
            Type::Enum(definition.enum_type())
        };
        let should_restore = ty.is_error();
        let result = if arity_matches {
            (
                ExpressionKind::EnumConstructor {
                    enumeration: enum_id,
                    variant_index,
                    payload,
                },
                ty,
            )
        } else {
            (ExpressionKind::Error, ty)
        };
        if should_restore {
            self.restore_reachable_state(aggregate_entry_state);
        }
        result
    }

    fn lower_match(
        &mut self,
        scrutinee: &ast::Expression,
        arms: &[ast::MatchArm],
        return_type: &Type,
        span: Span,
    ) -> (ExpressionKind, Type) {
        let scrutinee = self.lower_expression(scrutinee, return_type);
        let selected_variant_index = match (&scrutinee.kind, &scrutinee.ty) {
            (
                ExpressionKind::EnumConstructor {
                    enumeration,
                    variant_index,
                    ..
                },
                Type::Enum(scrutinee_type),
            ) if *enumeration == scrutinee_type.id => Some(*variant_index),
            _ => None,
        };
        let post_scrutinee_loop_stack = self.loop_stack.clone();
        let mut scrutinee_enum = match &scrutinee.ty {
            Type::Enum(enumeration) => Some(enumeration.clone()),
            Type::Error | Type::Never => None,
            actual => {
                self.diagnostics.push(
                    Diagnostic::error("N3025", "match requires an enum value").with_primary(
                        scrutinee.span,
                        format!("cannot match a value of type {actual}"),
                    ),
                );
                None
            }
        };
        let entry_scopes = self.scopes.clone();
        let mut seen = BTreeMap::<usize, Span>::new();
        let mut lowered_arms = Vec::with_capacity(arms.len());
        let mut branch_states = Vec::with_capacity(arms.len());
        let mut branch_types = Vec::with_capacity(arms.len());
        let mut selected_branch = None::<(ScopeState, Type)>;
        let mut structural_error = scrutinee_enum.is_none() && !scrutinee.ty.is_never();

        for arm in arms {
            self.scopes = entry_scopes.clone();
            self.scopes.push(BTreeMap::new());
            let mut valid_pattern = true;
            let mut resolved_index = None;
            let mut payload_binding = None;

            let symbol = self.types.get(&arm.pattern.enumeration.text).copied();
            let pattern_enum_id = match symbol {
                Some(TypeSymbol {
                    definition: TypeDefinition::Enum(id),
                    ..
                }) => Some(id),
                Some(symbol) => {
                    self.diagnostics.push(
                        Diagnostic::error("N3021", "invalid enum pattern")
                            .with_primary(
                                arm.pattern.enumeration.span,
                                format!(
                                    "`{}` is a record, not an enum",
                                    arm.pattern.enumeration.text
                                ),
                            )
                            .with_secondary(symbol.span, "record declared here"),
                    );
                    valid_pattern = false;
                    None
                }
                None => {
                    self.diagnostics
                        .push(Diagnostic::error("N3021", "unknown enum").with_primary(
                            arm.pattern.enumeration.span,
                            format!("cannot resolve enum `{}`", arm.pattern.enumeration.text),
                        ));
                    valid_pattern = false;
                    None
                }
            };

            if let Some(pattern_enum_id) = pattern_enum_id {
                let definition = self.enum_definitions[pattern_enum_id.index()].clone();
                if scrutinee.ty.is_never() && scrutinee_enum.is_none() {
                    scrutinee_enum = Some(definition.enum_type());
                }
                if let Some(expected) = &scrutinee_enum {
                    if expected.id != pattern_enum_id {
                        self.diagnostics.push(
                            Diagnostic::error("N3025", "pattern enum does not match scrutinee")
                                .with_primary(
                                    arm.pattern.enumeration.span,
                                    format!(
                                        "pattern names `{}`, but the scrutinee has type {}",
                                        arm.pattern.enumeration.text, expected.name
                                    ),
                                ),
                        );
                        valid_pattern = false;
                    }
                }

                if let Some(variant_index) = definition
                    .variants
                    .iter()
                    .position(|declared| declared.name == arm.pattern.variant.text)
                {
                    let declared = &definition.variants[variant_index];
                    resolved_index = Some(variant_index);
                    match (&declared.payload, &arm.pattern.binding) {
                        (Some(payload_type), Some(binding_name)) => {
                            let binding =
                                self.new_binding(binding_name, payload_type.clone(), false);
                            self.insert_local(&binding, true);
                            payload_binding = Some(binding);
                        }
                        (None, None) => {}
                        (Some(_), None) => {
                            self.diagnostics.push(
                                Diagnostic::error("N3022", "missing pattern payload binding")
                                    .with_primary(
                                        arm.pattern.variant.span,
                                        format!("variant `{}` carries one payload", declared.name),
                                    )
                                    .with_secondary(
                                        declared.span,
                                        "variant declared with a payload here",
                                    ),
                            );
                            valid_pattern = false;
                        }
                        (None, Some(binding_name)) => {
                            self.diagnostics.push(
                                Diagnostic::error("N3022", "unexpected pattern payload binding")
                                    .with_primary(
                                        binding_name.span,
                                        format!(
                                            "variant `{}` does not carry a payload",
                                            declared.name
                                        ),
                                    )
                                    .with_secondary(
                                        declared.span,
                                        "payload-free variant declared here",
                                    ),
                            );
                            valid_pattern = false;
                        }
                    }
                } else {
                    self.diagnostics.push(
                        Diagnostic::error("N3021", "unknown enum variant")
                            .with_primary(
                                arm.pattern.variant.span,
                                format!(
                                    "enum `{}` has no variant named `{}`",
                                    definition.name, arm.pattern.variant.text
                                ),
                            )
                            .with_secondary(definition.span, "enum declared here"),
                    );
                    valid_pattern = false;
                }
            }

            if payload_binding.is_none() {
                if let Some(binding_name) = &arm.pattern.binding {
                    let binding = self.new_binding(binding_name, Type::Error, false);
                    self.insert_local(&binding, true);
                    payload_binding = Some(binding);
                }
            }

            if valid_pattern {
                if let (Some(expected), Some(index)) = (&scrutinee_enum, resolved_index) {
                    if let Some(previous) = seen.get(&index).copied() {
                        self.diagnostics.push(
                            Diagnostic::error("N3024", "duplicate match variant")
                                .with_primary(
                                    arm.pattern.span,
                                    format!(
                                        "variant `{}::{}` is matched more than once",
                                        expected.name, arm.pattern.variant.text
                                    ),
                                )
                                .with_secondary(previous, "first matching arm is here"),
                        );
                        valid_pattern = false;
                    } else {
                        seen.insert(index, arm.pattern.span);
                    }
                }
            }

            let selected_arm = selected_variant_index
                .is_some_and(|selected| valid_pattern && resolved_index == Some(selected));
            let value = if selected_variant_index.is_some() && !selected_arm {
                self.lower_expression_for_diagnostics(&arm.value, return_type)
            } else {
                self.lower_expression(&arm.value, return_type)
            };
            let popped = self.scopes.pop();
            debug_assert!(popped.is_some());
            let branch_state = (self.scopes.clone(), value.ty.is_never());
            if selected_arm {
                selected_branch = Some((branch_state.0.clone(), value.ty.clone()));
            }
            branch_states.push(branch_state);
            branch_types.push((value.ty.clone(), value.span));

            if let Some(variant_index) = resolved_index {
                lowered_arms.push(MatchArm {
                    variant_index,
                    binding: payload_binding,
                    value,
                    span: arm.span,
                });
            }
            structural_error |= !valid_pattern;
        }

        if scrutinee.ty.is_never() && scrutinee_enum.is_none() {
            self.diagnostics.push(
                Diagnostic::error("N3025", "cannot determine matched enum").with_primary(
                    span,
                    "a match with a non-continuing scrutinee still needs a qualified variant arm",
                ),
            );
            structural_error = true;
        }

        if let Some(enumeration) = &scrutinee_enum {
            let definition = &self.enum_definitions[enumeration.id.index()];
            let missing = definition
                .variants
                .iter()
                .enumerate()
                .filter(|(index, _)| !seen.contains_key(index))
                .map(|(_, variant)| variant.name.as_str())
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                self.diagnostics.push(
                    Diagnostic::error("N3023", "non-exhaustive match")
                        .with_primary(span, format!("missing variant(s): {}", missing.join(", ")))
                        .with_secondary(definition.span, "enum variants declared here"),
                );
                structural_error = true;
            }
        }

        let joined_type = self.join_match_arm_types(&branch_types);
        let ty = if scrutinee.ty.is_never() {
            self.scopes = entry_scopes;
            self.loop_stack = post_scrutinee_loop_stack;
            Type::Never
        } else if structural_error {
            self.scopes = entry_scopes;
            self.loop_stack = post_scrutinee_loop_stack;
            Type::Error
        } else if let Some((selected_scopes, selected_type)) = selected_branch {
            self.scopes = selected_scopes;
            if joined_type.is_error() {
                Type::Error
            } else {
                selected_type
            }
        } else {
            self.merge_match_initialization(&entry_scopes, &branch_states);
            joined_type
        };

        match (structural_error, scrutinee_enum) {
            (false, Some(enumeration)) => (
                ExpressionKind::Match {
                    scrutinee: Box::new(scrutinee),
                    enumeration: enumeration.id,
                    arms: lowered_arms,
                },
                ty,
            ),
            _ => (ExpressionKind::Error, ty),
        }
    }

    fn lower_field_access(
        &mut self,
        base: &ast::Expression,
        field: &ast::Name,
        return_type: &Type,
    ) -> (ExpressionKind, Type) {
        let base = self.lower_expression(base, return_type);
        let Type::Record(record_type) = base.ty.clone() else {
            if base.ty.is_error() {
                return (ExpressionKind::Error, Type::Error);
            }
            self.diagnostics
                .push(Diagnostic::error("N3004", "type mismatch").with_primary(
                    field.span,
                    format!("field access requires a record value, found {}", base.ty),
                ));
            return (ExpressionKind::Error, Type::Error);
        };

        let definition = self.record_definitions[record_type.id.index()].clone();
        let Some(field_index) = definition
            .fields
            .iter()
            .position(|declared| declared.name == field.text)
        else {
            self.diagnostics.push(
                Diagnostic::error("N3011", "unknown record field")
                    .with_primary(
                        field.span,
                        format!(
                            "record `{}` has no field named `{}`",
                            definition.name, field.text
                        ),
                    )
                    .with_secondary(definition.span, "record declared here"),
            );
            return (ExpressionKind::Error, Type::Error);
        };
        let ty = definition.fields[field_index].ty.clone();
        (
            ExpressionKind::FieldAccess {
                base: Box::new(base),
                record: record_type.id,
                field_index,
            },
            ty,
        )
    }

    fn lower_name(&mut self, name: &ast::Name) -> (ExpressionKind, Type) {
        if let Some(symbol) = self.find_local(&name.text) {
            if !symbol.initialized {
                self.diagnostics.push(
                    Diagnostic::error("N3009", "binding may be uninitialized")
                        .with_primary(
                            name.span,
                            format!("`{}` is not definitely initialized on this path", name.text),
                        )
                        .with_secondary(symbol.span, "binding declared here"),
                );
                return (ExpressionKind::Binding(symbol.id), Type::Error);
            }
            return (ExpressionKind::Binding(symbol.id), symbol.ty);
        }
        if let Some(symbol) = self.functions.get(&name.text) {
            return (
                ExpressionKind::Function(symbol.id),
                Type::Function(symbol.signature.clone()),
            );
        }

        self.diagnostics.push(
            Diagnostic::error("N3003", "unknown name")
                .with_primary(name.span, format!("cannot resolve `{}`", name.text)),
        );
        (ExpressionKind::Error, Type::Error)
    }

    fn find_local(&self, name: &str) -> Option<LocalSymbol> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn mark_initialized(&mut self, name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                symbol.initialized = true;
                return;
            }
        }
    }

    fn record_loop_break_exit(&mut self) {
        let visible_scope_count = self
            .loop_stack
            .last()
            .expect("a legal break must have an active loop context")
            .visible_scope_count;
        let state = self
            .scopes
            .iter()
            .take(visible_scope_count)
            .cloned()
            .collect();
        self.loop_stack
            .last_mut()
            .expect("a legal break must have an active loop context")
            .break_states
            .push(state);
    }

    fn merge_loop_break_initialization(
        &mut self,
        entry_scopes: &[Scope],
        break_states: &[ScopeState],
    ) {
        debug_assert!(!break_states.is_empty());
        self.scopes = entry_scopes.to_vec();
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for name in entry_scope.keys() {
                let initialized = break_states.iter().all(|break_scopes| {
                    break_scopes
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|symbol| symbol.initialized)
                });
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = initialized;
                }
            }
        }
    }

    fn merge_optional_execution_initialization(
        &mut self,
        entry_scopes: &[Scope],
        executed_scopes: &[Scope],
        executed_never: bool,
    ) {
        self.scopes = entry_scopes.to_vec();
        if executed_never {
            return;
        }
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for (name, entry_symbol) in entry_scope {
                let executed_initialized = executed_scopes
                    .get(scope_index)
                    .and_then(|scope| scope.get(name))
                    .is_some_and(|symbol| symbol.initialized);
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = entry_symbol.initialized && executed_initialized;
                }
            }
        }
    }

    fn merge_branch_initialization(
        &mut self,
        entry_scopes: &[Scope],
        then_scopes: &[Scope],
        then_never: bool,
        else_scopes: &[Scope],
        else_never: bool,
    ) {
        self.scopes = entry_scopes.to_vec();
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for (name, entry_symbol) in entry_scope {
                let then_initialized = then_scopes
                    .get(scope_index)
                    .and_then(|scope| scope.get(name))
                    .is_some_and(|symbol| symbol.initialized);
                let else_initialized = else_scopes
                    .get(scope_index)
                    .and_then(|scope| scope.get(name))
                    .is_some_and(|symbol| symbol.initialized);
                let initialized = match (then_never, else_never) {
                    (true, true) => entry_symbol.initialized,
                    (true, false) => else_initialized,
                    (false, true) => then_initialized,
                    (false, false) => then_initialized && else_initialized,
                };
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = initialized;
                }
            }
        }
    }

    fn merge_match_initialization(
        &mut self,
        entry_scopes: &[Scope],
        branches: &[(ScopeState, bool)],
    ) {
        self.scopes = entry_scopes.to_vec();
        let continuing = branches
            .iter()
            .filter(|(_, never)| !never)
            .map(|(scopes, _)| scopes)
            .collect::<Vec<_>>();
        if continuing.is_empty() {
            return;
        }

        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for name in entry_scope.keys() {
                let initialized = continuing.iter().all(|branch_scopes| {
                    branch_scopes
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|symbol| symbol.initialized)
                });
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = initialized;
                }
            }
        }
    }

    fn check_unary(
        &mut self,
        operator: UnaryOperator,
        operand: &hir::Expression,
        span: Span,
    ) -> Type {
        if operand.ty.is_never() {
            return Type::Never;
        }
        let expected = match operator {
            UnaryOperator::Negate => Type::Int,
            UnaryOperator::Not => Type::Bool,
        };
        self.require_type(&operand.ty, &expected, span, "unary operator operand");
        if operand.ty.is_error() {
            Type::Error
        } else if types_compatible(&operand.ty, &expected) {
            expected
        } else {
            Type::Error
        }
    }

    fn check_binary(
        &mut self,
        operator: BinaryOperator,
        left: &hir::Expression,
        right: &hir::Expression,
        span: Span,
    ) -> Type {
        match operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder => {
                self.require_binary_operands(left, right, &Type::Int, span, "arithmetic operator");
                binary_result_type(left, right, Type::Int)
            }
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                self.require_binary_operands(left, right, &Type::Int, span, "comparison operator");
                binary_result_type(left, right, Type::Bool)
            }
            BinaryOperator::And | BinaryOperator::Or => {
                self.check_short_circuit_binary(operator, left, right, span)
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                self.check_equality(left, right, span)
            }
        }
    }

    fn check_short_circuit_binary(
        &mut self,
        operator: BinaryOperator,
        left: &hir::Expression,
        right: &hir::Expression,
        span: Span,
    ) -> Type {
        self.require_binary_operands(left, right, &Type::Bool, span, "boolean operator");
        if left.ty.is_never() {
            return Type::Never;
        }
        if left.ty.is_error() || right.ty.is_error() {
            return Type::Error;
        }
        let left_literal = match &left.kind {
            ExpressionKind::Boolean(value) => Some(*value),
            _ => None,
        };
        let right_is_required = matches!(
            (operator, left_literal),
            (BinaryOperator::And, Some(true)) | (BinaryOperator::Or, Some(false))
        );
        if right_is_required && right.ty.is_never() {
            Type::Never
        } else {
            Type::Bool
        }
    }

    fn require_binary_operands(
        &mut self,
        left: &hir::Expression,
        right: &hir::Expression,
        expected: &Type,
        span: Span,
        context: &str,
    ) {
        if left.ty.is_error() || right.ty.is_error() {
            return;
        }
        if !types_compatible(&left.ty, expected) || !types_compatible(&right.ty, expected) {
            self.diagnostics
                .push(Diagnostic::error("N3004", "type mismatch").with_primary(
                    span,
                    format!(
                        "{context} requires {expected} operands, found {} and {}",
                        left.ty, right.ty
                    ),
                ));
        }
    }

    fn check_equality(
        &mut self,
        left: &hir::Expression,
        right: &hir::Expression,
        span: Span,
    ) -> Type {
        if left.ty.is_error() || right.ty.is_error() {
            return Type::Error;
        }
        if left.ty.is_never() || right.ty.is_never() {
            let other = if left.ty.is_never() {
                &right.ty
            } else {
                &left.ty
            };
            if other.is_never() || matches!(other, Type::Int | Type::Bool) {
                return Type::Never;
            }
            self.diagnostics
                .push(Diagnostic::error("N3004", "type mismatch").with_primary(
                    span,
                    format!(
                        "equality requires Int or Bool operands, found {} and {}",
                        left.ty, right.ty
                    ),
                ));
            return Type::Error;
        }

        let primitive = matches!(left.ty, Type::Int | Type::Bool);
        if primitive && left.ty == right.ty {
            Type::Bool
        } else {
            self.diagnostics
                .push(Diagnostic::error("N3004", "type mismatch").with_primary(
                    span,
                    format!(
                        "equality requires matching Int or Bool operands, found {} and {}",
                        left.ty, right.ty
                    ),
                ));
            Type::Error
        }
    }

    fn check_call(
        &mut self,
        callee: &hir::Expression,
        arguments: &[hir::Expression],
        span: Span,
    ) -> Type {
        if callee.ty.is_never() {
            return Type::Never;
        }
        let Type::Function(signature) = callee.ty.clone() else {
            if callee.ty.is_error() {
                return Type::Error;
            }
            self.diagnostics.push(
                Diagnostic::error("N3005", "expression is not callable").with_primary(
                    callee.span,
                    format!("found {} instead of a function", callee.ty),
                ),
            );
            return Type::Error;
        };

        if arguments.len() != signature.parameters.len() {
            self.diagnostics.push(
                Diagnostic::error("N3006", "wrong number of arguments").with_primary(
                    span,
                    format!(
                        "expected {} argument(s), found {}",
                        signature.parameters.len(),
                        arguments.len()
                    ),
                ),
            );
        }
        for (index, (argument, expected)) in arguments
            .iter()
            .zip(signature.parameters.iter())
            .enumerate()
        {
            self.require_type(
                &argument.ty,
                expected,
                argument.span,
                &format!("argument {}", index + 1),
            );
        }
        if arguments.iter().any(|argument| argument.ty.is_never()) {
            Type::Never
        } else {
            *signature.return_type
        }
    }

    fn join_branch_types(
        &mut self,
        then_type: &Type,
        then_span: Span,
        else_type: &Type,
        else_span: Span,
    ) -> Type {
        if then_type.is_never() {
            return else_type.clone();
        }
        if else_type.is_never() {
            return then_type.clone();
        }
        if then_type.is_error() {
            return else_type.clone();
        }
        if else_type.is_error() {
            return then_type.clone();
        }
        if then_type == else_type {
            return then_type.clone();
        }

        self.diagnostics.push(
            Diagnostic::error("N3004", "type mismatch")
                .with_primary(
                    else_span,
                    format!("else branch has type {else_type}, expected {then_type}"),
                )
                .with_secondary(then_span, format!("then branch has type {then_type}")),
        );
        Type::Error
    }

    fn join_match_arm_types(&mut self, arms: &[(Type, Span)]) -> Type {
        let mut expected = None::<(Type, Span)>;
        let mut saw_error = false;
        let mut mismatch = false;

        for (ty, span) in arms {
            if ty.is_never() {
                continue;
            }
            if ty.is_error() {
                saw_error = true;
                continue;
            }
            if let Some((expected_type, expected_span)) = &expected {
                if ty != expected_type {
                    self.diagnostics.push(
                        Diagnostic::error("N3004", "type mismatch")
                            .with_primary(
                                *span,
                                format!("match arm has type {ty}, expected {expected_type}"),
                            )
                            .with_secondary(
                                *expected_span,
                                format!("first continuing arm has type {expected_type}"),
                            ),
                    );
                    mismatch = true;
                }
            } else {
                expected = Some((ty.clone(), *span));
            }
        }

        if mismatch || (expected.is_none() && saw_error) {
            Type::Error
        } else if let Some((ty, _)) = expected {
            ty
        } else {
            Type::Never
        }
    }

    fn require_type(&mut self, actual: &Type, expected: &Type, span: Span, context: &str) {
        if types_compatible(actual, expected) {
            return;
        }
        self.diagnostics
            .push(Diagnostic::error("N3004", "type mismatch").with_primary(
                span,
                format!("{context}: expected {expected}, found {actual}"),
            ));
    }

    fn new_binding(&mut self, name: &ast::Name, ty: Type, mutable: bool) -> hir::Binding {
        let id = BindingId::new(self.next_binding);
        self.next_binding += 1;
        hir::Binding {
            id,
            name: name.text.clone(),
            ty,
            mutable,
            span: name.span,
        }
    }

    fn insert_local(&mut self, binding: &hir::Binding, initialized: bool) {
        let scope = self
            .scopes
            .last_mut()
            .expect("semantic analysis must always have a lexical scope");
        if let Some(previous) = scope.get(&binding.name) {
            self.diagnostics.push(
                Diagnostic::error("N3002", "duplicate definition")
                    .with_primary(
                        binding.span,
                        format!("`{}` is already defined in this scope", binding.name),
                    )
                    .with_secondary(previous.span, "first definition is here"),
            );
            return;
        }
        scope.insert(
            binding.name.clone(),
            LocalSymbol {
                id: binding.id,
                ty: binding.ty.clone(),
                mutable: binding.mutable,
                initialized,
                span: binding.span,
            },
        );
    }
}

fn types_compatible(actual: &Type, expected: &Type) -> bool {
    actual.is_error() || expected.is_error() || actual.is_never() || actual == expected
}

fn binary_result_type(left: &hir::Expression, right: &hir::Expression, success: Type) -> Type {
    if left.ty.is_never() || right.ty.is_never() {
        Type::Never
    } else if left.ty.is_error() || right.ty.is_error() {
        Type::Error
    } else {
        success
    }
}

#[cfg(test)]
mod tests {
    use super::{AnalysisOutput, analyze};
    use crate::hir::{ExpressionKind, StatementKind, Type};
    use nova_lexer::lex;
    use nova_parser::parse;
    use nova_source::{SourceFile, SourceId};

    fn analyze_text(text: &str) -> AnalysisOutput {
        let source = SourceFile::new(SourceId::new(0), "test.nv", text);
        let lexed = lex(&source);
        assert!(
            lexed.is_success(),
            "lex diagnostics: {:?}",
            lexed.diagnostics
        );
        let parsed = parse(&source, &lexed.tokens);
        assert!(
            parsed.is_success(),
            "parse diagnostics: {:?}",
            parsed.diagnostics
        );
        analyze(&parsed.program)
    }

    fn codes(output: &AnalysisOutput) -> Vec<&str> {
        output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect()
    }

    #[test]
    fn resolves_records_construction_projection_and_signatures() {
        let output = analyze_text(
            "record Pair { left: Int, right: Bool }\n\
             fn project(pair: Pair) -> Int { pair.left }\n\
             fn make() -> Pair { new Pair { right: true, left: 7 } }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);
        assert_eq!(output.program.records.len(), 1);
        assert_eq!(output.program.records[0].fields.len(), 2);
        assert!(matches!(
            output.program.functions[0].parameters[0].ty,
            Type::Record(_)
        ));

        let tail = output.program.functions[1]
            .body
            .tail
            .as_deref()
            .expect("tail");
        let ExpressionKind::RecordLiteral { record, fields } = &tail.kind else {
            panic!("expected record literal: {tail:?}");
        };
        assert_eq!(record.index(), 0);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].field_index, 1);
        assert_eq!(fields[1].field_index, 0);
    }

    #[test]
    fn rejects_invalid_record_shapes_and_fields() {
        let output = analyze_text(
            "record Pair { left: Int, right: Bool }\n\
             fn f() -> Int {\n\
                 let a = new Pair { left: 1, left: 2, extra: 3 };\n\
                 a.missing\n\
             }",
        );
        assert!(codes(&output).contains(&"N3010"));
        assert!(codes(&output).contains(&"N3011"));
        assert!(codes(&output).contains(&"N3012"));
    }

    #[test]
    fn rejects_duplicate_record_declarations_fields_and_builtin_names() {
        let output = analyze_text(
            "record Pair { x: Int, x: Int }\n\
             record Pair { y: Int }\n\
             record Int { value: Int }\n\
             fn f() -> Int { 0 }",
        );
        assert_eq!(codes(&output), vec!["N3010", "N3002", "N3002"]);
    }

    #[test]
    fn checks_record_field_initializer_types_and_nominal_identity() {
        let output = analyze_text(
            "record A { value: Int }\n\
             record B { value: Int }\n\
             fn f() -> A { new A { value: true } }\n\
             fn g(flag: Bool) -> A { if flag { new A { value: 1 } } else { new B { value: 1 } } }",
        );
        assert_eq!(codes(&output), vec!["N3004", "N3004"]);
    }

    #[test]
    fn rejects_projection_from_non_record() {
        let output = analyze_text("fn f() -> Int { 1.value }");
        assert_eq!(codes(&output), vec!["N3004"]);
    }

    #[test]
    fn resolves_locals_forward_calls_and_recursion() {
        let output = analyze_text(
            "fn first(x: Int) -> Int { second(x) }\n\
             fn second(x: Int) -> Int { if x == 0 { 0 } else { first(x - 1) } }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);

        let first = &output.program.functions[0];
        assert_eq!(first.return_type, Type::Int);
        let tail = first.body.tail.as_deref().expect("function has a tail");
        let ExpressionKind::Call { callee, arguments } = &tail.kind else {
            panic!("expected call HIR, got {tail:?}");
        };
        assert!(matches!(callee.kind, ExpressionKind::Function(_)));
        assert!(matches!(arguments[0].kind, ExpressionKind::Binding(_)));
    }

    #[test]
    fn infers_bindings_and_allows_nested_shadowing() {
        let output =
            analyze_text("fn f(x: Int) -> Int { let y = x + 1; { let y: Bool = true; y; }; y }");
        assert!(output.is_success(), "{:?}", output.diagnostics);

        let function = &output.program.functions[0];
        let StatementKind::Binding { binding, .. } = &function.body.statements[0].kind else {
            panic!("expected binding statement");
        };
        assert_eq!(binding.ty, Type::Int);
    }

    #[test]
    fn resolves_and_checks_mutable_assignments() {
        let output = analyze_text("fn f() -> Int { var value = 1; value = value + 1; value }");
        assert!(output.is_success(), "{:?}", output.diagnostics);

        let function = &output.program.functions[0];
        let StatementKind::Binding { binding, .. } = &function.body.statements[0].kind else {
            panic!("expected binding statement");
        };
        let StatementKind::Assignment { target, .. } = &function.body.statements[1].kind else {
            panic!("expected assignment statement");
        };
        assert_eq!(*target, Some(binding.id));
    }

    #[test]
    fn permits_assignment_before_first_read() {
        let output = analyze_text("fn f() -> Int { var value: Int; value = 3; value }");
        assert!(output.is_success(), "{:?}", output.diagnostics);
        assert!(matches!(
            &output.program.functions[0].body.statements[0].kind,
            StatementKind::UninitializedBinding(_)
        ));
    }

    #[test]
    fn rejects_read_before_definite_assignment() {
        let output = analyze_text("fn f() -> Int { var value: Int; value }");
        assert_eq!(codes(&output), vec!["N3009"]);
    }

    #[test]
    fn merges_definite_assignment_across_if_branches() {
        let output = analyze_text(
            "fn f(flag: Bool) -> Int {\n\
                 var value: Int;\n\
                 if flag { value = 1; 0 } else { value = 2; 0 };\n\
                 value\n\
             }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);

        let output = analyze_text(
            "fn f(flag: Bool) -> Int {\n\
                 var value: Int;\n\
                 if flag { value = 1; 0 } else { 0 };\n\
                 value\n\
             }",
        );
        assert_eq!(codes(&output), vec!["N3009"]);
    }

    #[test]
    fn ignores_noncontinuing_branch_when_merging_initialization() {
        let output = analyze_text(
            "fn f(flag: Bool) -> Int {\n\
                 var value: Int;\n\
                 if flag { return 1; } else { value = 2; 0 };\n\
                 value\n\
             }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);
    }

    #[test]
    fn checks_while_condition_and_mutation() {
        let output = analyze_text(
            "fn f() -> Int { var value = 0; while value < 3 { value = value + 1; } value }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);
        assert!(matches!(
            &output.program.functions[0].body.statements[1].kind,
            StatementKind::While { .. }
        ));

        let output = analyze_text("fn f() -> Int { while 1 {} 0 }");
        assert_eq!(codes(&output), vec!["N3004"]);
    }

    #[test]
    fn checks_structured_break_continue_and_continuing_branch_facts() {
        let output = analyze_text(
            "fn f(flag: Bool) -> Int {\n\
                 while flag {\n\
                     var value: Int;\n\
                     if flag { continue; } else { value = 1; 0 };\n\
                     value;\n\
                     break;\n\
                 }\n\
                 0\n\
             }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);
        let StatementKind::While { body, .. } =
            &output.program.functions[0].body.statements[0].kind
        else {
            panic!("expected while HIR");
        };
        assert!(matches!(
            body.statements.last().map(|statement| &statement.kind),
            Some(StatementKind::Break)
        ));
    }

    #[test]
    fn rejects_loop_control_without_enclosing_while_body() {
        let output = analyze_text("fn f() -> Int { break; continue; 0 }");
        assert_eq!(codes(&output), vec!["N3013", "N3013"]);
    }

    #[test]
    fn rejects_loop_control_nested_in_while_condition() {
        let output = analyze_text("fn f() -> Int { while { break; true } {} 0 }");
        assert_eq!(codes(&output), vec!["N3013"]);
    }

    #[test]
    fn loop_body_initialization_does_not_escape_zero_iteration_path() {
        let output = analyze_text(
            "fn f(flag: Bool) -> Int { var value: Int; while flag { value = 1; } value }",
        );
        assert_eq!(codes(&output), vec!["N3009"]);
    }

    #[test]
    fn loop_condition_initialization_survives_the_pretest() {
        let output =
            analyze_text("fn f() -> Int { var value: Int; while { value = 1; false } {} value }");
        assert!(output.is_success(), "{:?}", output.diagnostics);
    }

    #[test]
    fn guaranteed_true_loop_merges_reachable_break_exit_states() {
        let output = analyze_text(
            "fn f() -> Int { var value: Int; while true { value = 42; break; } value }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);
    }

    #[test]
    fn guaranteed_true_loop_requires_initialization_on_every_break_exit() {
        let output = analyze_text(
            "fn f(flag: Bool) -> Int {\n\
                 var value: Int;\n\
                 while true {\n\
                     if flag { value = 1; break; } else { break; }\n\
                 }\n\
                 value\n\
             }",
        );
        assert_eq!(codes(&output), vec!["N3009"]);
    }

    #[test]
    fn guaranteed_true_loop_without_reachable_break_is_noncontinuing() {
        for text in [
            "fn f() -> Int { while true {} }",
            "fn f() -> Int { while true { continue; break; } }",
            "fn f() -> Int { while true { while true { break; } } }",
        ] {
            let output = analyze_text(text);
            assert!(output.is_success(), "{text}: {:?}", output.diagnostics);
            assert!(output.program.functions[0].body.ty.is_never(), "{text}");
        }
    }

    #[test]
    fn unreachable_expression_suffixes_cannot_create_loop_exits() {
        for text in [
            "fn f() -> Int { while true { { return 1; } + { break; 2 }; } }",
            "fn sink(a: Int, b: Int) -> Int { 0 } fn f() -> Int { while true { sink({ return 1; }, { break; 2 }); } }",
            "fn f() -> Int { while true { { return 1; }({ break; 2 }); } }",
            "record Pair { left: Int, right: Int } fn f() -> Int { while true { new Pair { left: { return 1; }, right: { break; 2 } }; } }",
        ] {
            let output = analyze_text(text);
            assert!(output.is_success(), "{text}: {:?}", output.diagnostics);
            let function = output
                .program
                .functions
                .iter()
                .find(|function| function.name == "f")
                .expect("test function");
            assert!(function.body.ty.is_never(), "{text}");
        }
    }

    #[test]
    fn unreachable_statements_after_loop_control_do_not_change_dataflow_facts() {
        let output = analyze_text(
            "fn f(flag: Bool) -> Int {\n\
                 while flag {\n\
                     var value: Int;\n\
                     if flag { continue; value = 1; } else { 0 };\n\
                     value;\n\
                 }\n\
                 0\n\
             }",
        );
        assert_eq!(codes(&output), vec!["N3009"]);
    }

    #[test]
    fn short_circuit_literals_control_definite_initialization() {
        for text in [
            "fn f() -> Int { var value: Int; true && { value = 1; true }; value }",
            "fn f() -> Int { var value: Int; false || { value = 1; true }; value }",
        ] {
            let output = analyze_text(text);
            assert!(output.is_success(), "{text}: {:?}", output.diagnostics);
        }

        for text in [
            "fn f() -> Int { var value: Int; false && { value = 1; true }; value }",
            "fn f() -> Int { var value: Int; true || { value = 1; false }; value }",
        ] {
            let output = analyze_text(text);
            assert_eq!(codes(&output), vec!["N3009"], "{text}");
        }
    }

    #[test]
    fn dynamic_short_circuit_rhs_is_only_conditionally_executed() {
        let output = analyze_text(
            "fn f(flag: Bool) -> Int { var value: Int; flag && { value = 1; true }; value }",
        );
        assert_eq!(codes(&output), vec!["N3009"]);

        let output = analyze_text("fn f(flag: Bool) -> Int { flag && { return 1; }; 2 }");
        assert!(output.is_success(), "{:?}", output.diagnostics);
        assert_eq!(output.program.functions[0].body.ty, Type::Int);
    }

    #[test]
    fn short_circuit_loop_breaks_follow_runtime_reachability() {
        let skipped = analyze_text("fn f() -> Int { while true { false && { break; true }; } }");
        assert!(skipped.is_success(), "{:?}", skipped.diagnostics);
        assert!(skipped.program.functions[0].body.ty.is_never());

        let dynamic =
            analyze_text("fn f(flag: Bool) -> Int { while true { flag && { break; true }; } }");
        assert_eq!(codes(&dynamic), vec!["N3007"]);

        let forced = analyze_text("fn f() -> Int { while true { true && { break; true }; } }");
        assert_eq!(codes(&forced), vec!["N3007"]);
    }

    #[test]
    fn skipped_short_circuit_rhs_still_reports_static_type_errors() {
        let output = analyze_text("fn f() -> Bool { false && 1 }");
        assert_eq!(codes(&output), vec!["N3004"]);
    }

    #[test]
    fn rejects_immutable_and_mistyped_assignments() {
        let output = analyze_text(
            "fn f(parameter: Int) -> Int {\n\
                 let fixed = 1; fixed = 2;\n\
                 parameter = 3;\n\
                 var count = 0; count = true;\n\
                 count\n\
             }",
        );
        assert_eq!(codes(&output), vec!["N3008", "N3008", "N3004"]);
    }

    #[test]
    fn rejects_unknown_and_function_assignment_targets() {
        let output = analyze_text("fn f() -> Int { missing = 1; f = 2; 0 }");
        assert_eq!(codes(&output), vec!["N3003", "N3008"]);
    }

    #[test]
    fn rejects_unknown_types_names_and_same_scope_duplicates() {
        let output =
            analyze_text("fn f(x: Number, x: Int) -> Int { let y = missing; let y = 2; y }");
        assert_eq!(codes(&output), vec!["N3001", "N3002", "N3003", "N3002"]);
    }

    #[test]
    fn checks_annotations_operators_and_if_branches() {
        let output = analyze_text(
            "fn f(flag: Bool) -> Int {\n\
                 let a: Bool = 1;\n\
                 let b = flag + 1;\n\
                 if flag { 1 } else { false }\n\
             }",
        );
        assert_eq!(codes(&output), vec!["N3004", "N3004", "N3004"]);
    }

    #[test]
    fn checks_calls_and_non_callable_values() {
        let output = analyze_text(
            "fn add(x: Int, y: Int) -> Int { x + y }\n\
             fn f() -> Int { let x = 1; add(true); x(2) }",
        );
        assert_eq!(codes(&output), vec!["N3006", "N3004", "N3005"]);
    }

    #[test]
    fn checks_explicit_and_implicit_returns() {
        let output = analyze_text(
            "fn bad() -> Bool { return 1; }\n\
             fn missing() -> Int { let x = 1; }\n\
             fn good(flag: Bool) -> Int { if flag { return 1; } else { return 2; } }",
        );
        assert_eq!(codes(&output), vec!["N3004", "N3007"]);
    }

    #[test]
    fn equality_accepts_only_matching_primitive_types() {
        let output = analyze_text(
            "fn f() -> Bool { 1 == 1 }\n\
             fn g() -> Bool { true != false }\n\
             fn h() -> Bool { f == g }",
        );
        assert_eq!(codes(&output), vec!["N3004"]);
    }

    #[test]
    fn never_does_not_hide_static_operator_or_callee_errors() {
        let output = analyze_text(
            "fn bad_op(flag: Bool) -> Int {\n\
                 flag + if true { return 1; } else { return 2; }\n\
             }\n\
             fn bad_call() -> Int {\n\
                 1(if true { return 1; } else { return 2; })\n\
             }",
        );
        assert_eq!(codes(&output), vec!["N3004", "N3005"]);
    }

    #[test]
    fn resolves_nominal_enums_constructors_payload_bindings_and_recursive_types() {
        let output = analyze_text(
            "enum Nat { Zero, Succ(Nat) }\n\
             fn value(number: Nat) -> Int {\n\
                 match number { Nat::Zero => 0, Nat::Succ(previous) => value(previous), }\n\
             }\n\
             fn one() -> Nat { Nat::Succ(Nat::Zero) }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);
        assert_eq!(output.program.enums.len(), 1);
        assert!(matches!(
            output.program.enums[0].variants[1].payload,
            Some(Type::Enum(_))
        ));
        assert!(matches!(
            output.program.functions[0].parameters[0].ty,
            Type::Enum(_)
        ));

        let tail = output.program.functions[0]
            .body
            .tail
            .as_deref()
            .expect("match tail");
        let ExpressionKind::Match {
            enumeration, arms, ..
        } = &tail.kind
        else {
            panic!("expected match HIR: {tail:?}");
        };
        assert_eq!(enumeration.index(), 0);
        assert_eq!(arms.len(), 2);
        assert!(arms[0].binding.is_none());
        assert_eq!(
            arms[1]
                .binding
                .as_ref()
                .map(|binding| binding.name.as_str()),
            Some("previous")
        );
    }

    #[test]
    fn rejects_duplicate_enum_variants_and_cross_kind_type_definitions() {
        let output = analyze_text(
            "enum Choice { A, A }\n\
             record Clash { value: Int }\n\
             enum Clash { Empty }\n\
             enum Bool { False }\n\
             fn main() -> Int { 0 }",
        );
        assert!(codes(&output).contains(&"N3020"));
        assert_eq!(
            codes(&output)
                .into_iter()
                .filter(|code| *code == "N3002")
                .count(),
            2
        );
    }

    #[test]
    fn checks_enum_constructor_variant_payload_arity_and_type() {
        let output = analyze_text(
            "enum Maybe { None, Some(Int) }\n\
             fn extra() -> Maybe { Maybe::None(1) }\n\
             fn missing() -> Maybe { Maybe::Some }\n\
             fn wrong() -> Maybe { Maybe::Some(true) }\n\
             fn unknown() -> Maybe { Maybe::Absent }",
        );
        assert_eq!(codes(&output), vec!["N3022", "N3022", "N3004", "N3021"]);
    }

    #[test]
    fn checks_match_exhaustiveness_duplicates_nominal_identity_and_arm_types() {
        let non_exhaustive = analyze_text(
            "enum Maybe { None, Some(Int) }\n\
             fn f(value: Maybe) -> Int { match value { Maybe::None => 0, } }",
        );
        assert_eq!(codes(&non_exhaustive), vec!["N3023"]);

        let duplicate = analyze_text(
            "enum Maybe { None, Some(Int) }\n\
             fn f(value: Maybe) -> Int {\n\
                 match value { Maybe::None => 0, Maybe::Some(x) => x, Maybe::None => 2, }\n\
             }",
        );
        assert_eq!(codes(&duplicate), vec!["N3024"]);

        let wrong_enum = analyze_text(
            "enum A { X } enum B { X }\n\
             fn f(value: A) -> Int { match value { B::X => 0, } }",
        );
        assert!(codes(&wrong_enum).contains(&"N3025"));
        assert!(codes(&wrong_enum).contains(&"N3023"));

        let wrong_type = analyze_text(
            "enum Flag { Off, On }\n\
             fn f(value: Flag) -> Int { match value { Flag::Off => 0, Flag::On => true, } }",
        );
        assert_eq!(codes(&wrong_type), vec!["N3004"]);
    }

    #[test]
    fn direct_enum_constructor_selects_only_one_match_arm_for_dataflow() {
        let initialized = analyze_text(
            "enum Choice { A, B }\n\
             fn f() -> Int {\n\
                 var value: Int;\n\
                 match Choice::A { Choice::A => { value = 1; 0 }, Choice::B => 0, };\n\
                 value\n\
             }",
        );
        assert!(initialized.is_success(), "{:?}", initialized.diagnostics);

        let uninitialized = analyze_text(
            "enum Choice { A, B }\n\
             fn f() -> Int {\n\
                 var value: Int;\n\
                 match Choice::A { Choice::A => 0, Choice::B => { value = 1; 0 }, };\n\
                 value\n\
             }",
        );
        assert_eq!(codes(&uninitialized), vec!["N3009"]);
    }

    #[test]
    fn direct_enum_constructor_selected_arm_controls_noncontinuation() {
        let returned = analyze_text(
            "enum Choice { A, B }\n\
             fn f() -> Int { match Choice::A { Choice::A => { return 1; }, Choice::B => 0, } }",
        );
        assert!(returned.is_success(), "{:?}", returned.diagnostics);
        assert!(returned.program.functions[0].body.ty.is_never());

        let selected_continue = analyze_text(
            "enum Choice { A, B }\n\
             fn f() -> Int {\n\
                 while true {\n\
                     match Choice::A { Choice::A => { continue; }, Choice::B => { break; }, };\n\
                 }\n\
             }",
        );
        assert!(
            selected_continue.is_success(),
            "{:?}",
            selected_continue.diagnostics
        );
        assert!(selected_continue.program.functions[0].body.ty.is_never());

        let selected_break = analyze_text(
            "enum Choice { A, B }\n\
             fn f() -> Int {\n\
                 while true {\n\
                     match Choice::B { Choice::A => { continue; }, Choice::B => { break; }, };\n\
                 }\n\
             }",
        );
        assert_eq!(codes(&selected_break), vec!["N3007"]);
    }

    #[test]
    fn direct_enum_constructor_dead_arms_still_receive_static_checks() {
        let output = analyze_text(
            "enum Choice { A, B }\n\
             fn f() -> Int { match Choice::A { Choice::A => 0, Choice::B => true, } }",
        );
        assert_eq!(codes(&output), vec!["N3004"]);
    }

    #[test]
    fn direct_enum_constructor_payload_binding_can_establish_flow_facts() {
        let output = analyze_text(
            "enum Maybe { None, Some(Int) }\n\
             fn f() -> Int {\n\
                 var value: Int;\n\
                 match Maybe::Some(42) {\n\
                     Maybe::None => 0,\n\
                     Maybe::Some(inner) => { value = inner; 0 },\n\
                 };\n\
                 value\n\
             }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);
    }

    #[test]
    fn match_merges_definite_assignment_across_only_continuing_arms() {
        let complete = analyze_text(
            "enum Flag { Off, On }\n\
             fn f(flag: Flag) -> Int {\n\
                 var value: Int;\n\
                 match flag {\n\
                     Flag::Off => { value = 1; 0 },\n\
                     Flag::On => { value = 2; 0 },\n\
                 };\n\
                 value\n\
             }",
        );
        assert!(complete.is_success(), "{:?}", complete.diagnostics);

        let continuing_only = analyze_text(
            "enum Flag { Off, On }\n\
             fn f(flag: Flag) -> Int {\n\
                 var value: Int;\n\
                 match flag {\n\
                     Flag::Off => { return 0; },\n\
                     Flag::On => { value = 2; 0 },\n\
                 };\n\
                 value\n\
             }",
        );
        assert!(
            continuing_only.is_success(),
            "{:?}",
            continuing_only.diagnostics
        );

        let incomplete = analyze_text(
            "enum Flag { Off, On }\n\
             fn f(flag: Flag) -> Int {\n\
                 var value: Int;\n\
                 match flag { Flag::Off => { value = 1; 0 }, Flag::On => 0, };\n\
                 value\n\
             }",
        );
        assert_eq!(codes(&incomplete), vec!["N3009"]);
    }

    #[test]
    fn match_loop_control_excludes_noncontinuing_arms_from_dataflow() {
        let output = analyze_text(
            "enum Choice { Skip, Set(Int) }\n\
             fn f(choice: Choice) -> Int {\n\
                 while true {\n\
                     var value: Int;\n\
                     match choice {\n\
                         Choice::Skip => { continue; },\n\
                         Choice::Set(inner) => { value = inner; 0 },\n\
                     };\n\
                     value;\n\
                     break;\n\
                 }\n\
                 0\n\
             }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);
    }

    #[test]
    fn qualified_patterns_type_a_match_with_a_noncontinuing_scrutinee() {
        let output = analyze_text(
            "enum Choice { A, B }\n\
             fn f() -> Int {\n\
                 match { return 5; } { Choice::A => 1, Choice::B => 2, }\n\
             }",
        );
        assert!(output.is_success(), "{:?}", output.diagnostics);
        let tail = output.program.functions[0]
            .body
            .tail
            .as_deref()
            .expect("match tail");
        assert!(tail.ty.is_never());
        assert!(matches!(tail.kind, ExpressionKind::Match { .. }));
    }
}
