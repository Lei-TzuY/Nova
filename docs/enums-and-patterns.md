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
because it always returns does not constrain the result type. `!` and `()`
remain internal HIR types and have no source spelling in this subset.

The scrutinee is evaluated exactly once before selection. Only the selected arm
is evaluated. Written arm order does not change selection because duplicate
variants are rejected, but HIR preserves source order for deterministic tooling
and diagnostics.

## Definite assignment

Each arm starts from the post-scrutinee initialization state. For a valid,
exhaustive match, a pre-existing local is definitely initialized afterward only
if every arm that can continue initializes it. Arms that cannot continue are
excluded from that intersection. If all arms cannot continue, the match itself
has type `!`.

An invalid or non-exhaustive match never establishes an initialization fact.
This fail-closed rule prevents a rejected control-flow shape from making a later
read appear safe during diagnostic recovery.

## Bootstrap HIR and execution

HIR assigns each enum an `EnumId` and each variant its declaration-order slot.
Constructors and match arms use these resolved identities rather than runtime
string lookup. Runtime enum values carry the `EnumId`, variant slot, and an
optional boxed payload. Before executing a match, the interpreter verifies the
resolved arms are exhaustive, non-duplicated, in range, and payload-compatible;
malformed HIR produces runtime invariant diagnostic `N4005`.

The boxed interpreter payload is a bootstrap implementation detail. It is not a
source-level allocation guarantee, object representation, stable layout,
serialization format, ownership rule, or ABI.

## Diagnostics

| Code | Meaning in this slice |
|---|---|
| `N3002` | duplicate or reserved type definition |
| `N3004` | constructor payload or match-arm type mismatch |
| `N3020` | duplicate variant within one enum declaration |
| `N3021` | unknown enum/variant or a non-enum qualifier |
| `N3022` | constructor or pattern payload arity mismatch |
| `N3023` | non-exhaustive match |
| `N3024` | duplicate variant arm |
| `N3025` | non-enum scrutinee or pattern from another nominal enum |
| `N4005` | invalid resolved enum/match HIR reached the interpreter |

Diagnostic codes remain bootstrap tooling contracts rather than a post-1.0
compatibility promise.

## Deliberate limitations

This slice has no wildcard, default arm, guard, literal pattern, nested pattern,
alternative pattern, multi-payload variant, named variant fields, record
destructuring, exhaustiveness usefulness warning, or stable enum layout. It also
does not add enum equality. Those features require separate semantic and
diagnostic designs rather than syntactic shortcuts.
