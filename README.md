# Nova

Nova is an early-stage programming-language project exploring whether one
coherent language can combine fast, readable application development with
predictable native performance and strong safety properties. Its intended
design space includes static typing with inference, explicit optional values,
algebraic data types, typed errors and effects, structured concurrency, and
low-level control through narrowly classified `unsafe` capabilities.

Those are design goals, not current claims. Nova is not production ready, does
not yet execute programs, and does not yet implement ownership, effects,
concurrency, native code generation, or memory-safety analysis.

## Current status

The repository contains the Phase 0 constitution, the executable Phase 1
frontend, and early Phase 2 semantic-core slices. The bootstrap toolchain is
written in Rust and can:

- read a Nova file while rejecting malformed UTF-8;
- lex the documented v0.1 subset with byte-exact source spans;
- parse functions, bindings, narrow assignments, expressions, blocks, calls,
  and `if` expressions;
- lower accepted syntax into a resolved, typed HIR;
- resolve top-level functions, parameters, and lexical local bindings;
- check bootstrap `Int`/`Bool` function signatures, local inference and
  annotations, calls, operators, block tails, `if` branches, returns, and
  assignment mutability/type constraints;
- emit structured, coded diagnostics rendered as human text or JSON Lines; and
- print a deterministic debug representation of the parsed AST.

`nova check` performs lexical, syntactic, name-resolution, and bootstrap type
validation. This is intentionally narrower than a complete Nova type system:
numeric semantics, user-defined types, effects, ownership, modules, generics,
and execution remain future work.

The implemented syntax is intentionally small:

```nova
fn choose(flag: Bool, left: Int, right: Int) -> Int {
    var selected = left;
    selected = if flag {
        left
    } else {
        right
    };

    selected + 1
}
```

See [the implemented grammar](docs/grammar.md) for the normative frontend
subset and [the language constitution](docs/language-constitution.md) for
decisions that extend beyond it.

## Current semantic rules

The Phase 2 bootstrap checker predeclares function signatures, so forward calls
and recursion resolve deterministically. A local initializer is checked before
its new binding enters scope, preventing accidental self-reference. Duplicate
names in the same lexical scope are rejected; nested lexical blocks may shadow
outer bindings in this slice. Function parameters and a function body's
outermost bindings share one scope.

`let` bindings and function parameters are immutable. `var` bindings may be
assigned with the narrow statement form `name = expression;`. The target must
resolve to a lexical `var`; functions, unknown names, `let` bindings, and
parameters are rejected as assignment targets. The replacement value must keep
the binding's established type. Assignment is not an expression and therefore
cannot be chained or embedded in another expression.

Only `Int` and `Bool` are recognized surface types today. Arithmetic and ordered
comparisons require `Int`; boolean operators require `Bool`; equality accepts
matching `Int` or matching `Bool`; calls require a function value with matching
arity and argument types. `if` conditions require `Bool`, its two branches must
have compatible value types, explicit `return` expressions are checked against
the function signature, and a function cannot fall through without a value.
Internal HIR uses `()` and `!` only to model value-less and non-continuing
control flow; neither is a surface type in the current grammar.

These rules are bootstrap semantics, not a promise that Nova's broader type,
mutation, and shadowing policies are frozen.

## Build and use

Nova declares Rust 1.85 as its bootstrap minimum and also tracks current stable
Rust in CI. With Rust and Cargo installed:

```console
cargo build --workspace
cargo run -p nova-cli -- check examples/basics.nv
cargo run -p nova-cli -- ast examples/basics.nv
```

Machine-readable diagnostics are available without changing the compiler's
internal diagnostic model:

```console
cargo run -p nova-cli -- check examples/broken.nv --message-format json
```

The installed binary is named `nova`:

```text
nova check <file> [--message-format human|json]
nova ast <file> [--message-format human|json]
```

Exit status `0` means the requested operation succeeded, `1` means the source
or program was rejected, and `2` means the command line was invalid. `nova ast`
intentionally stops after parsing, so it can inspect a syntactically valid AST
even when `nova check` would reject that program semantically.

## Bootstrap architecture

```text
source bytes
  -> nova-source        source identity, UTF-8 text, spans, locations
  -> nova-lexer         tokens and lexical diagnostics
  -> nova-parser        AST and syntactic diagnostics
  -> nova-sema          typed HIR, lexical resolution, bootstrap type checking
  -> nova-cli           check/ast commands and diagnostic presentation

nova-diagnostics        shared structured diagnostic model and renderers
```

Crate boundaries follow semantic responsibilities rather than intended future
compiler passes. Later work can deepen HIR, inference, effects, MIR, and
backends without making the AST or CLI the owner of language semantics.

## Engineering policy

- Unsupported constructs are errors; the compiler does not approximate them.
- Every implemented semantic or syntactic rule requires deterministic tests.
- Source positions are UTF-8 byte ranges internally and one-based line/column
  locations when rendered.
- CI checks Rust 1.85 compatibility, rejects formatting and Clippy warnings on
  current stable, and runs all tests, builds, and rustdoc.
- Roadmap status is evidence-based; planned properties are not reported as
  implemented guarantees.

The staged implementation plan is in [docs/roadmap.md](docs/roadmap.md).

## Contributing

Keep changes focused and pair grammar or semantic changes with specification
updates, positive tests, and negative tests. Prefer a small end-to-end slice
over disconnected scaffolding for several future phases.

No project license has been selected yet. Until a license file is added, the
repository remains under the rights granted by applicable copyright law and
GitHub's terms; do not infer an open-source license from public visibility.
