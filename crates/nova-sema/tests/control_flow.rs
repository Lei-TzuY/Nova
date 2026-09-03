use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    AnalysisOutput, analyze,
    control_flow::{FlowEdgeKind, FlowNodeKind, FlowTransfer},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "control-flow.nv", text);
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

#[test]
fn definite_initialization_is_decided_from_branch_intersection() {
    let incomplete = analyze_text(
        "fn f(flag: Bool) -> Int {\n\
             var value: Int;\n\
             if flag { value = 1; 0 } else { 0 };\n\
             value\n\
         }",
    );
    assert_eq!(
        incomplete
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec!["N3009"]
    );

    let graph = &incomplete.control_flow.functions()[0];
    assert_eq!(graph.function(), incomplete.program.functions[0].id);
    assert!(
        graph
            .nodes()
            .iter()
            .filter(|node| matches!(node.kind, FlowNodeKind::Branch))
            .count()
            >= 2
    );
    assert!(
        graph
            .nodes()
            .iter()
            .any(|node| matches!(node.kind, FlowNodeKind::Join))
    );

    let complete = analyze_text(
        "fn f(flag: Bool) -> Int {\n\
             var value: Int;\n\
             if flag { value = 1; 0 } else { value = 2; 0 };\n\
             value\n\
         }",
    );
    assert!(complete.is_success(), "{:?}", complete.diagnostics);
}

#[test]
fn closed_condition_refinements_shape_execution_edges() {
    let output = analyze_text(
        "fn f() -> Int {\n\
             var value: Int;\n\
             if 1 < 2 { value = 1; 0 } else { 0 };\n\
             value\n\
         }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let branch_edges = output.control_flow.functions()[0]
        .nodes()
        .iter()
        .filter(|node| matches!(node.kind, FlowNodeKind::Branch))
        .flat_map(|node| node.predecessors.iter().map(|edge| edge.kind))
        .collect::<Vec<_>>();
    assert!(branch_edges.contains(&FlowEdgeKind::Execution));
    assert!(branch_edges.contains(&FlowEdgeKind::Diagnostic));
}

#[test]
fn loops_record_transfers_and_a_condition_backedge() {
    let output = analyze_text(
        "fn f(flag: Bool) -> Int {\n\
             while flag {\n\
                 if flag { continue; } else { break; };\n\
             }\n\
             0\n\
         }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let graph = &output.control_flow.functions()[0];

    assert!(
        graph
            .nodes()
            .iter()
            .any(|node| { matches!(node.kind, FlowNodeKind::Transfer(FlowTransfer::Continue)) })
    );
    let break_node = graph
        .nodes()
        .iter()
        .find(|node| matches!(node.kind, FlowNodeKind::Transfer(FlowTransfer::Break)))
        .expect("loop contains a break transfer");
    assert!(graph.nodes().iter().any(|node| {
        node.predecessors
            .iter()
            .any(|edge| edge.from == break_node.id && edge.kind == FlowEdgeKind::Execution)
    }));
    assert!(graph.nodes().iter().any(|node| {
        matches!(node.kind, FlowNodeKind::Join)
            && node
                .predecessors
                .iter()
                .any(|edge| edge.kind == FlowEdgeKind::Backedge)
    }));
}

#[test]
fn unreachable_reads_stay_diagnostic_only_in_the_graph() {
    let output = analyze_text("fn f() -> Int { var value: Int; return 1; value; }");
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(output.diagnostics[0].code, "N3009");

    let graph = &output.control_flow.functions()[0];
    let read = graph
        .nodes()
        .iter()
        .find(|node| matches!(node.kind, FlowNodeKind::Read(_)))
        .expect("unreachable source is retained for diagnostics");
    assert_eq!(read.predecessors.len(), 1);
    assert_eq!(read.predecessors[0].kind, FlowEdgeKind::Diagnostic);
    assert!(graph.normal_exits().is_empty());
}

#[test]
fn unreachable_constant_failures_do_not_become_execution_diagnostics() {
    let output = analyze_text("fn f() -> Int { return 1; 9223372036854775807 + 1; }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn analyzed_graphs_expose_only_in_range_verified_references() {
    let output = analyze_text(
        "fn first(value: Int) -> Int { value }\n\
         fn second() -> Int { var value = 1; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(
        output.control_flow.functions().len(),
        output.program.functions.len()
    );

    for graph in output.control_flow.functions() {
        assert_eq!(graph.entry().index(), 0);
        for (index, node) in graph.nodes().iter().enumerate() {
            assert_eq!(node.id.index(), index);
            assert!(
                node.predecessors
                    .iter()
                    .all(|edge| edge.from.index() < graph.nodes().len())
            );
        }
        assert!(graph.normal_exits().iter().all(|exit| {
            matches!(
                graph.nodes().get(exit.index()).map(|node| &node.kind),
                Some(FlowNodeKind::Exit)
            )
        }));
    }
}

#[test]
fn returning_loop_branch_initialization_does_not_mask_break_path_self_read() {
    let output = analyze_text(
        "fn f(flag: Bool) -> Int {\n\
             var value: Int;\n\
             while true {\n\
                 if flag {\n\
                     value = 7;\n\
                     return 0;\n\
                 } else {\n\
                     value;\n\
                     break;\n\
                 };\n\
             }\n\
             value;\n\
             0\n\
         }",
    );

    let n3009_count = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "N3009")
        .count();

    // Initialization on the returning branch cannot reach either the sibling
    // break path or the loop exit. Both reachable reads therefore remain invalid.
    assert_eq!(n3009_count, 2, "{:?}", output.diagnostics);
}

#[test]
fn returning_loop_branch_initialization_does_not_mask_continue_backedge_self_read() {
    let output = analyze_text(
        "fn f(looping: Bool, stop: Bool) -> Int {\n\
             var value: Int;\n\
             while looping {\n\
                 if stop {\n\
                     value = 7;\n\
                     return 0;\n\
                 } else {\n\
                     value;\n\
                     continue;\n\
                 };\n\
             }\n\
             0\n\
         }",
    );

    let n3009_count = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "N3009")
        .count();

    // Initialization on the returning path cannot contribute to the loop backedge.
    // The sibling continuing path must therefore self-read `value` as uninitialized.
    assert_eq!(n3009_count, 1, "{:?}", output.diagnostics);
}
