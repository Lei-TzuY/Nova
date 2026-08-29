from pathlib import Path

lib = Path("crates/nova-interpreter/src/lib.rs")
text = lib.read_text()

record_old = '''                    let value = match self.eval_expression(&field.value, frame)? {
                        Flow::Value(value) => value,
                        flow => return Ok(flow),
                    };
                    let Some(slot) = slots.get_mut(field.field_index) else {'''
record_new = '''                    let value = match self.eval_expression(&field.value, frame)? {
                        Flow::Value(value) => value,
                        flow => return Ok(flow),
                    };
                    let Some(field_definition) = definition.fields.get(field.field_index) else {
                        return Err(self.invariant(
                            expression.span,
                            format!(
                                "record initializer targets field slot {} outside record `{}`",
                                field.field_index, definition.name
                            ),
                        ));
                    };
                    if !self.value_conforms_to_type(&value, &field_definition.ty) {
                        return Err(self.invariant(
                            field.value.span,
                            format!(
                                "record field `{}` of `{}` received a runtime value that does not conform to declared type {}",
                                field_definition.name, definition.name, field_definition.ty
                            ),
                        ));
                    }
                    let Some(slot) = slots.get_mut(field.field_index) else {'''
if record_old not in text:
    raise SystemExit("record constructor anchor not found")
text = text.replace(record_old, record_new, 1)

enum_old = '''                let payload = if let Some(payload) = payload {
                    match self.eval_expression(payload, frame)? {
                        Flow::Value(value) => Some(Box::new(value)),
                        flow => return Ok(flow),
                    }
                } else {
                    None
                };
                Ok(Flow::Value(Value::Enum {'''
enum_new = '''                let payload = if let Some(payload) = payload {
                    match self.eval_expression(payload, frame)? {
                        Flow::Value(value) => Some(Box::new(value)),
                        flow => return Ok(flow),
                    }
                } else {
                    None
                };
                if let (Some(payload_type), Some(payload_value)) =
                    (&variant.payload, payload.as_deref())
                {
                    if !self.value_conforms_to_type(payload_value, payload_type) {
                        return Err(self.invariant(
                            expression.span,
                            format!(
                                "enum variant `{}` of `{}` received a runtime payload that does not conform to declared type {}",
                                variant.name, definition.name, payload_type
                            ),
                        ));
                    }
                }
                Ok(Flow::Value(Value::Enum {'''
if enum_old not in text:
    raise SystemExit("enum constructor anchor not found")
text = text.replace(enum_old, enum_new, 1)
lib.write_text(text)

tests = Path("crates/nova-interpreter/tests/aggregate_boundary_integrity.rs")
tests.write_text('''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "aggregate-boundary.nv", text);
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
    let analyzed = analyze(&parsed.program);
    assert!(
        analyzed.is_success(),
        "semantic diagnostics: {:?}",
        analyzed.diagnostics
    );
    analyzed
}

#[test]
fn rejects_malformed_record_field_even_when_value_is_discarded_locally() {
    let mut analyzed = analyze_text(
        "record Box { value: Int } fn main() -> Unit { new Box { value: 42 }; }",
    );
    let main = analyzed.program.functions.iter_mut().find(|function| function.name == "main").unwrap();
    let statement = &mut main.body.statements[0];
    let nova_sema::hir::StatementKind::Expression(expression) = &mut statement.kind else {
        panic!("expected expression statement");
    };
    let ExpressionKind::RecordLiteral { fields, .. } = &mut expression.kind else {
        panic!("expected record literal");
    };
    fields[0].value.kind = ExpressionKind::Boolean(true);
    fields[0].value.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("record field drift must fail at construction");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_malformed_enum_payload_even_when_value_is_discarded_locally() {
    let mut analyzed = analyze_text(
        "enum Maybe { Some(Int), None } fn main() -> Unit { Maybe::Some(42); }",
    );
    let main = analyzed.program.functions.iter_mut().find(|function| function.name == "main").unwrap();
    let statement = &mut main.body.statements[0];
    let nova_sema::hir::StatementKind::Expression(expression) = &mut statement.kind else {
        panic!("expected expression statement");
    };
    let ExpressionKind::EnumConstructor { payload, .. } = &mut expression.kind else {
        panic!("expected enum constructor");
    };
    let payload = payload.as_deref_mut().expect("payload");
    payload.kind = ExpressionKind::Boolean(true);
    payload.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("enum payload drift must fail at construction");
    assert_eq!(error.code, "N4005");
}

#[test]
fn recursively_rejects_nested_nominal_record_identity_drift_at_construction() {
    let mut analyzed = analyze_text(
        "record Left { value: Int } record Right { value: Int } record Outer { inner: Left } fn main() -> Unit { new Outer { inner: new Left { value: 42 } }; }",
    );
    let right = analyzed.program.records[1].id;
    let main = analyzed.program.functions.iter_mut().find(|function| function.name == "main").unwrap();
    let statement = &mut main.body.statements[0];
    let nova_sema::hir::StatementKind::Expression(expression) = &mut statement.kind else {
        panic!("expected expression statement");
    };
    let ExpressionKind::RecordLiteral { fields, .. } = &mut expression.kind else {
        panic!("expected outer record literal");
    };
    let ExpressionKind::RecordLiteral { record, .. } = &mut fields[0].value.kind else {
        panic!("expected nested record literal");
    };
    *record = right;

    let error = execute(&analyzed.program).expect_err("nested nominal drift must fail at construction");
    assert_eq!(error.code, "N4005");
}

#[test]
fn valid_local_aggregates_remain_executable() {
    let analyzed = analyze_text(
        "record Box { value: Int } enum Maybe { Some(Int), None } fn main() -> Unit { new Box { value: 42 }; Maybe::Some(7); }",
    );
    let value = execute(&analyzed.program).expect("valid aggregate construction should execute");
    assert_eq!(value, Value::Unit);
}
''')

readme = Path("README.md")
text = readme.read_text()
anchor = '''Function calls also validate the runtime/HIR type boundary. Every argument must
recursively conform to its resolved parameter type, and every returned runtime
value must recursively conform to the function's declared type. Nominal record
and enum identities, record slots, and enum payloads are checked rather than
trusted from their outer value tag alone. Valid semantically produced HIR is
unaffected; malformed or contract-drifted HIR fails closed with `N4005`.
'''
replacement = anchor + '''
Aggregate construction enforces the same invariant locally: each evaluated record
field must conform to its declaration slot type, and each enum payload must conform
to its selected variant payload type before the aggregate value is created. This
catches malformed HIR even when the aggregate never crosses a function boundary.
'''
if anchor not in text:
    raise SystemExit("README runtime boundary anchor not found")
readme.write_text(text.replace(anchor, replacement, 1))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
status_old = "**Status: eleven vertical slices implemented; execution surface remains small.**"
if status_old not in text:
    raise SystemExit("roadmap Phase 3 status anchor not found")
text = text.replace(
    status_old,
    "**Status: twelve vertical slices implemented; execution surface remains small.**",
    1,
)
anchor = '''- malformed-HIR regressions lock argument, return, nested-record, and nominal
  identity drift to deterministic invariant diagnostic `N4005` without changing
  valid source behavior.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
contracts:
'''
insertion = '''- malformed-HIR regressions lock argument, return, nested-record, and nominal
  identity drift to deterministic invariant diagnostic `N4005` without changing
  valid source behavior.

Implemented in the twelfth Phase 3 slice:

- record construction validates each evaluated runtime field against the declared
  destination slot type before storing it in the aggregate value;
- enum construction validates an evaluated payload against the selected variant's
  declared payload type before creating the runtime enum value;
- validation reuses the recursive runtime/HIR conformance contract, so nested
  nominal record/enum identity and payload drift also fail closed at construction;
- malformed aggregates are rejected with invariant diagnostic `N4005` even when
  their values are discarded locally and never cross a function boundary; and
- adversarial malformed-HIR regressions plus a valid local-aggregate control case
  lock the new construction boundary without changing accepted source behavior.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
contracts:
'''
if anchor not in text:
    raise SystemExit("roadmap Phase 3 slice anchor not found")
roadmap.write_text(text.replace(anchor, insertion, 1))
