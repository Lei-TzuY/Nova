//! Versioned, deterministic semantic introspection for accepted Nova programs.
//!
//! This crate deliberately projects typed HIR into a tooling-owned schema. The
//! JSON representation is therefore not a serialization of compiler internals.

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;
pub mod v7;

use nova_parser::ast::{BinaryOperator, UnaryOperator};
use nova_sema::AnalysisOutput;
use nova_sema::control_flow::{
    ClosureControlFlow, ControlFlowProgram, FlowEdgeKind, FlowNodeKind, FlowTransfer,
    FunctionControlFlow,
};
use nova_sema::hir::{self, Type};
use nova_source::{SourceFile, Span};
use std::collections::{BTreeMap, BTreeSet};
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

/// Builds a schema-v2 document from one successful semantic analysis.
///
/// Version 2 preserves the complete schema-v1 HIR projection and independently
/// validates the verified CFG's function, binding, node, edge, and span
/// references before adding them to the document. Rejected analysis output and
/// mismatched HIR/CFG pairs fail closed.
pub fn build_document_v2(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<v2::Document, InspectionError> {
    if !analysis.is_success() {
        return Err(InspectionError::invalid(
            "schema v2 requires a successful semantic analysis",
        ));
    }

    let v1_document = build_document(&analysis.program, source)?;
    let control_flow = project_control_flow(
        &analysis.control_flow,
        &v1_document.program,
        analysis.program.module.id,
        source,
    )?;
    Ok(v2::Document {
        schema: v2::SCHEMA_NAME.to_owned(),
        schema_version: v2::SCHEMA_VERSION,
        producer: v1_document.producer,
        source: v1_document.source,
        program: v1_document.program,
        control_flow,
    })
}

/// Renders one successful analysis as deterministic, pretty-printed schema-v2 JSON.
pub fn render_json_v2(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<String, InspectionError> {
    let document = build_document_v2(analysis, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v2: {error}")))
}

/// Builds a schema-v3 document from one successful semantic analysis.
///
/// Version 3 preserves the established program and CFG structural projections and adds
/// explicit match-arm payload modes. This is the first inspection schema that can
/// represent `Enum::Variant(_)` payload discard without reinterpreting v1/v2 fields.
pub fn build_document_v3(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<v3::Document, InspectionError> {
    if !analysis.is_success() {
        return Err(InspectionError::invalid(
            "schema v3 requires a successful semantic analysis",
        ));
    }

    let (program_document, match_patterns, _) = Builder::new(&analysis.program, source)
        .with_payload_discard()
        .build_parts()?;
    let control_flow = project_control_flow(
        &analysis.control_flow,
        &program_document.program,
        analysis.program.module.id,
        source,
    )?;
    Ok(v3::Document {
        schema: v3::SCHEMA_NAME.to_owned(),
        schema_version: v3::SCHEMA_VERSION,
        producer: program_document.producer,
        source: program_document.source,
        program: program_document.program,
        control_flow,
        match_patterns,
    })
}

/// Renders one successful analysis as deterministic, pretty-printed schema-v3 JSON.
pub fn render_json_v3(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<String, InspectionError> {
    let document = build_document_v3(analysis, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v3: {error}")))
}

/// Builds a schema-v4 document from one successful semantic analysis.
///
/// Version 4 is the first tooling contract that can represent the `String`
/// scalar and string-literal expressions. It also retains v2 CFG facts and v3
/// match payload modes. Older schemas reject programs containing `String`.
pub fn build_document_v4(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<v4::Document, InspectionError> {
    if !analysis.is_success() {
        return Err(InspectionError::invalid(
            "schema v4 requires a successful semantic analysis",
        ));
    }

    let (program_document, match_patterns, _) = Builder::new(&analysis.program, source)
        .with_payload_discard()
        .with_string()
        .build_parts()?;
    let control_flow = project_control_flow(
        &analysis.control_flow,
        &program_document.program,
        analysis.program.module.id,
        source,
    )?;
    Ok(v4::Document {
        schema: v4::SCHEMA_NAME.to_owned(),
        schema_version: v4::SCHEMA_VERSION,
        producer: program_document.producer,
        source: program_document.source,
        program: program_document.program,
        control_flow,
        match_patterns,
    })
}

/// Renders one successful analysis as deterministic, pretty-printed schema-v4 JSON.
pub fn render_json_v4(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<String, InspectionError> {
    let document = build_document_v4(analysis, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v4: {error}")))
}

/// Builds a schema-v5 document from one successful semantic analysis.
///
/// Version 5 is the first tooling contract that represents anonymous closures,
/// immutable lexical captures, and closure-owned CFGs. Older schemas reject any
/// closure expression rather than silently dropping its callable body or environment.
pub fn build_document_v5(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<v5::Document, InspectionError> {
    if !analysis.is_success() {
        return Err(InspectionError::invalid(
            "schema v5 requires a successful semantic analysis",
        ));
    }

    let (program_document, match_patterns, closures) = Builder::new(&analysis.program, source)
        .with_payload_discard()
        .with_string()
        .with_closures()
        .build_parts()?;
    let control_flow = project_control_flow(
        &analysis.control_flow,
        &program_document.program,
        analysis.program.module.id,
        source,
    )?;
    let closure_control_flow = project_closure_control_flow(
        &analysis.control_flow,
        &program_document.program,
        &closures,
        analysis.program.module.id,
        source,
    )?;
    Ok(v5::Document {
        schema: v5::SCHEMA_NAME.to_owned(),
        schema_version: v5::SCHEMA_VERSION,
        producer: program_document.producer,
        source: program_document.source,
        program: program_document.program,
        control_flow,
        match_patterns,
        closures,
        closure_control_flow,
    })
}

/// Renders one successful analysis as deterministic, pretty-printed schema-v5 JSON.
pub fn render_json_v5(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<String, InspectionError> {
    let document = build_document_v5(analysis, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v5: {error}")))
}

/// Builds a schema-v6 document from one successful semantic analysis.
///
/// Version 6 exposes the module that owns every declaration, closure, and binding
/// identity. It is intentionally still a one-source document: import resolution,
/// visibility, module paths, and cross-module linking are not inferred here.
pub fn build_document_v6(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<v6::Document, InspectionError> {
    if !analysis.is_success() {
        return Err(InspectionError::invalid(
            "schema v6 requires a successful semantic analysis",
        ));
    }

    let (program_document, match_patterns, closures) = Builder::new(&analysis.program, source)
        .with_payload_discard()
        .with_string()
        .with_closures()
        .with_module_identity()
        .build_parts()?;
    let control_flow = project_control_flow(
        &analysis.control_flow,
        &program_document.program,
        analysis.program.module.id,
        source,
    )?;
    let closure_control_flow = project_closure_control_flow(
        &analysis.control_flow,
        &program_document.program,
        &closures,
        analysis.program.module.id,
        source,
    )?;
    let module = v6::Module {
        id: module_id(analysis.program.module.id),
        source: program_document.source.id.clone(),
        implicit_root: analysis.program.module.id == hir::ModuleId::ROOT,
        span: program_document.program.span.clone(),
        records: program_document
            .program
            .records
            .iter()
            .map(|record| record.id.clone())
            .collect(),
        enums: program_document
            .program
            .enums
            .iter()
            .map(|enumeration| enumeration.id.clone())
            .collect(),
        functions: program_document
            .program
            .functions
            .iter()
            .map(|function| function.id.clone())
            .collect(),
        bindings: program_document
            .program
            .bindings
            .iter()
            .map(|binding| binding.id.clone())
            .collect(),
        closures: closures.iter().map(|closure| closure.id.clone()).collect(),
    };
    Ok(v6::Document {
        schema: v6::SCHEMA_NAME.to_owned(),
        schema_version: v6::SCHEMA_VERSION,
        producer: program_document.producer,
        source: program_document.source,
        module,
        program: program_document.program,
        control_flow,
        match_patterns,
        closures,
        closure_control_flow,
    })
}

/// Renders one successful analysis as deterministic, pretty-printed schema-v6 JSON.
pub fn render_json_v6(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<String, InspectionError> {
    let document = build_document_v6(analysis, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v6: {error}")))
}

/// Builds a schema-v7 document from one successful semantic analysis.
///
/// Version 7 is the first tooling contract that can represent the `UInt` type,
/// unsigned literal values, checked `Int`/`UInt` conversion expressions, and
/// by-value capture mode for mutable-source snapshots. It otherwise preserves
/// the complete schema-v6 module, program, CFG, pattern, and closure projections.
/// Schemas v1-v6 reject facts they cannot represent instead of widening their
/// frozen contracts in place.
pub fn build_document_v7(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<v7::Document, InspectionError> {
    if !analysis.is_success() {
        return Err(InspectionError::invalid(
            "schema v7 requires a successful semantic analysis",
        ));
    }

    let (program_document, match_patterns, closures) = Builder::new(&analysis.program, source)
        .with_payload_discard()
        .with_string()
        .with_closures()
        .with_module_identity()
        .with_unsigned()
        .with_mutable_captures()
        .build_parts()?;
    let control_flow = project_control_flow(
        &analysis.control_flow,
        &program_document.program,
        analysis.program.module.id,
        source,
    )?;
    let closure_control_flow = project_closure_control_flow(
        &analysis.control_flow,
        &program_document.program,
        &closures,
        analysis.program.module.id,
        source,
    )?;
    let module = v6::Module {
        id: module_id(analysis.program.module.id),
        source: program_document.source.id.clone(),
        implicit_root: analysis.program.module.id == hir::ModuleId::ROOT,
        span: program_document.program.span.clone(),
        records: program_document
            .program
            .records
            .iter()
            .map(|record| record.id.clone())
            .collect(),
        enums: program_document
            .program
            .enums
            .iter()
            .map(|enumeration| enumeration.id.clone())
            .collect(),
        functions: program_document
            .program
            .functions
            .iter()
            .map(|function| function.id.clone())
            .collect(),
        bindings: program_document
            .program
            .bindings
            .iter()
            .map(|binding| binding.id.clone())
            .collect(),
        closures: closures.iter().map(|closure| closure.id.clone()).collect(),
    };
    let closures = closures.into_iter().map(v7::Closure::from).collect();
    Ok(v7::Document {
        schema: v7::SCHEMA_NAME.to_owned(),
        schema_version: v7::SCHEMA_VERSION,
        producer: program_document.producer,
        source: program_document.source,
        module,
        program: program_document.program,
        control_flow,
        match_patterns,
        closures,
        closure_control_flow,
    })
}

/// Renders one successful analysis as deterministic, pretty-printed schema-v7 JSON.
pub fn render_json_v7(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<String, InspectionError> {
    let document = build_document_v7(analysis, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v7: {error}")))
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
    match_patterns: Vec<v3::MatchPattern>,
    closures: Vec<Option<v5::Closure>>,
    allow_payload_discard: bool,
    allow_string: bool,
    allow_closures: bool,
    allow_module_identity: bool,
    allow_unsigned: bool,
    allow_mutable_captures: bool,
    active_scopes: Vec<String>,
    active_capture_bindings: Vec<BTreeSet<hir::BindingId>>,
    active_capture_uses: Vec<BTreeSet<hir::BindingId>>,
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
            match_patterns: Vec::new(),
            closures: Vec::new(),
            allow_payload_discard: false,
            allow_string: false,
            allow_closures: false,
            allow_module_identity: false,
            allow_unsigned: false,
            allow_mutable_captures: false,
            active_scopes: Vec::new(),
            active_capture_bindings: Vec::new(),
            active_capture_uses: Vec::new(),
            loop_depth: 0,
        }
    }

    fn with_payload_discard(mut self) -> Self {
        self.allow_payload_discard = true;
        self
    }

    fn with_string(mut self) -> Self {
        self.allow_string = true;
        self
    }

    fn with_closures(mut self) -> Self {
        self.allow_closures = true;
        self
    }

    fn with_module_identity(mut self) -> Self {
        self.allow_module_identity = true;
        self
    }

    fn with_unsigned(mut self) -> Self {
        self.allow_unsigned = true;
        self
    }

    fn with_mutable_captures(mut self) -> Self {
        self.allow_mutable_captures = true;
        self
    }

    fn build(self) -> Result<v1::Document, InspectionError> {
        self.build_parts().map(|(document, _, _)| document)
    }

    fn build_parts(
        mut self,
    ) -> Result<(v1::Document, Vec<v3::MatchPattern>, Vec<v5::Closure>), InspectionError> {
        let program_span = self.span(self.program.span)?;
        if self.program.module.span != self.program.span {
            return Err(InspectionError::invalid(
                "program module span does not match the complete program span",
            ));
        }
        if !self.allow_module_identity && self.program.module.id != hir::ModuleId::ROOT {
            return Err(InspectionError::invalid(
                "semantic-inspection schema v1-v5 cannot represent a non-root module; select schema v6",
            ));
        }
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
        let closures = take_complete("closure", self.closures)?;

        let match_patterns = self.match_patterns;
        let document = v1::Document {
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
        };
        Ok((document, match_patterns, closures))
    }

    fn prepare_type_order(&mut self) -> Result<(), InspectionError> {
        self.intern_type(&Type::Int)?;
        if self.allow_unsigned {
            self.intern_type(&Type::UInt)?;
        }
        self.intern_type(&Type::Bool)?;
        if self.allow_string {
            self.intern_type(&Type::String)?;
        }
        for ty in [Type::Unit, Type::Never] {
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
                let resolved = resolved.as_ref().ok_or_else(|| {
                    InspectionError::invalid(format!(
                        "accepted assignment {} has no resolved target",
                        statement_id(index)
                    ))
                })?;
                let binding = self.require_binding_reference(resolved, owner)?;
                if binding.owner != owner {
                    return Err(InspectionError::invalid(format!(
                        "assignment targets captured snapshot {}",
                        binding_id(resolved.binding.index())
                    )));
                }
                if !binding.mutable {
                    return Err(InspectionError::invalid(format!(
                        "assignment targets immutable {}",
                        binding_id(resolved.binding.index())
                    )));
                }
                target = Some(binding_id(resolved.binding.index()));
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
                if let Some(value) = value {
                    expressions.push(self.collect_expression(value, owner)?);
                }
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
            hir::ExpressionKind::Unsigned(_) => {
                if !self.allow_unsigned {
                    return Err(InspectionError::invalid(
                        "semantic-inspection schema v1-v6 cannot represent `UInt`; select schema v7",
                    ));
                }
                if expression.ty != Type::UInt {
                    return Err(InspectionError::invalid(format!(
                        "unsigned literal expression has HIR type {} instead of UInt",
                        expression.ty
                    )));
                }
                v1::ExpressionKind::UnsignedInteger
            }
            hir::ExpressionKind::String(_) => {
                if expression.ty != Type::String {
                    return Err(InspectionError::invalid(format!(
                        "string literal expression has HIR type {} instead of String",
                        expression.ty
                    )));
                }
                if !self.allow_string {
                    return Err(InspectionError::invalid(
                        "semantic-inspection schema v1/v2/v3 cannot represent `String`; select schema v4",
                    ));
                }
                v1::ExpressionKind::String
            }
            hir::ExpressionKind::Boolean(_) => v1::ExpressionKind::Boolean,
            hir::ExpressionKind::Unit => v1::ExpressionKind::Unit,
            hir::ExpressionKind::Closure(closure) => {
                if !self.allow_closures {
                    return Err(InspectionError::invalid(
                        "semantic-inspection schema v1/v2/v3/v4 cannot represent closures; select schema v5",
                    ));
                }
                let closure_index = self.closures.len();
                if closure.id.module() != self.program.module.id
                    || closure.id.index() != closure_index
                {
                    return Err(InspectionError::invalid(format!(
                        "closure identities must belong to module:{} and follow semantic traversal order: expected closure:{closure_index}, found module:{}/closure:{}",
                        self.program.module.id.raw(),
                        closure.id.module().raw(),
                        closure.id.index(),
                    )));
                }
                self.closures.push(None);
                let closure_owner = closure_id(closure_index);
                let expected_type = Type::Function(closure.function_type());
                if expression.ty != expected_type {
                    return Err(InspectionError::invalid(format!(
                        "{} expression type {} does not match closure signature {}",
                        closure_owner, expression.ty, expected_type
                    )));
                }
                if closure.span != expression.span {
                    return Err(InspectionError::invalid(format!(
                        "{} span does not match its creating expression",
                        closure_owner
                    )));
                }

                let mut capture_ids = BTreeSet::new();
                let mut captures = Vec::with_capacity(closure.captures.len());
                let mut previous_first_use = None;
                for capture in &closure.captures {
                    let binding = self.require_binding_reference(&capture.reference, owner)?;
                    if binding.mutable && !self.allow_mutable_captures {
                        return Err(InspectionError::invalid(
                            "semantic-inspection schema v5/v6 cannot represent a mutable-source snapshot capture; select schema v7",
                        ));
                    }
                    if !capture_ids.insert(capture.reference.binding) {
                        return Err(InspectionError::invalid(format!(
                            "{} repeats capture {}",
                            closure_owner, binding.id
                        )));
                    }
                    let capture_type = self.intern_type(&capture.ty)?;
                    if capture_type != binding.type_id {
                        return Err(InspectionError::invalid(format!(
                            "{} capture type does not match {}",
                            closure_owner, binding.id
                        )));
                    }
                    let first_use = self.span(capture.first_use)?;
                    if capture.first_use.start() < closure.body.span.start()
                        || capture.first_use.end() > closure.body.span.end()
                    {
                        return Err(InspectionError::invalid(format!(
                            "{} capture first use lies outside its body",
                            closure_owner
                        )));
                    }
                    if previous_first_use.is_some_and(|start| start >= capture.first_use.start()) {
                        return Err(InspectionError::invalid(format!(
                            "{} captures are not in first-lexical-use order",
                            closure_owner
                        )));
                    }
                    previous_first_use = Some(capture.first_use.start());
                    captures.push(v5::ClosureCapture {
                        binding: binding.id,
                        type_id: capture_type,
                        first_use,
                    });
                }

                let mut parameters = Vec::with_capacity(closure.parameters.len());
                for parameter in &closure.parameters {
                    parameters.push(self.add_binding(
                        parameter,
                        v1::BindingRole::Parameter,
                        &closure_owner,
                        &closure_owner,
                    )?);
                }

                self.active_capture_bindings.push(capture_ids.clone());
                self.active_capture_uses.push(BTreeSet::new());
                let outer_loop_depth = std::mem::replace(&mut self.loop_depth, 0);
                let body_result = self.collect_block(&closure.body, &closure_owner);
                self.loop_depth = outer_loop_depth;
                let used_captures = self
                    .active_capture_uses
                    .pop()
                    .expect("closure inspection must own one capture-use set");
                self.active_capture_bindings.pop();
                let body = body_result?;
                if used_captures != capture_ids {
                    return Err(InspectionError::invalid(format!(
                        "{} capture table does not exactly match its free binding references",
                        closure_owner
                    )));
                }
                let type_id = self.intern_type(&expected_type)?;
                let return_type = self.intern_type(&closure.return_type)?;
                self.closures[closure_index] = Some(v5::Closure {
                    id: closure_owner.clone(),
                    expression: id.clone(),
                    type_id,
                    return_type,
                    parameters,
                    captures,
                    body,
                    span: span.clone(),
                });
                target = Some(closure_owner);
                v1::ExpressionKind::Closure
            }
            hir::ExpressionKind::Binding(resolved) => {
                self.require_binding_reference(resolved, owner)?;
                target = Some(binding_id(resolved.binding.index()));
                v1::ExpressionKind::BindingReference
            }
            hir::ExpressionKind::Function {
                function,
                function_name,
            } => {
                let declaration = self.require_function(*function)?;
                if declaration.name != *function_name {
                    return Err(InspectionError::invalid(format!(
                        "function reference `{function_name}` does not match declaration id {} (`{}`)",
                        function.index(),
                        declaration.name
                    )));
                }
                let expected_type = function_type(declaration);
                if expression.ty != expected_type {
                    return Err(InspectionError::invalid(format!(
                        "function reference `{function_name}` type {} does not match declaration signature {}",
                        expression.ty, expected_type
                    )));
                }
                target = Some(function_id(function.index()));
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
                    let declared_name = self.require_record(*record)?.fields[field.field_index]
                        .name
                        .clone();
                    if declared_name != field.field_name {
                        return Err(InspectionError::invalid(format!(
                            "record construction resolved field `{}` to slot {}, declared as `{declared_name}`",
                            field.field_name, field.field_index
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
                variant_name,
                variant_index,
                payload,
            } => {
                let declaration = self.require_enum(*enumeration)?;
                let variant = declaration.variants.get(*variant_index).ok_or_else(|| {
                    InspectionError::invalid(format!(
                        "enum construction references out-of-range variant slot {variant_index}"
                    ))
                })?;
                if variant.name != *variant_name {
                    return Err(InspectionError::invalid(format!(
                        "enum construction variant `{variant_name}` does not match slot {variant_index} declaration `{}`",
                        variant.name
                    )));
                }
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
                field_name,
                field_index,
            } => {
                let declaration = self.require_record(*record)?;
                if *field_index >= declaration.fields.len() {
                    return Err(InspectionError::invalid(format!(
                        "field access references out-of-range field slot {field_index}"
                    )));
                }
                let declared_name = declaration.fields[*field_index].name.clone();
                if declared_name != *field_name {
                    return Err(InspectionError::invalid(format!(
                        "field access resolved field `{field_name}` to slot {field_index}, declared as `{declared_name}`"
                    )));
                }
                if expression.ty != declaration.fields[*field_index].ty {
                    return Err(InspectionError::invalid(format!(
                        "field access type {} does not match resolved field `{field_name}` type {}",
                        expression.ty, declaration.fields[*field_index].ty
                    )));
                }
                children.push(self.collect_expression(base, owner)?);
                target = Some(field_id(record.index(), *field_index));
                v1::ExpressionKind::FieldAccess
            }
            hir::ExpressionKind::IntToUInt { operand } => {
                if !self.allow_unsigned {
                    return Err(InspectionError::invalid(
                        "semantic-inspection schema v1-v6 cannot represent `UInt`; select schema v7",
                    ));
                }
                if expression.ty != Type::UInt || operand.ty != Type::Int {
                    return Err(InspectionError::invalid(
                        "IntToUInt HIR conversion has inconsistent types",
                    ));
                }
                operator = Some("int_to_uint".to_owned());
                children.push(self.collect_expression(operand, owner)?);
                v1::ExpressionKind::NumericConversion
            }
            hir::ExpressionKind::UIntToInt { operand } => {
                if !self.allow_unsigned {
                    return Err(InspectionError::invalid(
                        "semantic-inspection schema v1-v6 cannot represent `UInt`; select schema v7",
                    ));
                }
                if expression.ty != Type::Int || operand.ty != Type::UInt {
                    return Err(InspectionError::invalid(
                        "UIntToInt HIR conversion has inconsistent types",
                    ));
                }
                operator = Some("uint_to_int".to_owned());
                children.push(self.collect_expression(operand, owner)?);
                v1::ExpressionKind::NumericConversion
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
                    if variant.name != arm.variant_name {
                        return Err(InspectionError::invalid(format!(
                            "match variant `{}` does not match slot {} declaration `{}`",
                            arm.variant_name, arm.variant_index, variant.name
                        )));
                    }
                    if !seen.insert(arm.variant_index) {
                        return Err(InspectionError::invalid(format!(
                            "match repeats variant slot {}",
                            arm.variant_index
                        )));
                    }
                    let arm_identity = match_arm_id(match_index, arm_index);
                    self.active_scopes.push(arm_identity.clone());
                    let arm_contents = (|| {
                        let (binding, payload_mode) = match (
                            &variant.payload,
                            &arm.binding,
                            arm.payload_discarded,
                        ) {
                            (Some(expected), Some(binding), false) => {
                                if &binding.ty != expected {
                                    return Err(InspectionError::invalid(format!(
                                        "match payload binding type does not match {}",
                                        variant_id(enumeration.index(), arm.variant_index)
                                    )));
                                }
                                (
                                    Some(self.add_binding(
                                        binding,
                                        v1::BindingRole::MatchPayload,
                                        owner,
                                        &arm_identity,
                                    )?),
                                    v3::MatchPayloadMode::Bind,
                                )
                            }
                            (Some(_), None, true) if self.allow_payload_discard => {
                                (None, v3::MatchPayloadMode::Discard)
                            }
                            (Some(_), None, true) => {
                                return Err(InspectionError::invalid(
                                    "semantic-inspection schema v1/v2 cannot represent an explicitly discarded enum payload; select schema v3",
                                ));
                            }
                            (None, None, false) => (None, v3::MatchPayloadMode::None),
                            _ => {
                                return Err(InspectionError::invalid(format!(
                                    "match payload mode does not match {}",
                                    variant_id(enumeration.index(), arm.variant_index)
                                )));
                            }
                        };
                        let value = self.collect_expression(&arm.value, owner)?;
                        Ok::<_, InspectionError>((binding, value, payload_mode))
                    })();
                    self.active_scopes.pop();
                    let (binding, value, payload_mode) = arm_contents?;
                    self.match_patterns.push(v3::MatchPattern {
                        arm: arm_identity.clone(),
                        payload_mode,
                    });
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
        if binding.id.module() != self.program.module.id || binding.id.index() != expected {
            return Err(InspectionError::invalid(format!(
                "binding identities must belong to module:{} and be contiguous in semantic order: expected binding:{expected}, found module:{}/binding:{}",
                self.program.module.id.raw(),
                binding.id.module().raw(),
                binding.id.index(),
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

    fn require_binding_reference(
        &mut self,
        reference: &hir::BindingReference,
        owner: &str,
    ) -> Result<v1::Binding, InspectionError> {
        let binding = self.require_known_binding(reference.binding, owner)?;
        if binding.name != reference.binding_name {
            return Err(InspectionError::invalid(format!(
                "binding reference `{}` does not match {} declaration name `{}`",
                reference.binding_name,
                binding_id(reference.binding.index()),
                binding.name
            )));
        }
        let declaration_span = self.span(reference.declaration_span)?;
        if binding.span != declaration_span {
            return Err(InspectionError::invalid(format!(
                "binding reference `{}` does not match {} declaration span",
                reference.binding_name,
                binding_id(reference.binding.index())
            )));
        }
        Ok(binding)
    }

    fn require_known_binding(
        &mut self,
        id: hir::BindingId,
        owner: &str,
    ) -> Result<v1::Binding, InspectionError> {
        if id.module() != self.program.module.id {
            return Err(InspectionError::invalid(format!(
                "reference to binding:{} belongs to module:{}, not program module:{}",
                id.index(),
                id.module().raw(),
                self.program.module.id.raw()
            )));
        }
        let binding = self.bindings.get(id.index()).cloned().ok_or_else(|| {
            InspectionError::invalid(format!("reference to unknown {}", binding_id(id.index())))
        })?;
        let crosses_owner = binding.owner != owner;
        if crosses_owner {
            let permitted = self
                .active_capture_bindings
                .last()
                .is_some_and(|captures| captures.contains(&id));
            if !permitted {
                return Err(InspectionError::invalid(format!(
                    "reference to {} crosses callable ownership without a capture",
                    binding_id(id.index())
                )));
            }
            self.active_capture_uses
                .last_mut()
                .expect("an active capture table requires a use set")
                .insert(id);
        }
        if !crosses_owner
            && binding.scope != owner
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
        if id.module() != self.program.module.id {
            return Err(InspectionError::invalid(format!(
                "reference to record:{} belongs to module:{}, not program module:{}",
                id.index(),
                id.module().raw(),
                self.program.module.id.raw()
            )));
        }
        self.program.records.get(id.index()).ok_or_else(|| {
            InspectionError::invalid(format!("reference to unknown {}", record_id(id.index())))
        })
    }

    fn require_enum(&self, id: hir::EnumId) -> Result<&hir::Enum, InspectionError> {
        if id.module() != self.program.module.id {
            return Err(InspectionError::invalid(format!(
                "reference to enum:{} belongs to module:{}, not program module:{}",
                id.index(),
                id.module().raw(),
                self.program.module.id.raw()
            )));
        }
        self.program.enums.get(id.index()).ok_or_else(|| {
            InspectionError::invalid(format!("reference to unknown {}", enum_id(id.index())))
        })
    }

    fn require_function(&self, id: hir::FunctionId) -> Result<&hir::Function, InspectionError> {
        if id.module() != self.program.module.id {
            return Err(InspectionError::invalid(format!(
                "reference to function:{} belongs to module:{}, not program module:{}",
                id.index(),
                id.module().raw(),
                self.program.module.id.raw()
            )));
        }
        self.program.functions.get(id.index()).ok_or_else(|| {
            InspectionError::invalid(format!("reference to unknown {}", function_id(id.index())))
        })
    }

    fn require_record_id(
        &self,
        actual: hir::RecordId,
        expected: usize,
    ) -> Result<(), InspectionError> {
        if actual.module() == self.program.module.id && actual.index() == expected {
            Ok(())
        } else {
            Err(InspectionError::invalid(format!(
                "record identity at slot {expected} is module:{}/record:{}",
                actual.module().raw(),
                actual.index()
            )))
        }
    }

    fn require_enum_id(&self, actual: hir::EnumId, expected: usize) -> Result<(), InspectionError> {
        if actual.module() == self.program.module.id && actual.index() == expected {
            Ok(())
        } else {
            Err(InspectionError::invalid(format!(
                "enum identity at slot {expected} is module:{}/enum:{}",
                actual.module().raw(),
                actual.index()
            )))
        }
    }

    fn require_function_id(
        &self,
        actual: hir::FunctionId,
        expected: usize,
    ) -> Result<(), InspectionError> {
        if actual.module() == self.program.module.id && actual.index() == expected {
            Ok(())
        } else {
            Err(InspectionError::invalid(format!(
                "function identity at slot {expected} is module:{}/function:{}",
                actual.module().raw(),
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
            Type::String if !self.allow_string => {
                return Err(InspectionError::invalid(
                    "semantic-inspection schema v1/v2/v3 cannot represent `String`; select schema v4",
                ));
            }
            Type::UInt if !self.allow_unsigned => {
                return Err(InspectionError::invalid(
                    "semantic-inspection schema v1-v6 cannot represent `UInt`; select schema v7",
                ));
            }
            Type::Int
            | Type::UInt
            | Type::Bool
            | Type::String
            | Type::Unit
            | Type::Never
            | Type::Function(_) => {}
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
                    Type::UInt => (v1::TypeKind::UInt, None, Vec::new(), None),
                    Type::Bool => (v1::TypeKind::Bool, None, Vec::new(), None),
                    Type::String => (v1::TypeKind::String, None, Vec::new(), None),
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
        document_span(self.source, span)
    }
}

fn project_control_flow(
    control_flow: &ControlFlowProgram,
    program: &v1::Program,
    module: hir::ModuleId,
    source: &SourceFile,
) -> Result<Vec<v2::ControlFlowGraph>, InspectionError> {
    if control_flow.functions().len() != program.functions.len() {
        return Err(InspectionError::invalid(format!(
            "schema v2 requires one CFG per function: found {} graphs for {} functions",
            control_flow.functions().len(),
            program.functions.len()
        )));
    }

    let bindings_by_id = program
        .bindings
        .iter()
        .map(|binding| (binding.id.as_str(), binding))
        .collect::<BTreeMap<_, _>>();
    control_flow
        .functions()
        .iter()
        .enumerate()
        .map(|(index, graph)| {
            project_function_control_flow(graph, index, program, &bindings_by_id, module, source)
        })
        .collect()
}

fn project_closure_control_flow(
    control_flow: &ControlFlowProgram,
    program: &v1::Program,
    closures: &[v5::Closure],
    module: hir::ModuleId,
    source: &SourceFile,
) -> Result<Vec<v5::ClosureControlFlowGraph>, InspectionError> {
    if control_flow.closures().len() != closures.len() {
        return Err(InspectionError::invalid(format!(
            "schema v5 requires one CFG per closure: found {} graphs for {} closures",
            control_flow.closures().len(),
            closures.len()
        )));
    }
    let bindings_by_id = program
        .bindings
        .iter()
        .map(|binding| (binding.id.as_str(), binding))
        .collect::<BTreeMap<_, _>>();
    control_flow
        .closures()
        .iter()
        .enumerate()
        .map(|(index, graph)| {
            project_one_closure_control_flow(
                graph,
                index,
                program,
                closures,
                &bindings_by_id,
                module,
                source,
            )
        })
        .collect()
}

fn project_one_closure_control_flow(
    graph: &ClosureControlFlow,
    index: usize,
    program: &v1::Program,
    closures: &[v5::Closure],
    bindings_by_id: &BTreeMap<&str, &v1::Binding>,
    module: hir::ModuleId,
    source: &SourceFile,
) -> Result<v5::ClosureControlFlowGraph, InspectionError> {
    let closure = closures.get(index).ok_or_else(|| {
        InspectionError::invalid(format!("closure CFG at slot {index} has no closure fact"))
    })?;
    if graph.closure().module() != module
        || graph.closure().index() != index
        || closure.id != closure_id(index)
    {
        return Err(InspectionError::invalid(format!(
            "closure CFG identity at slot {index} does not match {}",
            closure.id
        )));
    }

    let allowed_captures = closure
        .captures
        .iter()
        .map(|capture| capture.binding.clone())
        .collect::<BTreeSet<_>>();
    let expected_bindings = program
        .bindings
        .iter()
        .filter(|binding| binding.owner == closure.id || allowed_captures.contains(&binding.id))
        .collect::<Vec<_>>();
    if graph.bindings().len() != expected_bindings.len() {
        return Err(InspectionError::invalid(format!(
            "{} CFG binding table has {} entries, expected {}",
            closure.id,
            graph.bindings().len(),
            expected_bindings.len()
        )));
    }
    let mut binding_ids = Vec::with_capacity(graph.bindings().len());
    for (flow_binding, expected) in graph.bindings().iter().zip(expected_bindings) {
        if flow_binding.id.module() != module {
            return Err(InspectionError::invalid(format!(
                "{} CFG contains a binding from module:{} instead of module:{}",
                closure.id,
                flow_binding.id.module().raw(),
                module.raw()
            )));
        }
        let id = binding_id(flow_binding.id.index());
        if id != expected.id
            || flow_binding.name != expected.name
            || document_span(source, flow_binding.span)? != expected.span
        {
            return Err(InspectionError::invalid(format!(
                "{} CFG metadata does not match its HIR binding",
                expected.id
            )));
        }
        binding_ids.push(id);
    }

    let entry = graph.entry();
    let entry_node = graph.nodes().get(entry.index()).ok_or_else(|| {
        InspectionError::invalid(format!("{} CFG entry is out of range", closure.id))
    })?;
    if !matches!(entry_node.kind, FlowNodeKind::Entry) {
        return Err(InspectionError::invalid(format!(
            "{} CFG entry is not an entry node",
            closure.id
        )));
    }
    if graph
        .nodes()
        .iter()
        .filter(|node| matches!(node.kind, FlowNodeKind::Entry))
        .count()
        != 1
    {
        return Err(InspectionError::invalid(format!(
            "{} CFG does not contain exactly one entry node",
            closure.id
        )));
    }

    let mut nodes = Vec::with_capacity(graph.nodes().len());
    for (node_index, node) in graph.nodes().iter().enumerate() {
        if node.id.index() != node_index {
            return Err(InspectionError::invalid(format!(
                "{} CFG node identity at slot {node_index} is {}",
                closure.id,
                node.id.index()
            )));
        }
        if node.id == entry {
            if !node.predecessors.is_empty() {
                return Err(InspectionError::invalid(format!(
                    "{} CFG entry has a predecessor",
                    closure.id
                )));
            }
        } else if node.predecessors.is_empty() {
            return Err(InspectionError::invalid(format!(
                "{} has no predecessor",
                closure_control_flow_node_id(index, node_index)
            )));
        }

        let (kind, binding) = project_flow_node_kind(
            &node.kind,
            &closure.id,
            &binding_ids,
            bindings_by_id,
            &allowed_captures,
            module,
        )?;
        let mut incoming = node.predecessors.iter().collect::<Vec<_>>();
        incoming.sort_by_key(|edge| (edge.from.index(), flow_edge_rank(edge.kind)));
        if incoming
            .windows(2)
            .any(|edges| edges[0].from == edges[1].from && edges[0].kind == edges[1].kind)
        {
            return Err(InspectionError::invalid(format!(
                "{} contains a duplicate predecessor edge",
                closure_control_flow_node_id(index, node_index)
            )));
        }
        let predecessors = incoming
            .into_iter()
            .map(|edge| {
                if edge.from.index() >= graph.nodes().len() {
                    return Err(InspectionError::invalid(format!(
                        "{} has an out-of-range predecessor",
                        closure_control_flow_node_id(index, node_index)
                    )));
                }
                Ok(v2::FlowEdge {
                    from: closure_control_flow_node_id(index, edge.from.index()),
                    kind: project_flow_edge_kind(edge.kind),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        nodes.push(v2::FlowNode {
            id: closure_control_flow_node_id(index, node_index),
            kind,
            binding,
            predecessors,
            span: node
                .span
                .map(|span| document_span(source, span))
                .transpose()?,
        });
    }

    let actual_exits = graph
        .nodes()
        .iter()
        .filter_map(|node| matches!(node.kind, FlowNodeKind::Exit).then_some(node.id.index()))
        .collect::<BTreeSet<_>>();
    let declared_exits = graph
        .normal_exits()
        .iter()
        .map(|exit| exit.index())
        .collect::<BTreeSet<_>>();
    if declared_exits.len() != graph.normal_exits().len() || declared_exits != actual_exits {
        return Err(InspectionError::invalid(format!(
            "{} CFG normal exits do not exactly match exit nodes",
            closure.id
        )));
    }
    let normal_exits = declared_exits
        .into_iter()
        .map(|exit| closure_control_flow_node_id(index, exit))
        .collect();

    Ok(v5::ClosureControlFlowGraph {
        id: closure_control_flow_id(index),
        closure: closure.id.clone(),
        entry: closure_control_flow_node_id(index, entry.index()),
        bindings: binding_ids,
        normal_exits,
        nodes,
    })
}

fn project_function_control_flow(
    graph: &FunctionControlFlow,
    index: usize,
    program: &v1::Program,
    bindings_by_id: &BTreeMap<&str, &v1::Binding>,
    module: hir::ModuleId,
    source: &SourceFile,
) -> Result<v2::ControlFlowGraph, InspectionError> {
    let function = program.functions.get(index).ok_or_else(|| {
        InspectionError::invalid(format!("CFG at slot {index} has no corresponding function"))
    })?;
    if graph.function().module() != module
        || graph.function().index() != index
        || function.id != function_id(index)
    {
        return Err(InspectionError::invalid(format!(
            "CFG identity at slot {index} does not match {}",
            function.id
        )));
    }

    let expected_bindings = program
        .bindings
        .iter()
        .filter(|binding| binding.owner == function.id)
        .collect::<Vec<_>>();
    if graph.bindings().len() != expected_bindings.len() {
        return Err(InspectionError::invalid(format!(
            "{} CFG binding table has {} entries, expected {}",
            function.id,
            graph.bindings().len(),
            expected_bindings.len()
        )));
    }
    let mut binding_ids = Vec::with_capacity(graph.bindings().len());
    for (flow_binding, expected) in graph.bindings().iter().zip(expected_bindings) {
        if flow_binding.id.module() != module {
            return Err(InspectionError::invalid(format!(
                "{} CFG contains a binding from module:{} instead of module:{}",
                function.id,
                flow_binding.id.module().raw(),
                module.raw()
            )));
        }
        let id = binding_id(flow_binding.id.index());
        if id != expected.id
            || flow_binding.name != expected.name
            || document_span(source, flow_binding.span)? != expected.span
        {
            return Err(InspectionError::invalid(format!(
                "{} CFG metadata does not match its HIR binding",
                expected.id
            )));
        }
        binding_ids.push(id);
    }

    let entry = graph.entry();
    let entry_node = graph.nodes().get(entry.index()).ok_or_else(|| {
        InspectionError::invalid(format!("{} CFG entry is out of range", function.id))
    })?;
    if !matches!(entry_node.kind, FlowNodeKind::Entry) {
        return Err(InspectionError::invalid(format!(
            "{} CFG entry is not an entry node",
            function.id
        )));
    }
    if graph
        .nodes()
        .iter()
        .filter(|node| matches!(node.kind, FlowNodeKind::Entry))
        .count()
        != 1
    {
        return Err(InspectionError::invalid(format!(
            "{} CFG does not contain exactly one entry node",
            function.id
        )));
    }

    let graph_id = control_flow_id(index);
    let allowed_cross_owner = BTreeSet::new();
    let mut nodes = Vec::with_capacity(graph.nodes().len());
    for (node_index, node) in graph.nodes().iter().enumerate() {
        if node.id.index() != node_index {
            return Err(InspectionError::invalid(format!(
                "{} CFG node identity at slot {node_index} is {}",
                function.id,
                node.id.index()
            )));
        }
        if node.id == entry {
            if !node.predecessors.is_empty() {
                return Err(InspectionError::invalid(format!(
                    "{} CFG entry has a predecessor",
                    function.id
                )));
            }
        } else if node.predecessors.is_empty() {
            return Err(InspectionError::invalid(format!(
                "{} has no predecessor",
                control_flow_node_id(index, node_index)
            )));
        }

        let (kind, binding) = project_flow_node_kind(
            &node.kind,
            &function.id,
            &binding_ids,
            bindings_by_id,
            &allowed_cross_owner,
            module,
        )?;
        let mut incoming = node.predecessors.iter().collect::<Vec<_>>();
        incoming.sort_by_key(|edge| (edge.from.index(), flow_edge_rank(edge.kind)));
        if incoming
            .windows(2)
            .any(|edges| edges[0].from == edges[1].from && edges[0].kind == edges[1].kind)
        {
            return Err(InspectionError::invalid(format!(
                "{} contains a duplicate predecessor edge",
                control_flow_node_id(index, node_index)
            )));
        }
        let predecessors = incoming
            .into_iter()
            .map(|edge| {
                if edge.from.index() >= graph.nodes().len() {
                    return Err(InspectionError::invalid(format!(
                        "{} has an out-of-range predecessor",
                        control_flow_node_id(index, node_index)
                    )));
                }
                Ok(v2::FlowEdge {
                    from: control_flow_node_id(index, edge.from.index()),
                    kind: project_flow_edge_kind(edge.kind),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        nodes.push(v2::FlowNode {
            id: control_flow_node_id(index, node_index),
            kind,
            binding,
            predecessors,
            span: node
                .span
                .map(|span| document_span(source, span))
                .transpose()?,
        });
    }

    let actual_exits = graph
        .nodes()
        .iter()
        .filter_map(|node| matches!(node.kind, FlowNodeKind::Exit).then_some(node.id.index()))
        .collect::<BTreeSet<_>>();
    let declared_exits = graph
        .normal_exits()
        .iter()
        .map(|exit| exit.index())
        .collect::<BTreeSet<_>>();
    if declared_exits.len() != graph.normal_exits().len() || declared_exits != actual_exits {
        return Err(InspectionError::invalid(format!(
            "{} CFG normal exits do not exactly match exit nodes",
            function.id
        )));
    }
    let normal_exits = declared_exits
        .into_iter()
        .map(|exit| control_flow_node_id(index, exit))
        .collect();

    Ok(v2::ControlFlowGraph {
        id: graph_id,
        function: function.id.clone(),
        entry: control_flow_node_id(index, entry.index()),
        bindings: binding_ids,
        normal_exits,
        nodes,
    })
}

fn project_flow_node_kind(
    kind: &FlowNodeKind,
    owner: &str,
    flow_bindings: &[String],
    bindings_by_id: &BTreeMap<&str, &v1::Binding>,
    allowed_cross_owner: &BTreeSet<String>,
    module: hir::ModuleId,
) -> Result<(v2::FlowNodeKind, Option<String>), InspectionError> {
    if let FlowNodeKind::Initialize(binding) | FlowNodeKind::Read(binding) = kind {
        if binding.module() != module {
            return Err(InspectionError::invalid(format!(
                "CFG binding event belongs to module:{} instead of module:{}",
                binding.module().raw(),
                module.raw()
            )));
        }
    }
    let (kind, binding) = match kind {
        FlowNodeKind::Entry => (v2::FlowNodeKind::Entry, None),
        FlowNodeKind::Branch => (v2::FlowNodeKind::Branch, None),
        FlowNodeKind::Join => (v2::FlowNodeKind::Join, None),
        FlowNodeKind::Initialize(binding) => (
            v2::FlowNodeKind::Initialize,
            Some(binding_id(binding.index())),
        ),
        FlowNodeKind::Read(binding) => (v2::FlowNodeKind::Read, Some(binding_id(binding.index()))),
        FlowNodeKind::Transfer(FlowTransfer::Return) => (v2::FlowNodeKind::Return, None),
        FlowNodeKind::Transfer(FlowTransfer::Break) => (v2::FlowNodeKind::Break, None),
        FlowNodeKind::Transfer(FlowTransfer::Continue) => (v2::FlowNodeKind::Continue, None),
        FlowNodeKind::Exit => (v2::FlowNodeKind::Exit, None),
    };
    if let Some(binding) = &binding {
        let declaration = bindings_by_id
            .get(binding.as_str())
            .ok_or_else(|| InspectionError::invalid(format!("CFG references unknown {binding}")))?;
        if (declaration.owner != owner && !allowed_cross_owner.contains(binding))
            || !flow_bindings.contains(binding)
        {
            return Err(InspectionError::invalid(format!(
                "CFG reference to {binding} crosses callable ownership"
            )));
        }
    }
    Ok((kind, binding))
}

const fn project_flow_edge_kind(kind: FlowEdgeKind) -> v2::FlowEdgeKind {
    match kind {
        FlowEdgeKind::Execution => v2::FlowEdgeKind::Execution,
        FlowEdgeKind::Diagnostic => v2::FlowEdgeKind::Diagnostic,
        FlowEdgeKind::Backedge => v2::FlowEdgeKind::Backedge,
    }
}

const fn flow_edge_rank(kind: FlowEdgeKind) -> u8 {
    match kind {
        FlowEdgeKind::Execution => 0,
        FlowEdgeKind::Diagnostic => 1,
        FlowEdgeKind::Backedge => 2,
    }
}

fn document_span(source: &SourceFile, span: Span) -> Result<v1::Span, InspectionError> {
    if source.slice(span).is_none() {
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

fn module_id(id: hir::ModuleId) -> String {
    format!("module:{}", id.raw())
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

fn closure_id(index: usize) -> String {
    format!("closure:{index}")
}

fn binding_id(index: usize) -> String {
    format!("binding:{index}")
}

fn control_flow_id(function: usize) -> String {
    format!("cfg:function:{function}")
}

fn control_flow_node_id(function: usize, node: usize) -> String {
    format!("cfg:function:{function}.node:{node}")
}

fn closure_control_flow_id(closure: usize) -> String {
    format!("cfg:closure:{closure}")
}

fn closure_control_flow_node_id(closure: usize, node: usize) -> String {
    format!("cfg:closure:{closure}.node:{node}")
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
    use super::{
        build_document, build_document_v2, build_document_v3, render_json, render_json_v2,
        render_json_v3,
    };
    use nova_lexer::lex;
    use nova_parser::parse;
    use nova_sema::{AnalysisOutput, analyze, hir};
    use nova_source::{SourceFile, SourceId, Span};
    use std::collections::BTreeSet;

    fn checked_analysis(text: &str) -> (SourceFile, AnalysisOutput) {
        let source = SourceFile::new(SourceId::new(7), "sample\"name.nv", text);
        let lexed = lex(&source);
        assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
        let parsed = parse(&source, &lexed.tokens);
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        let analyzed = analyze(&parsed.program);
        assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
        (source, analyzed)
    }

    fn checked(text: &str) -> (SourceFile, hir::Program) {
        let (source, analyzed) = checked_analysis(text);
        (source, analyzed.program)
    }

    #[test]
    fn schema_v2_projects_every_verified_cfg_node_and_edge_category() {
        let (source, analyzed) = checked_analysis(
            "fn early(flag: Bool) -> Int { return 1; flag; 2 }\n\
             fn main(flag: Bool) -> Int {\n\
                 var value: Int;\n\
                 while true {\n\
                     if flag { value = early(flag); break; } else { continue; };\n\
                 }\n\
                 value\n\
             }",
        );
        let v1 = build_document(&analyzed.program, &source).expect("v1 should inspect");
        let v2 = build_document_v2(&analyzed, &source).expect("v2 should inspect");

        assert_eq!(v2.schema, "nova.semantic-inspection");
        assert_eq!(v2.schema_version, 2);
        assert_eq!(v2.program, v1.program);
        assert_eq!(v2.control_flow.len(), v2.program.functions.len());
        assert_eq!(v2.control_flow[0].function, "function:0");
        assert_eq!(v2.control_flow[1].bindings, ["binding:1", "binding:2"]);

        use super::v2::{FlowEdgeKind, FlowNodeKind};
        let nodes = v2
            .control_flow
            .iter()
            .flat_map(|graph| graph.nodes.iter())
            .collect::<Vec<_>>();
        for expected in [
            FlowNodeKind::Entry,
            FlowNodeKind::Branch,
            FlowNodeKind::Join,
            FlowNodeKind::Initialize,
            FlowNodeKind::Read,
            FlowNodeKind::Return,
            FlowNodeKind::Break,
            FlowNodeKind::Continue,
            FlowNodeKind::Exit,
        ] {
            assert!(
                nodes.iter().any(|node| node.kind == expected),
                "missing {expected:?}"
            );
        }
        for expected in [
            FlowEdgeKind::Execution,
            FlowEdgeKind::Diagnostic,
            FlowEdgeKind::Backedge,
        ] {
            assert!(
                nodes
                    .iter()
                    .any(|node| { node.predecessors.iter().any(|edge| edge.kind == expected) }),
                "missing {expected:?}"
            );
        }
        assert!(nodes.iter().any(|node| {
            node.kind == FlowNodeKind::Read && node.binding.as_deref() == Some("binding:1")
        }));

        let first = render_json_v2(&analyzed, &source).expect("v2 JSON should render");
        let second = render_json_v2(&analyzed, &source).expect("v2 JSON should be repeatable");
        assert_eq!(first, second);
        let parsed: serde_json::Value =
            serde_json::from_str(&first).expect("rendered v2 document is valid JSON");
        assert_eq!(parsed["schema_version"], 2);
    }

    #[test]
    fn schema_v2_rejects_failed_or_mismatched_analysis_output() {
        let source = SourceFile::new(
            SourceId::new(7),
            "sample\"name.nv",
            "fn main() -> Int { missing }",
        );
        let lexed = lex(&source);
        let parsed = parse(&source, &lexed.tokens);
        let rejected = analyze(&parsed.program);
        assert!(!rejected.is_success());
        let error = build_document_v2(&rejected, &source)
            .expect_err("rejected analysis must not produce tooling facts");
        assert!(error.message().contains("successful semantic analysis"));

        let (source, mut first) = checked_analysis("fn main() -> Int { let alpha = 1; alpha }");
        let (_, second) = checked_analysis("fn main() -> Int { let bravo = 1; bravo }");
        first.control_flow = second.control_flow;
        let error = build_document_v2(&first, &source)
            .expect_err("mismatched HIR and CFG metadata must fail closed");
        assert!(error.message().contains("does not match its HIR binding"));

        let (source, mut one_function) = checked_analysis("fn main() -> Int { 0 }");
        let (_, two_functions) =
            checked_analysis("fn helper() -> Int { 0 } fn main() -> Int { 0 }");
        one_function.control_flow = two_functions.control_flow;
        let error = build_document_v2(&one_function, &source)
            .expect_err("one graph per function is required");
        assert!(error.message().contains("one CFG per function"));
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
    fn rejects_same_typed_record_member_identity_drift() {
        let text = "record Pair { left: Int, right: Int }\n\
                    fn main() -> Int { let pair = new Pair { left: 1, right: 2 }; pair.left }";

        let (source, mut constructor) = checked(text);
        let hir::StatementKind::Binding { initializer, .. } =
            &mut constructor.functions[0].body.statements[0].kind
        else {
            panic!("expected record binding");
        };
        let hir::ExpressionKind::RecordLiteral { fields, .. } = &mut initializer.kind else {
            panic!("expected record literal");
        };
        fields[0].field_index = 1;
        fields[1].field_index = 0;
        let error = build_document(&constructor, &source)
            .expect_err("same-typed constructor retargeting must fail closed");
        assert!(
            error
                .message()
                .contains("record construction resolved field `left`")
        );

        let (source, mut projection) = checked(text);
        let field = projection.functions[0]
            .body
            .tail
            .as_deref_mut()
            .expect("main tail");
        let hir::ExpressionKind::FieldAccess { field_index, .. } = &mut field.kind else {
            panic!("expected field access");
        };
        *field_index = 1;
        let error = build_document(&projection, &source)
            .expect_err("same-typed projection retargeting must fail closed");
        assert!(
            error
                .message()
                .contains("field access resolved field `left`")
        );
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
        let foreign = match &cross_owner.functions[0].body.statements[0].kind {
            hir::StatementKind::Binding { binding, .. } => binding.clone(),
            _ => panic!("expected foreign binding"),
        };
        let second_tail = cross_owner.functions[1]
            .body
            .tail
            .as_deref_mut()
            .expect("second has a tail expression");
        second_tail.kind = hir::ExpressionKind::Binding(hir::BindingReference {
            binding: foreign.id,
            binding_name: foreign.name,
            declaration_span: foreign.span,
        });
        let error =
            build_document(&cross_owner, &source).expect_err("cross-owner use must fail closed");
        assert!(
            error
                .message()
                .contains("crosses callable ownership without a capture")
        );

        let (source, mut escaped_scope) = checked(
            "fn main() -> Int {\n\
                 { let hidden = 1; hidden; };\n\
                 2\n\
             }",
        );
        let hidden = match &escaped_scope.functions[0].body.statements[0].kind {
            hir::StatementKind::Expression(expression) => {
                let hir::ExpressionKind::Block(block) = &expression.kind else {
                    panic!("expected nested block");
                };
                match &block.statements[0].kind {
                    hir::StatementKind::Binding { binding, .. } => binding.clone(),
                    _ => panic!("expected hidden binding"),
                }
            }
            _ => panic!("expected block expression statement"),
        };
        let tail = escaped_scope.functions[0]
            .body
            .tail
            .as_deref_mut()
            .expect("main has a tail expression");
        tail.kind = hir::ExpressionKind::Binding(hir::BindingReference {
            binding: hidden.id,
            binding_name: hidden.name,
            declaration_span: hidden.span,
        });
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
        let fixed = match &program.functions[0].body.statements[0].kind {
            hir::StatementKind::Binding { binding, .. } => binding.clone(),
            _ => panic!("expected immutable binding"),
        };
        let hir::StatementKind::Assignment { target, .. } =
            &mut program.functions[0].body.statements[2].kind
        else {
            panic!("expected assignment HIR");
        };
        *target = Some(hir::BindingReference {
            binding: fixed.id,
            binding_name: fixed.name,
            declaration_span: fixed.span,
        });

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

    #[test]
    fn published_json_schema_is_well_formed_and_names_v2() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/semantic-inspection-v2.schema.json"
        ))
        .expect("published schema must be valid JSON");

        assert_eq!(schema["$id"], "urn:nova:semantic-inspection:v2");
        assert_eq!(
            schema["properties"]["schema"]["const"],
            "nova.semantic-inspection"
        );
        assert_eq!(schema["properties"]["schema_version"]["const"], 2);
        assert_eq!(
            schema["properties"]["program"]["$ref"],
            "urn:nova:semantic-inspection:v1#/$defs/program"
        );

        let (source, analyzed) = checked_analysis("fn main() -> Unit {}");
        let document = serde_json::to_value(
            build_document_v2(&analyzed, &source).expect("valid analysis should inspect"),
        )
        .expect("document should serialize");

        assert_required_keys(&schema, &document);
        assert_required_keys(
            &schema["$defs"]["controlFlowGraph"],
            &document["control_flow"][0],
        );
        assert_required_keys(
            &schema["$defs"]["flowNode"],
            &document["control_flow"][0]["nodes"][0],
        );
        assert_required_keys(
            &schema["$defs"]["flowEdge"],
            &document["control_flow"][0]["nodes"][1]["predecessors"][0],
        );
    }

    #[test]
    fn published_json_schema_is_well_formed_and_names_v3() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/semantic-inspection-v3.schema.json"
        ))
        .expect("published schema must be valid JSON");

        assert_eq!(schema["$id"], "urn:nova:semantic-inspection:v3");
        assert_eq!(schema["properties"]["schema_version"]["const"], 3);
        assert_eq!(
            schema["properties"]["program"]["$ref"],
            "urn:nova:semantic-inspection:v1#/$defs/program"
        );
        assert_eq!(
            schema["properties"]["control_flow"]["items"]["$ref"],
            "urn:nova:semantic-inspection:v2#/$defs/controlFlowGraph"
        );

        let (source, analyzed) = checked_analysis(
            "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(1) { Maybe::None => 0, Maybe::Some(_) => 1 } }",
        );
        let document = serde_json::to_value(
            build_document_v3(&analyzed, &source).expect("valid v3 analysis should inspect"),
        )
        .expect("document should serialize");
        assert_required_keys(&schema, &document);
        assert_required_keys(
            &schema["$defs"]["matchPattern"],
            &document["match_patterns"][0],
        );
        assert_eq!(document["match_patterns"][1]["payload_mode"], "discard");

        let first = render_json_v3(&analyzed, &source).expect("v3 JSON should render");
        let second = render_json_v3(&analyzed, &source).expect("v3 JSON should repeat");
        assert_eq!(first, second);
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
