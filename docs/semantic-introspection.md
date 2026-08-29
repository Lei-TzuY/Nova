# Semantic Introspection v1

This document specifies Nova's first machine-readable semantic-introspection
contract. It is a bootstrap tooling protocol for successfully checked,
single-file programs. It is not a serialization of HIR, an execution IR, a
language ABI, or a claim that the represented semantics are stable for Nova
1.0.

The normative structural schema is
[`schemas/semantic-inspection-v1.schema.json`](schemas/semantic-inspection-v1.schema.json).
The CLI golden test locks the producer's deterministic presentation and one
complete example. Consumers must treat JSON object-member order as insignificant;
the documented array ordering remains semantic.

## Invocation and failure behavior

```text
nova inspect <file> --format json [--message-format human|json]
```

`inspect` runs the same UTF-8, lexical, syntactic, name-resolution, type, and
definite-assignment checks as `nova check`. It writes exactly one JSON document
to standard output only when all those stages succeed. A rejected source writes
ordinary structured diagnostics to standard error and writes no partial
document. `--message-format` controls those diagnostics; it does not change the
successful inspection document.

Before serialization, `nova-inspect` independently checks the HIR invariants
needed by the public document: source ranges, declaration and binding identity
order, binding ownership, lexical visibility and assignment mutability, nominal
references and slots, constructor payload arity, and exhaustive match coverage.
An invariant failure is diagnostic `N5001` and likewise emits no document. The
compiler does not repair or approximate malformed HIR.

## Envelope and compatibility

Every v1 document begins with:

```json
{
  "schema": "nova.semantic-inspection",
  "schema_version": 1,
  "producer": { "name": "nova", "version": "0.1.0" }
}
```

Consumers must check both `schema` and `schema_version` before interpreting the
rest of the document. The checked-in JSON Schema rejects unknown fields. A
field removal, field reinterpretation, identity-rule change, or other structural
change therefore requires a new schema version rather than a silent v1 change.
The schema version is independent of the Nova language version, package format,
diagnostic format, and future IR versions.

Schema v1 remains provisional before Nova 1.0. When it is replaced, the old
schema file and golden fixture remain repository evidence for what v1 meant;
support duration and migration tooling have not yet been promised.

## Sources and spans

The bootstrap accepts one source file, represented as `source:0`. Paths are not
canonicalized. The CLI stores a UTF-8 display name; non-UTF-8 path bytes are
represented with the platform's lossy replacement rather than recoverable raw
path bytes. Source contents are not copied into the document. Every span is a
source-qualified, half-open UTF-8 byte range:

```json
{ "source": "source:0", "start": 44, "end": 50 }
```

The inspector rejects foreign sources, out-of-bounds offsets, reversed ranges,
and offsets that are not UTF-8 character boundaries. Given the same validated
source text, display name, compiler version, and schema version, output ordering
and formatting are deterministic.

## Document-local identities

All identities are strings with an explicit namespace. They are deterministic
within one document and are used for cross-references instead of copying names
or relying on array positions.

| Entity | Form | Assignment rule |
| --- | --- | --- |
| Type | `type:N` | Fixed primitive/internal seeds, nominal declaration order, then first semantic use |
| Record | `record:N` | Source order among records |
| Record field | `record:R.field:N` | Declaration order within its record |
| Enum | `enum:N` | Source order among enums |
| Enum variant | `enum:E.variant:N` | Declaration order within its enum |
| Function | `function:N` | Source order among functions |
| Binding | `binding:N` | Semantic introduction order across functions |
| Block | `block:N` | Deterministic semantic traversal order |
| Statement | `statement:N` | Deterministic semantic traversal order |
| Expression | `expression:N` | Deterministic semantic traversal order |
| Match / arm | `match:N`, `match:M.arm:N` | Match and written arm order |

These are not persistent database keys. Editing a file, changing declaration
order, or selecting a later schema may renumber them. Tools that need
cross-revision identity must build that policy above v1 using names, spans, and
their own change tracking.

## Semantic fact tables

`program` contains ordered fact tables rather than compiler-owned Rust enum
layouts:

- `types` interns primitive, nominal, callable, and internal control-flow types;
- `records`, `enums`, and `functions` describe top-level declarations;
- `bindings` describes parameters, locals, and arm-local payload bindings with
  owner, lexical scope, mutability, type, and exact name span;
- `blocks`, `statements`, and `expressions` provide generic, typed relationships
  through document-local IDs without exposing HIR memory representation; and
- record-construction expressions carry `field_initializers` in written
  evaluation order, pairing every value expression with its resolved declared
  field identity; and
- `matches` records the nominal enum, scrutinee, written arms, resolved variant
  and payload binding, result types, and `exhaustive: true` proof outcome.

Expression `children` are in deterministic semantic traversal order. They do
not imply that every child executes: `if`, `match`, `&&`, and `||` retain their
documented conditional or short-circuit execution semantics. `target` identifies
the selected binding, function, nominal declaration/member, block, or match when
that fact exists in v1. A record construction's `field_initializers` repeats its
value children with resolved field identities without changing their written
evaluation order; this is empty on every other expression kind. Literal values
and source text are intentionally omitted.

Only accepted programs produce this schema, so the semantic recovery type and
error expressions are forbidden. Surface `Unit` and the internal `never` type
may appear because tooling needs to understand value-less and non-continuing
control flow; only `never` lacks a surface spelling.

## Deliberate v1 limits

The bootstrap is single-file and exposes no module graph, effects, ownership or
region facts, unsafe capabilities, lifetimes, generic substitutions, layout,
ABI, MIR transformations, runtime values, or incremental-compilation keys.
The compiler's verified bootstrap CFG and definite-initialization events are also
deliberately absent: they were introduced after schema v1 and will not be added by
silently changing an existing tooling contract.
Assignment targets currently carry their enclosing statement span because HIR
does not yet retain a separate target-name span. Match facts report exhaustive
coverage for the implemented qualified single-variant patterns; they do not
predict future wildcard, guard, or pattern-usefulness models.

These omissions are explicit schema limits, not empty promises or inferred
guarantees. Later schemas should add facts only after the corresponding language
semantics exist and have deterministic tests.
