use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::control_flow::FlowNodeKind;
use nova_sema::hir::{CaptureMode, ExpressionKind, StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closures.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn lowers_an_escaping_immutable_capture_with_its_own_verified_cfg() {
    let output = analyze_text(
        "fn make(base: Int) -> fn(Int) -> Int { fn(value: Int) -> Int { base + value } }\n\
         fn main() -> Int { make(40)(2) }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let closure_expression = output.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("make tail");
    let ExpressionKind::Closure(closure) = &closure_expression.kind else {
        panic!("make should return a closure");
    };
    assert_eq!(closure.id.index(), 0);
    assert_eq!(closure.parameters.len(), 1);
    assert_eq!(closure.captures.len(), 1);
    assert_eq!(closure.captures[0].reference.binding_name, "base");
    assert_eq!(closure.captures[0].ty, Type::Int);
    assert_eq!(output.control_flow.closures().len(), 1);
    let graph = &output.control_flow.closures()[0];
    assert_eq!(graph.closure().index(), 0);
    assert!(graph.nodes().iter().any(|node| {
        matches!(node.kind, FlowNodeKind::Read(binding) if binding == closure.captures[0].reference.binding)
    }));
}

#[test]
fn nested_closure_propagates_a_transitive_capture_to_its_creator() {
    let output = analyze_text(
        "fn make(base: Int) -> fn() -> fn() -> Int {\n\
             fn() -> fn() -> Int { fn() -> Int { base } }\n\
         }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let ExpressionKind::Closure(outer) = &output.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("outer closure")
        .kind
    else {
        panic!("expected outer closure");
    };
    let ExpressionKind::Closure(inner) = &outer.body.tail.as_deref().expect("inner closure").kind
    else {
        panic!("expected nested closure");
    };
    assert_eq!(outer.captures.len(), 1);
    assert_eq!(inner.captures.len(), 1);
    assert_eq!(
        outer.captures[0].reference.binding,
        inner.captures[0].reference.binding
    );
    assert_eq!(outer.id.index(), 0);
    assert_eq!(inner.id.index(), 1);
    assert_eq!(output.control_flow.closures().len(), 2);
}

#[test]
fn mutable_outer_reads_lower_as_creation_time_snapshot_captures() {
    let output = analyze_text(
        "fn main() -> Int { var value = 40; let get = fn() -> Int { value }; value = 99; get() }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let StatementKind::Binding { initializer, .. } =
        &output.program.functions[0].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &initializer.kind else {
        panic!("closure initializer");
    };
    assert_eq!(closure.captures.len(), 1);
    assert_eq!(closure.captures[0].reference.binding_name, "value");
    assert_eq!(closure.captures[0].ty, Type::Int);
}

#[test]
fn mutable_outer_write_upgrades_capture_to_by_reference() {
    let output = analyze_text(
        "fn main() -> Int { var value = 40; let set = fn() -> Int { value = 99; value }; set() }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let StatementKind::Binding { initializer, .. } =
        &output.program.functions[0].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &initializer.kind else {
        panic!("closure initializer");
    };
    assert_eq!(closure.captures.len(), 1);
    assert_eq!(closure.captures[0].mode, CaptureMode::ByReference);
}

#[test]
fn nested_closure_propagates_a_mutable_snapshot_capture_transitively() {
    let output = analyze_text(
        "fn main() -> Int { var value = 40; let outer = fn() -> fn() -> Int { fn() -> Int { value } }; value = 99; outer()() }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn mutable_snapshot_is_read_at_creation_for_definite_initialization() {
    let output = analyze_text(
        "fn main() -> Int { var value: Int; let get = fn() -> Int { value }; value = 42; get() }",
    );
    let uninitialized = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "N3009")
        .count();
    assert_eq!(uninitialized, 1, "{:?}", output.diagnostics);
}

#[test]
fn write_capture_does_not_initialize_outer_binding_before_call() {
    let output = analyze_text(
        "fn main() -> Int { var value: Int; let set = fn() -> Unit { value = 1; }; value }",
    );
    let uninitialized = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "N3009")
        .count();
    assert_eq!(uninitialized, 1, "{:?}", output.diagnostics);
}

#[test]
fn by_reference_assignment_preserves_rhs_initialization() {
    let output = analyze_text(
        "fn main() -> Int { var outer = 0; let set = fn() -> Int { var local: Int; outer = { local = 1; 0 }; local }; set() }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn by_reference_assignment_preserves_noncontinuing_rhs_flow() {
    let output = analyze_text(
        "fn stop() -> ! { while true {} }
\
         fn main() -> Int { var outer = 0; let set = fn() -> Int { outer = stop(); 42 }; 0 }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let StatementKind::Binding { initializer, .. } =
        &output.program.functions[1].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &initializer.kind else {
        panic!("closure initializer");
    };
    assert_eq!(closure.body.ty, Type::Never);
    let StatementKind::Assignment { value, .. } = &closure.body.statements[0].kind else {
        panic!("by-reference assignment");
    };
    assert_eq!(value.ty, Type::Never);
}

#[test]
fn closure_initializer_cannot_self_reference_before_its_binding_exists() {
    let output = analyze_text(
        "fn main() -> Int { let recurse: fn() -> Int = fn() -> Int { recurse() }; 0 }",
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3003"),
        "{:?}",
        output.diagnostics
    );
}

#[test]
fn closure_is_a_return_and_loop_control_boundary() {
    let output = analyze_text(
        "fn main() -> Int { while true { let invalid = fn() -> Int { break; 1 }; break; } 0 }",
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3013"),
        "{:?}",
        output.diagnostics
    );

    let accepted =
        analyze_text("fn main() -> Int { let stop = fn() -> Int { return 42; 0 }; stop() }");
    assert!(accepted.is_success(), "{:?}", accepted.diagnostics);
}
