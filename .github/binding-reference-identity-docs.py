from pathlib import Path


def replace_exact(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    actual = text.count(old)
    if actual != count:
        raise SystemExit(f"{path}: expected {count} occurrences, found {actual}: {old[:100]!r}")
    p.write_text(text.replace(old, new, count))

readme = "README.md"
replace_exact(
    readme,
    """parameters are rejected as assignment targets. The replacement value must keep\nthe binding's established type. Assignment is not an expression and therefore\ncannot be chained or embedded in another expression.\n""",
    """parameters are rejected as assignment targets. The replacement value must keep\nthe binding's established type. Resolved local/parameter reads and assignment targets\nretain the selected declaration's `BindingId`, source spelling, and declaration-name\nspan in HIR. The declaration span is part of the integrity pair because nested scopes\nmay legally shadow a binding with the same spelling and type; name/type alone cannot\ndistinguish those declarations. This metadata does not alter lexical resolution, CFG\nbinding IDs, or the semantic-inspection schema. Assignment is not an expression and\ntherefore cannot be chained or embedded in another expression.\n""",
)
replace_exact(
    readme,
    """Runtime frames preserve the resolved binding contract too. Each slot records its\nresolved type, mutability, and initialization state. Parameters, local bindings,\ndelayed `var` declarations, and match payload bindings reject non-conforming initial\nvalues or incompatible reuse of one binding identity; repeated execution of the same\nlexical binding may refresh its slot only with identical type/mutability metadata.\nAssignment additionally requires a mutable slot and a conforming replacement value.\nBinding reads verify that the HIR expression type and stored runtime value still agree\nwith the slot contract. Any such interpreter/HIR drift fails closed with `N4005`.\n""",
    """Runtime frames preserve the resolved binding contract too. Each slot records its\nresolved type, mutability, initialization state, declaration spelling, and declaration\nspan. Parameters, local bindings, delayed `var` declarations, and match payload bindings\nreject non-conforming initial values or incompatible reuse of one binding identity;\nrepeated execution of the same lexical binding may refresh its slot only with identical\ntype/mutability/name/span metadata. Binding reads revalidate the retained HIR reference\nagainst that slot before checking expression type and stored runtime-value conformance,\nso even same-name, same-type shadow retargeting fails closed. Assignment evaluates its\nRHS first; only an ordinary produced value triggers target identity, mutability, and\nreplacement-type validation, preserving structured `return`/`break`/`continue` precedence.\nAny such interpreter/HIR drift fails closed with `N4005`.\n""",
)

roadmap = "docs/roadmap.md"
replace_exact(
    roadmap,
    "**Status: forty-four vertical slices implemented; broader type-system work remains.**",
    "**Status: forty-five vertical slices implemented; broader type-system work remains.**",
)
replace_exact(
    roadmap,
    """- producer and inspection corruption regressions reject same-signature sibling retargeting and\n  reference-signature drift without changing syntax, CFG, ABI, or valid-source behavior.\n\nThe next Phase 2 slices should address semantic depth rather than widen syntax\n""",
    """- producer and inspection corruption regressions reject same-signature sibling retargeting and\n  reference-signature drift without changing syntax, CFG, ABI, or valid-source behavior.\n\nImplemented in the forty-fifth Phase 2 slice:\n\n- local/parameter reads and assignment targets retain a `BindingReference` containing the\n  resolved `BindingId`, declaration spelling, and declaration-name span rather than relying\n  on the numeric id and result type alone at downstream trust boundaries;\n- declaration span is intentionally retained because lexical shadowing can produce two\n  simultaneously valid bindings with the same name and type, making spelling/type\n  insufficient to detect same-shaped retargeting;\n- semantic lowering preserves this identity triple while CFG read/initialize events continue\n  to use the existing `BindingId`, so definite-initialization remains a single verified graph\n  contract rather than acquiring parallel name/span flow state;\n- semantic inspection independently cross-checks the retained name/id/span against the\n  already-projected binding declaration before publishing the existing stable binding target,\n  leaving schema v1/v2 unchanged; and\n- producer and adversarial inspection regressions lock assignment targets and same-name\n  shadow references without changing source scoping, assignment syntax, or valid behavior.\n\nThe next Phase 2 slices should address semantic depth rather than widen syntax\n""",
)
replace_exact(
    roadmap,
    "**Status: nineteen vertical slices implemented; execution surface remains small.**",
    "**Status: twenty vertical slices implemented; execution surface remains small.**",
)
replace_exact(
    roadmap,
    """- focused runtime regressions plus all-targets Clippy coverage lock direct corruption rejection,\n  valid alias execution, and adaptation of older malformed-HIR function-equality fixtures.\n\nNext Phase 3 slices should deepen executable semantics without bypassing Phase 2\n""",
    """- focused runtime regressions plus all-targets Clippy coverage lock direct corruption rejection,\n  valid alias execution, and adaptation of older malformed-HIR function-equality fixtures.\n\nImplemented in the twentieth Phase 3 slice:\n\n- runtime frame slots retain declaration spelling and declaration span alongside their existing\n  type, mutability, initialization state, and `BindingId` key;\n- binding reads require the HIR reference id/name/span triple to match the live slot before\n  returning a value, closing same-name, same-type shadow retargeting that type checks cannot see;\n- assignments evaluate the RHS first and validate target identity only after an ordinary value\n  is produced, so malformed target metadata cannot preempt structured return/break/continue flow;\n- target identity validation remains distinct from the existing mutability and replacement-type\n  checks, preserving defense in depth and repeated lexical-binding execution; and\n- adversarial runtime regressions lock same-type assignment retargeting, same-name shadow reads,\n  RHS structured-flow precedence, and unchanged valid frame behavior under `N4005` fail-closed policy.\n\nNext Phase 3 slices should deepen executable semantics without bypassing Phase 2\n""",
)
