# Nova semantic introspection v7

Status: **implemented, provisional tooling contract**

Schema v7 extends v6 with explicit projections for the executable `UInt`
numeric family and by-value closure captures, including snapshots of mutable-source
bindings. Select it with:

```console
nova inspect --format json --schema-version 7 program.nv
```

The schema is published at
[`schemas/semantic-inspection-v7.schema.json`](schemas/semantic-inspection-v7.schema.json).

## Compatibility

V7 preserves v6's module, program, function-CFG, match-pattern, closure, and
closure-CFG relationships. It deliberately extends two closed enums and the closure
capture record:

- type kind `uint` represents resolved `UInt`;
- expression kind `unsigned_integer` represents a canonical unsigned constant; and
- expression kind `numeric_conversion` carries operator `int_to_uint` or
  `uint_to_int` and one operand in `children`; and
- every `closures[].captures[]` entry adds `mode: "by_value"`.

The expression's `type_id` and its child's type must agree with the operator:
`int_to_uint` maps `Int` to `UInt`, while `uint_to_int` maps `UInt` to `Int`.
Literal values remain omitted, as in earlier schemas. The primitive type seed now
includes `UInt` after `Int`, so consumers must treat all document-local IDs as
version-local and must not compare their spelling across schema versions.

Every capture copies its value when the closure expression is evaluated. When the
referenced `program.bindings[]` fact has `mutable: true`, this is a creation-time
snapshot: later assignments to the outer slot do not update the environment. The
captured slot itself is immutable, so assignment through it is never representable as
accepted HIR. The explicit mode leaves room for a future schema to describe another
capture strategy without reinterpreting v7.

V1 remains the CLI default and no caller receives v7 implicitly. The published
v1-v6 schemas and serializers retain their earlier enums; UInt-bearing accepted HIR
fails those versions with inspection diagnostic `N5001`. V5 and v6 additionally reject
mutable-source snapshot captures because their contract admitted only immutable-source
capture edges. Neither failure emits partial output.

## Validation

Before emitting v7, `nova-inspect` requires successful semantic analysis and
independently validates all v6 source-span, module-ownership, identity, scope,
capture, nominal-member, match, and CFG invariants. It additionally requires:

- every unsigned constant expression to have `UInt` HIR type;
- every numeric conversion to have the exact input and result types named above;
- every capture to be copied by value, and no assignment target to cross callable
  ownership through a capture edge;
- all referenced types and child expressions to exist in the deterministic fact
  tables; and
- all identities, including values nested in closures and CFGs, to remain owned by
  the document's single module.

Any disagreement fails closed as `N5001`. Rendering is deterministic for the same
accepted analysis, source metadata, compiler version, and selected schema version.

## Non-claims

V7 does not add unsigned literal syntax, implicit conversions, constant values,
shared cells, by-reference mutation, capture lists, numeric or closure layout/ABI,
modules/imports, effects, ownership facts, MIR, or backend metadata. It records only
the implemented typed-HIR semantics. A future tooling change must use another schema
version rather than reinterpret v7 fields or enum members.
