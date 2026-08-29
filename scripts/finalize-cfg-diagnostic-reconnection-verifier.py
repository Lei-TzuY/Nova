from pathlib import Path

control_flow = Path("crates/nova-sema/src/control_flow.rs")
text = control_flow.read_text()
anchor = '''    #[test]
    fn builder_fails_closed_on_an_invalid_backedge_endpoint() {
'''
test = '''    #[test]
    fn verifier_rejects_diagnostic_predecessor_on_executable_join() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let left = builder.fork_from(entry, Some(span(1, 2)), FlowEdgeKind::Execution);
        let right = builder.fork_from(entry, Some(span(3, 4)), FlowEdgeKind::Execution);
        let join = builder.join([left, right], Some(span(5, 6)), FlowEdgeKind::Execution);
        let graph_exit = builder.cursor();
        let mut graph = builder.finish(Some(graph_exit)).expect("valid seed graph");
        graph.nodes[join.index()].predecessors[0].kind = FlowEdgeKind::Diagnostic;

        let error = super::verify(&graph, span(0, 20))
            .expect_err("executable continuation cannot consume a diagnostic predecessor");
        assert!(error.message().contains("diagnostic-only"));
    }

'''
if text.count(anchor) != 1:
    raise SystemExit("expected verifier-test insertion anchor once")
control_flow.write_text(text.replace(anchor, test + anchor))

contract = Path("docs/control-flow.md")
text = contract.read_text()
old = '''- every node is graph-reachable from the entry, including diagnostic source;
- every read/initialization event names graph binding metadata;
'''
new = '''- every node is graph-reachable from the entry, including diagnostic source;
- every node reachable from entry without crossing a `Diagnostic` edge has only
  non-diagnostic predecessors that are themselves reachable on such executable flow;
- every read/initialization event names graph binding metadata;
'''
if text.count(old) != 1:
    raise SystemExit("expected verification-list anchor once")
text = text.replace(old, new)
anchor = '''An internal verification failure is fail-closed diagnostic `N3999`; no invalid
graph is published for that function.
'''
addition = anchor + '''The verifier computes this executable-reachability set independently of lowering.
This makes diagnostic isolation a graph invariant rather than an analyzer convention:
a discarded recovery subtree may branch away from executable flow, but it cannot feed
an executable join, exit, or loop header through any edge class. The fixed-point
must-analysis can therefore safely intersect every recorded predecessor without
allowing recovery-only facts to constrain reachable continuation.
'''
if text.count(anchor) != 1:
    raise SystemExit("expected verifier explanation anchor once")
contract.write_text(text.replace(anchor, addition))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
status = "**Status: thirty vertical slices implemented; broader type-system work remains.**"
if text.count(status) != 1:
    raise SystemExit("expected Phase 2 status once")
text = text.replace(
    status,
    "**Status: thirty-one vertical slices implemented; broader type-system work remains.**",
)
anchor = '''The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
entry = '''Implemented in the thirty-first Phase 2 slice:

- CFG verification now derives the set of nodes reachable from function entry without
  crossing a `Diagnostic` edge, independently of the structured lowerer's snapshots;
- every executable-reachable node is required to have only non-diagnostic predecessors
  that are themselves executable-reachable, structurally forbidding discarded recovery
  subgraphs from reconnecting to live continuation through joins, exits, or backedges;
- this invariant protects the fixed-point definite-initialization solver, which can
  continue intersecting all recorded predecessors because recovery-only facts cannot
  enter an executable node;
- a direct verifier regression constructs the rejected-loop-style diagnostic branch and
  recovery backedge that slice thirty removed from the analyzer, proving the verifier
  now fails closed even if a future lowering regression recreates that shape;
- a second corruption regression injects a diagnostic predecessor into an otherwise
  executable join, locking the generic invariant rather than only the loop case; and
- existing valid cyclic CFG tests plus the full workspace suite preserve ordinary
  execution/backedge graphs without changing syntax, HIR, runtime semantics, or the
  semantic-inspection schema.

'''
if text.count(anchor) != 1:
    raise SystemExit("expected Phase 2 next-slices anchor once")
roadmap.write_text(text.replace(anchor, entry + anchor))
