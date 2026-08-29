from pathlib import Path

control_flow = Path("docs/control-flow.md")
text = control_flow.read_text()
old = '''A `while` whose condition is rejected as non-`Bool` may likewise retain body
nodes for static and lexical loop-control diagnostics, but that recovery-only
body is not an executable loop iteration. In particular, a `continue` retained
there has no `Backedge` successor to the condition header; rejected control flow
must not reconnect discarded recovery paths to reachable continuation.
'''
new = '''A `while` whose condition is rejected as non-`Bool` may likewise retain body
nodes for static and lexical loop-control diagnostics, but that recovery-only
body is not an executable loop iteration. Neither ordinary body fallthrough nor
a retained `continue` receives a `Backedge` successor to the condition header;
rejected control flow must not reconnect discarded recovery paths to reachable
continuation.
'''
if text.count(old) != 1:
    raise SystemExit("expected invalid-while CFG paragraph once")
control_flow.write_text(text.replace(old, new))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
old = '''- a `continue` reached only inside such an invalid-condition recovery body no longer
  receives a `Backedge` to the loop header, so discarded diagnostic flow cannot
  reconnect itself to reachable continuation;
'''
new = '''- neither ordinary body fallthrough nor a `continue` reached only inside such an
  invalid-condition recovery body receives a `Backedge` to the loop header, so
  discarded diagnostic flow cannot reconnect itself to reachable continuation;
'''
if text.count(old) != 1:
    raise SystemExit("expected slice 30 recovery-backedge bullet once")
roadmap.write_text(text.replace(old, new))
