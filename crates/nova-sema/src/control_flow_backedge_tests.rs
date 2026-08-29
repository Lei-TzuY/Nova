use crate::{
    control_flow::{FlowEdgeKind, FlowNodeKind, FunctionFlowBuilder},
    hir::{Binding, BindingId, FunctionId, Type},
};
use nova_source::{SourceId, Span};

fn span(start: usize, end: usize) -> Span {
    Span::new(SourceId::new(0), start, end).expect("valid test span")
}

fn binding(index: usize, name: &str, at: usize) -> Binding {
    Binding {
        id: BindingId::new(index),
        name: name.to_owned(),
        ty: Type::Int,
        mutable: true,
        span: span(at, at + name.len()),
    }
}

#[test]
fn verifier_rejects_backedge_targeting_non_join_node() {
    let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
    let value = binding(0, "value", 1);
    builder.register_binding(&value);

    let read = builder.advance(
        FlowNodeKind::Read(value.id),
        Some(span(5, 10)),
        FlowEdgeKind::Execution,
    );
    let tail = builder.fork_from(read, Some(span(11, 12)), FlowEdgeKind::Execution);
    builder.add_backedge(tail, read);

    let error = builder
        .finish(None)
        .expect_err("backedge targets must be loop-header joins");
    assert!(error.message().contains("backedge"));
    assert!(error.message().contains("Join"));
}

#[test]
fn verifier_rejects_backedge_cycle_confined_to_diagnostic_flow() {
    let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
    let entry = builder.cursor();
    let recovery = builder.fork_from(entry, Some(span(1, 2)), FlowEdgeKind::Diagnostic);
    let header = builder.join([recovery], Some(span(3, 4)), FlowEdgeKind::Diagnostic);
    let tail = builder.fork_from(header, Some(span(5, 6)), FlowEdgeKind::Diagnostic);
    builder.add_backedge(tail, header);

    let error = builder
        .finish(None)
        .expect_err("backedges must describe executable loop cycles");
    assert!(error.message().contains("backedge"));
    assert!(error.message().contains("executable"));
}
