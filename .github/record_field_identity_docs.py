from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected one anchor in {path}, found {text.count(old)}: {old[:80]!r}")
    file.write_text(text.replace(old, new, 1))


replace_once(
    "README.md",
    """initializers may be written in any order, but their expressions evaluate left\nto right in written source order. HIR resolves each initializer to a stable\nrecord identity and declaration-order field slot without reordering evaluation.\n`value.field` is read-only field projection in this slice. Record equality,""",
    """initializers may be written in any order, but their expressions evaluate left\nto right in written source order. HIR retains each resolved field spelling alongside\nthe nominal record identity and declaration-order slot without reordering evaluation.\nThe interpreter and semantic-inspection boundary independently recheck that name/slot\npair, so malformed HIR cannot silently retarget one field to a same-typed sibling while\nthe stable inspection schema continues to expose its existing declaration field ID.\n`value.field` is read-only field projection in this slice. Record equality,""",
)
replace_once(
    "README.md",
    """Aggregate construction enforces the same invariant locally: each evaluated record\nfield must conform to its declaration slot type, and each enum payload must conform\nto its selected variant payload type before the aggregate value is created. This\ncatches malformed HIR even when the aggregate never crosses a function boundary.""",
    """Aggregate construction enforces the same invariant locally: each evaluated record\nfield must conform to its declaration slot type, and each enum payload must conform\nto its selected variant payload type before the aggregate value is created. Record\nconstruction and projection also revalidate the HIR-resolved field spelling against\nits declaration-order slot, closing same-typed member-retargeting drift that a type\npostcondition alone cannot observe. These checks catch malformed HIR even when the\naggregate never crosses a function boundary.""",
)

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
text = text.replace(
    "**Status: forty-one vertical slices implemented; broader type-system work remains.**",
    "**Status: forty-two vertical slices implemented; broader type-system work remains.**",
    1,
)
phase2_anchor = """The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely."""
phase2 = """Implemented in the forty-second Phase 2 slice:\n\n- resolved record construction and projection HIR now retain the source-resolved field\n  spelling alongside nominal `RecordId` and declaration-order slot identity;\n- constructor initializers still preserve written source evaluation order while the\n  retained name/slot pair makes same-typed member retargeting observable to later trusted\n  consumers instead of relying on type equality alone;\n- `nova-inspect` independently validates each retained field name against the referenced\n  declaration slot before publishing the existing stable `record:R.field:F` target;\n- semantic-inspection v1/v2 schema shape and document IDs remain unchanged because the\n  additional spelling is compiler-owned integrity metadata, not a new tooling protocol field;\n- direct semantic and inspection corruption regressions lock reversed written initializer\n  order, same-typed constructor slot swaps, and same-typed projection retargeting; and\n- the slice changes no surface syntax, record layout, field mutability, ownership, ABI, CFG,\n  or valid-source behavior.\n\n"""
if text.count(phase2_anchor) != 1:
    raise SystemExit("Phase 2 anchor mismatch")
text = text.replace(phase2_anchor, phase2 + phase2_anchor, 1)
text = text.replace(
    "**Status: sixteen vertical slices implemented; execution surface remains small.**",
    "**Status: seventeen vertical slices implemented; execution surface remains small.**",
    1,
)
phase3_anchor = """Next Phase 3 slices should deepen executable semantics without bypassing Phase 2\ncontracts:"""
phase3 = """Implemented in the seventeenth Phase 3 slice:\n\n- record construction verifies that every value-producing initializer's retained field\n  spelling still names its resolved declaration-order destination slot before storage;\n- field projection preserves structured noncontinuation from its base, then rechecks the\n  retained field spelling, nominal record identity, slot, and declared result type before\n  returning an ordinary runtime value;\n- malformed HIR that swaps two same-typed constructor slots or retargets a projection to a\n  same-typed sibling now fails closed as `N4005` instead of silently changing program meaning;\n- the checks complement recursive runtime type conformance rather than duplicating it,\n  covering semantic identity drift that remains type-correct at runtime; and\n- focused malformed-HIR regressions plus normal record execution keep valid evaluation order,\n  runtime representation, and source semantics unchanged.\n\n"""
if text.count(phase3_anchor) != 1:
    raise SystemExit("Phase 3 anchor mismatch")
text = text.replace(phase3_anchor, phase3 + phase3_anchor, 1)
roadmap.write_text(text)
