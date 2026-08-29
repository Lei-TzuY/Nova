from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
CF = ROOT / "crates/nova-sema/src/control_flow.rs"
DOC = ROOT / "docs/control-flow.md"
ROAD = ROOT / "docs/roadmap.md"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


def add_tests() -> None:
    text = CF.read_text()
    if "verifier_rejects_root_kind_mismatch_and_duplicate_entry" in text:
        return
    marker = '''    #[test]\n    fn verifier_rejects_duplicate_and_out_of_order_binding_metadata() {\n'''
    tests = '''    #[test]\n    fn verifier_rejects_root_kind_mismatch_and_duplicate_entry() {\n        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));\n        builder.advance(\n            FlowNodeKind::Branch,\n            Some(span(1, 2)),\n            FlowEdgeKind::Execution,\n        );\n        let graph_exit = builder.cursor();\n        let graph = builder.finish(Some(graph_exit)).expect("valid seed graph");\n\n        let mut wrong_root_kind = graph.clone();\n        wrong_root_kind.nodes[wrong_root_kind.entry.index()].kind = FlowNodeKind::Branch;\n        let error = super::verify(&wrong_root_kind, span(0, 20))\n            .expect_err("graph.entry must identify an Entry-kind node");\n        assert!(error.message().contains("entry"));\n\n        let mut duplicate_entry = graph;\n        duplicate_entry.nodes[1].kind = FlowNodeKind::Entry;\n        let error = super::verify(&duplicate_entry, span(0, 20))\n            .expect_err("a verified CFG must contain exactly one Entry-kind node");\n        assert!(error.message().contains("entry"));\n    }\n\n'''
    text = replace_once(text, marker, tests + marker, "test insertion")
    CF.write_text(text)


def add_fix_and_docs() -> None:
    text = CF.read_text()
    if "flow entry does not reference the unique Entry node" not in text:
        marker = '''    let mut successors = vec![Vec::<(FlowNodeId, FlowEdgeKind)>::new(); graph.nodes.len()];\n'''
        replacement = '''    if !matches!(graph.nodes[graph.entry.index()].kind, FlowNodeKind::Entry) {\n        return Err(FlowError::invalid(\n            graph.nodes[graph.entry.index()].span.unwrap_or(fallback_span),\n            "flow entry does not reference the unique Entry node",\n        ));\n    }\n    if let Some(node) = graph\n        .nodes\n        .iter()\n        .find(|node| matches!(node.kind, FlowNodeKind::Entry) && node.id != graph.entry)\n    {\n        return Err(FlowError::invalid(\n            node.span.unwrap_or(fallback_span),\n            "control-flow graph contains more than one Entry node",\n        ));\n    }\n    let mut successors = vec![Vec::<(FlowNodeId, FlowEdgeKind)>::new(); graph.nodes.len()];\n'''
        text = replace_once(text, marker, replacement, "entry verifier insertion")
        CF.write_text(text)

    doc = DOC.read_text()
    if "the designated graph entry is the unique `Entry`-kind node" not in doc:
        marker = '''- entry and predecessor identities are in range;\n'''
        replacement = '''- entry and predecessor identities are in range;\n- the designated graph entry is the unique `Entry`-kind node, keeping the solver's\n  empty-lattice root aligned with the graph model;\n'''
        doc = replace_once(doc, marker, replacement, "control-flow doc insertion")
        DOC.write_text(doc)

    road = ROAD.read_text()
    road = road.replace(
        "**Status: thirty-three vertical slices implemented; broader type-system work remains.**",
        "**Status: thirty-four vertical slices implemented; broader type-system work remains.**",
        1,
    )
    if "Implemented in the thirty-fourth Phase 2 slice:" not in road:
        marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n'''
        section = '''Implemented in the thirty-fourth Phase 2 slice:\n\n- the verified CFG now requires `graph.entry` to identify the unique `Entry`-kind node,\n  making the fixed-point solver's distinguished empty-lattice root a checked invariant;\n- a graph whose designated root has another node kind is rejected, as is any graph that\n  contains a second `Entry` marker elsewhere in the node table;\n- the verifier deliberately does not require the entry to have index zero, avoiding an\n  unnecessary coupling between semantic graph meaning and the current builder's numbering;\n- direct corruption regressions prove both root-kind mismatch and duplicate Entry markers\n  were previously accepted and are now fail-closed; and\n- the change affects no syntax, HIR, runtime behavior, dataflow transfer function, or\n  semantic-inspection schema.\n\n'''
        road = replace_once(road, marker, section + marker, "roadmap insertion")
    ROAD.write_text(road)


mode = sys.argv[1]
if mode == "tests":
    add_tests()
elif mode == "fix":
    add_fix_and_docs()
else:
    raise SystemExit("usage: validate_cfg_entry_integrity.py [tests|fix]")
