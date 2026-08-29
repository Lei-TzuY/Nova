# Nova v0.1 Enums and Pattern Matching

Status: **normative for the implemented bootstrap semantic subset**

This document specifies the enum and `match` behavior implemented by the Rust
bootstrap toolchain. It intentionally does not settle general algebraic data
types, pattern ergonomics, memory layout, ownership, or backend ABI.

## Declarations and identity

An enum is a top-level nominal type with one or more variants:

```nova
enum OptionInt {
    None,
    Some(Int),
}
```

Each variant carries either no payload or one payload with an explicit type.
Variant names must be unique within their enum. Record and enum declarations
share one type namespace with the built-in `Int` and `Bool` names. A duplicate
type declaration is rejected even when the two declarations have different
kinds.

Enum identity comes from its declaration, not from its variant spelling or
shape. Separately declared enums are different types. All record and enum names
are collected before payload, field, function-signature, or body types are
resolved, so forward references and recursive enum payloads are deterministic.

## Construction

Constructors are always qualified:

```nova
OptionInt::None
OptionInt::Some(42)
```

A payload-free variant has no parentheses. A payload variant requires exactly
one expression whose type matches the declared payload type. Constructor payload
expressions are evaluated once in source order. A completed constructor has the
nominal type named by its enum declaration.

## Matching

The implemented pattern form is deliberately narrow:

```nova
match value {
    OptionInt::None => 0,
    OptionInt::Some(inner) => inner,
}
```

The scrutinee must have an enum type. Every arm must qualify a variant with that
same nominal enum, and every declared variant must occur exactly once. A variant
with a payload requires one binding; a payload-free variant forbids a binding.
The payload binding is immutable, definitely initialized, and visible only in
its arm expression. Different arms therefore may reuse the same spelling
without sharing identity.

The result type is determined from all arms that can continue. Continuing arms
must agree on one type. An arm whose expression has the internal bottom type `!`
because it returns, breaks, or continues does not constrain the result type.
`break` and `continue` are legal here only when the match expression itself is
inside an enclosing `while` body. `!` and `()` remain internal HIR types and
have no source spelling in this subset.

The scrutinee is evaluated exactly once before selection. Only the selected arm
is evaluated. Written arm order does not change selection because duplicate
variants are rejected, but HIR preserves source order for deterministic tooling
and diagnostics.

## Equality

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

## Definite assignment

Each arm starts from the post-scrutinee initialization state. For a valid,
exhaustive match whose scrutinee is dynamic, a pre-existing local is definitely
initialized afterward only if every arm that can continue initializes it. Arms
that cannot continue because of `return`, `break`, or `continue` are excluded from
that intersection. If all reachable arms cannot continue, the match itself has type
`!`.

A direct, successfully resolved enum constructor is a narrower bootstrap
reachability case. Because its variant is known after the constructor payload has
been evaluated, only the corresponding arm contributes reachable initialization,
non-continuation, and enclosing-loop transfer facts. Every non-selected arm is still
resolved and type checked, still participates in exhaustiveness and result-type
compatibility, and can still emit diagnostics; only its runtime flow facts are
discarded. Values flowing through locals, parameters, calls, or other expressions
do not receive this refinement.

An invalid or non-exhaustive match never establishes an initialization fact.
This fail-closed rule prevents a rejected control-flow shape from making a later
read appear safe during diagnostic recovery. Likewise, unreachable assignments
after a loop-control transfer are still diagnosed but do not become reachable
initialization evidence.

## Bootstrap HIR and execution

HIR assigns each enum an `EnumId` and each variant its declaration-order slot.
Constructors and match arms use these resolved identities rather than runtime
string lookup. Runtime enum values carry the `EnumId`, variant slot, and an
optional boxed payload. Before executing a match, the interpreter verifies the
resolved arms are exhaustive, non-duplicated, in range, and payload-compatible;
malformed HIR produces runtime invariant diagnostic `N4005`.

A selected match arm propagates structured control flow unchanged. Therefore a
`return` reaches the current function, while `break` or `continue` reaches only
the nearest enclosing `while`, where it is consumed. A loop-control transfer
that somehow escapes its lexical loop in malformed HIR fails closed with
`N4005` rather than crossing a function boundary.

The boxed interpreter payload is a bootstrap implementation detail. It is not a
source-level allocation guarantee, object representation, stable layout,
serialization format, ownership rule, or ABI.

## Diagnostics

| Code | Meaning in this slice |
|---|---|
| `N3002` | duplicate or reserved type definition |
| `N3004` | constructor payload or match-arm type mismatch |
| `N3013` | `break` or `continue` without an enclosing `while` body |
| `N3020` | duplicate variant within one enum declaration |
| `N3021` | unknown enum/variant or a non-enum qualifier |
| `N3022` | constructor or pattern payload arity mismatch |
| `N3023` | non-exhaustive match |
| `N3024` | duplicate variant arm |
| `N3025` | non-enum scrutinee or pattern from another nominal enum |
| `N4005` | invalid resolved enum/match/control-flow HIR reached the interpreter |

Diagnostic codes remain bootstrap tooling contracts rather than a post-1.0
compatibility promise.

## Deliberate limitations

This slice has no wildcard, default arm, guard, literal pattern, nested pattern,
alternative pattern, multi-payload variant, named variant fields, record
destructuring, exhaustiveness usefulness warning, or stable enum layout. Enums
with payload variants and records do not yet receive recursively derived value
equality. Those features require separate semantic and diagnostic designs rather
than syntactic shortcuts.
