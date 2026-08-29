from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if text.count(old) != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {text.count(old)}")
    return text.replace(old, new, 1)

# CLI fixture registration.
path = Path("crates/nova-cli/tests/cli.rs")
text = path.read_text()
text = replace_once(
    text,
    '        "valid/unit-equality.nv",\n        "valid/records.nv",\n',
    '        "valid/unit-equality.nv",\n        "valid/payload-free-enum-equality.nv",\n        "valid/records.nv",\n',
    "CLI positive fixture anchor",
)
text = replace_once(
    text,
    '        ("valid/unit-equality.nv", "true\\n"),\n        ("valid/records.nv", "42\\n"),\n',
    '        ("valid/unit-equality.nv", "true\\n"),\n        ("valid/payload-free-enum-equality.nv", "true\\n"),\n        ("valid/records.nv", "42\\n"),\n',
    "CLI run fixture anchor",
)
text = replace_once(
    text,
    '        ("invalid/unit-type-mismatch.nv", "N3004"),\n        ("invalid/assignment-type-mismatch.nv", "N3004"),\n',
    '        ("invalid/unit-type-mismatch.nv", "N3004"),\n        ("invalid/payload-enum-equality.nv", "N3004"),\n        ("invalid/assignment-type-mismatch.nv", "N3004"),\n',
    "CLI negative fixture anchor",
)
path.write_text(text)

# Enum contract.
path = Path("docs/enums-and-patterns.md")
text = path.read_text()
equality_section = '''## Equality

A nominal enum is equality-comparable in the bootstrap subset only when every
declared variant is payload-free. For such an enum, `==` and `!=` require both
operands to have that exact nominal enum type and compare the resolved variant
slot after ordinary left-to-right operand evaluation. Same-spelled variants from
different enum declarations are never comparable.

If any variant carries a payload, the entire enum type remains non-comparable in
this slice; Nova does not recursively derive payload or aggregate equality yet.
Direct payload-free constructors may participate in the existing closed-condition
proof (`Color::Red == Color::Red`, for example), but locals, parameters, calls,
and blocks remain dynamic values even when their runtime result is predictable to
a human.

'''
text = replace_once(text, "## Definite assignment\n", equality_section + "## Definite assignment\n", "enum equality section anchor")
text = replace_once(
    text,
    "multi-payload variant, named variant fields, record\ndestructuring, exhaustiveness usefulness warning, or stable enum layout. It also\ndoes not add enum equality. Those features require separate semantic and\ndiagnostic designs rather than syntactic shortcuts.\n",
    "multi-payload variant, named variant fields, record\ndestructuring, exhaustiveness usefulness warning, or stable enum layout. Enums\nwith payload variants and records do not yet receive recursively derived value\nequality. Those features require separate semantic and diagnostic designs rather\nthan syntactic shortcuts.\n",
    "enum limitation anchor",
)
path.write_text(text)

# Constitution: remove stale pre-Unit statement and describe the narrow enum rule.
path = Path("docs/language-constitution.md")
text = path.read_text()
text = replace_once(
    text,
    "`Unit` is reserved alongside `Int` and `Bool` and cannot be redefined as a nominal\nrecord or enum. Unit equality is not part of this bootstrap operator subset.\n",
    "`Unit` is reserved alongside `Int` and `Bool` and cannot be redefined as a nominal\nrecord or enum.\n",
    "constitution stale Unit equality sentence",
)
old = '''Matching `Int`, `Bool`, and `Unit` values are equality-comparable with `==` and
`!=`. Unit has exactly one bootstrap value, so two normally evaluated Unit values compare
equal. Nominal records, enums, and function values remain non-comparable. Closed-condition
reasoning may prove equality only for literal `()` operands; it does not erase evaluation
of Unit-returning calls, names, or blocks.
'''
new = '''Matching `Int`, `Bool`, and `Unit` values are equality-comparable with `==` and
`!=`. Unit has exactly one bootstrap value, so two normally evaluated Unit values compare
equal. A nominal enum is also equality-comparable when every one of its declared variants
is payload-free; both operands must have that same nominal enum identity, and equality
compares the resolved variant identity after ordinary left-to-right evaluation. Enums with
any payload variant, nominal records, and function values remain non-comparable. Closed-
condition reasoning may prove equality for literal Unit values and direct payload-free enum
constructors; it does not erase evaluation of calls, names, parameters, or other dynamic
values.
'''
text = replace_once(text, old, new, "constitution equality paragraph")
path.write_text(text)

# README user-facing status and stale equality descriptions.
path = Path("README.md")
text = path.read_text()
old = '''Matching `Int`, `Bool`, and `Unit` values support `==` and `!=`. `Unit` has a
single runtime value, so Unit equality is always true and Unit inequality is always false
once both operands have evaluated normally. Record, enum, and function values remain
non-comparable. Closed-condition analysis recognizes only literal `()` equality; Unit
locals and calls remain dynamic and are still evaluated at runtime.
'''
new = '''Matching `Int`, `Bool`, and `Unit` values support `==` and `!=`. `Unit` has a
single runtime value, so Unit equality is always true and Unit inequality is always false
once both operands have evaluated normally. A nominal enum also supports equality when
every declared variant is payload-free; operands must have the same enum identity and
comparison uses the resolved variant slot. Enums with any payload variant, records, and
functions remain non-comparable. Closed-condition analysis can prove literal Unit and
direct payload-free enum-constructor comparisons, while locals and calls remain dynamic
and are still evaluated at runtime.
'''
text = replace_once(text, old, new, "README equality paragraph")
text = replace_once(
    text,
    "one arm. Wildcards, guards, nested patterns, multi-payload variants, enum\nequality, layout, and ABI guarantees are not implemented.\n",
    "one arm. Wildcards, guards, nested patterns, multi-payload variants, equality for\npayload-bearing enums, layout, and ABI guarantees are not implemented.\n",
    "README enum limitation",
)
text = replace_once(
    text,
    "an explicit return on every continuing path. Arithmetic and ordered comparisons\nrequire `Int`; boolean operators require `Bool`; equality currently accepts only\nmatching `Int` or matching `Bool`; calls require matching arity and argument types.\n",
    "an explicit return on every continuing path. Arithmetic and ordered comparisons\nrequire `Int`; boolean operators require `Bool`; equality accepts matching `Int`, `Bool`,\n`Unit`, or the same nominal payload-free enum type; calls require matching arity and\nargument types.\n",
    "README type summary equality",
)
path.write_text(text)

# Roadmap Phase 2 + Phase 3 slices.
path = Path("docs/roadmap.md")
text = path.read_text()
text = replace_once(
    text,
    "**Status: thirty-six vertical slices implemented; broader type-system work remains.**",
    "**Status: thirty-seven vertical slices implemented; broader type-system work remains.**",
    "Phase 2 status",
)
slice37 = '''Implemented in the thirty-seventh Phase 2 slice:

- nominal enums whose every declared variant is payload-free join `Int`, `Bool`, and
  `Unit` as equality-comparable bootstrap value types;
- `==` and `!=` require the exact same enum identity on both operands and compare the
  resolved variant slot rather than variant spelling or declaration shape;
- if any variant carries a payload, the entire enum remains non-comparable in this
  slice, deliberately avoiding recursive payload/aggregate equality semantics;
- closed-condition reasoning may prove equality and inequality for direct payload-free
  enum constructors, extending flow precision without propagating locals or executing
  enum-returning calls;
- differently declared enums, records, functions, and all payload-bearing enums remain
  rejected with the existing type-mismatch diagnostic; and
- semantic and CLI regressions lock nominal identity, payload boundaries, direct-
  constructor reachability, dynamic-call conservatism, and retained HIR shape.

'''
text = replace_once(
    text,
    "The next Phase 2 slices should address semantic depth rather than widen syntax\n",
    slice37 + "The next Phase 2 slices should address semantic depth rather than widen syntax\n",
    "Phase 2 next-slices anchor",
)
text = replace_once(
    text,
    "**Status: eight vertical slices implemented; execution surface remains small.**",
    "**Status: nine vertical slices implemented; execution surface remains small.**",
    "Phase 3 status",
)
slice9 = '''Implemented in the ninth Phase 3 slice:

- the interpreter executes equality and inequality for semantically accepted
  payload-free enum values after ordinary left-to-right operand evaluation;
- runtime comparison requires the same nominal `EnumId` and compares declaration-order
  variant slots, so same-spelled variants from distinct enum declarations cannot acquire
  accidental structural equality;
- payload-bearing enum values remain outside the semantic operator contract rather than
  triggering recursive runtime comparison;
- direct enum-constructor equality used by semantic reachability and runtime execution
  agrees on variant identity, while enum-returning calls remain dynamically evaluated;
- CLI check/run coverage locks parameter, direct-constructor, equality, and inequality
  behavior end to end; and
- the interpreter's boxed payload representation, enum layout, ownership, and ABI remain
  explicitly provisional and unaffected by this equality slice.

'''
text = replace_once(text, "Next Phase 3 slices should deepen executable semantics without bypassing Phase 2\n", slice9 + "Next Phase 3 slices should deepen executable semantics without bypassing Phase 2\n", "Phase 3 next-slices anchor")
path.write_text(text)
