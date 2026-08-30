from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))

# Grammar: every existing type position now accepts recursive function types.
p = Path("docs/grammar.md")
text = p.read_text()
text = text.replace("type_name", "type_ref")
old = '''parameter           = identifier , ":" , type_ref ;
type_ref           = identifier ;
'''
new = '''parameter           = identifier , ":" , type_ref ;
type_ref            = identifier | function_type ;
function_type       = "fn" , "(" , [ type_ref_list ] , ")" , "->" , type_ref ;
type_ref_list       = type_ref , { "," , type_ref } , [ "," ] ;
'''
if old not in text:
    # spacing after global rename is inherited from the old grammar line.
    old = '''parameter           = identifier , ":" , type_ref ;
type_ref          = identifier ;
'''
if old not in text:
    raise SystemExit("grammar type-ref production anchor not found")
text = text.replace(old, new, 1)
anchor = '''`Unit` is a built-in surface type and `()` is its sole literal. A block with no
tail expression also has type `Unit`. A function declared `-> Unit` may therefore
fall through a value-less body; `return ();` is the explicit equivalent. Other
return types still require a compatible tail or an explicit return on every
continuing path. Parenthesized non-empty expressions retain ordinary grouping, so
`(value)` is not a Unit literal.
'''
insert = anchor + '''\nFunction types use the recursive surface form `fn(T1, T2) -> U`; zero parameters and
a trailing comma are allowed, and parameter/return positions may themselves be function
types. The form is accepted anywhere a type reference is accepted, including function
signatures, local annotations, record fields, and enum payloads. This enables named
top-level function values to be passed, returned, stored, and invoked through explicit
signatures. It does not introduce lambdas, closures, captured environments, methods, or
implicit callable coercions. Recursive type parsing has its own finite nesting budget and
reports `N2009` rather than recursing without bound.
'''
if anchor not in text:
    raise SystemExit("grammar Unit prose anchor not found")
text = text.replace(anchor, insert, 1)
old = '''The parser enforces a finite nesting budget and emits diagnostic `N2008` rather
than continuing unbounded recursive descent. This budget is an implementation
limit, not a promise that deeply nested source will remain portable unchanged.
'''
new = '''The parser enforces finite nesting budgets. Expression recursion emits `N2008`,
and recursive type syntax emits `N2009`, rather than continuing unbounded recursive
descent. These budgets are implementation limits, not promises that pathologically
deep source will remain portable unchanged.
'''
if old not in text:
    raise SystemExit("grammar nesting prose anchor not found")
text = text.replace(old, new, 1)
p.write_text(text)

# Constitution: document the implemented higher-order named-function boundary and fix stale equality wording.
p = Path("docs/language-constitution.md")
text = p.read_text()
anchor = '''`Unit` is reserved alongside `Int` and `Bool` and cannot be redefined as a nominal
record or enum.
'''
insert = anchor + '''\nThe bootstrap surface also admits explicit recursive function types written
`fn(T1, T2) -> U`. These lower directly to the existing resolved `FunctionType`, so a
named top-level function may be passed as an argument, returned from another function,
stored in a typed binding, and called through that binding. Function types are structural
signatures while runtime function values retain top-level declaration identity. This is
higher-order named-function support only: lambda syntax, closure creation, lexical capture,
method values, callable objects, and closure allocation/ownership semantics remain
unimplemented and must not be inferred from this slice.
'''
if anchor not in text:
    raise SystemExit("constitution surface type anchor not found")
text = text.replace(anchor, insert, 1)
old = '''compares the resolved variant identity after ordinary left-to-right evaluation. Enums with
any payload variant, nominal records, and function values remain non-comparable. Closed-
condition reasoning may prove equality for literal Unit values and direct payload-free enum
constructors; it does not erase evaluation of calls, names, parameters, or other dynamic
values.
'''
new = '''compares the resolved variant identity after ordinary left-to-right evaluation. Enums with
any payload variant and nominal records remain non-comparable. Function values are
comparable only when their fully resolved signatures match, and equality compares top-level
declaration identity rather than code addresses or bodies. Closed-condition reasoning may
prove equality for literal Unit values, direct payload-free enum constructors, and direct
top-level function references; it does not erase evaluation of calls, local aliases,
parameters, or other dynamic values.
'''
if old not in text:
    raise SystemExit("constitution equality anchor not found")
text = text.replace(old, new, 1)
p.write_text(text)

# README: advertise the usable surface capability without claiming closures.
p = Path("README.md")
text = p.read_text()
old = '''- parse functions, nominal records and enums, explicit aggregate construction,
  exhaustive enum matching, field projection, initialized bindings, typed
  delayed `var` initialization, narrow assignments, expressions, blocks, calls,
  `if` expressions, pre-test `while` loops, and statement-only `break`/`continue`;
'''
new = '''- parse functions, recursive explicit function types, nominal records and enums,
  explicit aggregate construction, exhaustive enum matching, field projection,
  initialized bindings, typed delayed `var` initialization, narrow assignments,
  expressions, blocks, calls, `if` expressions, pre-test `while` loops, and
  statement-only `break`/`continue`;
'''
if old not in text:
    raise SystemExit("README parser capability anchor not found")
text = text.replace(old, new, 1)
anchor = '''Function parameters and a function body's outermost bindings share one scope.
'''
insert = anchor + '''\nExplicit function types use `fn(T1, T2) -> U` and may nest recursively in any type
position. They expose the callable signatures the HIR/runtime already use internally, so
named top-level functions can now be passed, returned, stored in typed locals, and invoked
through those values. For example, a parameter `transform: fn(Int) -> Int` can be called
like any other function value. This slice deliberately does not add lambda expressions,
closures, captured environments, methods, or implicit callable conversions.
'''
if anchor not in text:
    raise SystemExit("README semantic intro anchor not found")
text = text.replace(anchor, insert, 1)
p.write_text(text)

# Roadmap: frontend grammar slice + semantic resolver slice; runtime count unchanged.
p = Path("docs/roadmap.md")
text = p.read_text()
old = "**Status: two vertical slices implemented; broader grammar work remains.**"
new = "**Status: three vertical slices implemented; broader grammar work remains.**"
if old not in text:
    raise SystemExit("Phase 1 status anchor not found")
text = text.replace(old, new, 1)
anchor = '''- focused lexer regressions plus a CLI check/run fixture cover all prefixes, separator
  policy, invalid digits, range failure, and exact hexadecimal `Int::MIN` execution.

Next Phase 1 refinements should be driven by the needs of later semantic work,
'''
insert = '''- focused lexer regressions plus a CLI check/run fixture cover all prefixes, separator
  policy, invalid digits, range failure, and exact hexadecimal `Int::MIN` execution.

Implemented in the third Phase 1 slice:

- type references become recursive syntax and accept explicit `fn(T1, T2) -> U` function
  types, including zero-argument signatures, trailing parameter commas, and nested
  function parameter/return types;
- the same type-ref production is used by function signatures, local annotations, record
  fields, and enum payloads instead of creating context-specific callable syntax;
- recursive type parsing has an independent finite depth budget with parser diagnostic
  `N2009`, preserving fail-closed behavior for pathological nesting;
- AST type references distinguish named and function forms while retaining one exact span
  for the complete type expression; and
- parser regressions cover recursive signatures and depth failure, while the CLI fixture
  exercises the syntax through the complete executable pipeline.

Next Phase 1 refinements should be driven by the needs of later semantic work,
'''
if anchor not in text:
    raise SystemExit("Phase 1 insertion anchor not found")
text = text.replace(anchor, insert, 1)
old = "**Status: forty-nine vertical slices implemented; broader type-system work remains.**"
new = "**Status: fifty vertical slices implemented; broader type-system work remains.**"
if old not in text:
    raise SystemExit("Phase 2 status anchor not found")
text = text.replace(old, new, 1)
anchor = '''The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
insert = '''Implemented in the fiftieth Phase 2 slice:

- recursive surface function types resolve directly into the existing HIR `FunctionType`
  rather than introducing a parallel callable representation or nominal function aliases;
- higher-order source programs may accept named function values as parameters, return them,
  store them under explicit function annotations, and call them through ordinary expression
  invocation with the existing arity and argument/return type checks;
- nested function signatures participate in the same structural type equality, runtime
  conformance, function-reference identity checks, and semantic-inspection type graph that
  already existed for compiler-produced function values;
- a mismatched higher-order call remains ordinary `N3004` type failure, while semantic
  inspection v1/v2 require no schema change because function types were already representable;
- lambdas, closures, lexical capture, methods, callable objects, and closure ownership/layout
  remain explicitly outside the slice; and
- focused semantic tests plus an end-to-end `nova run` program lock parameter, return,
  local-storage, and invocation behavior with the final result `42`.

The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
if anchor not in text:
    raise SystemExit("Phase 2 insertion anchor not found")
text = text.replace(anchor, insert, 1)
p.write_text(text)
