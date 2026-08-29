from pathlib import Path

constitution = Path("docs/language-constitution.md")
text = constitution.read_text()
old = '''The lowerer temporarily retains an inline initialized bit
only to select Error-typed recovery HIR at an invalid read; it is not a second
diagnostic authority or accepted-program proof.
'''
new = '''Lexical symbols carry no parallel initialized flag: resolved reads retain their
declared HIR type, and the verified CFG is the single source of definite-
initialization truth.
'''
if text.count(old) != 1:
    raise SystemExit("expected stale inline-initialization paragraph once")
constitution.write_text(text.replace(old, new))

control_flow = Path("docs/control-flow.md")
text = control_flow.read_text()
anchor = '''An invalid construct may leave a reachable-from-entry diagnostic subgraph with
no continuation edge. This is intentional: rollback moves the lowering cursor
back to the last valid state, but does not erase source events or binding
identities needed for diagnostics.
'''
replacement = anchor + '''A `while` whose condition is rejected as non-`Bool` may likewise retain body
nodes for static and lexical loop-control diagnostics, but that recovery-only
body is not an executable loop iteration. In particular, a `continue` retained
there has no `Backedge` successor to the condition header; rejected control flow
must not reconnect discarded recovery paths to reachable continuation.
'''
if text.count(anchor) != 1:
    raise SystemExit("expected invalid-construct CFG paragraph once")
control_flow.write_text(text.replace(anchor, replacement))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
status = "**Status: twenty-nine vertical slices implemented; broader type-system work remains.**"
if text.count(status) != 1:
    raise SystemExit("expected Phase 2 status once")
text = text.replace(
    status,
    "**Status: thirty vertical slices implemented; broader type-system work remains.**",
)
anchor = '''The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
entry = '''Implemented in the thirtieth Phase 2 slice:

- rejected `while` conditions continue to retain their body in the CFG for static
  diagnostics and lexical `break`/`continue` checking without treating that body as
  an executable loop iteration;
- a `continue` reached only inside such an invalid-condition recovery body no longer
  receives a `Backedge` to the loop header, so discarded diagnostic flow cannot
  reconnect itself to reachable continuation;
- valid dynamic and proven-entered Boolean loops retain their ordinary continue
  backedges, preserving runtime loop semantics and fixed-point graph shape;
- the change tightens the documented fail-closed invalid-control contract without
  changing syntax, HIR, runtime behavior, or semantic-inspection schema v1;
- CFG-shape regressions lock both the rejected-loop isolation rule and the valid-loop
  positive control; and
- the language constitution is synchronized with slice twenty-eight by removing the
  obsolete claim that lexical symbols still carry a parallel initialization bit.

'''
if text.count(anchor) != 1:
    raise SystemExit("expected Phase 2 next-slices anchor once")
roadmap.write_text(text.replace(anchor, entry + anchor))
