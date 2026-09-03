# Semantic Introspection v4

This document specifies Nova semantic-introspection schema version 4. V4 is the
first tooling contract that represents the implemented `String` scalar. It
preserves v2 control-flow graphs and v3 match-pattern payload modes while
extending the program projection's closed type and expression category sets.
It is not HIR serialization, runtime state, string layout, allocation metadata,
or an ABI.

The normative structural schema is
[`schemas/semantic-inspection-v4.schema.json`](schemas/semantic-inspection-v4.schema.json).
V4 references the frozen v1 producer, source, declaration, statement, aggregate,
and match definitions; the v2 `controlFlowGraph`; and the v3 `matchPattern`.
It owns new `program`, `type`, and `expression` definitions because silently
broadening the v1 enums would violate the older contracts. Validators therefore
register all four published schema files together.

## Invocation and compatibility

```text
nova inspect <file|-> --format json --schema-version 4 [--source-name name] [--message-format human|json] [--fail-on-warnings]
```

Omitting `--schema-version` still selects v1. Explicit versions 1, 2, and 3
retain their exact contracts. A semantically valid program whose declarations,
function signatures, bindings, blocks, or expressions contain `String` fails
v1-v3 inspection with `N5001` and no partial output; the diagnostic directs the
caller to v4. Nova never silently upgrades the requested schema.

V4 uses schema family `nova.semantic-inspection`, sets `schema_version` to `4`,
and retains the required `producer`, `source`, `program`, `control_flow`, and
`match_patterns` members. Warning and source-name behavior is unchanged from v3.

## String facts

V4 adds exactly two category values:

| Location | New value | Meaning |
| --- | --- | --- |
| `program.types[].kind` | `string` | the built-in semantic `String` type |
| `program.expressions[].kind` | `string` | a decoded string-literal expression |

A String literal expression has a `type_id` whose type fact is `kind: "string"`
and `display: "String"`. Like integer and Boolean literals in the established
projection, its value is not duplicated into the semantic document; its exact
UTF-8 source range remains available through `span`. V4 type interning seeds
`Int`, `Bool`, `String`, `Unit`, and `Never` in that order. Consumers must not
compare document-local IDs across schema versions.

Before publishing v4, the inspector independently verifies that every String
literal carries HIR type `String`, that all String types are internally valid,
and that all established declaration, identity, CFG, match-pattern, and span
invariants still hold. A forged literal type or any other mismatch fails closed
as `N5001` rather than being repaired from expression shape.

## Deliberate limits

V4 adds no decoded-literal-value field, concatenation or method facts, encoding
conversion, ownership, lifetime, allocation, layout, ABI, backend, effect, or
module information. It does not reinterpret v1-v3 output. Schema v4 remains
provisional before Nova 1.0; an incompatible change still requires a later
version rather than mutation in place.
