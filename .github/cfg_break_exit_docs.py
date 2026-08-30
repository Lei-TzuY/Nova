from pathlib import Path

# docs/control-flow.md
path = Path('docs/control-flow.md')
text = path.read_text()
old = '''- `return`, `break`, and `continue` successor edge classes respect their transfer
  behavior;
'''
new = '''- `return`, `break`, and `continue` successor edge classes respect their transfer
  behavior; in particular, an executable successor of `break` must be a compiler-created
  `Join`, so a loop exit cannot bypass the continuation merge while diagnostic-only
  unreachable source may still follow the transfer through `Diagnostic` edges;
'''
if old not in text:
    raise SystemExit('control-flow transfer bullet anchor not found')
path.write_text(text.replace(old, new, 1))

# docs/semantic-introspection-v2.md
path = Path('docs/semantic-introspection-v2.md')
text = path.read_text()
old = '''that receives any `backedge` also publishes at least one earlier `execution` predecessor,
so tooling can rely on every executable loop header retaining its first-entry path.
'''
new = '''that receives any `backedge` also publishes at least one earlier `execution` predecessor,
so tooling can rely on every executable loop header retaining its first-entry path.
Executable continuation from a `break` node is likewise canonical: when present it targets
only a `join`; diagnostic successors may retain unreachable source but cannot masquerade as
the loop's executable exit continuation.
'''
if old not in text:
    raise SystemExit('v2 predecessor contract anchor not found')
path.write_text(text.replace(old, new, 1))

# docs/roadmap.md
path = Path('docs/roadmap.md')
text = path.read_text()
old = '**Status: forty-eight vertical slices implemented; broader type-system work remains.**'
new = '**Status: forty-nine vertical slices implemented; broader type-system work remains.**'
if old not in text:
    raise SystemExit('roadmap status anchor not found')
text = text.replace(old, new, 1)
anchor = '''The next Phase 2 slices should address semantic depth rather than widen syntax'''
entry = '''Implemented in the forty-ninth Phase 2 slice:

- verified `break` transfer topology now matches structured while lowering rather than
  accepting any forward executable successor;
- a `break` may retain `Diagnostic` successors for statically checked unreachable source,
  but any `Execution` successor must target a compiler-created `Join`, and `Backedge`
  remains forbidden;
- malformed CFGs can therefore no longer bypass the loop-exit merge and feed an arbitrary
  executable read, initialization, branch, or exit directly from a `break` transfer;
- an adversarial verifier regression retargets a valid break-to-join continuation to a
  non-Join node and now fails closed while the complete `nova-sema` suite and workspace
  all-targets Clippy keep valid loop-control behavior unchanged; and
- semantic-inspection v2 keeps the same schema while gaining the stronger analyzer-side
  transfer-topology guarantee.

'''
if anchor not in text:
    raise SystemExit('roadmap next-slices anchor not found')
text = text.replace(anchor, entry + anchor, 1)
path.write_text(text)
