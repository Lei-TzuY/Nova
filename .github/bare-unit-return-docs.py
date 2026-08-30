from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:140]!r}")
    p.write_text(text.replace(old, new, 1))


replace_once(
    "docs/grammar.md",
    'return_statement    = "return" , expression , ";" ;\n',
    'return_statement    = "return" , [ expression ] , ";" ;\n',
)
replace_once(
    "docs/grammar.md",
    '''`Unit` is a built-in surface type and `()` is its sole literal. A block with no\ntail expression also has type `Unit`. A function declared `-> Unit` may therefore\nfall through a value-less body; `return ();` is the explicit equivalent. Other\nreturn types still require a compatible tail or an explicit return on every\ncontinuing path. Parenthesized non-empty expressions retain ordinary grouping, so\n`(value)` is not a Unit literal.\n''',
    '''`Unit` is a built-in surface type and `()` is its sole literal. A block with no\ntail expression also has type `Unit`. A function declared `-> Unit` may therefore\nfall through a value-less body, return Unit explicitly as `return ();`, or use the\ncompact `return;` spelling. Bare `return;` is semantically a Unit return rather than a\nvalueless control operation, so it is rejected by the ordinary return-type check in\n`Int`, `Bool`, nominal, function-valued, or `!`-returning functions. The AST and HIR\nnevertheless preserve whether source wrote a bare return instead of manufacturing a\nsynthetic `()` expression. Other return types still require a compatible tail or an\nexplicit value-returning path. Parenthesized non-empty expressions retain ordinary\ngrouping, so `(value)` is not a Unit literal.\n''',
)

replace_once(
    "README.md",
    '''  expressions, blocks, calls, `if` expressions, pre-test `while` loops, and\n  statement-only `break`/`continue`;\n''',
    '''  expressions, blocks, calls, `if` expressions, pre-test `while` loops, bare Unit\n  returns, and statement-only `break`/`continue`;\n''',
)
replace_once(
    "README.md",
    '''Rejected calls are fail-closed for continuing flow recovery.\n''',
    '''A `Unit`-returning function may now write `return;` as the compact explicit form of\nreturning Unit. Semantic analysis checks the bare form as `Unit` against the declared\nreturn type, so non-Unit functions receive the same `N3004` mismatch used for an explicit\nwrongly typed return expression. AST and HIR retain a bare return separately from\n`return ();`; the interpreter produces the ordinary `Value::Unit`, and the existing\nfunction-boundary conformance check still rejects malformed HIR that claims a different\nreturn type. Semantic-inspection v1/v2/v3 already model a return statement with zero child\nexpressions, so this source distinction needs no schema version bump.\n\nRejected calls are fail-closed for continuing flow recovery.\n''',
)

replace_once(
    "docs/language-constitution.md",
    '''initialization. A function declared to return `Unit` may complete through a body\nwith no tail expression; `return ();` is the explicit Unit return. Functions with\nother return types still require a compatible value on every continuing path.\n''',
    '''initialization. A function declared to return `Unit` may complete through a body\nwith no tail expression, return Unit explicitly as `return ();`, or use the compact\n`return;` spelling. The bare form is semantically a Unit return, not a second valueless\nreturn category; functions with other return types therefore reject it through the same\nreturn-type compatibility rule that governs explicit expressions. AST/HIR retain the\nsource distinction instead of synthesizing a Unit expression. Functions with other return\ntypes still require a compatible value on every continuing path.\n''',
)

replace_once(
    "docs/roadmap.md",
    '**Status: five vertical slices implemented; broader grammar work remains.**',
    '**Status: six vertical slices implemented; broader grammar work remains.**',
)
replace_once(
    "docs/roadmap.md",
    '''Next Phase 1 refinements should be driven by the needs of later semantic work,\nnot by adding unrelated syntax.\n''',
    '''Implemented in the sixth Phase 1 slice:\n\n- `return_statement` accepts an optional expression, admitting the compact `return;` form\n  without changing expression grammar or semicolon rules;\n- parser AST retains `None` for a bare return and `Some(expression)` for value-bearing\n  returns, so source intent is not rewritten into a synthetic `()` node;\n- `return ();` remains valid and distinct in the syntax tree while both forms can denote\n  the same Unit result after semantic checking; and\n- parser plus CLI regressions cover bare/value-bearing preservation and complete\n  check/AST/run/inspection traversal.\n\nNext Phase 1 refinements should be driven by the needs of later semantic work,\nnot by adding unrelated syntax.\n''',
)

replace_once(
    "docs/roadmap.md",
    '**Status: fifty-three vertical slices implemented; broader type-system work remains.**',
    '**Status: fifty-four vertical slices implemented; broader type-system work remains.**',
)
replace_once(
    "docs/roadmap.md",
    '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n''',
    '''Implemented in the fifty-fourth Phase 2 slice:\n\n- bare `return;` is checked as an explicit Unit result against the function's declared\n  return type, reusing ordinary expected-type compatibility and `N3004` rather than\n  introducing a return-specific exception or diagnostic family;\n- accepted HIR preserves the absence of a source expression as `Return(None)`, while\n  `return ();` remains `Return(Some(Unit))`, keeping syntax identity separate from value\n  semantics;\n- every accepted bare return emits the same verified CFG `Return` transfer used by an\n  ordinary continuing return expression, while noncontinuing value expressions retain\n  their established rule against appending a duplicate transfer;\n- the bare form makes its containing path noncontinuing exactly like other returns and\n  therefore participates in existing branch, loop, definite-initialization, and\n  unreachable-warning behavior without new side state; and\n- semantic-inspection v1/v2/v3 naturally publish the existing Return statement with an\n  empty expression list, requiring no schema reinterpretation or version bump.\n\nThe next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n''',
)

replace_once(
    "docs/roadmap.md",
    '**Status: twenty-two vertical slices implemented; execution surface remains small.**',
    '**Status: twenty-three vertical slices implemented; execution surface remains small.**',
)
replace_once(
    "docs/roadmap.md",
    '''Next Phase 3 slices should deepen executable semantics without bypassing Phase 2\ncontracts:\n''',
    '''Implemented in the twenty-third Phase 3 slice:\n\n- the interpreter executes HIR `Return(None)` as structured `Flow::Return(Value::Unit)`\n  without allocating or evaluating a synthetic expression;\n- calls to Unit procedures using bare return therefore reuse the same ordinary Unit runtime\n  value and function-call continuation behavior as `return ();` and Unit fallthrough;\n- the existing function-boundary value/type conformance check independently rejects\n  malformed HIR that retags a bare-Unit-returning function as `Int`, `Bool`, nominal,\n  function, or Never, preserving `N4005` defense in depth;\n- structured return propagation through blocks, loops, calls, and expressions remains\n  unchanged because the new form enters the already-established Return flow channel; and\n- runtime plus end-to-end CLI regressions prove successful Unit return and a `42` caller\n  result without changing runtime `Value`, CFG, inspection schema, layout, or ABI.\n\nNext Phase 3 slices should deepen executable semantics without bypassing Phase 2\ncontracts:\n''',
)
