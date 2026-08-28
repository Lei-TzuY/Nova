from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:140]!r}")
    file.write_text(text.replace(old, new, 1))


patch(
    "README.md",
    '''execution, semantic analysis also preflights closed arithmetic trees made entirely\nfrom `Int` literals and arithmetic operators: statically certain overflow is `N3031`\nand a statically certain zero divisor is `N3032`. Successful constant arithmetic is\nnot folded, and any expression with a dynamic operand remains runtime checked. Such\n''',
    '''execution, semantic analysis also preflights reachable closed arithmetic trees made\nentirely from `Int` literals and arithmetic operators: statically certain overflow is\n`N3031` and a statically certain zero divisor is `N3032`. Source lowered only for\ndiagnostics because control flow proves it unreachable does not manufacture these\nexecution-failure diagnostics. Successful constant arithmetic is not folded, and any\nexpression with a dynamic operand remains runtime checked. Such\n''',
)

patch(
    "docs/language-constitution.md",
    '''Semantic analysis preflights only closed arithmetic trees composed entirely of\n`Int` literals and arithmetic operators: a provable overflow is `N3031`, while a\nprovable zero divisor is `N3032`. Successful trees are deliberately not folded,\nand names, calls, blocks, or other dynamic operands stop this preflight rather than\ntriggering general constant propagation. Dynamic arithmetic remains checked by the\n''',
    '''Semantic analysis preflights only reachable closed arithmetic trees composed entirely\nof `Int` literals and arithmetic operators: a provable overflow is `N3031`, while a\nprovable zero divisor is `N3032`. Source lowered only for diagnostics on a statically\nunreachable path is excluded from these execution-failure diagnostics. Successful\ntrees are deliberately not folded, and names, calls, blocks, or other dynamic\noperands stop this preflight rather than triggering general constant propagation.\nDynamic arithmetic remains checked by the\n''',
)

patch(
    "docs/roadmap.md",
    '''- outer operations remain explicit HIR, so negating the minimum again still reaches\n  the interpreter's checked arithmetic and reports runtime `N4002`;\n''',
    '''- outer operations remain explicit HIR rather than being folded; subsequent semantic\n  preflight may reject a provably failing closed operation while preserving that HIR\n  shape for recovery and keeping dynamic equivalents runtime checked;\n''',
)

patch(
    "docs/roadmap.md",
    '''- semantic analysis preflights closed `Int` arithmetic trees made only from literal\n  values and arithmetic operators, without introducing name propagation, function\n  evaluation, block evaluation, or a general constant folder;\n''',
    '''- semantic analysis preflights reachable closed `Int` arithmetic trees made only from\n  literal values and arithmetic operators, without introducing name propagation,\n  function evaluation, block evaluation, or a general constant folder;\n''',
)

patch(
    "docs/roadmap.md",
    '''- successful constant arithmetic preserves its original unary/binary HIR so semantic\n  validation does not change runtime evaluation shape or execution-step accounting;\n- dynamic operands stop preflight and retain the interpreter's `N4002` overflow and\n''',
    '''- successful constant arithmetic preserves its original unary/binary HIR so semantic\n  validation does not change runtime evaluation shape or execution-step accounting;\n- statically unreachable source lowered only for deterministic diagnostics suppresses\n  `N3031`/`N3032`, preserving established literal-if, direct-match, short-circuit, and\n  post-noncontinuation reachability semantics;\n- dynamic operands stop preflight and retain the interpreter's `N4002` overflow and\n''',
)
