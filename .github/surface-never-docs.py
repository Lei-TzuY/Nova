from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))


replace_once(
    "docs/grammar.md",
    'type_ref            = identifier | function_type ;\n',
    'type_ref            = identifier | "!" | function_type ;\n',
)

replace_once(
    "docs/grammar.md",
    '''Function types use the recursive surface form `fn(T1, T2) -> U`; zero parameters and\na trailing comma are allowed, and parameter/return positions may themselves be function\ntypes. The form is accepted anywhere a type reference is accepted, including function\nsignatures, local annotations, record fields, and enum payloads. This enables named\ntop-level function values to be passed, returned, stored, and invoked through explicit\nsignatures. It does not introduce lambdas, closures, captured environments, methods, or\nimplicit callable coercions. Recursive type parsing has its own finite nesting budget and\nreports `N2009` rather than recursing without bound.\n''',
    '''Function types use the recursive surface form `fn(T1, T2) -> U`; zero parameters and\na trailing comma are allowed, and parameter/return positions may themselves be function\ntypes. The form is accepted anywhere a type reference is accepted, including function\nsignatures, local annotations, record fields, and enum payloads. This enables named\ntop-level function values to be passed, returned, stored, and invoked through explicit\nsignatures. It does not introduce lambdas, closures, captured environments, methods, or\nimplicit callable coercions. Recursive type parsing has its own finite nesting budget and\nreports `N2009` rather than recursing without bound.\n\n`!` is the surface spelling of Nova's uninhabited bottom type. It is accepted anywhere a\ntype reference is accepted, including nested function signatures. A continuing expression\ncan never produce a value of type `!`; instead, expressions such as a call to a declared\n`fn() -> !` are non-continuing and remain compatible with any expected result position. A\nfunction declared `-> !` must itself be proven non-continuing on every path, for example by\na guaranteed loop with no reachable `break`; ordinary fallthrough or a continuing tail is\nrejected. `!` introduces no runtime value, layout, allocation, or ABI representation.\n''',
)

replace_once(
    "README.md",
    '''- check bootstrap `Int`, `Bool`, `Unit`, and nominal aggregate types, function\n''',
    '''- check bootstrap `Int`, `Bool`, `Unit`, the uninhabited `!` bottom type, and nominal aggregate types, function\n''',
)

replace_once(
    "README.md",
    '''closures, captured environments, methods, or implicit callable conversions.\n\nRejected calls are fail-closed for continuing flow recovery.\n''',
    '''closures, captured environments, methods, or implicit callable conversions.\n\nThe surface type `!` exposes the semantic core's existing uninhabited bottom type. A\n`fn forever() -> !` signature states that the function has no continuing return path; calls\nto such a function therefore fit any expected value position without manufacturing a value.\n`!` may appear in any type-reference position, including inside function types, but no\nordinary runtime `Value` can inhabit it. A `-> !` body that falls through or produces a\ncontinuing tail is rejected, while proven non-continuation such as `while true {}` with no\nreachable `break` satisfies the contract. Semantic-inspection v1/v2/v3 already represent\nNever, so exposing the spelling does not change any inspection schema.\n\nRejected calls are fail-closed for continuing flow recovery.\n''',
)

replace_once(
    "docs/language-constitution.md",
    '''surface types `Int`, `Bool`, `Unit`, and declared nominal record and enum types.\n''',
    '''surface types `Int`, `Bool`, `Unit`, the uninhabited bottom type `!`, and declared nominal record and enum types.\n''',
)

replace_once(
    "docs/language-constitution.md",
    '''unimplemented and must not be inferred from this slice.\n\nA bootstrap record declares explicitly typed, uniquely named fields.\n''',
    '''unimplemented and must not be inferred from this slice.\n\nThe bootstrap surface spelling `!` denotes the semantic core's existing uninhabited bottom\ntype. It is a real type rather than a runtime sentinel: no ordinary value can conform to it,\nand a function declared `-> !` is accepted only when every reachable path is non-continuing.\nBecause `!` is bottom for expected-type compatibility and branch/match joins, a call that\nreturns `!` can occupy an otherwise value-producing position without inventing a coercion or\nvalue. The spelling is accepted in any type-reference position, including nested function\ntypes; uninhabited fields, payloads, parameters, or locals remain type-correct declarations\nthat cannot receive an ordinary runtime value. This surface exposure adds no layout, ABI,\nallocation, exception, panic, or process-termination semantics.\n\nA bootstrap record declares explicitly typed, uniquely named fields.\n''',
)

replace_once(
    "docs/roadmap.md",
    '**Status: four vertical slices implemented; broader grammar work remains.**',
    '**Status: five vertical slices implemented; broader grammar work remains.**',
)

replace_once(
    "docs/roadmap.md",
    '''Next Phase 1 refinements should be driven by the needs of later semantic work,\nnot by adding unrelated syntax.\n''',
    '''Implemented in the fifth Phase 1 slice:\n\n- `!` becomes an explicit surface type reference using the lexer\'s existing bang token in\n  type context, so no new lexical form or keyword is introduced;\n- AST type references preserve a dedicated Never form rather than encoding `!` as a magic\n  identifier, and the same recursive type grammar permits it in direct or nested positions;\n- parser regressions cover direct parameter/return positions plus nested `fn() -> !` and\n  `fn(!) -> !` signatures without changing the existing type-depth budget; and\n- an end-to-end CLI fixture exercises the spelling through check, run, and all supported\n  semantic-inspection schema versions while the executing branch still returns `42`.\n\nNext Phase 1 refinements should be driven by the needs of later semantic work,\nnot by adding unrelated syntax.\n''',
)

replace_once(
    "docs/roadmap.md",
    '**Status: fifty-one vertical slices implemented; broader type-system work remains.**',
    '**Status: fifty-two vertical slices implemented; broader type-system work remains.**',
)

replace_once(
    "docs/roadmap.md",
    '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n''',
    '''Implemented in the fifty-second Phase 2 slice:\n\n- surface `!` resolves directly to the existing HIR `Type::Never`, preserving one bottom-type\n  identity across source signatures, expected-type compatibility, control-flow typing,\n  semantic inspection, and runtime invariants;\n- a function declared `-> !` must have a body whose reachable result is Never: continuing\n  fallthrough remains `N3007`, while a continuing tail remains the ordinary `N3004` type\n  mismatch instead of receiving a special exception;\n- calls returning Never automatically reuse the established bottom rule, so a diverging arm\n  can join with an `Int`, `Bool`, Unit, nominal, or function-valued continuing alternative\n  without a coercion or fabricated runtime value;\n- `!` is legal in every existing type-reference position, making uninhabited parameters,\n  fields, enum payloads, locals, and nested callable signatures expressible while runtime\n  conformance continues to reject any ordinary value pretending to inhabit Never; and\n- semantic and inspection regressions prove that v1/v2/v3 already publish the existing\n  `never` type fact and display `!`, so no tooling schema or runtime representation changes.\n\nThe next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n''',
)
