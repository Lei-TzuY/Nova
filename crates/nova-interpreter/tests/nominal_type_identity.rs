use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, FunctionType, RecordType, StatementKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "nominal-type-identity.nv", text);
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

fn drifted_record_type(id: nova_sema::hir::RecordId) -> Type {
    Type::Record(RecordType {
        id,
        name: "Ghost".to_owned(),
    })
}

#[test]
fn rejects_expression_nominal_name_drift_with_the_same_record_id() {
    let mut analyzed = analyze_text(
        "record Box { value: Int } fn main() -> Unit { let item = new Box { value: 1 }; }",
    );
    let record = analyzed.program.records[0].id;
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Binding { initializer, .. } = &mut main.body.statements[0].kind else {
        panic!("expected binding statement");
    };
    initializer.ty = drifted_record_type(record);

    let error = execute(&analyzed.program).expect_err("nominal spelling drift must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_nested_record_field_type_name_drift() {
    let mut analyzed = analyze_text(
        "record Inner { value: Int } record Outer { inner: Inner } fn main() -> Unit { let wrapped = new Outer { inner: new Inner { value: 1 } }; }",
    );
    let inner = analyzed.program.records[0].id;
    analyzed.program.records[1].fields[0].ty = drifted_record_type(inner);

    let error = execute(&analyzed.program).expect_err("nested record type drift must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_enum_payload_type_name_drift() {
    let mut analyzed = analyze_text(
        "record Payload { value: Int } enum Maybe { Some(Payload), None } fn main() -> Unit { let value = Maybe::Some(new Payload { value: 1 }); }",
    );
    let payload = analyzed.program.records[0].id;
    analyzed.program.enums[0].variants[0].payload = Some(drifted_record_type(payload));

    let error = execute(&analyzed.program).expect_err("enum payload type drift must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_nominal_name_drift_nested_in_a_function_signature() {
    let mut analyzed = analyze_text(
        "record Payload { value: Int } fn take(value: Payload) -> Unit {} fn main() -> Unit { let alias = take; }",
    );
    let payload = analyzed.program.records[0].id;
    let ghost = drifted_record_type(payload);
    let signature = Type::Function(FunctionType {
        parameters: vec![ghost.clone()],
        return_type: Box::new(Type::Unit),
    });

    let take = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "take")
        .expect("take function");
    take.parameters[0].ty = ghost;

    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Binding {
        binding,
        initializer,
    } = &mut main.body.statements[0].kind
    else {
        panic!("expected alias binding");
    };
    binding.ty = signature.clone();
    initializer.ty = signature;
    assert!(matches!(initializer.kind, ExpressionKind::Function { .. }));

    let error = execute(&analyzed.program).expect_err("nested signature drift must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn valid_nested_nominal_types_remain_executable() {
    let analyzed = analyze_text(
        "record Inner { value: Int } record Outer { inner: Inner } enum Maybe { Some(Outer), None } fn main() -> Int { let wrapped = new Outer { inner: new Inner { value: 7 } }; let value = Maybe::Some(wrapped); match value { Maybe::Some(found) => found.inner.value, Maybe::None => 0 } }",
    );
    let value = execute(&analyzed.program).expect("valid nominal types should execute");
    assert_eq!(value, Value::Int(7));
}
