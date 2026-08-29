from pathlib import Path

contract = Path("docs/control-flow.md")
text = contract.read_text()
old = '''- every read/initialization event names graph binding metadata;
- declared normal exits name `Exit` nodes; and
- `return`, `break`, `continue`, and `Exit` successor edge classes respect their
  transfer behavior; and
'''
new = '''- every read/initialization event names graph binding metadata;
- the normal-exit table contains each `Exit` node exactly once and no other node;
- every declared normal exit is executable-reachable and every `Exit` is terminal;
- `return`, `break`, and `continue` successor edge classes respect their transfer
  behavior; and
'''
if text.count(old) != 1:
    raise SystemExit("expected normal-exit verification bullets once")
text = text.replace(old, new)
anchor = '''The verifier computes this executable-reachability set independently of lowering.
This makes diagnostic isolation a graph invariant rather than an analyzer convention:
a discarded recovery subtree may branch away from executable flow, but it cannot feed
an executable join, exit, or loop header through any edge class. The fixed-point
must-analysis can therefore safely intersect every recorded predecessor without
allowing recovery-only facts to constrain reachable continuation.
'''
addition = anchor + '''Normal completion has a similarly closed contract. `normal_exits` is not advisory
metadata: it must exactly enumerate the graph's `Exit` nodes without duplicates, each
such node must belong to executable-reachable flow, and an `Exit` has no successor of
any edge class. A diagnostic-only recovery path therefore cannot be mislabeled as a
successful function completion, and post-exit diagnostic nodes cannot extend a graph
past its terminal boundary.
'''
if text.count(anchor) != 1:
    raise SystemExit("expected executable-reachability explanation once")
contract.write_text(text.replace(anchor, addition))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
status = "**Status: thirty-one vertical slices implemented; broader type-system work remains.**"
if text.count(status) != 1:
    raise SystemExit("expected Phase 2 status once")
text = text.replace(
    status,
    "**Status: thirty-two vertical slices implemented; broader type-system work remains.**",
)
anchor = '''The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
entry = '''Implemented in the thirty-second Phase 2 slice:

- CFG normal completion is now verified as a structural invariant rather than inferred
  only from the `normal_exits` metadata vector;
- the verifier requires the normal-exit table to equal the graph's `Exit` node set
  exactly, rejecting missing, extra, or duplicate declarations;
- every normal `Exit` must be reachable from function entry without crossing a
  `Diagnostic` edge, so recovery-only source cannot masquerade as successful function
  completion;
- `Exit` nodes are strictly terminal and may not have even diagnostic successors,
  separating source-level `return` recovery from the compiler-generated function-end
  marker;
- direct corruption regressions cover diagnostic-only exits, unlisted exit nodes, and
  post-exit diagnostic successors; and
- existing function completion, divergent-function, CFG isolation, and full workspace
  tests remain green without changing syntax, HIR, runtime semantics, or inspection v1.

'''
if text.count(anchor) != 1:
    raise SystemExit("expected Phase 2 next-slices anchor once")
roadmap.write_text(text.replace(anchor, entry + anchor))
