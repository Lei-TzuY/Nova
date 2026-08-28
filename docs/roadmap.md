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

**Status: eleven vertical slices implemented; broader type-system work remains.**

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

- top-level nominal `enum` declarations with stable source-order `EnumId`
  identities and zero-or-one explicitly typed payloads per variant;
- one deterministic type namespace shared by records, enums, and reserved
  primitive names, with all nominal names collected before member types resolve;
- qualified `Enum::Variant` construction with variant and payload-arity/type
  checking, including recursive enum payload references;
- qualified single-variant patterns with immutable, arm-local payload bindings;
- exhaustive match checking that rejects missing, duplicate, unknown, and
  differently qualified variants without wildcard approximation;
- continuing-arm type joining that respects the internal non-continuing `!`
  type; and
- definite-assignment merging by intersection across every continuing arm of a
  valid exhaustive match, while invalid matches establish no flow facts.

Implemented in the sixth Phase 2 slice:

- reserved, statement-only `break;` and `continue;` syntax represented explicitly
  in AST and typed HIR rather than encoded as calls or special identifiers;
- lexical loop-control legality checked against the nearest enclosing `while`
  body, with diagnostic `N3013` outside that scope and the loop condition
  deliberately excluded from it;
- legal loop transfers represented as non-continuing paths so `if` and valid
  exhaustive `match` joins consider only branches that can reach the following
  source;
- unreachable statements and tails still lowered for deterministic name/type
  diagnostics while their scope and definite-initialization mutations are
  discarded from the reachable continuation state;
- loop-body-only initialization continues to be excluded from the post-loop
  state, preserving the zero-iteration proof; and
- adversarial semantic tests cover transfers nested in conditions, dead
  assignments after transfers, and `continue` inside selected match paths.

Implemented in the seventh Phase 2 slice:

- direct literal `while true` recognized as a guaranteed-entry bootstrap loop
  without introducing constant folding or a general termination analysis;
- loop contexts carry reachable `break` exit states and keep nested-loop exits
  attributed only to the nearest enclosing loop;
- definite-initialization after a literal-true loop is the intersection of every
  reachable break exit targeting that loop;
- a literal-true loop with no reachable break is classified as non-continuing,
  improving function-fallthrough reasoning without changing runtime semantics;
- ordinary pre-test loops remain conservative because their zero-iteration exit
  is still possible;
- strict left-to-right expression suffixes after an earlier non-continuing
  subexpression are lowered for diagnostics while their scope and loop-exit
  mutations are discarded; and
- semantic unit tests plus CLI check/run fixtures lock the positive and negative
  guaranteed-loop behavior end to end.

Implemented in the eighth Phase 2 slice:

- semantic flow for `&&` and `||` now follows the interpreter's established
  short-circuit evaluation rather than treating both operands as unconditionally
  executed;
- a directly skipped RHS (`false && rhs`, `true || rhs`) is still lowered for
  deterministic name/type diagnostics while its assignment and loop-exit facts
  are discarded;
- a directly forced RHS (`true && rhs`, `false || rhs`) contributes ordinary
  definite-initialization and non-continuation facts;
- a dynamic Boolean LHS keeps both the short-circuit continuation and RHS
  continuation reachable, so post-expression initialization is their
  intersection rather than an RHS-only fact;
- an optionally executed non-continuing RHS does not make the whole Boolean
  expression non-continuing, while reachable RHS `break` transfers remain valid
  exits from the enclosing loop; and
- analyzer adversarial tests plus CLI check/run fixtures lock literal, dynamic,
  initialization, return, and loop-exit behavior against runtime semantics.

Implemented in the ninth Phase 2 slice:

- direct Boolean-literal `if` conditions refine branch reachability without
  introducing general constant folding;
- only the selected literal branch contributes definite-initialization,
  non-continuation, and loop-exit facts to reachable continuation state;
- the unselected branch remains fully lowered for deterministic name/type
  diagnostics and branch type compatibility while its flow mutations are
  discarded;
- non-literal conditions preserve the existing conservative merge across every
  continuing branch;
- literal-selected `return`, `break`, and `continue` now agree with interpreter
  execution when classifying `!` expressions and guaranteed-loop exits; and
- analyzer adversarial tests plus CLI check/run fixtures lock selected/dead
  initialization and loop-control behavior end to end.

Implemented in the tenth Phase 2 slice:

- a direct, successfully resolved enum constructor used as a `match` scrutinee now
  supplies its known variant to semantic reachability analysis without introducing
  general constant propagation;
- only the selected constructor arm contributes definite-initialization,
  non-continuation, and enclosing-loop exit facts;
- non-selected arms are lowered in diagnostic-only mode so assignments and loop
  transfers cannot leak into reachable state;
- every arm still participates in nominal pattern validation, exhaustiveness, static
  diagnostics, and result-type compatibility, preserving fail-closed checking;
- dynamic enum scrutinees retain the conservative intersection across all continuing
  exhaustive arms; and
- analyzer adversarial tests plus CLI check/run fixtures lock selected-arm payload,
  initialization, return, continue, break, and dead-arm diagnostic behavior.

Implemented in the eleventh Phase 2 slice:

- `Unit` promoted from an internal HIR-only concept to a reserved surface built-in
  type, with `()` as its sole literal;
- AST and typed HIR represent explicit Unit literals directly while value-less blocks
  continue to produce the same semantic `Unit` type;
- functions declared `-> Unit` may fall through a body with no tail expression, while
  explicit `return ();` remains type checked through the ordinary return path;
- non-Unit functions retain the existing all-continuing-path value requirement, and
  non-Unit values returned from Unit functions are rejected with `N3004`;
- `Unit` cannot be redefined as a record or enum, and Unit equality remains outside
  the bootstrap operator contract; and
- parser, semantic, CLI, grammar, constitution, README, and runtime-entry-point tests
  distinguish surface Unit support from the still-Int/Bool-only `main` contract.

The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:

- split and stabilize HIR/resolution/type-analysis/dataflow contracts as
  implementation evidence accumulates, introducing an explicit CFG or flow-state
  representation if ad-hoc structured snapshots stop scaling;
- define language-level numeric types, defaulting, conversions, and overflow
  behavior beyond the provisional interpreter contract;
- deepen the pattern model only with a specified usefulness and diagnostic
  strategy rather than adding ad-hoc wildcard or guard behavior;
- specify aggregate mutation/ownership and layout semantics before field mutation
  or ABI claims are added;
- define a versioned semantic-introspection schema rather than exposing debug
  HIR as a tooling protocol; and
- expand negative and adversarial tests as each rule becomes implemented.

Phase 2 is not complete until its implemented type, name, mutation, aggregate,
and dataflow semantics are sufficiently specified for the executable subset and
no roadmap item is being silently approximated.

## Phase 3 — Executable language subset

**Status: six vertical slices implemented; execution surface remains small.**

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

- executable nominal enum values carrying `EnumId`, a declaration-order variant
  slot, and an optional boxed bootstrap payload;
- source semantics in which a match evaluates its scrutinee exactly once and
  evaluates only the selected arm;
- payload binding and explicit-return propagation through selected arms;
- recursive enum values and recursive matching functions under the existing
  call-depth and execution-step guards;
- interpreter verification that accepted match HIR is exhaustive, non-duplicated,
  in range, and payload-compatible before dispatch; and
- CLI check/run fixtures plus semantic and runtime tests for successful and
  rejected enum/match programs.

Implemented in the fifth Phase 3 slice:

- structured interpreter flow for `Return`, `Break`, and `Continue` rather than
  an ad-hoc Boolean or sentinel attached to loop execution;
- propagation of loop transfers through nested blocks, `if`, aggregate
  initializers, function-call operands, Boolean/arithmetic operands, and selected
  exhaustive-match arms without changing their established evaluation order;
- nearest-loop execution semantics in which `break` exits only the active
  enclosing `while` and `continue` re-enters that loop at its condition test;
- nested-loop behavior that leaves outer loops untouched by an inner `break`;
- fail-closed runtime invariant `N4005` if malformed HIR lets loop control reach a
  condition or escape a lexical loop/function boundary; and
- interpreter and CLI end-to-end tests covering `break`, `continue`, nested
  loops, selected match-arm propagation, invalid placement, and deterministic
  results.

Implemented in the sixth Phase 3 slice:

- the interpreter executes the explicit HIR Unit literal as the existing `Value::Unit`
  representation rather than introducing a second runtime sentinel;
- value-less Unit-returning functions complete normally with Unit, and explicit
  `return ();` propagates through the same structured return machinery as other values;
- Unit values may flow through ordinary local bindings, parameters, calls, returns,
  record fields, and enum payloads under the existing static type checks;
- CLI execution fixtures confirm Unit-valued helper procedures compose with an Int
  `main`, while `main() -> Unit` remains rejected at runtime with `N4001`; and
- no Unit equality, special calling convention, layout promise, or widened entry-point
  ABI is inferred from this bootstrap execution support.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
contracts:

- consider labelled loops, value-producing loop expressions, or value-carrying
  `break` only after their target identity, type-join, and CFG/dataflow contracts
  are explicit rather than extending the current nearest-`while` rule ad hoc;
- consider richer patterns only after their usefulness, binding, dataflow, and
  execution contracts can remain deterministic;
- decide whether aggregate update/mutation requires a dedicated semantic model
  rather than extending the current identifier-only assignment form;
- introduce a small explicit execution IR if interpreter complexity begins to
  leak backend concerns into HIR; and
- keep runtime diagnostics source-qualified and reproducible.

Record runtime slots and boxed enum payloads are interpreter implementation
evidence, not source-level layout, allocation, ownership, or ABI guarantees.
Native code generation is not implied by the bootstrap interpreter. Backend work
remains a later phase and must consume verified shared IR rather than reimplement
source semantics independently.

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
