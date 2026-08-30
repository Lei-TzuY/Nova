from pathlib import Path

readme = Path('README.md')
text = readme.read_text()
old = '''Every expression that completes with an ordinary runtime value also has a final
interpreter postcondition: the value must recursively conform to that expression's
typed-HIR result type. This closes gaps for local or discarded literals, projections,
operators, blocks, conditionals, matches, and other values that may never cross a
function, aggregate, or frame-storage boundary. Equality adds an operator-level
'''
new = '''Every expression that completes with an ordinary runtime value also has a final
interpreter postcondition: the value must recursively conform to that expression's
typed-HIR result type. Runtime conformance first validates the resolved type itself:
nominal record/enum names must still match their declaration IDs, and function
signatures recursively apply the same rule to parameter and return types. This closes
a malformed-HIR gap where a record or enum value with the correct nominal ID could
previously satisfy a drifted `Type::Record`/`Type::Enum` spelling. The same entry gate
therefore protects local or discarded literals, projections, operators, blocks,
conditionals, matches, call boundaries, frame storage, and aggregate nesting without
changing the compact runtime value representation. Equality adds an operator-level
'''
if old not in text:
    raise SystemExit('README runtime postcondition anchor not found')
readme.write_text(text.replace(old, new, 1))

roadmap = Path('docs/roadmap.md')
text = roadmap.read_text()
status_old = '**Status: twenty vertical slices implemented; execution surface remains small.**'
status_new = '**Status: twenty-one vertical slices implemented; execution surface remains small.**'
if status_old not in text:
    raise SystemExit('Phase 3 status anchor not found')
text = text.replace(status_old, status_new, 1)
anchor = '''Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
contracts:
'''
entry = '''Implemented in the twenty-first Phase 3 slice:

- runtime value/type conformance now first validates the resolved HIR type identity instead of
  accepting nominal IDs while ignoring retained record/enum declaration spellings;
- `Type::Record` and `Type::Enum` require both stable declaration identity and declared name to
  agree, while `FunctionType` recursively validates every parameter and return type under the
  same rule before a function value can conform;
- the single conformance entry gate automatically strengthens expression postconditions,
  function arguments/returns, frame storage, aggregate field/payload validation, and nested
  nominal values without adding per-boundary duplicate checks;
- malformed HIR with same-ID/wrong-name outer types, nested record-field or enum-payload types,
  or drifted nominal function signatures now fails closed as `N4005`, while `Never`/`Error`
  remain impossible runtime value types; and
- focused adversarial regressions plus a nested record/enum/match positive control and
  all-targets Clippy coverage lock the contract without changing HIR shape, semantic-inspection
  schemas, runtime value representation, syntax, layout, ABI, or valid-source behavior.

'''
if anchor not in text:
    raise SystemExit('Phase 3 next-slices anchor not found')
roadmap.write_text(text.replace(anchor, entry + anchor, 1))
