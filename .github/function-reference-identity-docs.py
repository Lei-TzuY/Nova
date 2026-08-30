from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {count}")
    file.write_text(text.replace(old, new, 1))

replace_once(
    "README.md",
    "comparison uses the resolved variant slot. Function values are comparable only at the\nsame fully resolved signature and compare top-level declaration identity. Enums with any\npayload variant and records remain non-comparable. Closed-condition analysis can prove\nliteral Unit, direct payload-free enum-constructor, and direct function-reference\ncomparisons, while locals and calls remain dynamic and are still evaluated at runtime.",
    "comparison uses the resolved variant slot. Function values are comparable only at the\nsame fully resolved signature and compare top-level declaration identity. Direct top-level\nfunction-reference HIR retains the source-resolved spelling alongside `FunctionId`; runtime\nand semantic-inspection consumers recheck that name/id/signature contract, so malformed HIR\ncannot silently retarget a reference to a same-signature sibling declaration. Validated local\naliases still carry only runtime declaration identity rather than source spelling. Enums with\nany payload variant and records remain non-comparable. Closed-condition analysis can prove\nliteral Unit, direct payload-free enum-constructor, and direct function-reference\ncomparisons, while locals and calls remain dynamic and are still evaluated at runtime.",
    "README semantic function identity",
)
replace_once(
    "README.md",
    "Function calls also validate the runtime/HIR type boundary. Every argument must\nrecursively conform to its resolved parameter type, and every returned runtime\nvalue must recursively conform to the function's declared type. Nominal record\nand enum identities, record slots, and enum payloads are checked rather than\ntrusted from their outer value tag alone. Valid semantically produced HIR is\nunaffected; malformed or contract-drifted HIR fails closed with `N4005`.",
    "Function calls also validate the runtime/HIR type boundary. Every argument must\nrecursively conform to its resolved parameter type, and every returned runtime\nvalue must recursively conform to the function's declared type. A direct top-level\nfunction reference additionally revalidates its retained source spelling against the\nreferenced declaration's `FunctionId` before producing `Value::Function`; the ordinary\nexpression postcondition then independently checks the declaration signature against\nthe reference HIR type. Local aliases receive only that already-validated runtime\nidentity. Nominal record and enum identities, record slots, and enum payloads are\nchecked rather than trusted from their outer value tag alone. Valid semantically\nproduced HIR is unaffected; malformed or contract-drifted HIR fails closed with `N4005`.",
    "README runtime function identity",
)

replace_once(
    "docs/roadmap.md",
    "**Status: forty-three vertical slices implemented; broader type-system work remains.**",
    "**Status: forty-four vertical slices implemented; broader type-system work remains.**",
    "Phase 2 status",
)
replace_once(
    "docs/roadmap.md",
    "- producer plus inspection corruption regressions lock same-shaped sibling variants against\n  silent retargeting while keeping nominal enum and pattern semantics unchanged.\n\nThe next Phase 2 slices should address semantic depth rather than widen syntax",
    "- producer plus inspection corruption regressions lock same-shaped sibling variants against\n  silent retargeting while keeping nominal enum and pattern semantics unchanged.\n\nImplemented in the forty-fourth Phase 2 slice:\n\n- direct top-level function-reference HIR retains the source-resolved function spelling\n  alongside stable source-order `FunctionId` identity instead of relying on signature shape\n  alone to identify a declaration;\n- semantic lowering preserves that name/id pair while first-class local aliases continue to\n  use the existing function type and runtime declaration identity without carrying source text;\n- semantic inspection independently requires function spelling, `FunctionId`, and resolved\n  signature to agree before publishing the existing stable function target ID, leaving schema\n  v1/v2 shape unchanged;\n- closed-condition identity proof remains `FunctionId`-based inside the analyzer-owned HIR\n  consumer, avoiding a second source of truth in the same trusted lowering phase; and\n- producer and inspection corruption regressions reject same-signature sibling retargeting and\n  reference-signature drift without changing syntax, CFG, ABI, or valid-source behavior.\n\nThe next Phase 2 slices should address semantic depth rather than widen syntax",
    "Phase 2 slice 44",
)
replace_once(
    "docs/roadmap.md",
    "**Status: eighteen vertical slices implemented; execution surface remains small.**",
    "**Status: nineteen vertical slices implemented; execution surface remains small.**",
    "Phase 3 status",
)
replace_once(
    "docs/roadmap.md",
    "- adversarial runtime regressions prove both corruption rejection and structured-flow\n  precedence without changing valid execution, enum runtime representation, layout, or ABI.\n\nNext Phase 3 slices should deepen executable semantics without bypassing Phase 2",
    "- adversarial runtime regressions prove both corruption rejection and structured-flow\n  precedence without changing valid execution, enum runtime representation, layout, or ABI.\n\nImplemented in the nineteenth Phase 3 slice:\n\n- direct function-reference evaluation validates retained source spelling against the referenced\n  `FunctionId` declaration before producing the compact `Value::Function(FunctionId)` runtime value;\n- the existing expression-result postcondition remains responsible for signature conformance, so\n  declaration identity drift and function-type drift are checked as distinct runtime invariants;\n- same-signature sibling retargeting now fails closed as `N4005` instead of silently changing call\n  or equality behavior, while validated local aliases continue to execute by declaration identity; and\n- focused runtime regressions plus all-targets Clippy coverage lock direct corruption rejection,\n  valid alias execution, and adaptation of older malformed-HIR function-equality fixtures.\n\nNext Phase 3 slices should deepen executable semantics without bypassing Phase 2",
    "Phase 3 slice 19",
)
