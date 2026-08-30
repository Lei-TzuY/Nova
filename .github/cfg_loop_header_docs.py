from pathlib import Path

# control-flow.md
path = Path('docs/control-flow.md')
text = path.read_text()
old = '''- every `Backedge` targets an executable-reachable `Join` node and originates on the
  same executable flow, so loop cycles cannot be attached to arbitrary nodes or live
  only inside diagnostic recovery;
- `Execution` and `Diagnostic` edges point strictly from lower to higher graph-local
'''
new = '''- every `Backedge` targets an executable-reachable `Join` node and originates on the
  same executable flow, so loop cycles cannot be attached to arbitrary nodes or live
  only inside diagnostic recovery;
- every `Join` that receives a `Backedge` also retains at least one earlier `Execution`
  predecessor, preserving the loop's first-entry path in the fixed-point intersection
  instead of allowing backedge-only loop headers to erase pre-iteration facts;
- `Execution` and `Diagnostic` edges point strictly from lower to higher graph-local
'''
if old not in text:
    raise SystemExit('control-flow anchor not found')
path.write_text(text.replace(old, new, 1))

# semantic-introspection-v2.md
path = Path('docs/semantic-introspection-v2.md')
text = path.read_text()
old = '''canonical: every non-`join` node has exactly one incoming edge, only `join` may merge
multiple paths, and no node publishes the same source/edge-class pair twice.
'''
new = '''canonical: every non-`join` node has exactly one incoming edge, only `join` may merge
multiple paths, and no node publishes the same source/edge-class pair twice. A `join`
that receives any `backedge` also publishes at least one earlier `execution` predecessor,
so tooling can rely on every executable loop header retaining its first-entry path.
'''
if old not in text:
    raise SystemExit('v2 anchor not found')
path.write_text(text.replace(old, new, 1))

# roadmap.md
path = Path('docs/roadmap.md')
text = path.read_text()
text = text.replace(
    '**Status: forty-seven vertical slices implemented; broader type-system work remains.**',
    '**Status: forty-eight vertical slices implemented; broader type-system work remains.**',
    1,
)
anchor = '''Implemented in the forty-seventh Phase 2 slice:

- verified CFG predecessor cardinality now matches the structured builder contract:
  only `Join` may merge multiple incoming paths, while every other non-entry node has
  exactly one predecessor;
- each node's predecessor list rejects duplicate source/edge-class pairs instead of
  allowing redundant graph facts to reach fixed-point dataflow or semantic inspection;
- the stronger check prevents malformed extra predecessors from silently changing the
  must-analysis intersection for reads, initialization events, transfers, or exits;
- edge-specific topology diagnostics such as an invalid backedge target retain precedence
  over the generic cardinality error, keeping earlier invariant failures precise; and
- adversarial verifier regressions lock both non-`Join` multi-predecessor corruption and
  duplicate `Join` edges while schema v2 shape and all valid lowering behavior remain unchanged.

'''
addition = anchor + '''Implemented in the forty-eighth Phase 2 slice:

- every verified loop-header `Join` that receives a `Backedge` must also retain at least
  one forward `Execution` predecessor from an earlier graph-local node;
- this makes the loop's first-entry path a verifier invariant rather than a builder-only
  convention, preventing malformed graphs from deleting pre-iteration facts before the
  definite-initialization fixed point is solved;
- an adversarial cyclic graph keeps an alternate path that initializes a binding before
  entering the cycle, then removes the header's original entry edge; the verifier now
  rejects that corruption before it can erase the seed graph's required `N3009` read;
- existing direction, executable-reachability, backedge-target, predecessor-cardinality,
  and diagnostic-isolation invariants remain independently enforced; and
- CFG/v2 schema shape and all valid structured lowering remain unchanged.

'''
if anchor not in text:
    raise SystemExit('roadmap slice anchor not found')
text = text.replace(anchor, addition, 1)
path.write_text(text)
