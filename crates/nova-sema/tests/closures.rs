use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::control_flow::FlowNodeKind;
use nova_sema::hir::{ExpressionKind, Type};
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
fn rejects_mutable_capture_without_inventing_shared_cell_semantics() {
    let output = analyze_text(
        "fn main() -> Int { var value = 40; let add = fn(x: Int) -> Int { value + x }; add(2) }",
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3035"),
        "{:?}",
        output.diagnostics
    );
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
