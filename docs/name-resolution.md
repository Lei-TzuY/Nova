# Nova v0.1 Implemented Name Resolution

Status: **normative for the single-file semantic subset implemented by
`nova check`**

This document specifies only the current AST-to-HIR lowering and name rules. It
does not define modules, imports, user-defined types, overloads, traits, type
inference, or a stable tooling protocol.

## Resolution domains

The current syntax makes value and type positions unambiguous, and the
bootstrap resolver treats them as separate domains:

- value declarations are top-level functions, parameters, `let` bindings, and
  `var` bindings;
- the only declared type names are the built-ins `Int` and `Bool`.

This is an implementation boundary, not a final decision about Nova's future
namespace model. Resolving `Int` or `Bool` establishes type-name identity only;
the compiler does not yet check whether expressions have those types.

## Top-level functions

All unique top-level function names in a source file are collected before any
function body is resolved. Therefore a function may refer to itself or to a
function declared later in the file. Two functions with the same name are an
error. The first declaration remains the recovery target; the duplicate body
does not produce HIR.

There are no other top-level declarations in the implemented grammar.

## Lexical value scopes

Resolution starts at the innermost lexical scope and proceeds outward, then
checks the file's top-level functions.

- Parameters and declarations directly in the function's outermost body share
  one scope.
- Every nested block creates a child scope. This includes block expressions
  and both branches of an `if` expression.
- A name may shadow a declaration from an outer scope, including a top-level
  function.
- Declaring the same name twice in one scope is an error. This includes a local
  binding that repeats a parameter name in the function's outermost body.
- A binding initializer and its optional type annotation are resolved before
  the new value binding enters scope. In a nested scope, `let x = x;` therefore
  refers to an outer `x` when one exists; otherwise the initializer contains an
  unknown name.
- A declaration remains available after an erroneous initializer for recovery,
  preventing avoidable unknown-name cascades. No HIR is returned while any
  semantic error remains.

## Symbol identities and HIR

Successful lowering replaces every value-name reference with a `SymbolId` and
stores declaration metadata in a dense table. IDs are deterministic for
identical parsed input:

1. unique top-level functions receive IDs in source order;
2. parameters and local bindings then receive IDs while functions are lowered
   in source order and bodies are traversed deterministically.

These IDs are local to one lowering session. They are not stable across source
edits, builds, files, or compiler versions and must not be persisted as a
tooling protocol. Exact declaration and reference spans remain in HIR.

## Diagnostics and failure

The implemented semantic diagnostic codes are:

| Code | Meaning |
|---|---|
| `N3001` | duplicate value declaration in one lexical scope |
| `N3002` | unknown value name |
| `N3003` | unknown type name |
| `N3004` | guarded HIR-lowering nesting limit exceeded |
| `N3005` | per-file bootstrap symbol identity space exhausted |

Duplicate diagnostics label both the repeated declaration and the original.
Semantic diagnostics are deterministically ordered by their primary source
position. If any semantic error is emitted, lowering returns no program HIR and
`nova check` exits unsuccessfully.

`nova ast` intentionally stops after parsing. It can inspect a syntactically
valid tree even when that tree contains unresolved names.

## Deliberate limitations

There is no cross-file lookup, module or import scope, prelude, local function,
user-defined type, constructor, field, method, label, macro, overload set, or
visibility rule. There is also no type checking, definite-assignment analysis,
unused-name warning, assignment, or closure capture analysis. Unsupported
semantics are not inferred from spelling.
