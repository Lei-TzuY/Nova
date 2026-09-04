use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_sema::hir::{ExpressionKind, StatementKind, Type};
use nova_source::{SourceFile, SourceId};

fn accepted(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closures.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn executes_an_escaping_capture_and_nested_closure() {
    let analyzed = accepted(
        "fn make(base: Int) -> fn(Int) -> Int { fn(value: Int) -> Int { base + value } }\n\
         fn wrap(base: Int) -> fn() -> fn() -> Int {\n\
             fn() -> fn() -> Int { fn() -> Int { base } }\n\
         }\n\
         fn main() -> Int { make(40)(2) + wrap(0)()() }",
    );
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(42)));
}

#[test]
fn closure_equality_uses_runtime_instance_identity() {
    let analyzed = accepted(
        "fn named(value: Int) -> Int { value }\n\
         fn main() -> Bool {\n\
             let first = fn(value: Int) -> Int { value };\n\
             let alias = first;\n\
             (first == alias) &&\n\
             (first != fn(value: Int) -> Int { value }) &&\n\
             (named != first)\n\
         }",
    );
    assert_eq!(execute(&analyzed.program), Ok(Value::Bool(true)));
}

#[test]
fn malformed_capture_type_fails_closed_before_retargeting_runtime_behavior() {
    let mut analyzed = accepted(
        "fn main() -> Int { let base = 40; let add = fn(value: Int) -> Int { base + value }; add(2) }",
    );
    let StatementKind::Binding { initializer, .. } =
        &mut analyzed.program.functions[0].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &mut initializer.kind else {
        panic!("closure initializer");
    };
    closure.captures[0].ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("malformed capture must fail closed");
    assert_eq!(error.code, "N4005");
    assert!(
        error
            .labels
            .iter()
            .any(|label| label.message.contains("capture") || label.message.contains("runtime")),
        "{error:?}"
    );
}

#[test]
fn missing_capture_cannot_silently_read_an_unrelated_frame_slot() {
    let mut analyzed = accepted(
        "fn main() -> Int { let base = 40; let add = fn(value: Int) -> Int { base + value }; add(2) }",
    );
    let StatementKind::Binding { initializer, .. } =
        &mut analyzed.program.functions[0].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &mut initializer.kind else {
        panic!("closure initializer");
    };
    closure.captures.clear();

    let error = execute(&analyzed.program).expect_err("missing capture must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn mutable_outer_capture_is_a_creation_time_snapshot() {
    let analyzed = accepted(
        "fn main() -> Int { var value = 40; let get = fn() -> Int { value }; value = 99; get() }",
    );
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(40)));
}

#[test]
fn nested_mutable_outer_capture_keeps_the_original_snapshot() {
    let analyzed = accepted(
        "fn main() -> Int { var value = 40; let outer = fn() -> fn() -> Int { fn() -> Int { value } }; value = 99; outer()() }",
    );
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(40)));
}

#[test]
fn mutable_snapshot_respects_lexical_shadowing() {
    let analyzed = accepted(
        "fn main() -> Int { var value = 1; let get = { var value = 2; fn() -> Int { value } }; value = 3; get() }",
    );
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(2)));
}

#[test]
fn malformed_assignment_through_snapshot_capture_fails_closed() {
    let mut analyzed = accepted(
        "fn main() -> Int { var value = 40; let update = fn() -> Int { var local = 0; local = value; local }; update() }",
    );
    let StatementKind::Binding { initializer, .. } =
        &mut analyzed.program.functions[0].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &mut initializer.kind else {
        panic!("closure initializer");
    };
    let captured = closure.captures[0].reference.clone();
    let StatementKind::Assignment { target, .. } = &mut closure.body.statements[1].kind else {
        panic!("closure assignment");
    };
    *target = Some(captured);

    let error = execute(&analyzed.program)
        .expect_err("assignment through an immutable snapshot slot must fail closed");
    assert_eq!(error.code, "N4005");
}
