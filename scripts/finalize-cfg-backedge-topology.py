from pathlib import Path

control = Path("docs/control-flow.md")
text = control.read_text()
old = '''- `return`, `break`, and `continue` successor edge classes respect their transfer
  behavior; and
- a syntactic parent transfer does not append an execution node when evaluating its
'''
new = '''- `return`, `break`, and `continue` successor edge classes respect their transfer
  behavior;
- every `Backedge` targets an executable-reachable `Join` node and originates on the
  same executable flow, so loop cycles cannot be attached to arbitrary nodes or live
  only inside diagnostic recovery; and
- a syntactic parent transfer does not append an execution node when evaluating its
'''
if text.count(old) != 1:
    raise SystemExit("control-flow verification anchor not found exactly once")
control.write_text(text.replace(old, new))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
old_status = "**Status: thirty-five vertical slices implemented; broader type-system work remains.**"
new_status = "**Status: thirty-six vertical slices implemented; broader type-system work remains.**"
if text.count(old_status) != 1:
    raise SystemExit("Phase 2 status anchor not found exactly once")
text = text.replace(old_status, new_status)
anchor = '''Implemented in the thirty-fifth Phase 2 slice:

- `Unit` joins `Int` and `Bool` as an equality-comparable bootstrap value type without
  making nominal aggregates or function references comparable;
- semantic `==` / `!=` accepts matching Unit operands, including parameters and call
  results, while preserving existing Never/Error recovery precedence;
- closed-condition reasoning recognizes only literal `()` equality and inequality, so
  known Unit comparisons can refine reachability without treating Unit-returning calls,
  locals, or blocks as compile-time values;
- semantic regressions lock Unit comparison, literal-condition flow, dynamic-call
  conservatism, and continued rejection of record/function equality; and
- no parser, HIR, CFG shape, or semantic-inspection schema change is required.

'''
addition = anchor + '''Implemented in the thirty-sixth Phase 2 slice:

- CFG verification treats `Backedge` topology as an explicit invariant rather than a
  convention of the structured loop builder;
- every backedge must target a `Join` node, matching the current pre-test loop-header
  representation, instead of being accepted on reads, initialization events, branches,
  transfers, or exits;
- both backedge endpoints must belong to executable-reachable control flow, rejecting
  cycles that exist only inside retained diagnostic/recovery source;
- the rule composes with diagnostic-reconnection verification so the fixed-point solver
  consumes only graph cycles that can represent real loop execution;
- direct corruption regressions prove the previous verifier accepted both malformed
  target kinds and diagnostic-only cycles, while existing valid cyclic CFG tests remain
  green; and
- the change affects no syntax, HIR shape, analyzer reachability policy, runtime
  behavior, dataflow transfer function, or semantic-inspection schema.

'''
if text.count(anchor) != 1:
    raise SystemExit("slice 35 anchor not found exactly once")
roadmap.write_text(text.replace(anchor, addition))
