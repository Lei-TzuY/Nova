# Nova

Nova is an early-stage programming-language project exploring whether one
coherent language can combine fast, readable application development with
predictable native performance and strong safety properties. Its intended
design space includes static typing with inference, explicit optional values,
algebraic data types, typed errors and effects, structured concurrency, and
low-level control through narrowly classified `unsafe` capabilities.

Those are design goals, not current claims. Nova is not production ready, does
not yet execute programs, and does not yet implement type checking, ownership,
effects, concurrency, native code generation, or memory-safety analysis.

## Current status

The repository contains the Phase 0 constitution, the executable Phase 1
frontend, and the first narrow Phase 2 semantic slice. The bootstrap toolchain
is written in Rust and can:

- read a Nova file while rejecting malformed UTF-8;
- lex the documented v0.1 subset with byte-exact source spans;
- parse functions, bindings, expressions, blocks, calls, and `if` expressions;
- lower valid syntax to a purpose-built HIR with deterministic symbol IDs;
- resolve single-file functions, parameters, local bindings, `Int`, and `Bool`;
- emit structured, coded diagnostics rendered as human text or JSON Lines; and
- print a deterministic debug representation of the parsed AST.

`nova check` validates UTF-8, tokens, syntax, and the implemented single-file
name rules. It does **not** imply that a program is well typed. `nova ast`
intentionally remains a syntax-tree inspection command and does not require
names to resolve.

The implemented syntax is intentionally small:

```nova
fn choose(flag: Bool, left: Int, right: Int) -> Int {
    let selected = if flag {
        left
    } else {
        right
    };

    selected + 1
}
```

See [the implemented grammar](docs/grammar.md) and
[name-resolution rules](docs/name-resolution.md) for the normative executable
subset, and [the language constitution](docs/language-constitution.md) for
decisions that extend beyond it.

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

Exit status `0` means the requested frontend operation succeeded, `1` means
the source or program was rejected, and `2` means the command line was invalid.

## Bootstrap architecture

```text
source bytes
  -> nova-source        source identity, UTF-8 text, spans, locations
  -> nova-lexer         tokens and lexical diagnostics
  -> nova-parser        AST and syntactic diagnostics
  -> nova-hir           resolved HIR, lexical scopes, symbol identities
  -> nova-cli           check/ast commands and diagnostic presentation

nova-diagnostics        shared structured diagnostic model and renderers
```

Crate boundaries follow semantic responsibilities rather than intended future
compiler passes. Later work can add type inference, effects, MIR, and backends
without making the AST or CLI the owner of language semantics.

## Engineering policy

- Unsupported constructs are errors; the frontend does not approximate them.
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
