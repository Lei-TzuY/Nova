//! Versioned, deterministic semantic introspection for accepted Nova programs.
//!
//! This crate deliberately projects typed HIR into a tooling-owned schema. The
//! JSON representation is therefore not a serialization of compiler internals.

pub mod v1;

use nova_parser::ast::{BinaryOperator, UnaryOperator};
use nova_sema::hir::{self, Type};
use nova_source::{SourceFile, Span};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

const SOURCE_ID: &str = "source:0";

/// A violated HIR or source invariant that prevents trustworthy inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectionError {
    message: String,
}

impl InspectionError {
    /// Returns the invariant failure without presentation decoration.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for InspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for InspectionError {}

/// Builds a schema-v1 document from semantically accepted typed HIR.
///
/// The function validates document-facing identities, spans, nominal slots,
/// binding order, and exhaustive-match structure. It returns an error instead
/// of publishing a partial or internally inconsistent document.
pub fn build_document(
    program: &hir::Program,
    source: &SourceFile,
) -> Result<v1::Document, InspectionError> {
    Builder::new(program, source).build()
}

/// Renders one accepted program as deterministic, pretty-printed schema-v1 JSON.
pub fn render_json(program: &hir::Program, source: &SourceFile) -> Result<String, InspectionError> {
    let document = build_document(program, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v1: {error}")))
}

struct Builder<'a> {
    program: &'a hir::Program,
    source: &'a SourceFile,
    types: Vec<Type>,
    bindings: Vec<v1::Binding>,
    blocks: Vec<Option<v1::Block>>,
    statements: Vec<Option<v1::Statement>>,
    expressions: Vec<Option<v1::Expression>>,
    matches: Vec<Option<v1::Match>>,
    active_scopes: Vec<String>,
    loop_depth: usize,
}

impl<'a> Builder<'a> {
    fn new(program: &'a hir::Program, source: &'a SourceFile) -> Self {
        Self {
            program,
            source,
            types: Vec::new(),
            bindings: Vec::new(),
            blocks: Vec::new(),
            statements: Vec::new(),
            expressions: Vec::new(),
            matches: Vec::new(),
            active_scopes: Vec::new(),
            loop_depth: 0,
        }
    }

    fn build(mut self) -> Result<v1::Document, InspectionError> {
        let program_span = self.span(self.program.span)?;
        if self.program.span.start() != 0 || self.program.span.end() != self.source.len() {
            return Err(InspectionError::invalid(format!(
                "program span must cover source bytes 0..{}, found {}..{}",
                self.source.len(),
                self.program.span.start(),
                self.program.span.end()
            )));
        }

        self.prepare_type_order()?;
        let records = self.collect_records()?;
        let enums = self.collect_enums()?;
        let functions = self.collect_functions()?;
        let types = self.type_facts()?;

        let blocks = take_complete("block", self.blocks)?;
        let statements = take_complete("statement", self.statements)?;
        let expressions = take_complete("expression", self.expressions)?;
        let matches = take_complete("match", self.matches)?;

        Ok(v1::Document {
            schema: v1::SCHEMA_NAME.to_owned(),
            schema_version: v1::SCHEMA_VERSION,
            producer: v1::Producer {
                name: "nova".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
            source: v1::Source {
                id: SOURCE_ID.to_owned(),
                name: self.source.name().to_owned(),
                byte_length: self.source.len(),
            },
            program: v1::Program {
                span: program_span,
                types,
                records,
                enums,
                functions,
                bindings: self.bindings,
                blocks,
                statements,
                expressions,
                matches,
            },
        })
    }

    fn prepare_type_order(&mut self) -> Result<(), InspectionError> {
        for ty in [Type::Int, Type::Bool, Type::Unit, Type::Never] {
            self.intern_type(&ty)?;
        }

        for index in 0..self.program.records.len() {
            let record = self.program.records[index].clone();
            self.require_record_id(record.id, index)?;
            self.span(record.span)?;
            self.intern_type(&Type::Record(hir::RecordType {
                id: record.id,
                name: record.name,
            }))?;
        }
        for index in 0..self.program.enums.len() {
            let enumeration = self.program.enums[index].clone();
            self.require_enum_id(enumeration.id, index)?;
            self.span(enumeration.span)?;
            self.intern_type(&Type::Enum(hir::EnumType {
                id: enumeration.id,
                name: enumeration.name,
            }))?;
        }
        for index in 0..self.program.functions.len() {
            let function = self.program.functions[index].clone();
            self.require_function_id(function.id, index)?;
            let signature = function_type(&function);
            self.intern_type(&signature)?;
        }
        Ok(())
    }

    fn collect_records(&mut self) -> Result<Vec<v1::Record>, InspectionError> {
        let mut facts = Vec::with_capacity(self.program.records.len());
        for index in 0..self.program.records.len() {
            let record = self.program.records[index].clone();
            self.require_record_id(record.id, index)?;
            let id = record_id(record.id.index());
            let type_id = self.intern_type(&Type::Record(hir::RecordType {
                id: record.id,
                name: record.name.clone(),
            }))?;
            let mut fields = Vec::with_capacity(record.fields.len());
            for (field_index, field) in record.fields.iter().enumerate() {
                fields.push(v1::RecordField {
                    id: field_id(record.id.index(), field_index),
                    name: field.name.clone(),
                    type_id: self.intern_type(&field.ty)?,
                    span: self.span(field.span)?,
                });
            }
            facts.push(v1::Record {
                id,
                name: record.name,
                type_id,
                span: self.span(record.span)?,
                fields,
            });
        }
        Ok(facts)
    }

    fn collect_enums(&mut self) -> Result<Vec<v1::Enum>, InspectionError> {
        let mut facts = Vec::with_capacity(self.program.enums.len());
        for index in 0..self.program.enums.len() {
            let enumeration = self.program.enums[index].clone();
            self.require_enum_id(enumeration.id, index)?;
            let id = enum_id(enumeration.id.index());
            let type_id = self.intern_type(&Type::Enum(hir::EnumType {
                id: enumeration.id,
                name: enumeration.name.clone(),
            }))?;
            let mut variants = Vec::with_capacity(enumeration.variants.len());
            for (variant_index, variant) in enumeration.variants.iter().enumerate() {
                variants.push(v1::EnumVariant {
                    id: variant_id(enumeration.id.index(), variant_index),
                    name: variant.name.clone(),
                    payload_type: variant
                        .payload
                        .as_ref()
                        .map(|ty| self.intern_type(ty))
                        .transpose()?,
                    span: self.span(variant.span)?,
                });
            }
            facts.push(v1::Enum {
                id,
                name: enumeration.name,
                type_id,
                span: self.span(enumeration.span)?,
                variants,
            });
        }
        Ok(facts)
    }

    fn collect_functions(&mut self) -> Result<Vec<v1::Function>, InspectionError> {
        let mut facts = Vec::with_capacity(self.program.functions.len());
        for index in 0..self.program.functions.len() {
            let function = self.program.functions[index].clone();
            self.require_function_id(function.id, index)?;
            let owner = function_id(function.id.index());
            let signature = function_type(&function);
            let type_id = self.intern_type(&signature)?;
            let return_type = self.intern_type(&function.return_type)?;
            let mut parameters = Vec::with_capacity(function.parameters.len());
            for parameter in &function.parameters {
                parameters.push(self.add_binding(
                    parameter,
                    v1::BindingRole::Parameter,
                    &owner,
                    &owner,
                )?);
            }
            let body = self.collect_block(&function.body, &owner)?;
            facts.push(v1::Function {
                id: owner,
                name: function.name,
                type_id,
                return_type,
                parameters,
                body,
                span: self.span(function.span)?,
            });
        }
        Ok(facts)
    }

    fn collect_block(
        &mut self,
        block: &hir::Block,
        owner: &str,
    ) -> Result<String, InspectionError> {
        let index = self.blocks.len();
        self.blocks.push(None);
        let id = block_id(index);
        let type_id = self.intern_type(&block.ty)?;
        let span = self.span(block.span)?;
        self.active_scopes.push(id.clone());
        let contents = (|| {
            let mut statements = Vec::with_capacity(block.statements.len());
            for statement in &block.statements {
                statements.push(self.collect_statement(statement, owner, &id)?);
            }
            let tail_expression = block
                .tail
                .as_deref()
                .map(|expression| self.collect_expression(expression, owner))
                .transpose()?;
            Ok::<_, InspectionError>((statements, tail_expression))
        })();
        self.active_scopes.pop();
        let (statements, tail_expression) = contents?;
        self.blocks[index] = Some(v1::Block {
            id: id.clone(),
            owner: owner.to_owned(),
            type_id,
            span,
            statements,
            tail_expression,
        });
        Ok(id)
    }

    fn collect_statement(
        &mut self,
        statement: &hir::Statement,
        owner: &str,
        block: &str,
    ) -> Result<String, InspectionError> {
        let index = self.statements.len();
        self.statements.push(None);
        let id = statement_id(index);
        let span = self.span(statement.span)?;
        let mut binding = None;
        let mut target = None;
        let mut expressions = Vec::new();
        let mut blocks = Vec::new();

        let kind = match &statement.kind {
            hir::StatementKind::Binding {
                binding: declared,
                initializer,
            } => {
                expressions.push(self.collect_expression(initializer, owner)?);
                binding = Some(self.add_binding(declared, v1::BindingRole::Local, owner, block)?);
                v1::StatementKind::InitializedBinding
            }
            hir::StatementKind::UninitializedBinding(declared) => {
                if !declared.mutable {
                    return Err(InspectionError::invalid(format!(
                        "uninitialized binding {} is not mutable",
                        binding_id(declared.id.index())
                    )));
                }
                binding = Some(self.add_binding(declared, v1::BindingRole::Local, owner, block)?);
                v1::StatementKind::UninitializedBinding
            }
            hir::StatementKind::Assignment {
                target: resolved,
                value,
            } => {
                expressions.push(self.collect_expression(value, owner)?);
                let resolved = resolved.ok_or_else(|| {
                    InspectionError::invalid(format!(
                        "accepted assignment {} has no resolved target",
                        statement_id(index)
                    ))
                })?;
                let binding = self.require_known_binding(resolved, owner)?;
                if !binding.mutable {
                    return Err(InspectionError::invalid(format!(
                        "assignment targets immutable {}",
                        binding_id(resolved.index())
                    )));
                }
                target = Some(binding_id(resolved.index()));
                v1::StatementKind::Assignment
            }
            hir::StatementKind::While { condition, body } => {
                expressions.push(self.collect_expression(condition, owner)?);
                self.loop_depth += 1;
                let body = self.collect_block(body, owner);
                self.loop_depth -= 1;
                blocks.push(body?);
                v1::StatementKind::While
            }
            hir::StatementKind::Break => {
                self.require_loop_control("break", index)?;
                v1::StatementKind::Break
            }
            hir::StatementKind::Continue => {
                self.require_loop_control("continue", index)?;
                v1::StatementKind::Continue
            }
            hir::StatementKind::Return(value) => {
                expressions.push(self.collect_expression(value, owner)?);
                v1::StatementKind::Return
            }
            hir::StatementKind::Expression(value) => {
                expressions.push(self.collect_expression(value, owner)?);
                v1::StatementKind::Expression
            }
        };

        self.statements[index] = Some(v1::Statement {
            id: id.clone(),
            owner: owner.to_owned(),
            block: block.to_owned(),
            kind,
            binding,
            target,
            expressions,
            blocks,
            span,
        });
        Ok(id)
    }

    fn collect_expression(
        &mut self,
        expression: &hir::Expression,
        owner: &str,
    ) -> Result<String, InspectionError> {
        let index = self.expressions.len();
        self.expressions.push(None);
        let id = expression_id(index);
        let type_id = self.intern_type(&expression.ty)?;
        let span = self.span(expression.span)?;
        let mut target = None;
        let mut operator = None;
        let mut children = Vec::new();
        let mut blocks = Vec::new();
        let mut field_initializers = Vec::new();

        let kind = match &expression.kind {
            hir::ExpressionKind::Integer(_) => v1::ExpressionKind::Integer,
            hir::ExpressionKind::Boolean(_) => v1::ExpressionKind::Boolean,
            hir::ExpressionKind::Unit => v1::ExpressionKind::Unit,
            hir::ExpressionKind::Binding(resolved) => {
                self.require_known_binding(*resolved, owner)?;
                target = Some(binding_id(resolved.index()));
                v1::ExpressionKind::BindingReference
            }
            hir::ExpressionKind::Function(resolved) => {
                self.require_function(*resolved)?;
                target = Some(function_id(resolved.index()));
                v1::ExpressionKind::FunctionReference
            }
            hir::ExpressionKind::RecordLiteral { record, fields } => {
                let declared_field_count = self.require_record(*record)?.fields.len();
                let mut seen = BTreeSet::new();
                for field in fields {
                    if field.field_index >= declared_field_count {
                        return Err(InspectionError::invalid(format!(
                            "record construction references out-of-range field slot {}",
                            field.field_index
                        )));
                    }
                    if !seen.insert(field.field_index) {
                        return Err(InspectionError::invalid(format!(
                            "record construction repeats field slot {}",
                            field.field_index
                        )));
                    }
                    let value = self.collect_expression(&field.value, owner)?;
                    children.push(value.clone());
                    field_initializers.push(v1::RecordFieldInitializer {
                        field: field_id(record.index(), field.field_index),
                        value,
                    });
                }
                if seen.len() != declared_field_count {
                    return Err(InspectionError::invalid(format!(
                        "record construction for {} does not cover every field slot",
                        record_id(record.index())
                    )));
                }
                target = Some(record_id(record.index()));
                v1::ExpressionKind::RecordConstruction
            }
            hir::ExpressionKind::EnumConstructor {
                enumeration,
                variant_index,
                payload,
            } => {
                let declaration = self.require_enum(*enumeration)?;
                let variant = declaration.variants.get(*variant_index).ok_or_else(|| {
                    InspectionError::invalid(format!(
                        "enum construction references out-of-range variant slot {variant_index}"
                    ))
                })?;
                if variant.payload.is_some() != payload.is_some() {
                    return Err(InspectionError::invalid(format!(
                        "enum construction payload does not match {}",
                        variant_id(enumeration.index(), *variant_index)
                    )));
                }
                if let Some(payload) = payload {
                    children.push(self.collect_expression(payload, owner)?);
                }
                target = Some(variant_id(enumeration.index(), *variant_index));
                v1::ExpressionKind::EnumConstruction
            }
            hir::ExpressionKind::FieldAccess {
                base,
                record,
                field_index,
            } => {
                let declaration = self.require_record(*record)?;
                if *field_index >= declaration.fields.len() {
                    return Err(InspectionError::invalid(format!(
                        "field access references out-of-range field slot {field_index}"
                    )));
                }
                children.push(self.collect_expression(base, owner)?);
                target = Some(field_id(record.index(), *field_index));
                v1::ExpressionKind::FieldAccess
            }
            hir::ExpressionKind::Unary {
                operator: resolved,
                operand,
            } => {
                operator = Some(unary_operator(*resolved).to_owned());
                children.push(self.collect_expression(operand, owner)?);
                v1::ExpressionKind::Unary
            }
            hir::ExpressionKind::Binary {
                operator: resolved,
                left,
                right,
            } => {
                operator = Some(binary_operator(*resolved).to_owned());
                children.push(self.collect_expression(left, owner)?);
                children.push(self.collect_expression(right, owner)?);
                v1::ExpressionKind::Binary
            }
            hir::ExpressionKind::Call { callee, arguments } => {
                children.push(self.collect_expression(callee, owner)?);
                for argument in arguments {
                    children.push(self.collect_expression(argument, owner)?);
                }
                v1::ExpressionKind::Call
            }
            hir::ExpressionKind::Block(block) => {
                blocks.push(self.collect_block(block, owner)?);
                target = blocks.first().cloned();
                v1::ExpressionKind::Block
            }
            hir::ExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                children.push(self.collect_expression(condition, owner)?);
                blocks.push(self.collect_block(then_branch, owner)?);
                children.push(self.collect_expression(else_branch, owner)?);
                v1::ExpressionKind::If
            }
            hir::ExpressionKind::Match {
                scrutinee,
                enumeration,
                arms,
            } => {
                let match_index = self.matches.len();
                self.matches.push(None);
                let match_identity = match_id(match_index);
                let declaration = self.require_enum(*enumeration)?.clone();
                let scrutinee_identity = self.collect_expression(scrutinee, owner)?;
                children.push(scrutinee_identity.clone());
                let mut seen = BTreeSet::new();
                let mut arm_facts = Vec::with_capacity(arms.len());

                for (arm_index, arm) in arms.iter().enumerate() {
                    let variant = declaration.variants.get(arm.variant_index).ok_or_else(|| {
                        InspectionError::invalid(format!(
                            "match references out-of-range variant slot {}",
                            arm.variant_index
                        ))
                    })?;
                    if !seen.insert(arm.variant_index) {
                        return Err(InspectionError::invalid(format!(
                            "match repeats variant slot {}",
                            arm.variant_index
                        )));
                    }
                    let arm_identity = match_arm_id(match_index, arm_index);
                    self.active_scopes.push(arm_identity.clone());
                    let arm_contents = (|| {
                        let binding = match (&variant.payload, &arm.binding) {
                            (Some(expected), Some(binding)) => {
                                if &binding.ty != expected {
                                    return Err(InspectionError::invalid(format!(
                                        "match payload binding type does not match {}",
                                        variant_id(enumeration.index(), arm.variant_index)
                                    )));
                                }
                                Some(self.add_binding(
                                    binding,
                                    v1::BindingRole::MatchPayload,
                                    owner,
                                    &arm_identity,
                                )?)
                            }
                            (None, None) => None,
                            _ => {
                                return Err(InspectionError::invalid(format!(
                                    "match payload binding arity does not match {}",
                                    variant_id(enumeration.index(), arm.variant_index)
                                )));
                            }
                        };
                        let value = self.collect_expression(&arm.value, owner)?;
                        Ok::<_, InspectionError>((binding, value))
                    })();
                    self.active_scopes.pop();
                    let (binding, value) = arm_contents?;
                    children.push(value.clone());
                    arm_facts.push(v1::MatchArm {
                        id: arm_identity,
                        variant: variant_id(enumeration.index(), arm.variant_index),
                        binding,
                        value,
                        result_type: self.intern_type(&arm.value.ty)?,
                        span: self.span(arm.span)?,
                    });
                }
                if seen.len() != declaration.variants.len() {
                    return Err(InspectionError::invalid(format!(
                        "match over {} is not exhaustive",
                        enum_id(enumeration.index())
                    )));
                }

                self.matches[match_index] = Some(v1::Match {
                    id: match_identity.clone(),
                    owner: owner.to_owned(),
                    expression: id.clone(),
                    enumeration: enum_id(enumeration.index()),
                    exhaustive: true,
                    scrutinee: scrutinee_identity,
                    scrutinee_type: self.intern_type(&scrutinee.ty)?,
                    arms: arm_facts,
                    span: span.clone(),
                });
                target = Some(match_identity);
                v1::ExpressionKind::Match
            }
            hir::ExpressionKind::Error => {
                return Err(InspectionError::invalid(format!(
                    "accepted program contains error expression at {}..{}",
                    expression.span.start(),
                    expression.span.end()
                )));
            }
        };

        self.expressions[index] = Some(v1::Expression {
            id: id.clone(),
            owner: owner.to_owned(),
            kind,
            type_id,
            target,
            operator,
            children,
            blocks,
            field_initializers,
            span,
        });
        Ok(id)
    }

    fn add_binding(
        &mut self,
        binding: &hir::Binding,
        role: v1::BindingRole,
        owner: &str,
        scope: &str,
    ) -> Result<String, InspectionError> {
        let expected = self.bindings.len();
        if binding.id.index() != expected {
            return Err(InspectionError::invalid(format!(
                "binding identities must be contiguous in semantic order: expected binding:{expected}, found binding:{}",
                binding.id.index()
            )));
        }
        if matches!(
            role,
            v1::BindingRole::Parameter | v1::BindingRole::MatchPayload
        ) && binding.mutable
        {
            return Err(InspectionError::invalid(format!(
                "{} cannot be mutable",
                binding_id(binding.id.index())
            )));
        }
        let id = binding_id(binding.id.index());
        let type_id = self.intern_type(&binding.ty)?;
        let span = self.span(binding.span)?;
        self.bindings.push(v1::Binding {
            id: id.clone(),
            name: binding.name.clone(),
            role,
            owner: owner.to_owned(),
            scope: scope.to_owned(),
            type_id,
            mutable: binding.mutable,
            span,
        });
        Ok(id)
    }

    fn require_known_binding(
        &self,
        id: hir::BindingId,
        owner: &str,
    ) -> Result<&v1::Binding, InspectionError> {
        let binding = self.bindings.get(id.index()).ok_or_else(|| {
            InspectionError::invalid(format!("reference to unknown {}", binding_id(id.index())))
        })?;
        if binding.owner != owner {
            return Err(InspectionError::invalid(format!(
                "reference to {} crosses function ownership",
                binding_id(id.index())
            )));
        }
        if binding.scope != owner
            && !self
                .active_scopes
                .iter()
                .any(|scope| scope == &binding.scope)
        {
            return Err(InspectionError::invalid(format!(
                "reference to {} is outside its lexical scope",
                binding_id(id.index())
            )));
        }
        Ok(binding)
    }

    fn require_loop_control(&self, keyword: &str, statement: usize) -> Result<(), InspectionError> {
        if self.loop_depth == 0 {
            Err(InspectionError::invalid(format!(
                "{keyword} in {} has no enclosing loop",
                statement_id(statement)
            )))
        } else {
            Ok(())
        }
    }

    fn require_record(&self, id: hir::RecordId) -> Result<&hir::Record, InspectionError> {
        self.program.records.get(id.index()).ok_or_else(|| {
            InspectionError::invalid(format!("reference to unknown {}", record_id(id.index())))
        })
    }

    fn require_enum(&self, id: hir::EnumId) -> Result<&hir::Enum, InspectionError> {
        self.program.enums.get(id.index()).ok_or_else(|| {
            InspectionError::invalid(format!("reference to unknown {}", enum_id(id.index())))
        })
    }

    fn require_function(&self, id: hir::FunctionId) -> Result<&hir::Function, InspectionError> {
        self.program.functions.get(id.index()).ok_or_else(|| {
            InspectionError::invalid(format!("reference to unknown {}", function_id(id.index())))
        })
    }

    fn require_record_id(
        &self,
        actual: hir::RecordId,
        expected: usize,
    ) -> Result<(), InspectionError> {
        if actual.index() == expected {
            Ok(())
        } else {
            Err(InspectionError::invalid(format!(
                "record identity at slot {expected} is record:{}",
                actual.index()
            )))
        }
    }

    fn require_enum_id(&self, actual: hir::EnumId, expected: usize) -> Result<(), InspectionError> {
        if actual.index() == expected {
            Ok(())
        } else {
            Err(InspectionError::invalid(format!(
                "enum identity at slot {expected} is enum:{}",
                actual.index()
            )))
        }
    }

    fn require_function_id(
        &self,
        actual: hir::FunctionId,
        expected: usize,
    ) -> Result<(), InspectionError> {
        if actual.index() == expected {
            Ok(())
        } else {
            Err(InspectionError::invalid(format!(
                "function identity at slot {expected} is function:{}",
                actual.index()
            )))
        }
    }

    fn intern_type(&mut self, ty: &Type) -> Result<String, InspectionError> {
        self.validate_type(ty)?;
        if let Type::Function(signature) = ty {
            for parameter in &signature.parameters {
                self.intern_type(parameter)?;
            }
            self.intern_type(&signature.return_type)?;
        }
        if let Some(index) = self.types.iter().position(|known| known == ty) {
            return Ok(type_id(index));
        }
        let index = self.types.len();
        self.types.push(ty.clone());
        Ok(type_id(index))
    }

    fn validate_type(&self, ty: &Type) -> Result<(), InspectionError> {
        match ty {
            Type::Record(record) => {
                let declaration = self.require_record(record.id)?;
                if declaration.name != record.name {
                    return Err(InspectionError::invalid(format!(
                        "{} carries nominal name `{}` instead of `{}`",
                        record_id(record.id.index()),
                        record.name,
                        declaration.name
                    )));
                }
            }
            Type::Enum(enumeration) => {
                let declaration = self.require_enum(enumeration.id)?;
                if declaration.name != enumeration.name {
                    return Err(InspectionError::invalid(format!(
                        "{} carries nominal name `{}` instead of `{}`",
                        enum_id(enumeration.id.index()),
                        enumeration.name,
                        declaration.name
                    )));
                }
            }
            Type::Error => {
                return Err(InspectionError::invalid(
                    "accepted program contains the semantic error type",
                ));
            }
            Type::Int | Type::Bool | Type::Unit | Type::Never | Type::Function(_) => {}
        }
        Ok(())
    }

    fn type_facts(&self) -> Result<Vec<v1::Type>, InspectionError> {
        self.types
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                let (kind, declaration, parameters, return_type) = match ty {
                    Type::Int => (v1::TypeKind::Int, None, Vec::new(), None),
                    Type::Bool => (v1::TypeKind::Bool, None, Vec::new(), None),
                    Type::Record(record) => (
                        v1::TypeKind::Record,
                        Some(record_id(record.id.index())),
                        Vec::new(),
                        None,
                    ),
                    Type::Enum(enumeration) => (
                        v1::TypeKind::Enum,
                        Some(enum_id(enumeration.id.index())),
                        Vec::new(),
                        None,
                    ),
                    Type::Unit => (v1::TypeKind::Unit, None, Vec::new(), None),
                    Type::Never => (v1::TypeKind::Never, None, Vec::new(), None),
                    Type::Function(signature) => (
                        v1::TypeKind::Function,
                        None,
                        signature
                            .parameters
                            .iter()
                            .map(|parameter| self.known_type_id(parameter))
                            .collect::<Result<Vec<_>, _>>()?,
                        Some(self.known_type_id(&signature.return_type)?),
                    ),
                    Type::Error => {
                        return Err(InspectionError::invalid(
                            "type table contains the semantic error type",
                        ));
                    }
                };
                Ok(v1::Type {
                    id: type_id(index),
                    kind,
                    display: ty.to_string(),
                    declaration,
                    parameters,
                    return_type,
                })
            })
            .collect()
    }

    fn known_type_id(&self, ty: &Type) -> Result<String, InspectionError> {
        self.types
            .iter()
            .position(|known| known == ty)
            .map(type_id)
            .ok_or_else(|| {
                InspectionError::invalid(format!("type `{ty}` was not interned before rendering"))
            })
    }

    fn span(&self, span: Span) -> Result<v1::Span, InspectionError> {
        if self.source.slice(span).is_none() {
            return Err(InspectionError::invalid(format!(
                "invalid or foreign source span {}..{}",
                span.start(),
                span.end()
            )));
        }
        Ok(v1::Span {
            source: SOURCE_ID.to_owned(),
            start: span.start(),
            end: span.end(),
        })
    }
}

fn function_type(function: &hir::Function) -> Type {
    Type::Function(hir::FunctionType {
        parameters: function
            .parameters
            .iter()
            .map(|parameter| parameter.ty.clone())
            .collect(),
        return_type: Box::new(function.return_type.clone()),
    })
}

fn take_complete<T>(kind: &str, entries: Vec<Option<T>>) -> Result<Vec<T>, InspectionError> {
    entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            entry.ok_or_else(|| {
                InspectionError::invalid(format!("unfinished {kind} fact at slot {index}"))
            })
        })
        .collect()
}

fn unary_operator(operator: UnaryOperator) -> &'static str {
    match operator {
        UnaryOperator::Negate => "-",
        UnaryOperator::Not => "!",
    }
}

fn binary_operator(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Or => "||",
        BinaryOperator::And => "&&",
        BinaryOperator::Equal => "==",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::Less => "<",
        BinaryOperator::LessEqual => "<=",
        BinaryOperator::Greater => ">",
        BinaryOperator::GreaterEqual => ">=",
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Remainder => "%",
    }
}

fn type_id(index: usize) -> String {
    format!("type:{index}")
}

fn record_id(index: usize) -> String {
    format!("record:{index}")
}

fn field_id(record: usize, field: usize) -> String {
    format!("record:{record}.field:{field}")
}

fn enum_id(index: usize) -> String {
    format!("enum:{index}")
}

fn variant_id(enumeration: usize, variant: usize) -> String {
    format!("enum:{enumeration}.variant:{variant}")
}

fn function_id(index: usize) -> String {
    format!("function:{index}")
}

fn binding_id(index: usize) -> String {
    format!("binding:{index}")
}

fn block_id(index: usize) -> String {
    format!("block:{index}")
}

fn statement_id(index: usize) -> String {
    format!("statement:{index}")
}

fn expression_id(index: usize) -> String {
    format!("expression:{index}")
}

fn match_id(index: usize) -> String {
    format!("match:{index}")
}

fn match_arm_id(matched: usize, arm: usize) -> String {
    format!("match:{matched}.arm:{arm}")
}

#[cfg(test)]
mod tests {
    use super::{build_document, render_json};
    use nova_lexer::lex;
    use nova_parser::parse;
    use nova_sema::{analyze, hir};
    use nova_source::{SourceFile, SourceId, Span};
    use std::collections::BTreeSet;

    fn checked(text: &str) -> (SourceFile, hir::Program) {
        let source = SourceFile::new(SourceId::new(7), "sample\"name.nv", text);
        let lexed = lex(&source);
        assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
        let parsed = parse(&source, &lexed.tokens);
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        let analyzed = analyze(&parsed.program);
        assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
        (source, analyzed.program)
    }

    #[test]
    fn projects_symbols_types_spans_and_exhaustive_match_facts() {
        let (source, program) = checked(
            "record Box { value: Int, }\n\
             enum Maybe { None, Some(Box), }\n\
             fn boxed(value: Int) -> Box { new Box { value: value } }\n\
             fn read(value: Maybe) -> Int {\n\
                 match value {\n\
                     Maybe::None => 0,\n\
                     Maybe::Some(item) => item.value,\n\
                 }\n\
             }",
        );
        let document = build_document(&program, &source).expect("valid HIR should inspect");

        assert_eq!(document.schema, "nova.semantic-inspection");
        assert_eq!(document.schema_version, 1);
        assert_eq!(document.program.records[0].id, "record:0");
        assert_eq!(document.program.enums[0].variants[1].id, "enum:0.variant:1");
        assert_eq!(document.program.functions[0].parameters, ["binding:0"]);
        assert_eq!(document.program.matches.len(), 1);
        assert!(document.program.matches[0].exhaustive);
        assert_eq!(
            document.program.matches[0].arms[1].binding.as_deref(),
            Some("binding:2")
        );
        assert!(
            document
                .program
                .expressions
                .iter()
                .any(|expression| { expression.target.as_deref() == Some("record:0.field:0") })
        );

        let rendered = render_json(&program, &source).expect("JSON should render");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("rendered document is valid JSON");
        assert_eq!(parsed["source"]["name"], "sample\"name.nv");
    }

    #[test]
    fn projects_control_flow_mutation_records_and_operators() {
        let (source, program) = checked(
            "record Pair { left: Int, right: Int, }\n\
             fn main() -> Int {\n\
                 var total: Int;\n\
                 total = 1;\n\
                 while total < 2 { total = total + 1; continue; }\n\
                 while true { break; }\n\
                 ();\n\
                 total;\n\
                 let pair = new Pair { right: -1, left: total };\n\
                 if true && false || true { pair.left } else { { return pair.right; } }\n\
             }",
        );
        let document = build_document(&program, &source).expect("valid HIR should inspect");

        use super::v1::{ExpressionKind, StatementKind};
        for expected in [
            StatementKind::UninitializedBinding,
            StatementKind::Assignment,
            StatementKind::While,
            StatementKind::Break,
            StatementKind::Continue,
            StatementKind::Expression,
            StatementKind::InitializedBinding,
            StatementKind::Return,
        ] {
            assert!(
                document
                    .program
                    .statements
                    .iter()
                    .any(|statement| statement.kind == expected),
                "missing {expected:?}"
            );
        }
        for expected in [
            ExpressionKind::RecordConstruction,
            ExpressionKind::FieldAccess,
            ExpressionKind::Unit,
            ExpressionKind::Unary,
            ExpressionKind::Binary,
            ExpressionKind::Block,
            ExpressionKind::If,
        ] {
            assert!(
                document
                    .program
                    .expressions
                    .iter()
                    .any(|expression| expression.kind == expected),
                "missing {expected:?}"
            );
        }
        assert!(document.program.expressions.iter().any(|expression| {
            expression.kind == ExpressionKind::RecordConstruction
                && expression.target.as_deref() == Some("record:0")
                && expression
                    .field_initializers
                    .iter()
                    .map(|initializer| initializer.field.as_str())
                    .eq(["record:0.field:1", "record:0.field:0"])
        }));
        for operator in ["<", "+", "-", "&&", "||"] {
            assert!(
                document
                    .program
                    .expressions
                    .iter()
                    .any(|expression| expression.operator.as_deref() == Some(operator)),
                "missing operator {operator}"
            );
        }
    }

    #[test]
    fn rejects_non_contiguous_declaration_identities() {
        let (source, mut program) = checked("fn main() -> Int { 0 }");
        program.functions[0].id = hir::FunctionId::new(4);

        let error = build_document(&program, &source).expect_err("invalid HIR must fail closed");
        assert!(error.message().contains("function identity at slot 0"));
    }

    #[test]
    fn rejects_binding_references_outside_their_owner_or_scope() {
        let (source, mut cross_owner) = checked(
            "fn first() -> Int { let value = 1; value }\n\
             fn second() -> Int { 2 }",
        );
        let second_tail = cross_owner.functions[1]
            .body
            .tail
            .as_deref_mut()
            .expect("second has a tail expression");
        second_tail.kind = hir::ExpressionKind::Binding(hir::BindingId::new(0));
        let error =
            build_document(&cross_owner, &source).expect_err("cross-owner use must fail closed");
        assert!(error.message().contains("crosses function ownership"));

        let (source, mut escaped_scope) = checked(
            "fn main() -> Int {\n\
                 { let hidden = 1; hidden; };\n\
                 2\n\
             }",
        );
        let tail = escaped_scope.functions[0]
            .body
            .tail
            .as_deref_mut()
            .expect("main has a tail expression");
        tail.kind = hir::ExpressionKind::Binding(hir::BindingId::new(0));
        let error =
            build_document(&escaped_scope, &source).expect_err("escaped use must fail closed");
        assert!(error.message().contains("outside its lexical scope"));
    }

    #[test]
    fn rejects_assignment_to_an_immutable_binding_in_hir() {
        let (source, mut program) = checked(
            "fn main() -> Int {\n\
                 let fixed = 1;\n\
                 var changing = 2;\n\
                 changing = 3;\n\
                 changing\n\
             }",
        );
        let hir::StatementKind::Assignment { target, .. } =
            &mut program.functions[0].body.statements[2].kind
        else {
            panic!("expected assignment HIR");
        };
        *target = Some(hir::BindingId::new(0));

        let error = build_document(&program, &source)
            .expect_err("assignment to immutable binding must fail closed");
        assert!(
            error
                .message()
                .contains("assignment targets immutable binding:0")
        );
    }

    #[test]
    fn rejects_foreign_spans_and_error_types() {
        let (source, mut foreign_span) = checked("fn main() -> Int { 0 }");
        foreign_span.span = Span::empty(SourceId::new(99), 0);
        assert!(build_document(&foreign_span, &source).is_err());

        let (source, mut error_type) = checked("fn main() -> Int { 0 }");
        error_type.functions[0].return_type = hir::Type::Error;
        let error = build_document(&error_type, &source).expect_err("error type must fail closed");
        assert!(error.message().contains("semantic error type"));
    }

    #[test]
    fn rejects_non_exhaustive_match_hir() {
        let (source, mut program) = checked(
            "enum Flag { Off, On, }\n\
             fn read(flag: Flag) -> Int {\n\
                 match flag { Flag::Off => 0, Flag::On => 1, }\n\
             }",
        );
        let tail = program.functions[0]
            .body
            .tail
            .as_deref_mut()
            .expect("match is the function tail");
        let hir::ExpressionKind::Match { arms, .. } = &mut tail.kind else {
            panic!("expected match HIR");
        };
        arms.pop();

        let error = build_document(&program, &source).expect_err("invalid HIR must fail closed");
        assert!(error.message().contains("is not exhaustive"));
    }

    #[test]
    fn rejects_loop_control_outside_a_loop_in_hir() {
        let (source, mut program) = checked("fn main() -> Int { while true { break; } 0 }");
        program.functions[0].body.statements[0].kind = hir::StatementKind::Break;

        let error = build_document(&program, &source).expect_err("invalid HIR must fail closed");
        assert!(
            error
                .message()
                .contains("break in statement:0 has no enclosing loop")
        );
    }

    #[test]
    fn published_json_schema_is_well_formed_and_names_v1() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/semantic-inspection-v1.schema.json"
        ))
        .expect("published schema must be valid JSON");

        assert_eq!(schema["$id"], "urn:nova:semantic-inspection:v1");
        assert_eq!(
            schema["properties"]["schema"]["const"],
            "nova.semantic-inspection"
        );
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);
        assert_eq!(
            schema["$defs"]["match"]["properties"]["exhaustive"]["const"],
            true
        );

        let (source, program) = checked(
            "record Box { value: Int, }\n\
             enum Maybe { None, Some(Box), }\n\
             fn boxed(value: Int) -> Box { new Box { value: value } }\n\
             fn read(value: Maybe) -> Int {\n\
                 let fallback = 0;\n\
                 match value {\n\
                     Maybe::None => fallback,\n\
                     Maybe::Some(item) => item.value,\n\
                 }\n\
             }",
        );
        let document = serde_json::to_value(
            build_document(&program, &source).expect("valid HIR should inspect"),
        )
        .expect("document should serialize");

        assert_required_keys(&schema, &document);
        assert_required_keys(&schema["$defs"]["producer"], &document["producer"]);
        assert_required_keys(&schema["$defs"]["source"], &document["source"]);
        assert_required_keys(&schema["$defs"]["program"], &document["program"]);
        assert_required_keys(&schema["$defs"]["span"], &document["program"]["span"]);
        for (definition, value) in [
            ("type", &document["program"]["types"][0]),
            ("record", &document["program"]["records"][0]),
            (
                "recordField",
                &document["program"]["records"][0]["fields"][0],
            ),
            ("enum", &document["program"]["enums"][0]),
            (
                "enumVariant",
                &document["program"]["enums"][0]["variants"][0],
            ),
            ("function", &document["program"]["functions"][0]),
            ("binding", &document["program"]["bindings"][0]),
            ("block", &document["program"]["blocks"][0]),
            ("statement", &document["program"]["statements"][0]),
            ("expression", &document["program"]["expressions"][0]),
            (
                "recordFieldInitializer",
                document["program"]["expressions"]
                    .as_array()
                    .expect("expressions are an array")
                    .iter()
                    .find_map(|expression| expression["field_initializers"].get(0))
                    .expect("representative document has a field initializer"),
            ),
            ("match", &document["program"]["matches"][0]),
            ("matchArm", &document["program"]["matches"][0]["arms"][0]),
        ] {
            assert_required_keys(&schema["$defs"][definition], value);
        }
    }

    fn assert_required_keys(schema: &serde_json::Value, value: &serde_json::Value) {
        let expected = schema["required"]
            .as_array()
            .expect("schema object declares required fields")
            .iter()
            .map(|key| key.as_str().expect("required field is a string"))
            .collect::<BTreeSet<_>>();
        let actual = value
            .as_object()
            .expect("serialized schema value is an object")
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
        assert_eq!(schema["additionalProperties"], false);
    }
}
