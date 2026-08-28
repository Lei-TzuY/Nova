# Nova Roadmap

This roadmap orders work by semantic dependency. A phase is complete only when
its implemented behavior is documented, tested, and exercised by CI. Later
phases may be researched in isolation, but they do not become product claims or
stable syntax ahead of their prerequisites.

## Phase 0 — Language constitution

**Status: initial baseline in this repository.**

- goals, non-goals, and design constraints;
- provisional lexical and syntax policy;
- value, mutability, name, error, effect, memory, concurrency, module, package,
  unsafe, compatibility, and implementation directions; and
- an explicit unresolved-research register.

Completion of the initial document does not freeze unfinished semantics.

## Phase 1 — Executable frontend foundation

**Status: first vertical slice implemented; broader grammar work remains.**

- Rust workspace and official `nova` CLI bootstrap;
- source identity, exact spans, and locations;
- structured human and JSON diagnostics;
- lexer and parser for the grammar in `docs/grammar.md`;
- AST inspection with `nova ast`;
- positive, negative, span, precedence, overflow, comment, recovery, and depth
  tests; and
- Rust 1.85 MSRV checking plus current-stable formatting, Clippy, test, build,
  and rustdoc CI.

Next Phase 1 refinements should be driven by the needs of later semantic work,
not by adding unrelated syntax.

## Phase 2 — Semantic core

**Status: five vertical slices implemented; broader type-system work remains.**

Implemented in the first Phase 2 slice:

- a purpose-built resolved, typed HIR in `nova-sema`;
- deterministic source-order function identities and analysis-order binding
  identities;
- lexical scopes with function predeclaration for forward calls and recursion;
- same-scope duplicate rejection, unknown-name diagnostics, and nested-block
  shadowing as an explicit bootstrap policy;
- primitive `Int` and `Bool` type-name resolution;
- function signatures, local initializer inference, optional local annotations,
  calls, operators, block tails, and `if` typing;
- explicit-return checking plus rejection of value-returning functions that can
  fall through;
- semantic diagnostics in the existing human and JSON Lines formats; and
- `nova check` upgraded from syntax validation to semantic validation while
  `nova ast` remains a parser-inspection command.

Implemented in the second Phase 2 slice:

- narrow assignment syntax as the statement `identifier = expression;`;
- assignment kept outside expression precedence, so chaining and embedded
  assignment are rejected by construction;
- `let` bindings and function parameters treated as immutable, while `var`
  bindings are assignable;
- assignment targets resolved lexically to stable binding identities;
- assignment RHS values checked against each binding's established type;
- unknown, function, and immutable assignment targets rejected deterministically
  with semantic diagnostics; and
- parser, semantic, CLI, fixture, example, grammar, constitution, and README
  coverage kept in sync.

Implemented in the third Phase 2 slice:

- typed delayed initialization with `var identifier: Type;`, while `let` and
  untyped `var identifier;` declarations remain rejected;
- per-binding definite-initialization state tracked through semantic analysis;
- diagnostic `N3009` for reads that can observe an uninitialized local;
- successful, type-correct assignment transitions a delayed `var` to initialized;
- `if` branches analyzed from the same entry state and merged by intersection
  across paths that can continue;
- noncontinuing branches such as explicit returns excluded from the surviving
  path's initialization requirement; and
- no runtime default value or silent initialization inserted by the compiler.

Implemented in the fourth Phase 2 slice:

- top-level nominal `record` declarations with stable source-order `RecordId`
  identities rather than structural type equivalence;
- two-pass record collection so declared record names are available to field and
  function type resolution before function bodies are lowered;
- explicit `new Record { field: expression, ... }` construction and postfix
  `value.field` projection;
- deterministic diagnostics for duplicate record/type definitions, duplicate
  declared fields, unknown/duplicate/missing constructor fields, projection of
  unknown fields, and field initializer type mismatches;
- typed HIR record construction that resolves each named field to its
  declaration-order slot while preserving written source evaluation order; and
- record types integrated with function signatures, local inference,
  annotations, assignment type preservation, returns, and branch type joining.

Implemented in the fifth Phase 2 slice:

- statement-only `break;` and `continue;` with lexical legality restricted to
  the nearest enclosing `while` body and semantic diagnostic `N3013` otherwise;
- explicit non-continuing HIR paths for loop transfer so `if` dataflow merges can
  ignore branches that do not reach the following statement;
- `continue` modeled as returning to the loop's pre-test condition and `break`
  as leaving the nearest active loop, without introducing value-producing loop
  semantics;
- unreachable source remains name/type checked for deterministic diagnostics,
  while its declarations and assignments are rolled back from reachable
  definite-assignment state;
- the `while` condition deliberately remains outside its body's loop-control
  scope, avoiding an implicit exit protocol inside condition expressions; and
- semantic/adversarial tests covering valid nested transfer, misuse outside a
  loop, and unreachable assignments that must not manufacture initialization
  evidence.

The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:

- split and stabilize HIR/resolution/type-analysis/dataflow contracts as
  implementation evidence accumulates;
- define language-level numeric types, defaulting, conversions, and overflow
  behavior beyond the provisional interpreter contract;
- add enums only with stable nominal identity and deterministic diagnostics;
- introduce exhaustive pattern checking after algebraic data types exist;
- specify aggregate mutation/ownership and layout semantics before field mutation
  or ABI claims are added;
- define labelled/value-producing loop semantics only with an explicit CFG and
  type/dataflow contract rather than extending bootstrap transfer ad hoc;
- define a versioned semantic-introspection schema rather than exposing debug
  HIR as a tooling protocol; and
- expand negative and adversarial tests as each rule becomes implemented.

Phase 2 is not complete until its implemented type, name, mutation, aggregate,
control-flow, and dataflow semantics are sufficiently specified for the
executable subset and no roadmap item is being silently approximated.

## Phase 3 — Executable language subset

**Status: four vertical slices implemented; execution surface remains small.**

Implemented in the first Phase 3 slice:

- `nova-interpreter`, a deterministic interpreter over semantically accepted
  typed HIR rather than raw syntax;
- `nova run`, which reuses the exact lex/parse/semantic pipeline before execution;
- a zero-argument `main` entry-point contract with `Int` or `Bool` result;
- function calls, recursion, block values, `if`, explicit return propagation,
  initialized and delayed mutable locals, and assignment;
- left-to-right expression evaluation with short-circuit `&&` and `||`;
- provisional checked signed-64-bit `Int` execution, with runtime diagnostics
  for overflow and zero divisors instead of host-profile-dependent behavior;
- a guarded active-call limit for deterministic recursion failure; and
- interpreter unit tests plus CLI success, human-runtime-error, and JSON-runtime-
  error fixtures.

Implemented in the second Phase 3 slice:

- pre-test `while condition { body }` syntax represented explicitly in AST and
  typed HIR;
- semantic checking that requires a `Bool` condition;
- conservative loop definite-assignment: the mandatory condition pre-test may
  establish facts, while body-only initialization cannot escape because the body
  may execute zero times;
- interpreter execution of loop mutation and return propagation;
- a shared deterministic statement/expression step budget with runtime
  diagnostic `N4006`, so nonterminating loops fail closed instead of hanging the
  host; and
- parser, semantic, interpreter, CLI, positive, negative, and nontermination
  fixtures kept in sync with the grammar and language documentation.

Implemented in the third Phase 3 slice:

- executable nominal record values carrying `RecordId` identity plus
  declaration-order field slots inside the bootstrap interpreter;
- source-order evaluation of named record initializers even when their written
  order differs from declaration order;
- record values passed through ordinary function parameters and returns under
  the same semantic type checks as primitive values;
- resolved field projection without runtime string lookup;
- interpreter invariants that fail closed if malformed HIR supplies an invalid
  record identity, duplicate destination slot, missing slot, or mismatched field
  projection; and
- CLI end-to-end fixtures for record checking/execution plus negative missing-
  field diagnostics.

Implemented in the fourth Phase 3 slice:

- interpreter `Flow` extended with structured `Break` and `Continue` alongside
  ordinary values and function returns;
- loop transfer propagated through nested blocks, conditionals, record
  initializers, call arguments, unary/binary expressions, and other evaluated
  subexpressions until the nearest active `while` consumes it;
- `break` exits only the nearest loop while nested outer loops continue normally;
- `continue` starts the next iteration by re-evaluating the pre-test condition;
- invariant failures guard impossible loop-control escape across semantic
  boundaries rather than silently assigning ad-hoc runtime meaning; and
- interpreter and CLI end-to-end tests cover skipping iterations, early loop
  exit, nested-loop targeting, and semantic rejection outside a loop.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
contracts:

- execute enums and exhaustive pattern matching only after their semantic model
  and exhaustiveness checking exist;
- decide whether aggregate update/mutation requires a dedicated semantic model
  rather than extending the current identifier-only assignment form;
- introduce labelled or value-producing loop execution only after matching
  Phase 2 CFG/type/dataflow contracts exist;
- introduce a small explicit execution IR if interpreter complexity begins to
  leak backend concerns into HIR; and
- keep runtime diagnostics source-qualified and reproducible.

Record runtime slots are interpreter implementation evidence, not source-level
layout or ABI guarantees. Native code generation is not implied by the bootstrap
interpreter. Backend work remains a later phase and must consume verified shared
IR rather than reimplement source semantics independently.

## Phase 4 — Typed errors and effects

**Status: research required.** Specify recoverable errors, propagation,
interfaces, effect inference, cancellation, and panic boundaries before adding
surface syntax.

## Phase 5 — Generics and traits

**Status: planned.** Define coherence, inference boundaries, specialization
policy, and compilation strategy with diagnostic quality as a primary metric.

## Phase 6 — Ownership, regions, and resource lifetimes

**Status: research required.** Prototype and measure the hybrid memory model.
Do not label Nova memory safe until accepted programs and unsafe boundaries are
checked by an implemented model with adversarial tests.

## Phase 7 — Stable intermediate representation and native backend

**Status: planned.** Introduce verified MIR, explicit layout and ABI rules,
optimization contracts, debug information, and an initial native backend.

## Phase 8 — Structured concurrency

**Status: research required.** Add scoped tasks, cancellation, race-freedom
rules, and executor contracts only after effects and ownership can express them.

## Phase 9 — Package and build ecosystem

**Status: planned.** One official manifest, resolver, lockfile, package tool,
formatter, linter, test runner, documentation generator, and reproducible build
protocol.

## Phase 10 — WebAssembly backend

**Status: planned.** Reuse verified shared IR; document target restrictions and
component/host interoperability without creating a Wasm-only language dialect.

## Phase 11 — C interoperability

**Status: planned.** Specify ABI, layout, ownership, error, callback, and unwind
boundaries; add narrowly classified unsafe capabilities and conformance tests.

## Phase 12 — SIMD and GPU research

**Status: research required.** Determine which shared-IR abstractions preserve
Nova semantics and which target constraints require explicit APIs or effects.

## Phase 13 — Self-hosting

**Status: long-term.** Begin only after the language subset needed by the
compiler is stable, bootstrap reproducibility is demonstrated, and builds can
compare trusted Rust-bootstrap and Nova-hosted outputs.
