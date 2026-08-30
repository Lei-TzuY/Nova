from pathlib import Path

control = Path('docs/control-flow.md')
text = control.read_text()
old = '''- every `Backedge` targets an executable-reachable `Join` node and originates on the
  same executable flow, so loop cycles cannot be attached to arbitrary nodes or live
  only inside diagnostic recovery; and
- a syntactic parent transfer does not append an execution node when evaluating its
'''
new = '''- every `Backedge` targets an executable-reachable `Join` node and originates on the
  same executable flow, so loop cycles cannot be attached to arbitrary nodes or live
  only inside diagnostic recovery;
- `Execution` and `Diagnostic` edges point strictly from lower to higher graph-local
  node identities, while every `Backedge` points strictly from a higher identity to an
  earlier loop-header `Join`, making the edge class the only legal encoding of a cycle;
  and
- a syntactic parent transfer does not append an execution node when evaluating its
'''
if old not in text:
    raise SystemExit('control-flow verification anchor not found')
control.write_text(text.replace(old, new, 1))

inspect = Path('docs/semantic-introspection-v2.md')
text = inspect.read_text()
old = '''Every node carries incoming `predecessors`. An `execution` edge may contribute
facts to runtime-reachable continuation. A `diagnostic` edge retains statically
checked source that cannot contribute facts to executable continuation. A
`backedge` represents executable loop fallthrough or `continue` to a verified
loop-header `join`.
'''
new = '''Every node carries incoming `predecessors`. An `execution` edge may contribute
facts to runtime-reachable continuation. A `diagnostic` edge retains statically
checked source that cannot contribute facts to executable continuation. A
`backedge` represents executable loop fallthrough or `continue` to a verified
loop-header `join`. The verified graph is direction-canonical: `execution` and
`diagnostic` edges always point from a lower node identity to a higher one, while
`backedge` always points from a higher identity to an earlier `join`. Tooling can
therefore treat `backedge` as the only published cycle-closing edge class rather
than inferring cycles from arbitrary predecessor order.
'''
if old not in text:
    raise SystemExit('inspection edge anchor not found')
inspect.write_text(text.replace(old, new, 1))

roadmap = Path('docs/roadmap.md')
text = roadmap.read_text()
text = text.replace(
    '**Status: forty-five vertical slices implemented; broader type-system work remains.**',
    '**Status: forty-six vertical slices implemented; broader type-system work remains.**',
    1,
)
anchor = '''The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
slice_text = '''Implemented in the forty-sixth Phase 2 slice:

- verified CFG edge classes now carry a canonical direction contract in addition to
  the existing range, reachability, transfer, exit, and backedge-target invariants;
- ordinary `Execution` and recovery-only `Diagnostic` edges must point strictly from
  an earlier graph-local node to a later node, matching deterministic lowering order;
- `Backedge` must point strictly from a later node to an earlier executable `Join`, so
  an unclassified backward execution cycle or a forward edge mislabeled as a loop edge
  fails closed instead of entering definite-initialization or inspection as verified input;
- adversarial verifier regressions corrupt both directions while the complete `nova-sema`
  suite and workspace all-targets Clippy lock valid loop/recovery graphs unchanged; and
- schema v2 shape is unchanged: semantic inspection publishes the same edge kinds, now
  with a stronger analyzer-side canonicality guarantee.

'''
if anchor not in text:
    raise SystemExit('roadmap insertion anchor not found')
roadmap.write_text(text.replace(anchor, slice_text + anchor, 1))
