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
shape. Separately declared enums are different types. Within one enum, semantic HIR
retains each source-resolved variant spelling together with its declaration-order slot;
the spelling is compiler-owned integrity metadata rather than runtime string lookup. All
record and enum names are collected before payload, field, function-signature, or body
types are resolved, so forward references and recursive enum payloads are deterministic.

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
    OptionInt::Some(_) => 1,
}
```

The scrutinee must have an enum type. Every arm must qualify a variant with that
same nominal enum, and every declared variant must occur exactly once. A payload-bearing
variant must either bind its payload with `Variant(name)` or explicitly discard it with
`Variant(_)`; silently omitting the payload position is still `N3022`. A payload-free
variant accepts neither a binding nor `_`. A payload binding is immutable, definitely
initialized, and visible only in its arm expression. A discard creates no binding and the
payload value is unavailable to the arm. Different binding arms may reuse the same spelling
without sharing identity. `_` is not a catch-all arm in this slice: a bare `_ => ...` pattern
is not implemented and every concrete enum variant must still occur exactly once.

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
discarded. When the complete semantic analysis is otherwise error-free, each such
otherwise-valid non-selected arm also receives nonfatal warning `N3034`, making this
existing proof visible as the first narrow match-usefulness diagnostic. Values flowing
through locals, parameters, calls, or other expressions remain dynamic and receive no
`N3034` guess. An error anywhere in semantic analysis suppresses the deferred warning,
including an error found while checking a non-selected arm.

An invalid or non-exhaustive match never establishes an initialization fact.
This fail-closed rule prevents a rejected control-flow shape from making a later
read appear safe during diagnostic recovery. Likewise, unreachable assignments
after a loop-control transfer are still diagnosed but do not become reachable
initialization evidence.

## Bootstrap HIR and execution

HIR assigns each enum an `EnumId` and each variant its declaration-order slot.
Constructors and match arms retain the source-resolved variant spelling alongside that
slot, allowing trusted consumers to verify that the two still identify the same declared
member. Match arms additionally retain whether a payload was explicitly discarded, so a
malformed HIR mutation that merely deletes a real binding cannot be reinterpreted as `_`.
Runtime enum values remain compact and carry only the `EnumId`, variant slot, and optional
boxed payload; a selected discard arm consumes that payload without creating a frame slot.

Semantic-inspection v1 and v2 keep their published meaning: they do not reinterpret a
payload-bearing arm with `binding: null` as discard and therefore fail closed with `N5001`
when source uses `Variant(_)`. Explicit schema v3 preserves the established program and CFG
projections and adds `match_patterns`, whose `payload_mode` is `none`, `bind`, or `discard`.
This makes the new language fact representable without silently mutating older protocol
versions.

At execution time, constructor payload evaluation happens before value-only variant
identity validation, and a match evaluates its scrutinee before validating the complete
resolved arm table. Thus a payload or scrutinee that returns, breaks, or continues keeps
its established structured flow. Once an ordinary enum value is required, the interpreter
rechecks enum identity, variant spelling/slot agreement, payload arity/type, exhaustiveness,
and duplicate coverage; malformed HIR produces invariant diagnostic `N4005`.

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
| `N3034` | non-selected concrete arm under a direct-constructor match scrutinee (warning) |
| `N4005` | invalid resolved enum/match/control-flow HIR reached the interpreter |

Diagnostic codes remain bootstrap tooling contracts rather than a post-1.0
compatibility promise.

## Deliberate limitations

This slice has no catch-all/default arm, guard, literal pattern, nested pattern,
alternative pattern, multi-payload variant, named variant fields, record destructuring,
general wildcard/guard/nested-pattern usefulness analysis, or stable enum layout. The
implemented `N3034` proof covers only non-selected concrete arms under a direct constructor.
`_` exists only as the payload-discard subpattern of an already resolved concrete variant;
it does not cover other variants. Enums
with payload variants and records do not yet receive recursively derived value equality.
Those features require separate semantic and diagnostic designs rather than syntactic
shortcuts.
