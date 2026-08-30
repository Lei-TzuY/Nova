from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))


replace_once(
    "README.md",
    "  enum construction, match exhaustiveness and arm types, assignment\n  mutability/type constraints, and CFG-based definite initialization;",
    "  enum construction, match exhaustiveness and arm types, direct-constructor arm\n  usefulness, assignment mutability/type constraints, and CFG-based definite initialization;",
)
replace_once(
    "README.md",
    "  text or JSON Lines, including non-fatal unreachable-code warnings;",
    "  text or JSON Lines, including non-fatal reachability and match-usefulness warnings;",
)
replace_once(
    "README.md",
    "Semantic warnings do not reject an otherwise valid program. The bootstrap\ncurrently reports `N3033` when the verified CFG proves that source follows an\nexecutable `return`, `break`, or `continue`; the warning is written to standard\nerror while `check`, `run`, or `inspect` continues normally. Existing errors\nsuppress this warning pass to avoid recovery cascades.",
    "Semantic warnings do not reject an otherwise valid program. The bootstrap reports\n`N3033` when the verified CFG proves that source follows an executable `return`,\n`break`, or `continue`, and `N3034` when a direct enum-constructor match scrutinee\nproves that an otherwise-valid concrete variant arm can never be selected. Warnings are\nwritten to standard error while `check`, `run`, or `inspect` continues normally. Any\nsemantic error suppresses these deferred warnings to avoid recovery cascades.",
)

replace_once(
    "docs/diagnostics.md",
    "This slice does not warn for constant-selected `if` or `match` branches,\nshort-circuit operands, a statically skipped `while` body, or code after a proven\nnonterminating loop. Those paths may also use diagnostic CFG edges, but expanding\nthe warning policy requires separate noise, usefulness, and suppression design.\n\n## Deliberate limits",
    "`N3033` itself does not report constant-selected `if` or `match` branches,\nshort-circuit operands, a statically skipped `while` body, or code after a proven\nnonterminating loop. Direct-constructor match usefulness has its own `N3034` policy\nbelow rather than being inferred generically from diagnostic CFG edges.\n\n## `N3034` statically unreachable match arm\n\nWhen a successfully resolved `match` scrutinee is a direct enum constructor, semantic\nanalysis already knows its exact declaration-order variant before arm execution. For each\notherwise-valid concrete arm that names a different variant, Nova reports `N3034`:\n\n- primary label: the arm pattern that can never be selected;\n- secondary label: the direct constructor that proves the selected variant;\n- one warning for each non-selected valid arm; and\n- no warning when the scrutinee reaches the match through a local, parameter, call, or\n  other dynamic expression.\n\nA warned arm is still fully lowered and name/type checked. Its CFG path remains\ndiagnostic-only and cannot contribute definite-initialization, non-continuation, or loop\ntransfer facts. `N3034` therefore exposes an existing reachability proof; it does not\nchange HIR, CFG shape, runtime dispatch, or semantic-inspection schemas. Warning candidates\nare deferred until semantic analysis is otherwise error-free, so an error inside a\nnon-selected arm suppresses `N3034` and remains the actionable diagnostic.\n\nThis is the first narrow usefulness diagnostic, not a general pattern-usefulness matrix.\nThere is still no catch-all arm, guard usefulness, nested-pattern coverage, or warning for\ndynamic enum matches.\n\n## Deliberate limits",
)
replace_once(
    "docs/diagnostics.md",
    "Nova has no warning configuration, lint groups, source attributes, command-line\nallow/deny switches, warnings-as-errors mode, cap-lints policy, or cross-package\ndiagnostic aggregation yet. `N3033` is not a claim that CFG reachability is a\ngeneral-purpose linter or that the current warning set is complete.",
    "Nova has no warning configuration, lint groups, source attributes, command-line\nallow/deny switches, warnings-as-errors mode, cap-lints policy, or cross-package\ndiagnostic aggregation yet. `N3033` and `N3034` are narrow implemented proofs, not a\nclaim that CFG reachability or pattern usefulness is a general-purpose linter or that the\ncurrent warning set is complete.",
)

replace_once(
    "docs/enums-and-patterns.md",
    "A direct, successfully resolved enum constructor is a narrower bootstrap\nreachability case. Because its variant is known after the constructor payload has\nbeen evaluated, only the corresponding arm contributes reachable initialization,\nnon-continuation, and enclosing-loop transfer facts. Every non-selected arm is still\nresolved and type checked, still participates in exhaustiveness and result-type\ncompatibility, and can still emit diagnostics; only its runtime flow facts are\ndiscarded. Values flowing through locals, parameters, calls, or other expressions\ndo not receive this refinement.",
    "A direct, successfully resolved enum constructor is a narrower bootstrap\nreachability case. Because its variant is known after the constructor payload has\nbeen evaluated, only the corresponding arm contributes reachable initialization,\nnon-continuation, and enclosing-loop transfer facts. Every non-selected arm is still\nresolved and type checked, still participates in exhaustiveness and result-type\ncompatibility, and can still emit diagnostics; only its runtime flow facts are\ndiscarded. When the complete semantic analysis is otherwise error-free, each such\notherwise-valid non-selected arm also receives nonfatal warning `N3034`, making this\nexisting proof visible as the first narrow match-usefulness diagnostic. Values flowing\nthrough locals, parameters, calls, or other expressions remain dynamic and receive no\n`N3034` guess. An error anywhere in semantic analysis suppresses the deferred warning,\nincluding an error found while checking a non-selected arm.",
)
replace_once(
    "docs/enums-and-patterns.md",
    "| `N3025` | non-enum scrutinee or pattern from another nominal enum |\n| `N4005` | invalid resolved enum/match/control-flow HIR reached the interpreter |",
    "| `N3025` | non-enum scrutinee or pattern from another nominal enum |\n| `N3034` | non-selected concrete arm under a direct-constructor match scrutinee (warning) |\n| `N4005` | invalid resolved enum/match/control-flow HIR reached the interpreter |",
)
replace_once(
    "docs/enums-and-patterns.md",
    "pattern-usefulness analysis, or stable enum layout. `_` exists only as the payload-discard\nsubpattern of an already resolved concrete variant; it does not cover other variants. Enums",
    "general wildcard/guard/nested-pattern usefulness analysis, or stable enum layout. The\nimplemented `N3034` proof covers only non-selected concrete arms under a direct constructor.\n`_` exists only as the payload-discard subpattern of an already resolved concrete variant;\nit does not cover other variants. Enums",
)

replace_once(
    "docs/roadmap.md",
    "**Status: fifty-one vertical slices implemented; broader type-system work remains.**",
    "**Status: fifty-two vertical slices implemented; broader type-system work remains.**",
)
replace_once(
    "docs/roadmap.md",
    "- semantic, inspection, CLI, schema, and malformed-HIR regressions lock both the new language\n  fact and backward-compatible tooling version boundary without introducing catch-all\n  usefulness semantics.\n\nThe next Phase 2 slices should address semantic depth rather than widen syntax",
    "- semantic, inspection, CLI, schema, and malformed-HIR regressions lock both the new language\n  fact and backward-compatible tooling version boundary without introducing catch-all\n  usefulness semantics.\n\nImplemented in the fifty-second Phase 2 slice:\n\n- a direct, successfully resolved enum constructor now turns the analyzer's existing exact\n  selected-variant proof into nonfatal `N3034` warnings for every otherwise-valid concrete\n  arm that cannot be selected;\n- dynamic scrutinees flowing through locals, parameters, calls, or other expressions remain\n  outside the proof, so the checker does not guess usefulness beyond facts it already owns;\n- warning candidates are deferred until the complete semantic analysis is error-free, which\n  keeps an error discovered inside a diagnostic-only arm actionable instead of pairing it with\n  recovery noise;\n- warned arms remain fully name/type checked and retain their diagnostic-only CFG edges, while\n  continuing to contribute no definite-initialization, non-continuation, or loop-transfer facts;\n  and\n- the slice changes no syntax, HIR, CFG shape, runtime dispatch, or semantic-inspection schema,\n  and deliberately does not introduce catch-all arms or a general pattern-usefulness matrix.\n\nThe next Phase 2 slices should address semantic depth rather than widen syntax",
)
