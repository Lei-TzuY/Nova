from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {count}")
    return text.replace(old, new, 1)

# README
path = Path("README.md")
text = path.read_text()
text = replace_once(
    text,
    "- resolve top-level functions and nominal types, parameters, lexical local\n  bindings, record field slots, enum variant slots, and match payload bindings;",
    "- resolve top-level functions and nominal types, parameters, lexical local\n  bindings, record-field and enum-variant name/slot identities, and match payload bindings;",
    "README resolution bullet",
)
text = replace_once(
    text,
    "variant must occur exactly once. Payload bindings are immutable and scoped to\none arm. Wildcards, guards, nested patterns, multi-payload variants, equality for\npayload-bearing enums, layout, and ABI guarantees are not implemented.",
    "variant must occur exactly once. Payload bindings are immutable and scoped to\none arm. HIR retains the source-resolved variant spelling alongside the nominal enum\nidentity and declaration-order slot for both constructors and match arms. Runtime and\nsemantic-inspection consumers recheck that name/slot pair, so malformed HIR cannot\nsilently retarget a constructor or pattern to a same-shaped sibling variant. Wildcards,\nguards, nested patterns, multi-payload variants, equality for payload-bearing enums,\nlayout, and ABI guarantees are not implemented.",
    "README enum identity paragraph",
)
text = replace_once(
    text,
    "construction and projection also revalidate the HIR-resolved field spelling against\nits declaration-order slot, closing same-typed member-retargeting drift that a type\npostcondition alone cannot observe. These checks catch malformed HIR even when the\naggregate never crosses a function boundary.",
    "construction and projection also revalidate the HIR-resolved field spelling against\nits declaration-order slot, closing same-typed member-retargeting drift that a type\npostcondition alone cannot observe. Enum construction and matching apply the analogous\nvariant spelling/slot check after payload or scrutinee evaluation has produced an ordinary\nvalue, preserving structured return/break/continue propagation before value-only invariant\nvalidation. These checks catch malformed HIR even when the aggregate never crosses a\nfunction boundary.",
    "README runtime aggregate identity",
)
path.write_text(text)

# Normative enum/pattern document.
path = Path("docs/enums-and-patterns.md")
text = path.read_text()
text = replace_once(
    text,
    "Enum identity comes from its declaration, not from its variant spelling or\nshape. Separately declared enums are different types. All record and enum names\nare collected before payload, field, function-signature, or body types are\nresolved, so forward references and recursive enum payloads are deterministic.",
    "Enum identity comes from its declaration, not from its variant spelling or\nshape. Separately declared enums are different types. Within one enum, semantic HIR\nretains each source-resolved variant spelling together with its declaration-order slot;\nthe spelling is compiler-owned integrity metadata rather than runtime string lookup. All\nrecord and enum names are collected before payload, field, function-signature, or body\ntypes are resolved, so forward references and recursive enum payloads are deterministic.",
    "enum doc declaration identity",
)
text = replace_once(
    text,
    "HIR assigns each enum an `EnumId` and each variant its declaration-order slot.\nConstructors and match arms use these resolved identities rather than runtime\nstring lookup. Runtime enum values carry the `EnumId`, variant slot, and an\noptional boxed payload. Before executing a match, the interpreter verifies the\nresolved arms are exhaustive, non-duplicated, in range, and payload-compatible;\nmalformed HIR produces runtime invariant diagnostic `N4005`.",
    "HIR assigns each enum an `EnumId` and each variant its declaration-order slot.\nConstructors and match arms retain the source-resolved variant spelling alongside that\nslot, allowing trusted consumers to verify that the two still identify the same declared\nmember. Runtime enum values remain compact and carry only the `EnumId`, variant slot,\nand optional boxed payload; the retained spelling does not become a runtime layout or\nABI field. Semantic inspection independently checks the name/slot pair before publishing\nits existing stable variant IDs, so schema v1/v2 do not gain a new field.\n\nAt execution time, constructor payload evaluation happens before value-only variant\nidentity validation, and a match evaluates its scrutinee before validating the complete\nresolved arm table. Thus a payload or scrutinee that returns, breaks, or continues keeps\nits established structured flow. Once an ordinary enum value is required, the interpreter\nrechecks enum identity, variant spelling/slot agreement, payload arity/type, exhaustiveness,\nand duplicate coverage; malformed HIR produces invariant diagnostic `N4005`.",
    "enum doc HIR execution",
)
path.write_text(text)

# Roadmap counters and new slices.
path = Path("docs/roadmap.md")
text = path.read_text()
text = replace_once(
    text,
    "**Status: forty-two vertical slices implemented; broader type-system work remains.**",
    "**Status: forty-three vertical slices implemented; broader type-system work remains.**",
    "Phase 2 count",
)
phase2_entry = """Implemented in the forty-third Phase 2 slice:

- enum-constructor and exhaustive-match HIR retain the source-resolved variant spelling
  alongside nominal `EnumId` and declaration-order variant slot identity;
- constructor and pattern lowering preserve that name/slot pair without changing source
  syntax, payload evaluation order, match-arm order, or the compact runtime enum value;
- semantic inspection independently rejects variant spelling/slot drift before projecting
  the existing stable variant IDs, leaving schema v1/v2 byte shape unchanged;
- closed-condition proof remains slot-based inside semantic analysis because it consumes
  HIR produced in the same trusted lowering pass rather than treating the spelling as a
  second semantic source of truth; and
- producer plus inspection corruption regressions lock same-shaped sibling variants against
  silent retargeting while keeping nominal enum and pattern semantics unchanged.

"""
text = replace_once(
    text,
    "The next Phase 2 slices should address semantic depth rather than widen syntax",
    phase2_entry + "The next Phase 2 slices should address semantic depth rather than widen syntax",
    "Phase 2 insertion",
)
text = replace_once(
    text,
    "**Status: seventeen vertical slices implemented; execution surface remains small.**",
    "**Status: eighteen vertical slices implemented; execution surface remains small.**",
    "Phase 3 count",
)
phase3_entry = """Implemented in the eighteenth Phase 3 slice:

- enum construction evaluates an optional payload first and only revalidates resolved
  enum/variant name-slot identity when that payload completes with an ordinary value;
- exhaustive matching likewise evaluates the scrutinee before validating the complete arm
  identity/exhaustiveness table, so structured return/break/continue cannot be preempted by
  value-only malformed-HIR checks;
- the interpreter rejects same-payload-type constructor retargeting and exhaustive
  same-shape pattern-slot swaps with `N4005`, while independently preserving payload arity,
  payload type, duplicate-arm, and nominal-enum invariants; and
- adversarial runtime regressions prove both corruption rejection and structured-flow
  precedence without changing valid execution, enum runtime representation, layout, or ABI.

"""
text = replace_once(
    text,
    "Next Phase 3 slices should deepen executable semantics without bypassing Phase 2\ncontracts:",
    phase3_entry + "Next Phase 3 slices should deepen executable semantics without bypassing Phase 2\ncontracts:",
    "Phase 3 insertion",
)
path.write_text(text)
