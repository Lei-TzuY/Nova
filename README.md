# Nova

Nova is an early-stage programming-language project exploring whether one
coherent language can combine fast, readable application development with
predictable native performance and strong safety properties. Its intended
design space includes static typing with inference, explicit optional values,
algebraic data types, typed errors and effects, structured concurrency, and
low-level control through narrowly classified `unsafe` capabilities.

Those are design goals, not current claims. Nova is not production ready. The
current bootstrap can interpret its small checked subset, but it does not yet
implement ownership, effects, concurrency, native code generation, a standard
library, or memory-safety analysis.

## Current status

The repository contains the Phase 0 constitution, the executable Phase 1
frontend, Phase 2 semantic-core slices, and executable Phase 3 bootstrap
interpreter slices. The toolchain is written in Rust and can:

- read a Nova file while rejecting malformed UTF-8;
- lex the documented v0.1 subset with byte-exact source spans;
- parse functions, nominal records and enums, explicit aggregate construction,
  exhaustive enum matching, field projection, initialized bindings, typed
  delayed `var` initialization, narrow assignments, expressions, blocks, calls,
  `if` expressions, pre-test `while` loops, and statement-only `break`/`continue`;
- lower accepted syntax into a resolved, typed HIR with stable function,
  binding, record, and enum identities;
- resolve top-level functions and nominal types, parameters, lexical local
  bindings, record field slots, enum variant slots, and match payload bindings;
- check bootstrap `Int`, `Bool`, `Unit`, and nominal aggregate types, function
  signatures, local inference and annotations, calls, operators, block tails, branches,
  returns, loop conditions, loop-control legality, record construction/projection,
  enum construction, match exhaustiveness and arm types, assignment
  mutability/type constraints, and definite initialization;
- execute semantically accepted programs through a deterministic bootstrap
  interpreter with function calls, recursion, Unit-valued procedures, records, enums,
  pattern matching, mutation, blocks, conditionals, bounded loops, and structured
  `break`/`continue`;
- emit structured, coded compile-time and runtime diagnostics rendered as human
  text or JSON Lines; and
- print a deterministic debug representation of the parsed AST.

`nova check` performs lexical, syntactic, name-resolution, bootstrap type, and
definite-assignment validation. `nova run` performs those same checks and then
executes zero-argument `main`. The interpreter is evidence for the executable
subset, not a claim that Nova's final runtime representation, numeric model,
aggregate layout, ABI, or backend is stable.

The implemented syntax is intentionally small:

```nova
enum Result {
    Empty,
    Value(Int),
}

fn main() -> Int {
    let result = Result::Value(42);
    match result {
        Result::Empty => 0,
        Result::Value(value) => value,
    }
}
```

See [the implemented grammar](docs/grammar.md) for the normative frontend
subset, [the enum and pattern semantics](docs/enums-and-patterns.md) for that
aggregate slice's semantic contract, and
[the language constitution](docs/language-constitution.md) for decisions that
extend beyond them.

## Current semantic rules

The Phase 2 bootstrap checker predeclares function signatures and nominal record
and enum identities, so forward calls, recursion, forward aggregate type
references, and recursive enum payload types resolve deterministically. A local
initializer is checked before its new binding enters scope, preventing
accidental self-reference. Duplicate names in the same lexical scope are
rejected; nested lexical blocks may shadow outer bindings in this slice.
Function parameters and a function body's outermost bindings share one scope.

Rejected calls are fail-closed for continuing flow recovery. Callees and arguments
are still lowered left-to-right for deterministic diagnostics, but a non-callable
callee, wrong arity, argument type mismatch, or erroneous argument yields Error HIR
and cannot export assignments or loop-exit facts. An actually evaluated child that
is already non-continuing keeps its `!` flow.

Rejected field access follows the same recovery discipline. The base is evaluated
first, but a continuing non-record base or unknown field yields Error HIR and cannot
export flow facts produced only inside the rejected access. A base that is already
non-continuing keeps `!` without a secondary field-type cascade.

Rejected unary and binary operators are also fail-closed. Concrete operand type
mismatches produce Error HIR rather than a nominal success type, and flow facts from
a continuing rejected operator are rolled back. Non-continuation from an operand that
must be evaluated keeps `!` precedence; short-circuit operators retain their existing
conditional right-hand evaluation rules.

Invalid continuing control conditions are fail-closed too. A non-Bool or erroneous
`if` condition makes the expression Error-typed and discards condition/branch flow
facts; a rejected `while` condition likewise cannot export pre-test initialization or
loop exits. A condition that is already `!` keeps its non-continuation, while valid
Bool conditions retain the established pre-test and branch dataflow rules.

`record Name { field: Type, ... }` declares a nominal type: two separately
declared records are distinct even if their fields have the same shape. Field
names must be unique. `new Name { field: expression, ... }` must initialize every
declared field exactly once with a value of the declared type. Named
initializers may be written in any order, but their expressions evaluate left
to right in written source order. HIR resolves each initializer to a stable
record identity and declaration-order field slot without reordering evaluation.
`value.field` is read-only field projection in this slice. Record equality,
field assignment, layout, and ABI guarantees are not implemented.

Rejected aggregate construction is fail-closed for continuing flow recovery. A
record with an unresolved/non-record target or invalid field shape/type, or an enum
constructor with invalid target/payload shape/type, may still emit deterministic
child diagnostics, but assignments or loop
exits created only inside that rejected continuing expression do not become
post-expression facts. A child that is already non-continuing keeps its `!` flow.

`enum Name { Empty, Value(Type) }` declares a nominal sum type whose variants
carry zero or one payload in this slice. Construction is explicitly qualified as
`Name::Empty` or `Name::Value(expression)`. A `match` scrutinee must have an enum
type, every pattern must name a variant of that same nominal enum, and every
variant must occur exactly once. Payload bindings are immutable and scoped to
one arm. Wildcards, guards, nested patterns, multi-payload variants, enum
equality, layout, and ABI guarantees are not implemented.

`let` bindings and function parameters are immutable. `var` bindings may be
assigned with the narrow statement form `name = expression;`. The target must
resolve to a lexical `var`; functions, unknown names, `let` bindings, and
parameters are rejected as assignment targets. The replacement value must keep
the binding's established type. Assignment is not an expression and therefore
cannot be chained or embedded in another expression.

A mutable local may also be declared as `var name: Type;` and initialized by a
later assignment. The explicit type is required. Reading such a binding before
it is definitely initialized is diagnostic `N3009`. For `if` expressions with a non-literal condition,
analysis evaluates the branch states independently and keeps a binding
initialized afterward only when every branch that can continue has initialized
it. For a valid exhaustive match with a dynamic scrutinee, the same intersection
rule applies across every arm that can continue. When the scrutinee is a direct,
successfully resolved enum constructor, its variant is already known: only the
selected arm may contribute definite-initialization, non-continuation, or loop-exit
facts. Non-selected arms are still fully checked for pattern validity, static
diagnostics, exhaustiveness, and arm type compatibility. A branch or reachable arm
that returns, breaks, or continues does not constrain a surviving continuation.
Unreachable code is still analyzed for deterministic diagnostics, but its
assignments cannot manufacture reachable definite-initialization facts.
For strict left-to-right expression forms, once an earlier subexpression cannot
continue, later operands, call arguments, or record initializers are likewise
lowered only for diagnostics and cannot create reachable scope or loop-exit
facts.

`&&` and `||` preserve static checking of both operands while modeling their
runtime reachability explicitly. A direct `false && rhs` or `true || rhs` lowers
`rhs` only for diagnostics, so skipped assignments and loop transfers contribute
no reachable flow facts. A direct `true && rhs` or `false || rhs` treats `rhs` as
mandatory. With a dynamic Boolean left operand, `rhs` is optional: post-expression
definite initialization is the intersection of the short-circuit continuation
and every continuing RHS path. An RHS that returns, breaks, or continues therefore
does not make the whole Boolean expression non-continuing when the left operand
can bypass it, although a reachable RHS `break` still remains a possible exit
from its enclosing loop.

A direct `if true` or `if false` refines control-flow reachability without
turning the checker into a general constant folder. Only the selected branch may
contribute definite-initialization, non-continuation, or loop-exit facts; the
unselected branch is still lowered for deterministic static diagnostics and still
participates in branch type compatibility. Block-valued or computed Boolean
conditions keep the ordinary conservative two-branch merge.

`while condition { body }` is a pre-test statement. The condition must be
`Bool`. For an ordinary condition, the body may execute zero times, so
definite-assignment facts established while evaluating the mandatory first
condition test may survive the loop while facts established only inside the body
do not. This preserves the zero-iteration exit rather than manufacturing
initialization evidence.

A direct literal `while true { body }` is the one bootstrap exception because it
has no condition-false exit. The checker treats it as guaranteed-entry and
records only reachable `break` transfers that target that exact loop. If there
are such exits, a pre-existing binding is definitely initialized afterward only
when it is initialized at every reachable break exit. If there is no reachable
break, the loop is non-continuing. A `break` consumed by a nested loop does not
count as an exit from an outer loop. This is deliberately not constant folding:
`while { true }`, computed booleans, and other equivalent-looking conditions keep
the ordinary conservative rule.

`break;` and `continue;` are legal only inside an enclosing `while` body. The
condition expression is intentionally outside that loop-control scope. `break;`
exits the nearest enclosing loop; `continue;` skips the rest of the current
iteration and re-evaluates that same loop's condition. Neither carries a value
or acts as an expression. Labelled loops and value-carrying breaks are not part
of the bootstrap subset.

`Int`, `Bool`, `Unit`, and declared nominal records and enums are recognized
surface types today. `()` is the sole Unit literal, and a block with no tail also
produces Unit. A function declared `-> Unit` may fall through such a body or use
the explicit `return ();` form; non-Unit functions still need a compatible tail or
an explicit return on every continuing path. Arithmetic and ordered comparisons
require `Int`; boolean operators require `Bool`; equality currently accepts only
matching `Int` or matching `Bool`; calls require matching arity and argument types.
`if` conditions require `Bool`, and continuing branches or match arms must remain
type-compatible. The internal `!` bottom type still has no surface spelling.

These rules are bootstrap semantics, not a promise that Nova's broader type,
mutation, control-flow, aggregate, and shadowing policies are frozen.

## Bootstrap execution rules

`nova run` requires one top-level `main` with no parameters and an `Int` or
`Bool` return type; surface `Unit` is available to helper functions but does not
widen the bootstrap entry-point contract. Execution evaluates expressions left to
right. Record initializer
expressions follow the same rule even when named fields are written out of
declaration order. `&&` and `||` are short-circuiting, so a skipped right operand
performs no mutation, call, return, or loop transfer. Semantic dataflow follows
that same reachability while still type-checking the skipped source. A match
evaluates its scrutinee exactly once and then only its selected arm.

The interpreter propagates structured control flow through nested blocks,
conditionals, aggregate initializers, call arguments, and selected match arms.
`return` reaches the current function call. `break` and `continue` travel only
to the nearest enclosing `while`; that loop consumes them by exiting or starting
the next condition test. If malformed HIR lets loop control escape its lexical
loop or cross a function boundary, execution fails closed with invariant
diagnostic `N4005` rather than guessing a target.

For deterministic execution while the numeric design remains provisional, the
bootstrap frontend now covers the complete signed 64-bit literal endpoints: positive
literals end at `9223372036854775807`, while `-9223372036854775808` is normalized
during semantic lowering to the exact minimum `Int`. Positive `9223372036854775808`
is `N3030`; any larger decimal magnitude is lexical `N1004`. The interpreter
represents `Int` as signed 64-bit at runtime and uses checked arithmetic.
Signed division truncates the quotient toward zero; a non-zero remainder has the
same sign as the dividend and satisfies `a = (a / b) * b + (a % b)`. Both
`i64::MIN / -1` and `i64::MIN % -1` are classified as integer overflow. Overflow
produces `N4002`; division or remainder by zero produces `N4003`. Arithmetic
policy lives in a pure interpreter contract rather than being inferred from host
operator edge cases. Recursive execution is guarded by a finite active-call budget
and reports `N4004`. All statement/expression evaluation also shares a finite
execution-step budget; a nonterminating loop therefore reports `N4006` instead
of hanging indefinitely. Missing or invalid `main` is `N4001`. Record values
currently use declaration-order slots; enum values use a variant slot and an
optional boxed payload. Those interpreter-owned nominal representations are not
stabilized source layouts, allocation promises, ownership rules, or ABI
contracts.

## Build and use

Nova declares Rust 1.85 as its bootstrap minimum and also tracks current stable
Rust in CI. With Rust and Cargo installed:

```console
cargo build --workspace
cargo run -p nova-cli -- check examples/basics.nv
cargo run -p nova-cli -- run examples/basics.nv
cargo run -p nova-cli -- ast examples/basics.nv
```

The `run` command prints the returned value from `main`.

Machine-readable diagnostics are available without changing the compiler's
internal diagnostic model:

```console
cargo run -p nova-cli -- run examples/broken.nv --message-format json
```

The installed binary is named `nova`:

```text
nova check <file> [--message-format human|json]
nova run <file> [--message-format human|json]
nova ast <file> [--message-format human|json]
```

Exit status `0` means the requested operation succeeded, `1` means the source or
execution was rejected, and `2` means the command line was invalid. `nova ast`
intentionally stops after parsing, so it can inspect a syntactically valid AST
even when `nova check` or `nova run` would reject that program later.

## Bootstrap architecture

```text
source bytes
  -> nova-source        source identity, UTF-8 text, spans, locations
  -> nova-lexer         tokens and lexical diagnostics
  -> nova-parser        AST and syntactic diagnostics
  -> nova-sema          typed HIR, nominal identity, resolution, typing, dataflow
  -> nova-interpreter   deterministic checked, bounded execution of accepted HIR
  -> nova-cli           check/run/ast commands and diagnostic presentation

nova-diagnostics        shared structured diagnostic model and renderers
```

Crate boundaries follow semantic responsibilities rather than intended future
compiler passes. Later work can deepen HIR, inference, effects, MIR, layout, and
backends without making the AST, interpreter, or CLI the owner of unfinished
language semantics.

## Engineering policy

- Unsupported constructs are errors; the compiler does not approximate them.
- Every implemented semantic, syntactic, or execution rule requires
  deterministic tests.
- Source positions are UTF-8 byte ranges internally and one-based line/column
  locations when rendered.
- Runtime arithmetic is checked; host build mode never decides Nova results.
- Observable evaluation order is explicit; named record fields do not reorder
  their initializer expressions, and a match evaluates only its selected arm.
- Short-circuit reachability in semantic flow must agree with runtime `&&`/`||`
  execution while skipped source remains statically checked.
- Non-continuing control-flow paths cannot contribute definite-assignment or loop-
  exit facts to code they cannot reach.
- Potentially nonterminating bootstrap execution is bounded and fails with a
  structured diagnostic rather than intentionally hanging the host.
- CI checks Rust 1.85 compatibility, rejects formatting and Clippy warnings on
  current stable, and runs all tests, builds, and rustdoc.
- Roadmap status is evidence-based; planned properties are not reported as
  implemented guarantees.

The staged implementation plan is in [docs/roadmap.md](docs/roadmap.md).

## Contributing

Keep changes focused and pair grammar, semantic, or runtime changes with
specification updates, positive tests, and negative tests. Prefer a small
end-to-end slice over disconnected scaffolding for several future phases.

No project license has been selected yet. Until a license file is added, the
repository remains under the rights granted by applicable copyright law and
GitHub's terms; do not infer an open-source license from public visibility.
