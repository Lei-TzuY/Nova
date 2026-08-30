from pathlib import Path

# Control-flow contract.
path = Path('docs/control-flow.md')
text = path.read_text()
old = '''- node identities equal their deterministic vector positions;\n- only the entry lacks predecessors;\n- every node is graph-reachable from the entry, including diagnostic source;\n'''
new = '''- node identities equal their deterministic vector positions;\n- only the entry lacks predecessors; every non-`Join` node has exactly one predecessor,\n  making `Join` the only legal merge point for multiple incoming paths;\n- each node's predecessor list contains no duplicate source/edge-class pair, keeping\n  the graph representation canonical before fixed-point dataflow or tooling projection;\n- every node is graph-reachable from the entry, including diagnostic source;\n'''
if old not in text:
    raise SystemExit('control-flow verification anchor not found')
text = text.replace(old, new, 1)
path.write_text(text)

# Semantic-introspection v2 contract.
path = Path('docs/semantic-introspection-v2.md')
text = path.read_text()
old = '''therefore treat `backedge` as the only published cycle-closing edge class rather\nthan inferring cycles from arbitrary predecessor order.\n\nNode spans are source-qualified v1 spans when a source action or function\n'''
new = '''therefore treat `backedge` as the only published cycle-closing edge class rather\nthan inferring cycles from arbitrary predecessor order. Predecessor lists are also\ncanonical: every non-`join` node has exactly one incoming edge, only `join` may merge\nmultiple paths, and no node publishes the same source/edge-class pair twice.\n\nNode spans are source-qualified v1 spans when a source action or function\n'''
if old not in text:
    raise SystemExit('v2 edge anchor not found')
text = text.replace(old, new, 1)
path.write_text(text)

# Roadmap closure.
path = Path('docs/roadmap.md')
text = path.read_text()
old = '**Status: forty-six vertical slices implemented; broader type-system work remains.**'
new = '**Status: forty-seven vertical slices implemented; broader type-system work remains.**'
if old not in text:
    raise SystemExit('roadmap status anchor not found')
text = text.replace(old, new, 1)
anchor = '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n'''
block = '''Implemented in the forty-seventh Phase 2 slice:\n\n- verified CFG predecessor cardinality now matches the structured builder contract:\n  only `Join` may merge multiple incoming paths, while every other non-entry node has\n  exactly one predecessor;\n- each node's predecessor list rejects duplicate source/edge-class pairs instead of\n  allowing redundant graph facts to reach fixed-point dataflow or semantic inspection;\n- the stronger check prevents malformed extra predecessors from silently changing the\n  must-analysis intersection for reads, initialization events, transfers, or exits;\n- edge-specific topology diagnostics such as an invalid backedge target retain precedence\n  over the generic cardinality error, keeping earlier invariant failures precise; and\n- adversarial verifier regressions lock both non-`Join` multi-predecessor corruption and\n  duplicate `Join` edges while schema v2 shape and all valid lowering behavior remain unchanged.\n\n'''
if anchor not in text:
    raise SystemExit('roadmap insertion anchor not found')
text = text.replace(anchor, block + anchor, 1)
path.write_text(text)
