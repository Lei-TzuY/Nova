from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))


def insert_before_once(path: str, marker: str, addition: str) -> None:
    replace_once(path, marker, addition + marker)


replace_once(
    "docs/grammar.md",
    'type_ref            = identifier | function_type ;\n',
    'type_ref            = identifier | "!" | function_type ;\n',
)
insert_before_once(
    "docs/grammar.md",
    "Records are nominal top-level types. Each field has an explicit type and field\n",
    "`!` is the surface spelling of Nova's uninhabited bottom type. It is accepted anywhere a\n"
    "type reference is accepted, including nested function signatures. A continuing expression\n"
    "can never produce a value of type `!`; instead, a call to a declared `fn() -> !` is\n"
    "non-continuing and remains compatible with any expected result position. A function\n"
    "declared `-> !` must itself be proven non-continuing on every path, for example by a\n"
    "guaranteed loop with no reachable `break`; ordinary fallthrough or a continuing tail is\n"
    "rejected. `!` introduces no runtime value, layout, allocation, or ABI representation.\n\n",
)

replace_once(
    "README.md",
    '- check bootstrap `Int`, `Bool`, `Unit`, and nominal aggregate types, function\n',
    '- check bootstrap `Int`, `Bool`, `Unit`, the uninhabited `!` bottom type, and nominal aggregate types, function\n',
)
insert_before_once(
    "README.md",
    "Rejected calls are fail-closed for continuing flow recovery. Callees and arguments\n",
    "The surface type `!` exposes the semantic core's existing uninhabited bottom type. A\n"
    "`fn forever() -> !` signature states that the function has no continuing return path; calls\n"
    "to such a function therefore fit any expected value position without manufacturing a value.\n"
    "`!` may appear in any type-reference position, including inside function types, but no\n"
    "ordinary runtime `Value` can inhabit it. A `-> !` body that falls through or produces a\n"
    "continuing tail is rejected, while proven non-continuation such as `while true {}` with no\n"
    "reachable `break` satisfies the contract. Semantic-inspection v1/v2/v3 already represent\n"
    "Never, so exposing the spelling does not change any inspection schema.\n\n",
)

replace_once(
    "docs/language-constitution.md",
    'surface types `Int`, `Bool`, `Unit`, and declared nominal record and enum types.\n',
    'surface types `Int`, `Bool`, `Unit`, the uninhabited bottom type `!`, and declared nominal record and enum types.\n',
)
insert_before_once(
    "docs/language-constitution.md",
    "A bootstrap record declares explicitly typed, uniquely named fields.\n",
    "The bootstrap surface spelling `!` denotes the semantic core's existing uninhabited bottom\n"
    "type. It is a real type rather than a runtime sentinel: no ordinary value can conform to it,\n"
    "and a function declared `-> !` is accepted only when every reachable path is non-continuing.\n"
    "Because `!` is bottom for expected-type compatibility and branch/match joins, a call that\n"
    "returns `!` can occupy an otherwise value-producing position without inventing a coercion or\n"
    "value. The spelling is accepted in any type-reference position, including nested function\n"
    "types; uninhabited fields, payloads, parameters, or locals remain type-correct declarations\n"
    "that cannot receive an ordinary runtime value. This surface exposure adds no layout, ABI,\n"
    "allocation, exception, panic, or process-termination semantics.\n\n",
)

replace_once(
    "docs/roadmap.md",
    '**Status: four vertical slices implemented; broader grammar work remains.**',
    '**Status: five vertical slices implemented; broader grammar work remains.**',
)
insert_before_once(
    "docs/roadmap.md",
    "Next Phase 1 refinements should be driven by the needs of later semantic work,\n",
    "Implemented in the fifth Phase 1 slice:\n\n"
    "- `!` becomes an explicit surface type reference using the lexer's existing bang token in\n"
    "  type context, so no new lexical form or keyword is introduced;\n"
    "- AST type references preserve a dedicated Never form rather than encoding `!` as a magic\n"
    "  identifier, and the same recursive type grammar permits it in direct or nested positions;\n"
    "- parser regressions cover direct parameter/return positions plus nested `fn() -> !` and\n"
    "  `fn(!) -> !` signatures without changing the existing type-depth budget; and\n"
    "- an end-to-end CLI fixture exercises the spelling through check, run, and all supported\n"
    "  semantic-inspection schema versions while the executing branch still returns `42`.\n\n",
)
replace_once(
    "docs/roadmap.md",
    '**Status: fifty-one vertical slices implemented; broader type-system work remains.**',
    '**Status: fifty-two vertical slices implemented; broader type-system work remains.**',
)
insert_before_once(
    "docs/roadmap.md",
    "The next Phase 2 slices should address semantic depth rather than widen syntax\n",
    "Implemented in the fifty-second Phase 2 slice:\n\n"
    "- surface `!` resolves directly to the existing HIR `Type::Never`, preserving one bottom-type\n"
    "  identity across source signatures, expected-type compatibility, control-flow typing,\n"
    "  semantic inspection, and runtime invariants;\n"
    "- a function declared `-> !` must have a body whose reachable result is Never: continuing\n"
    "  fallthrough remains `N3007`, while a continuing tail remains the ordinary `N3004` type\n"
    "  mismatch instead of receiving a special exception;\n"
    "- calls returning Never automatically reuse the established bottom rule, so a diverging arm\n"
    "  can join with an `Int`, `Bool`, Unit, nominal, or function-valued continuing alternative\n"
    "  without a coercion or fabricated runtime value;\n"
    "- `!` is legal in every existing type-reference position, making uninhabited parameters,\n"
    "  fields, enum payloads, locals, and nested callable signatures expressible while runtime\n"
    "  conformance continues to reject any ordinary value pretending to inhabit Never; and\n"
    "- semantic and inspection regressions prove that v1/v2/v3 already publish the existing\n"
    "  `never` type fact and display `!`, so no tooling schema or runtime representation changes.\n\n",
)
