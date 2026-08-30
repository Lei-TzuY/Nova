from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected exactly one anchor in {path}: {old[:80]!r}, found {text.count(old)}")
    file.write_text(text.replace(old, new, 1))


# HIR keeps the resolved member spelling independently from the declaration-order slot.
replace_once(
    "crates/nova-sema/src/hir.rs",
    """pub struct RecordFieldValue {\n    /// Zero-based destination slot in the record's declaration order.\n    pub field_index: usize,\n    /// Typed initializer expression, evaluated in source order.\n    pub value: Expression,\n}""",
    """pub struct RecordFieldValue {\n    /// Resolved field spelling paired with the destination slot.\n    pub field_name: String,\n    /// Zero-based destination slot in the record's declaration order.\n    pub field_index: usize,\n    /// Typed initializer expression, evaluated in source order.\n    pub value: Expression,\n}""",
)
replace_once(
    "crates/nova-sema/src/hir.rs",
    """    FieldAccess {\n        /// Typed base expression.\n        base: Box<Expression>,\n        /// Nominal record identity expected at runtime.\n        record: RecordId,\n        /// Zero-based field slot in declaration order.\n        field_index: usize,\n    },""",
    """    FieldAccess {\n        /// Typed base expression.\n        base: Box<Expression>,\n        /// Nominal record identity expected at runtime.\n        record: RecordId,\n        /// Resolved field spelling paired with the declaration-order slot.\n        field_name: String,\n        /// Zero-based field slot in declaration order.\n        field_index: usize,\n    },""",
)

# Semantic lowering records both halves of the resolved member identity.
replace_once(
    "crates/nova-sema/src/analyzer.rs",
    "            resolved.push(RecordFieldValue { field_index, value });",
    """            resolved.push(RecordFieldValue {\n                field_name: field.name.text.clone(),\n                field_index,\n                value,\n            });""",
)
replace_once(
    "crates/nova-sema/src/analyzer.rs",
    """            ExpressionKind::FieldAccess {\n                base: Box::new(base),\n                record: record_type.id,\n                field_index,\n            },""",
    """            ExpressionKind::FieldAccess {\n                base: Box::new(base),\n                record: record_type.id,\n                field_name: field.text.clone(),\n                field_index,\n            },""",
)

# Interpreter: validate the resolved name/slot pair whenever the expression actually produces a value.
replace_once(
    "crates/nova-interpreter/src/lib.rs",
    """                    let Some(field_definition) = definition.fields.get(field.field_index) else {\n                        return Err(self.invariant(\n                            expression.span,\n                            format!(\n                                \"record initializer targets field slot {} outside record `{}`\",\n                                field.field_index, definition.name\n                            ),\n                        ));\n                    };""",
    """                    let field_definition = self.resolved_record_field(\n                        *record,\n                        field.field_index,\n                        &field.field_name,\n                        field.value.span,\n                    )?;""",
)
replace_once(
    "crates/nova-interpreter/src/lib.rs",
    """            ExpressionKind::FieldAccess {\n                base,\n                record,\n                field_index,\n            } => {""",
    """            ExpressionKind::FieldAccess {\n                base,\n                record,\n                field_name,\n                field_index,\n            } => {""",
)
replace_once(
    "crates/nova-interpreter/src/lib.rs",
    """                if actual != *record {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"field access expected record {}, found record {}\",\n                            record.index(),\n                            actual.index()\n                        ),\n                    ));\n                }\n                let Some(value) = fields.get(*field_index).cloned() else {""",
    """                if actual != *record {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"field access expected record {}, found record {}\",\n                            record.index(),\n                            actual.index()\n                        ),\n                    ));\n                }\n                let field_definition = self.resolved_record_field(\n                    *record,\n                    *field_index,\n                    field_name,\n                    expression.span,\n                )?;\n                if expression.ty != field_definition.ty {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"field access result type {} does not match resolved field `{}` type {}\",\n                            expression.ty, field_name, field_definition.ty\n                        ),\n                    ));\n                }\n                let Some(value) = fields.get(*field_index).cloned() else {""",
)
replace_once(
    "crates/nova-interpreter/src/lib.rs",
    """    fn equality_types_match(&self, left: &Type, right: &Type) -> bool {""",
    """    fn resolved_record_field(\n        &self,\n        record: RecordId,\n        field_index: usize,\n        field_name: &str,\n        span: nova_source::Span,\n    ) -> Result<&nova_sema::hir::RecordField, Diagnostic> {\n        let Some(definition) = self.program.records.get(record.index()) else {\n            return Err(self.invariant(\n                span,\n                format!(\"resolved record id {} is outside the program\", record.index()),\n            ));\n        };\n        if definition.id != record {\n            return Err(self.invariant(\n                span,\n                \"record declaration index does not match its resolved identity\",\n            ));\n        }\n        let Some(field) = definition.fields.get(field_index) else {\n            return Err(self.invariant(\n                span,\n                format!(\n                    \"resolved field `{field_name}` targets slot {field_index} outside record `{}`\",\n                    definition.name\n                ),\n            ));\n        };\n        if field.name != field_name {\n            return Err(self.invariant(\n                span,\n                format!(\n                    \"resolved field `{field_name}` targets slot {field_index}, declared as `{}` in record `{}`\",\n                    field.name, definition.name\n                ),\n            ));\n        }\n        Ok(field)\n    }\n\n    fn equality_types_match(&self, left: &Type, right: &Type) -> bool {""",
)

# Semantic inspection independently cross-checks the HIR-only name against the declaration slot.
replace_once(
    "crates/nova-inspect/src/lib.rs",
    """                for field in fields {\n                    if field.field_index >= declared_field_count {\n                        return Err(InspectionError::invalid(format!(\n                            \"record construction references out-of-range field slot {}\",\n                            field.field_index\n                        )));\n                    }\n                    if !seen.insert(field.field_index) {""",
    """                for field in fields {\n                    if field.field_index >= declared_field_count {\n                        return Err(InspectionError::invalid(format!(\n                            \"record construction references out-of-range field slot {}\",\n                            field.field_index\n                        )));\n                    }\n                    let declared_name = self.require_record(*record)?.fields[field.field_index]\n                        .name\n                        .clone();\n                    if declared_name != field.field_name {\n                        return Err(InspectionError::invalid(format!(\n                            \"record construction resolved field `{}` to slot {}, declared as `{declared_name}`\",\n                            field.field_name, field.field_index\n                        )));\n                    }\n                    if !seen.insert(field.field_index) {""",
)
replace_once(
    "crates/nova-inspect/src/lib.rs",
    """            hir::ExpressionKind::FieldAccess {\n                base,\n                record,\n                field_index,\n            } => {\n                let declaration = self.require_record(*record)?;\n                if *field_index >= declaration.fields.len() {\n                    return Err(InspectionError::invalid(format!(\n                        \"field access references out-of-range field slot {field_index}\"\n                    )));\n                }\n                children.push(self.collect_expression(base, owner)?);""",
    """            hir::ExpressionKind::FieldAccess {\n                base,\n                record,\n                field_name,\n                field_index,\n            } => {\n                let declaration = self.require_record(*record)?;\n                if *field_index >= declaration.fields.len() {\n                    return Err(InspectionError::invalid(format!(\n                        \"field access references out-of-range field slot {field_index}\"\n                    )));\n                }\n                let declared_name = declaration.fields[*field_index].name.clone();\n                if declared_name != *field_name {\n                    return Err(InspectionError::invalid(format!(\n                        \"field access resolved field `{field_name}` to slot {field_index}, declared as `{declared_name}`\"\n                    )));\n                }\n                if expression.ty != declaration.fields[*field_index].ty {\n                    return Err(InspectionError::invalid(format!(\n                        \"field access type {} does not match resolved field `{field_name}` type {}\",\n                        expression.ty, declaration.fields[*field_index].ty\n                    )));\n                }\n                children.push(self.collect_expression(base, owner)?);""",
)

# Add inspector corruption coverage near the existing invariant tests.
inspect_test_anchor = """    #[test]\n    fn rejects_non_contiguous_declaration_identities() {"""
inspect_test = r'''    #[test]
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
        assert!(error.message().contains("record construction resolved field `left`"));

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
        assert!(error.message().contains("field access resolved field `left`"));
    }

'''
replace_once(
    "crates/nova-inspect/src/lib.rs",
    inspect_test_anchor,
    inspect_test + inspect_test_anchor,
)

# Interpreter corruption coverage: the same HIR retargeting must not silently change values.
Path("crates/nova-interpreter/tests/record_field_identity.rs").write_text(r'''use nova_interpreter::execute;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "record-field-identity.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "parse diagnostics: {:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(
        analyzed.is_success(),
        "semantic diagnostics: {:?}",
        analyzed.diagnostics
    );
    analyzed
}

const SOURCE: &str = "record Pair { left: Int, right: Int }\n\
                      fn main() -> Int { let pair = new Pair { left: 1, right: 2 }; pair.left }";

#[test]
fn malformed_same_typed_constructor_slot_retargeting_fails_closed() {
    let mut analyzed = analyze_text(SOURCE);
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Binding { initializer, .. } = &mut main.body.statements[0].kind else {
        panic!("expected record binding");
    };
    let ExpressionKind::RecordLiteral { fields, .. } = &mut initializer.kind else {
        panic!("expected record literal");
    };
    fields[0].field_index = 1;
    fields[1].field_index = 0;

    let error = execute(&analyzed.program)
        .expect_err("same-typed constructor retargeting must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn malformed_same_typed_projection_slot_retargeting_fails_closed() {
    let mut analyzed = analyze_text(SOURCE);
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let expression = main.body.tail.as_deref_mut().expect("main tail");
    let ExpressionKind::FieldAccess { field_index, .. } = &mut expression.kind else {
        panic!("expected field access");
    };
    *field_index = 1;

    let error = execute(&analyzed.program)
        .expect_err("same-typed projection retargeting must fail closed");
    assert_eq!(error.code, "N4005");
}
''')
