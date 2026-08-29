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
    if "verifier_rejects_duplicate_and_out_of_order_binding_metadata" in text:
        return
    marker = '''    #[test]\n    fn builder_fails_closed_on_an_invalid_backedge_endpoint() {\n'''
    tests = '''    #[test]\n    fn verifier_rejects_duplicate_and_out_of_order_binding_metadata() {\n        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));\n        let first = binding(0, "first", 1);\n        let second = binding(1, "second", 7);\n        builder.register_binding(&first);\n        builder.register_binding(&second);\n        let exit = builder.cursor();\n        let graph = builder.finish(Some(exit)).expect("valid seed graph");\n\n        let mut duplicate = graph.clone();\n        duplicate.bindings.push(duplicate.bindings[1].clone());\n        let error = super::verify(&duplicate, span(0, 20))\n            .expect_err("duplicate binding identities must be rejected");\n        assert!(error.message().contains("binding metadata"));\n\n        let mut out_of_order = graph;\n        out_of_order.bindings.swap(0, 1);\n        let error = super::verify(&out_of_order, span(0, 20))\n            .expect_err("binding metadata must remain in semantic identity order");\n        assert!(error.message().contains("binding metadata"));\n    }\n\n'''
    text = replace_once(text, marker, tests + marker, "test insertion")
    CF.write_text(text)


def add_fix_and_docs() -> None:
    text = CF.read_text()
    if "flow binding metadata is not in strict semantic identity order" not in text:
        marker = '''    let known_bindings = graph\n        .bindings\n        .iter()\n        .map(|binding| binding.id)\n        .collect::<BTreeSet<_>>();\n'''
        replacement = '''    for bindings in graph.bindings.windows(2) {\n        if bindings[0].id >= bindings[1].id {\n            return Err(FlowError::invalid(\n                bindings[1].span,\n                "flow binding metadata is not in strict semantic identity order",\n            ));\n        }\n    }\n    let known_bindings = graph\n        .bindings\n        .iter()\n        .map(|binding| binding.id)\n        .collect::<BTreeSet<_>>();\n'''
        text = replace_once(text, marker, replacement, "binding verifier insertion")
        CF.write_text(text)

    doc = DOC.read_text()
    if "binding metadata identities are strictly increasing" not in doc:
        marker = '''- every read/initialization event names graph binding metadata;\n'''
        replacement = '''- binding metadata identities are strictly increasing, making the table canonical and\n  preventing duplicate identities from being silently overwritten during dataflow;\n- every read/initialization event names graph binding metadata;\n'''
        doc = replace_once(doc, marker, replacement, "control-flow doc insertion")
        DOC.write_text(doc)

    road = ROAD.read_text()
    road = road.replace(
        "**Status: thirty-two vertical slices implemented; broader type-system work remains.**",
        "**Status: thirty-three vertical slices implemented; broader type-system work remains.**",
        1,
    )
    if "Implemented in the thirty-third Phase 2 slice:" not in road:
        marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n'''
        section = '''Implemented in the thirty-third Phase 2 slice:\n\n- verified CFG binding metadata now has a canonical identity-order invariant rather than\n  relying only on the builder's current `BTreeMap` implementation;\n- binding identities must be strictly increasing, simultaneously rejecting duplicate and\n  out-of-order metadata without assuming function-local identities are contiguous;\n- this closes a corruption path where `definite_initialization_diagnostics` could collect\n  duplicate identities into a `BTreeMap` and silently select the wrong declaration name or\n  span for N3009;\n- direct verifier corruption regressions prove both duplicate and out-of-order tables were\n  previously accepted and are now rejected; and\n- the change affects no syntax, HIR, runtime behavior, dataflow transfer function, or\n  semantic-inspection schema.\n\n'''
        road = replace_once(road, marker, section + marker, "roadmap insertion")
    ROAD.write_text(road)


mode = sys.argv[1]
if mode == "tests":
    add_tests()
elif mode == "fix":
    add_fix_and_docs()
else:
    raise SystemExit("usage: validate_cfg_binding_table.py [tests|fix]")
