use crate::hir::{
    self, BindingId, ExpressionKind, FunctionId, FunctionType, RecordFieldValue, RecordId,
    RecordType, StatementKind, Type,
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
    analyzer.collect_record_definitions(program);
    analyzer.collect_function_signatures(program);

    let records = analyzer
        .record_definitions
        .iter()
        .map(RecordDefinition::to_hir)
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

struct Analyzer {
    diagnostics: Vec<Diagnostic>,
    record_definitions: Vec<RecordDefinition>,
    records: BTreeMap<String, RecordId>,
    signatures: Vec<SignatureRecord>,
    functions: BTreeMap<String, FunctionSymbol>,
    scopes: Vec<BTreeMap<String, LocalSymbol>>,
    next_binding: usize,
    loop_depth: usize,
}

impl Analyzer {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            record_definitions: Vec::new(),
            records: BTreeMap::new(),
            signatures: Vec::new(),
            functions: BTreeMap::new(),
            scopes: Vec::new(),
            next_binding: 0,
            loop_depth: 0,
        }
    }

    fn collect_record_definitions(&mut self, program: &ast::Program) {
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

        for (index, record) in program.records.iter().enumerate() {
            let id = RecordId::new(index);
            if matches!(record.name.text.as_str(), "Int" | "Bool") {
                self.diagnostics.push(
                    Diagnostic::error("N3002", "duplicate type definition").with_primary(
                        record.name.span,
                        format!("`{}` is a built-in type name", record.name.text),
                    ),
                );
                continue;
            }
            if let Some(previous_id) = self.records.get(&record.name.text).copied() {
                let previous = &self.record_definitions[previous_id.index()];
                self.diagnostics.push(
                    Diagnostic::error("N3002", "duplicate type definition")
                        .with_primary(
                            record.name.span,
                            format!("record `{}` is defined more than once", record.name.text),
                        )
                        .with_secondary(previous.span, "first record definition is here"),
                );
            } else {
                self.records.insert(record.name.text.clone(), id);
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
                if let Some(id) = self.records.get(unknown).copied() {
                    return Type::Record(RecordType {
                        id,
                        name: unknown.to_owned(),
                    });
                }
                self.diagnostics.push(
                    Diagnostic::error("N3001", "unknown type")
                        .with_primary(reference.span, format!("unknown type `{unknown}`"))
                        .with_note(
                            "the bootstrap semantic core recognizes Int, Bool, and declared record names",
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
        self.loop_depth = 0;

        let mut parameters = Vec::with_capacity(function.parameters.len());
        for (parameter, ty) in function.parameters.iter().zip(&signature.parameters) {
            let binding = self.new_binding(&parameter.name, ty.clone(), false);
            self.insert_local(&binding, true);
            parameters.push(binding);
        }

        let body = self.lower_block(&function.body, &signature.return_type, false);
        debug_assert_eq!(self.loop_depth, 0);
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
            let reachable_scopes = self.scopes.clone();
            let (statement, diverges) = self.lower_statement(statement, return_type);
            statements.push(statement);
            if terminated {
                self.scopes = reachable_scopes;
            } else if diverges {
                terminated = true;
            }
        }

        let tail = block.tail.as_deref().map(|expression| {
            let reachable_scopes = self.scopes.clone();
            let expression = Box::new(self.lower_expression(expression, return_type));
            if terminated {
                self.scopes = reachable_scopes;
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
                self.loop_depth += 1;
                let body = self.lower_block(body, return_type, true);
                self.loop_depth -= 1;
                self.scopes = post_condition_scopes;
                let diverges = condition.ty.is_never();
                (StatementKind::While { condition, body }, diverges)
            }
            ast::StatementKind::Break => {
                if self.loop_depth == 0 {
                    self.diagnostics.push(
                        Diagnostic::error("N3013", "loop control outside loop").with_primary(
                            statement.span,
                            "`break` requires a lexically enclosing `while` body",
                        ),
                    );
                }
                (StatementKind::Break, true)
            }
            ast::StatementKind::Continue => {
                if self.loop_depth == 0 {
                    self.diagnostics.push(
                        Diagnostic::error("N3013", "loop control outside loop").with_primary(
                            statement.span,
                            "`continue` requires a lexically enclosing `while` body",
                        ),
                    );
                }
                (StatementKind::Continue, true)
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
                let right = self.lower_expression(right, return_type);
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
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_expression(argument, return_type))
                    .collect::<Vec<_>>();
                let ty = self.check_call(&callee, &arguments, expression.span);
                (
                    ExpressionKind::Call {
                        callee: Box::new(callee),
                        arguments,
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

                let entry_scopes = self.scopes.clone();
                let then_branch = self.lower_block(then_branch, return_type, true);
                let then_scopes = self.scopes.clone();

                self.scopes = entry_scopes.clone();
                let else_branch = self.lower_expression(else_branch, return_type);
                let else_scopes = self.scopes.clone();

                let ty = if condition.ty.is_never() {
                    self.scopes = entry_scopes;
                    Type::Never
                } else {
                    self.merge_branch_initialization(
                        &entry_scopes,
                        &then_scopes,
                        then_branch.ty.is_never(),
                        &else_scopes,
                        else_branch.ty.is_never(),
                    );
                    self.join_branch_types(
                        &then_branch.ty,
                        then_branch.span,
                        &else_branch.ty,
                        else_branch.span,
                    )
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
        };

        hir::Expression {
            kind,
            ty,
            span: expression.span,
        }
    }

    fn lower_record_literal(
        &mut self,
        name: &ast::Name,
        fields: &[ast::RecordLiteralField],
        return_type: &Type,
        _span: Span,
    ) -> (ExpressionKind, Type) {
        let Some(record_id) = self.records.get(&name.text).copied() else {
            for field in fields {
                self.lower_expression(&field.value, return_type);
            }
            self.diagnostics.push(
                Diagnostic::error("N3001", "unknown type")
                    .with_primary(name.span, format!("unknown record type `{}`", name.text)),
            );
            return (ExpressionKind::Error, Type::Error);
        };
        let definition = self.record_definitions[record_id.index()].clone();
        let mut seen = BTreeMap::<String, Span>::new();
        let mut resolved = Vec::with_capacity(fields.len());
        let mut structural_error = false;
        let mut contains_error = false;
        let mut contains_never = false;

        for field in fields {
            let value = self.lower_expression(&field.value, return_type);
            contains_error |= value.ty.is_error();
            contains_never |= value.ty.is_never();

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
            self.require_type(&value.ty, expected, value.span, "record field initializer");
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
        } else if structural_error || contains_error {
            Type::Error
        } else {
            Type::Record(definition.record_type())
        };
        if structural_error {
            (ExpressionKind::Error, ty)
        } else {
            (
                ExpressionKind::RecordLiteral {
                    record: record_id,
                    fields: resolved,
                },
                ty,
            )
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

    fn merge_branch_initialization(
        &mut self,
        entry_scopes: &[BTreeMap<String, LocalSymbol>],
        then_scopes: &[BTreeMap<String, LocalSymbol>],
        then_never: bool,
        else_scopes: &[BTreeMap<String, LocalSymbol>],
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
                self.require_binary_operands(left, right, &Type::Bool, span, "boolean operator");
                binary_result_type(left, right, Type::Bool)
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                self.check_equality(left, right, span)
            }
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
        let StatementKind::While { body, .. } = &output.program.functions[0].body.statements[0].kind
        else {
            panic!("expected while HIR");
        };
        assert!(matches!(body.statements.last().map(|statement| &statement.kind), Some(StatementKind::Break)));
    }

    #[test]
    fn rejects_loop_control_without_enclosing_while_body() {
        let output = analyze_text("fn f() -> Int { break; continue; 0 }");
        assert_eq!(codes(&output), vec!["N3013", "N3013"]);
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
}
