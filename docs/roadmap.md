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

**Status: forty-three vertical slices implemented; broader type-system work remains.**

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

- semantic rollback now has an explicit internal `ReachableState` that bundles
  lexical scope/definite-initialization state with enclosing loop contexts;
- diagnostic-only expression/block lowering captures and restores that state
  atomically instead of cloning the two components independently;
- unreachable statement suffixes and block tails use the same state contract,
  preventing dead assignments or loop transfers from leaking reachable facts;
- binding identity allocation and diagnostics intentionally remain outside the
  rollback snapshot, preserving deterministic HIR identities and error reporting;
- scope-only merges for dynamic branches, matches, and optional short-circuit RHS
  remain separate because reachable loop exits from those paths must accumulate; and
- dedicated regression tests plus the existing adversarial semantic suite lock the
  refactor to behavior-preserving state restoration.

Implemented in the twelfth Phase 2 slice:

- `Unit` is promoted from an internal value-less HIR marker to a reserved surface
  type available in signatures, annotations, record fields, and enum payloads;
- `()` is the sole surface Unit literal and lowers to explicit typed HIR rather than
  being confused with grouping or an empty argument list;
- Unit-returning functions may complete through an empty body, while explicit tails
  and `return` expressions remain type checked against `Unit`;
- non-Unit fallthrough rules remain unchanged, and `Unit` is still excluded from the
  bootstrap equality operators;
- reserved-type diagnostics prevent user-defined `Unit` declarations from shadowing
  the built-in type; and
- parser, semantic, interpreter, CLI, aggregate, call, match, negative-type, and
  invalid-entry-point coverage lock the surface contract end to end.

Implemented in the thirteenth Phase 2 slice:

- continuing record construction that fails structural, child-error, or field-type
  validation restores its pre-construction reachable state instead of exporting
  assignments or loop-exit facts from rejected source;
- enum constructors apply the same rollback to unknown/invalid constructors, payload
  arity errors, child errors, and payload type mismatches;
- aggregate field/payload type mismatches now produce `Type::Error` HIR rather than
  retaining a nominal aggregate type after diagnostic `N3004`;
- a child expression that is already non-continuing still yields `!`, preserving
  reachable return/break/continue precedence over continuing recovery rollback; and
- adversarial tests lock both definite-initialization rollback and non-continuing
  aggregate inputs across records and enums.

Implemented in the fourteenth Phase 2 slice:

- recovery-aware expected-type compatibility is centralized in an internal pure
  `type_rules` contract instead of being duplicated inside analyzer helpers;
- strict binary result typing explicitly records that reachable non-continuation
  (`!`) dominates recovery errors while ordinary successful operands produce the
  operator's declared result type;
- `if` and exhaustive `match` now share one `TypeJoin` state machine in which `!` is
  bottom, `<error>` is recovery-neutral when a concrete alternative exists, and the
  first concrete continuing type remains the diagnostic anchor;
- analyzer-owned source spans and N3004 wording remain unchanged while pure type
  decisions are separated from diagnostic rendering; and
- truth-table tests lock primitive, `Unit`, nominal, `!`, `<error>`, compatible, and
  mismatched joins so later type-system work has an executable semantic contract.

Implemented in the fifteenth Phase 2 slice:

- definite-initialization joins now share an internal pure `InitializationJoin`
  contract instead of encoding continuing-path intersection separately for loops,
  optional execution, `if`, and exhaustive `match`;
- only paths that can reach the join point participate, and a binding is considered
  initialized exactly when every such path reports it initialized;
- when every alternative is non-continuing, the entry fact is retained for later
  diagnostic-only lowering rather than inventing a reachable initialization fact;
- scope lookup and binding identity remain analyzer responsibilities, keeping the
  reusable flow rule independent of lexical representation; and
- truth-table tests plus the existing loop, short-circuit, branch, match, and invalid-
  aggregate adversarial suites lock the refactor to behavior-preserving dataflow.

Implemented in the sixteenth Phase 2 slice:

- record construction now captures reachable state before resolving its nominal
  target, matching the fail-closed policy already applied to enum constructors;
- unresolved record names and enum-as-record category errors still lower field
  expressions in written order for deterministic diagnostics, but continuing
  assignments and loop-exit facts from the rejected construction are rolled back;
- a field expression that is already non-continuing keeps `!` precedence, so a
  reachable `return`, `break`, or `continue` is not hidden by the invalid record head;
- record-head recovery now returns `Type::Never` for an actually non-continuing child
  instead of unconditionally collapsing the rejected expression to `Type::Error`; and
- adversarial tests lock definite-initialization, conditional break-exit, HIR type,
  and non-continuation behavior for both unknown targets and enum-as-record misuse.

Implemented in the seventeenth Phase 2 slice:

- call lowering snapshots reachable state before evaluating the callee and arguments,
  so a continuing rejected call cannot export assignments or loop-exit facts;
- wrong arity, non-callable callees, argument type mismatches, and evaluated argument
  errors now produce `Type::Error` HIR rather than retaining a normal return type;
- callee and argument HIR remain intact and are still lowered left-to-right for
  deterministic diagnostics and source-qualified recovery;
- an actually evaluated non-continuing callee or argument keeps `!` precedence even
  when the call is otherwise invalid, preserving reachable return/break/continue; and
- red-to-green adversarial tests lock definite-initialization, conditional break,
  recovery typing, child-error, and non-continuation behavior across invalid calls.

Implemented in the eighteenth Phase 2 slice:

- field-access lowering snapshots reachable state before evaluating its base, so a
  continuing rejected projection cannot export assignments or loop-exit facts;
- non-record bases and unknown record fields remain source-diagnosed and produce
  `Type::Error` HIR, while valid record projection keeps its established slot/type;
- a base expression that is already non-continuing yields `!` directly instead of
  receiving a cascading N3004 record-type error and being flattened to recovery Error;
- the fix is deliberately local to field access rather than declaring a global
  transactional policy for every erroneous expression category; and
- red-to-green adversarial tests lock definite-initialization, conditional break,
  unknown-field, recovery typing, and non-continuation behavior.

Implemented in the nineteenth Phase 2 slice:

- unary and binary lowering snapshot reachable state so a continuing rejected
  operator cannot export assignments or loop-exit facts from its operands;
- strict arithmetic and comparison result typing now validates concrete operand
  types in the pure `type_rules` contract instead of returning a nominal success
  type after N3004;
- boolean `&&`/`||` likewise become Error-typed on concrete Bool mismatches while
  retaining existing short-circuit reachability for valid Bool operands;
- definitely evaluated `!` operands keep non-continuation precedence over recovery
  errors, while optional short-circuit RHS non-continuation remains path-sensitive;
- equality already produced Error on concrete mismatches and now gains the same
  flow rollback at the shared binary lowering boundary; and
- red-to-green adversarial tests lock typing, definite-initialization, loop-exit,
  strict non-continuation, and short-circuit behavior.

Implemented in the twentieth Phase 2 slice:

- `if` and `while` capture reachable state before evaluating their condition so a
  continuing rejected condition cannot export assignments or loop-exit facts;
- concrete non-Bool and recovery-Error `if` conditions make the whole expression
  `Type::Error` instead of retaining a normal branch join type;
- invalid `while` conditions still lower their body under a lexical loop context for
  deterministic diagnostics, but condition/body flow is rolled back afterward;
- nested invalid loop conditions cannot manufacture break exits for an enclosing
  otherwise-infinite loop;
- conditions already typed `!` retain non-continuation precedence, while valid Bool
  pre-test initialization and ordinary branch merging remain unchanged; and
- red-to-green adversarial tests lock condition-side initialization, branch flow,
  nested break exits, Error typing, valid pre-test facts, and `!` behavior.

Implemented in the twenty-first Phase 2 slice:

- lexer and AST integer tokens preserve checked unsigned decimal magnitude through
  parsing instead of prematurely forcing every literal through positive `i64`;
- the bootstrap lexical ceiling becomes `2^63`, with larger magnitudes remaining
  deterministic `N1004` errors and no wrapping or truncation;
- semantic lowering accepts positive magnitudes only through `i64::MAX`, reports
  `N3030` for positive `2^63`, and normalizes prefix `-2^63` to exact `i64::MIN`;
- outer operations remain explicit HIR rather than being folded; subsequent semantic
  preflight may reject a provably failing closed operation while preserving that HIR
  shape for recovery and keeping dynamic equivalents runtime checked;
- CLI fixtures cover both signed endpoints, semantic-vs-lexical overflow separation,
  and minimum-value negation end to end; and
- the change remains a bootstrap signed-64 contract, not a decision on future numeric
  families, conversions, literal suffixes, or backend-wide overflow policy.

Implemented in the twenty-second Phase 2 slice:

- semantic analysis preflights reachable closed `Int` arithmetic trees made only from
  literal values and arithmetic operators, without introducing name propagation,
  function evaluation, block evaluation, or a general constant folder;
- statically certain signed-64 overflow is rejected as `N3031`, while a statically
  certain division/remainder zero divisor is rejected as `N3032`;
- successful constant arithmetic preserves its original unary/binary HIR so semantic
  validation does not change runtime evaluation shape or execution-step accounting;
- statically unreachable source lowered only for deterministic diagnostics suppresses
  `N3031`/`N3032`, preserving established literal-if, direct-match, short-circuit, and
  post-noncontinuation reachability semantics;
- dynamic operands stop preflight and retain the interpreter's `N4002` overflow and
  `N4003` zero-divisor checks, keeping compile-time and runtime failure boundaries
  independently exercised;
- constant failures become Error-typed through the existing operator fail-closed
  recovery path, so rejected source cannot export reachable flow facts; and
- semantic integration tests plus CLI static/runtime fixtures lock overflow, zero
  divisors, extreme signed edges, non-folding, and dynamic deferral end to end.

Implemented in the twenty-third Phase 2 slice:

- `nova-inspect`, a tooling-owned projection from accepted typed HIR rather than
  a serialization of compiler debug structures;
- `nova inspect <file> --format json`, which reuses the complete check pipeline
  and emits no partial document for rejected source;
- semantic-inspection schema v1 with an explicit family/version envelope and a
  checked-in normative JSON Schema;
- deterministic document-local identities for types, nominal declarations and
  members, functions, bindings, blocks, statements, expressions, matches, and
  arms;
- resolved type, owner, scope, target, record-field input, span, and
  exhaustive-match fact tables with schema documentation and an exact CLI golden
  fixture; and
- fail-closed validation of source spans, identity order, binding ownership,
  lexical visibility and assignment mutability, nominal slots, constructor arity,
  and match coverage before serialization, reported as `N5001` on internal
  inconsistency.

Implemented in the twenty-fourth Phase 2 slice:

- checked signed-64 arithmetic policy moves into a dependency-free
  `nova-int-semantics` leaf crate consumed by both semantic preflight and execution;
- the shared contract uniquely owns overflow, zero-divisor, truncating division,
  signed remainder, and `Int::MIN / -1` / `% -1` behavior plus their truth-table
  tests, eliminating two previously duplicated implementations;
- `nova-sema::constant_int` retains only HIR-closure traversal and operator dispatch,
  while analyzer diagnostics remain separate and the new `nova-inspect` tooling
  boundary continues to consume accepted HIR without owning arithmetic policy;
- `nova-interpreter` retains only runtime value evaluation and N4002/N4003 diagnostic
  mapping instead of carrying a private arithmetic copy;
- the shared crate depends on no parser, HIR, source, diagnostic, inspection, or
  interpreter type, preserving a one-way dependency graph and keeping numeric policy
  representation-independent; and
- structural gates plus the full existing static/runtime and semantic-inspection suites
  verify the refactor is behavior-preserving while removing future drift risk.

Implemented in the twenty-fifth Phase 2 slice:

- semantic reachability gains a pure closed-condition evaluator over already typed HIR,
  recognizing Bool literals, Boolean negation/short-circuiting, and Int/Bool equality
  or Int ordering when all required operands are side-effect-free known values;
- checked literal arithmetic feeding those comparisons reuses the existing constant-Int
  evaluator and shared `nova-int-semantics` policy instead of inventing another numeric
  implementation;
- `if`, `while`, and `&&`/`||` share the derived truth result, extending earlier direct-
  literal flow precision without propagating names or executing calls, blocks, matches,
  field access, or aggregate construction;
- a loop condition proven false lowers its body in diagnostic-only mode under a lexical
  loop context, so dead execution failures such as `1 / 0` do not manufacture N3032
  while `break`/`continue` remain statically legal and ordinary type/name diagnostics run;
- a condition proven true extends guaranteed-loop/noncontinuation reasoning beyond raw
  `true`, and derived short-circuit truths control optional RHS dataflow exactly as the
  interpreter does; and
- semantic regressions plus a CLI check/run fixture lock flow precision, dead-path
  execution-diagnostic suppression, dynamic-boundary conservatism, and HIR non-folding.

Implemented in the twenty-sixth Phase 2 slice:

- successor lowering now treats an already non-continuing (`!`) discriminator as a
  first-class reachability boundary for `if`, `while`, and `match`;
- both `if` branches, a `while` body, and every match arm are still lowered for static
  name/type/pattern/exhaustiveness diagnostics when their predecessor cannot continue,
  but use diagnostic-only state so execution-failure preflight and flow mutations do
  not leak from runtime-impossible successors;
- dead loop bodies retain their lexical loop context, so `break`/`continue` stay legal
  even though their exit facts are discarded along with the unreachable path;
- reachable successors continue to report N3031/N3032 normally, preventing the
  diagnostic-only mechanism from becoming a blanket constant-error suppression rule;
- the change complements closed constant-condition reachability without broadening the
  constant evaluator or changing HIR/schema shape; and
- semantic regressions plus a CLI check/run fixture lock noncontinuing conditions and
  scrutinees, static-diagnostic preservation, runtime return propagation, and reachable
  constant-error behavior end to end.

Implemented in the twenty-seventh Phase 2 slice:

- one verified function-level CFG per HIR function, constructed during lowering so
  recovery-only source remains diagnosable even when invalid executable HIR is
  intentionally discarded;
- deterministic entry, branch, join, binding initialization/read, structured
  transfer, exit, diagnostic-path, and loop-backedge representation, with exact
  source-qualified spans on source-associated events;
- fail-closed graph verification for identity/range, reachability, binding-reference,
  exit, and transfer-successor invariants, with internal diagnostic `N3999`;
- a fixed-point must analysis over predecessor intersections that is now the sole
  producer of definite-initialization diagnostic `N3009`;
- read-only CFG access from semantic analysis without silently changing the separately
  versioned semantic-inspection v1 document;
- unreachable statement/tail lowering unified with diagnostic-only edge semantics,
  preventing unreachable constant arithmetic from becoming an execution failure; and
- unit, integration, structural, branch-intersection, loop-backedge, transfer,
  diagnostic-path, and full regression coverage.

Implemented in the twenty-eighth Phase 2 slice:

- lexical `LocalSymbol` state no longer carries a parallel `initialized` Boolean;
  definite initialization exists only as verified CFG events and fixed-point facts;
- resolved binding reads always retain their declared HIR type, while `N3009` remains
  independently produced by CFG dataflow, allowing orthogonal type diagnostics when
  source violates both typing and initialization rules;
- the obsolete structured `InitializationJoin` lattice and `flow_rules` module are
  removed, and branch/loop/match helpers now merge only CFG continuation cursors;
- initialized parameters, completed declarations, payload bindings, and successful
  assignments emit explicit `Initialize` nodes without mutating lexical symbols;
- a declaration whose initializer is already non-continuing still enters lexical scope
  for deterministic dead-source diagnostics but emits no impossible execution
  initialization after the transfer, eliminating the corresponding N3999 graph error;
- semantic regression tests lock HIR type preservation, independent N3004/N3009
  reporting, and noncontinuing-initializer graph validity; and
- the CFG is now the single source of truth for definite initialization in both accepted
  and recovery analysis, completing the transition introduced by slice twenty-seven.

Implemented in the twenty-ninth Phase 2 slice:

- a `return expression;` emits its own CFG `Return` transfer only when evaluating the
  expression can complete normally;
- if the return expression already transfers control through an inner `return`,
  `break`, or `continue`, the parent statement remains non-continuing in HIR but does
  not append an impossible execution successor after that child transfer;
- nested-return CFGs therefore satisfy the verifier's transfer-successor invariant
  instead of failing closed with internal diagnostic `N3999`;
- a `break` reached while evaluating a return expression continues to target and exit
  the nearest lexical loop rather than being overwritten by the syntactic parent
  return;
- ordinary completed return expressions still emit exactly one `Return` transfer; and
- focused CFG regressions plus the full workspace suite lock child-transfer precedence
  without changing syntax, runtime semantics, or semantic-inspection schema v1.

Implemented in the thirtieth Phase 2 slice:

- rejected `while` conditions continue to retain their body in the CFG for static
  diagnostics and lexical `break`/`continue` checking without treating that body as
  an executable loop iteration;
- neither ordinary body fallthrough nor a `continue` reached only inside such an
  invalid-condition recovery body receives a `Backedge` to the loop header, so
  discarded diagnostic flow cannot reconnect itself to reachable continuation;
- valid dynamic and proven-entered Boolean loops retain their ordinary continue
  backedges, preserving runtime loop semantics and fixed-point graph shape;
- the change tightens the documented fail-closed invalid-control contract without
  changing syntax, HIR, runtime behavior, or semantic-inspection schema v1;
- CFG-shape regressions lock both the rejected-loop isolation rule and the valid-loop
  positive control; and
- the language constitution is synchronized with slice twenty-eight by removing the
  obsolete claim that lexical symbols still carry a parallel initialization bit.

Implemented in the thirty-first Phase 2 slice:

- CFG verification now derives the set of nodes reachable from function entry without
  crossing a `Diagnostic` edge, independently of the structured lowerer's snapshots;
- every executable-reachable node is required to have only non-diagnostic predecessors
  that are themselves executable-reachable, structurally forbidding discarded recovery
  subgraphs from reconnecting to live continuation through joins, exits, or backedges;
- this invariant protects the fixed-point definite-initialization solver, which can
  continue intersecting all recorded predecessors because recovery-only facts cannot
  enter an executable node;
- a direct verifier regression constructs the rejected-loop-style diagnostic branch and
  recovery backedge that slice thirty removed from the analyzer, proving the verifier
  now fails closed even if a future lowering regression recreates that shape;
- a second corruption regression injects a diagnostic predecessor into an otherwise
  executable join, locking the generic invariant rather than only the loop case; and
- existing valid cyclic CFG tests plus the full workspace suite preserve ordinary
  execution/backedge graphs without changing syntax, HIR, runtime semantics, or the
  semantic-inspection schema.

Implemented in the thirty-second Phase 2 slice:

- CFG normal completion is now verified as a structural invariant rather than inferred
  only from the `normal_exits` metadata vector;
- the verifier requires the normal-exit table to equal the graph's `Exit` node set
  exactly, rejecting missing, extra, or duplicate declarations;
- every normal `Exit` must be reachable from function entry without crossing a
  `Diagnostic` edge, so recovery-only source cannot masquerade as successful function
  completion;
- `Exit` nodes are strictly terminal and may not have even diagnostic successors,
  separating source-level `return` recovery from the compiler-generated function-end
  marker;
- direct corruption regressions cover diagnostic-only exits, unlisted exit nodes, and
  post-exit diagnostic successors; and
- existing function completion, divergent-function, CFG isolation, and full workspace
  tests remain green without changing syntax, HIR, runtime semantics, or inspection v1.

Implemented in the thirty-third Phase 2 slice:

- verified CFG binding metadata now has a canonical identity-order invariant rather than
  relying only on the builder's current `BTreeMap` implementation;
- binding identities must be strictly increasing, simultaneously rejecting duplicate and
  out-of-order metadata without assuming function-local identities are contiguous;
- this closes a corruption path where `definite_initialization_diagnostics` could collect
  duplicate identities into a `BTreeMap` and silently select the wrong declaration name or
  span for N3009;
- direct verifier corruption regressions prove both duplicate and out-of-order tables were
  previously accepted and are now rejected; and
- the change affects no syntax, HIR, runtime behavior, dataflow transfer function, or
  semantic-inspection schema.

Implemented in the thirty-fourth Phase 2 slice:

- the verified CFG now requires `graph.entry` to identify the unique `Entry`-kind node,
  making the fixed-point solver's distinguished empty-lattice root a checked invariant;
- a graph whose designated root has another node kind is rejected, as is any graph that
  contains a second `Entry` marker elsewhere in the node table;
- the verifier deliberately does not require the entry to have index zero, avoiding an
  unnecessary coupling between semantic graph meaning and the current builder's numbering;
- direct corruption regressions prove both root-kind mismatch and duplicate Entry markers
  were previously accepted and are now fail-closed; and
- the change affects no syntax, HIR, runtime behavior, dataflow transfer function, or
  semantic-inspection schema.

Implemented in the thirty-fifth Phase 2 slice:

- `Unit` joins `Int` and `Bool` as an equality-comparable bootstrap value type without
  making nominal aggregates or function references comparable;
- semantic `==` / `!=` accepts matching Unit operands, including parameters and call
  results, while preserving existing Never/Error recovery precedence;
- closed-condition reasoning recognizes only literal `()` equality and inequality, so
  known Unit comparisons can refine reachability without treating Unit-returning calls,
  locals, or blocks as compile-time values;
- semantic regressions lock Unit comparison, literal-condition flow, dynamic-call
  conservatism, and continued rejection of record/function equality; and
- no parser, HIR, CFG shape, or semantic-inspection schema change is required.

Implemented in the thirty-sixth Phase 2 slice:

- CFG verification treats `Backedge` topology as an explicit invariant rather than a
  convention of the structured loop builder;
- every backedge must target a `Join` node, matching the current pre-test loop-header
  representation, instead of being accepted on reads, initialization events, branches,
  transfers, or exits;
- both backedge endpoints must belong to executable-reachable control flow, rejecting
  cycles that exist only inside retained diagnostic/recovery source;
- the rule composes with diagnostic-reconnection verification so the fixed-point solver
  consumes only graph cycles that can represent real loop execution;
- direct corruption regressions prove the previous verifier accepted both malformed
  target kinds and diagnostic-only cycles, while existing valid cyclic CFG tests remain
  green; and
- the change affects no syntax, HIR shape, analyzer reachability policy, runtime
  behavior, dataflow transfer function, or semantic-inspection schema.

Implemented in the thirty-seventh Phase 2 slice:

- nominal enums whose every declared variant is payload-free join `Int`, `Bool`, and
  `Unit` as equality-comparable bootstrap value types;
- `==` and `!=` require the exact same enum identity on both operands and compare the
  resolved variant slot rather than variant spelling or declaration shape;
- if any variant carries a payload, the entire enum remains non-comparable in this
  slice, deliberately avoiding recursive payload/aggregate equality semantics;
- closed-condition reasoning may prove equality and inequality for direct payload-free
  enum constructors, extending flow precision without propagating locals or executing
  enum-returning calls;
- differently declared enums, records, functions, and all payload-bearing enums remain
  rejected with the existing type-mismatch diagnostic; and
- semantic and CLI regressions lock nominal identity, payload boundaries, direct-
  constructor reachability, dynamic-call conservatism, and retained HIR shape.

Implemented in the thirty-eighth Phase 2 slice:

- semantic-inspection schema v2 preserves the complete strict v1 program fact table
  and adds an explicitly selected, tooling-owned projection of verified function CFGs;
- graph/function, binding metadata/ownership, graph-local node, predecessor, normal-
  exit, and UTF-8 source-span references are independently checked at the inspection
  boundary, with mismatches failing closed as `N5001` and no partial document;
- stable v2 node categories cover entry, branch, join, initialize, read, structured
  return/break/continue transfers, and normal exit, while edges distinguish execution,
  diagnostic-only, and loop-backedge flow;
- document-local CFG identities and canonical array ordering are specified without
  stabilizing `nova-sema`'s Rust graph representation, a MIR, or backend blocks;
- v1 remains the CLI default and explicit schema version 1 is byte-for-byte identical,
  while `--schema-version 2` is required to select the new contract; and
- separate JSON Schema, golden CLI output, cross-model corruption tests, all node/edge
  category coverage, and documentation lock compatibility and fail-closed behavior.

Implemented in the thirty-ninth Phase 2 slice:

- semantic diagnostic success is severity-aware: errors reject, while warnings remain
  attached to an accepted `AnalysisOutput` without blocking HIR consumers;
- `N3033` reuses verified CFG executable reachability to identify the first diagnostic-
  only source region following an executable `return`, `break`, or `continue`;
- warning roots are span-deduplicated, diagnostic-only transfers cannot cascade nested
  warnings, and any semantic error suppresses the warning pass;
- `nova check`, `nova run`, and `nova inspect` render warnings to stderr in human or
  JSON Lines form while retaining status `0` and their ordinary successful stdout;
- semantic-inspection v1 and v2 continue to accept warning-bearing analysis without
  embedding diagnostic presentation into either schema; and
- semantic, CFG, renderer, and CLI regressions lock severity, spans, deduplication,
  suppression, execution, inspection, and exit-status behavior.


Implemented in the fortieth Phase 2 slice:

- equality and inequality accept function values only when both operands have the same
  fully resolved `FunctionType`; different parameter or return types remain `N3004`;
- equality denotes top-level declaration identity rather than code-address, layout, ABI,
  or structural body equality, keeping the contract independent from future backends;
- the closed-condition evaluator can prove equality/inequality of direct function
  references, including statement-free block wrappers, while local aliases and call
  results remain dynamic and cannot manufacture definite-assignment reachability; and
- semantic regressions lock same-signature acceptance, cross-signature rejection,
  direct-reference flow refinement, and alias conservatism.

Implemented in the forty-first Phase 2 slice:

- bootstrap equality type admissibility is factored into a small public semantic rule
  over resolved HIR types instead of leaving primitive/function/enum classification
  embedded only in analyzer implementation code;
- the shared rule keeps exact-type matching explicit and delegates payload-free enum
  eligibility to declaration context, so consumers cannot infer comparability from one
  runtime variant shape alone;
- semantic analysis continues to own source diagnostics and enum declaration lookup,
  preserving accepted/rejected source behavior while making the equality contract
  reusable at later trusted boundaries; and
- focused truth-table tests lock primitive, Unit, function, record, Never/Error, nominal
  enum, and cross-signature behavior without changing syntax, HIR, CFG, or inspection
  schema shape.

Implemented in the forty-second Phase 2 slice:

- resolved record construction and projection HIR now retain the source-resolved field
  spelling alongside nominal `RecordId` and declaration-order slot identity;
- constructor initializers still preserve written source evaluation order while the
  retained name/slot pair makes same-typed member retargeting observable to later trusted
  consumers instead of relying on type equality alone;
- `nova-inspect` independently validates each retained field name against the referenced
  declaration slot before publishing the existing stable `record:R.field:F` target;
- semantic-inspection v1/v2 schema shape and document IDs remain unchanged because the
  additional spelling is compiler-owned integrity metadata, not a new tooling protocol field;
- direct semantic and inspection corruption regressions lock reversed written initializer
  order, same-typed constructor slot swaps, and same-typed projection retargeting; and
- the slice changes no surface syntax, record layout, field mutability, ownership, ABI, CFG,
  or valid-source behavior.

Implemented in the forty-third Phase 2 slice:

- enum-constructor and exhaustive-match HIR retain the source-resolved variant spelling
  alongside nominal `EnumId` and declaration-order variant slot identity;
- constructor and pattern lowering preserve that name/slot pair without changing source
  syntax, payload evaluation order, match-arm order, or the compact runtime enum value;
- semantic inspection independently rejects variant spelling/slot drift before projecting
  the existing stable variant IDs, leaving schema v1/v2 byte shape unchanged;
- closed-condition proof remains slot-based inside semantic analysis because it consumes
  HIR produced in the same trusted lowering pass rather than treating the spelling as a
  second semantic source of truth; and
- producer plus inspection corruption regressions lock same-shaped sibling variants against
  silent retargeting while keeping nominal enum and pattern semantics unchanged.

The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
- define language-level numeric types, defaulting, conversions, and overflow
  behavior beyond the provisional interpreter contract;
- deepen the pattern model only with a specified usefulness and diagnostic
  strategy rather than adding ad-hoc wildcard or guard behavior;
- specify aggregate mutation/ownership and layout semantics before field mutation
  or ABI claims are added;
- evolve semantic introspection only when implemented modules, effects,
  ownership, or transformations provide real facts to expose; and
- expand negative and adversarial tests as each rule becomes implemented.

Phase 2 is not complete until its implemented type, name, mutation, aggregate,
and dataflow semantics are sufficiently specified for the executable subset and
no roadmap item is being silently approximated.

## Phase 3 — Executable language subset

**Status: eighteen vertical slices implemented; execution surface remains small.**

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

- the interpreter executes explicit Unit literals as the existing `Value::Unit`
  runtime value and empty Unit-returning bodies produce the same value;
- Unit values pass through ordinary function parameters and returns, record slots,
  enum payloads, and match payload bindings without a special runtime channel;
- `nova run` keeps its zero-argument `main -> Int | Bool` entry-point contract, so a
  semantically valid `main() -> Unit` still fails deterministically with `N4001`; and
- CLI end-to-end fixtures prove Unit execution while retaining the existing runtime
  entry-point boundary.

Implemented in the seventh Phase 3 slice:

- bootstrap integer arithmetic policy is centralized in an interpreter-owned pure
  `int_semantics` contract instead of scattering host `checked_*` behavior across
  expression evaluation;
- signed division is explicitly truncation toward zero and signed remainder is tied
  to that quotient, carries the dividend's sign when non-zero, and obeys the usual
  division identity for successful operations;
- zero divisors are a distinct `ZeroDivisor` arithmetic failure mapped to runtime
  `N4003`, while representability failures map to `Overflow` / `N4002`;
- both `Int::MIN / -1` and `Int::MIN % -1` remain deliberate overflow edges rather
  than accidental consequences of Rust's operators;
- truth-table unit tests cover all sign combinations, zero divisors, extreme values,
  the quotient/remainder identity, and checked add/subtract/multiply/negate; and
- CLI fixtures lock negative division/remainder results plus both extreme overflow
  and zero-divisor classes end to end without adding new syntax or numeric types.

Implemented in the eighth Phase 3 slice:

- the interpreter executes equality and inequality for the existing first-class Unit
  runtime value, yielding `true` for Unit equality and `false` for Unit inequality after
  normal left-to-right operand evaluation;
- Unit parameters and call results use the same equality path as literal `()`, with no
  special source-only shortcut;
- record, enum, and function values remain rejected by semantic analysis rather than
  acquiring structural or identity equality accidentally; and
- a CLI check/run fixture locks Unit equality end to end while preserving the existing
  `main -> Int | Bool` entry-point contract.

Implemented in the ninth Phase 3 slice:

- the interpreter executes equality and inequality for semantically accepted
  payload-free enum values after ordinary left-to-right operand evaluation;
- runtime comparison requires the same nominal `EnumId` and compares declaration-order
  variant slots, so same-spelled variants from distinct enum declarations cannot acquire
  accidental structural equality;
- payload-bearing enum values remain outside the semantic operator contract rather than
  triggering recursive runtime comparison;
- direct enum-constructor equality used by semantic reachability and runtime execution
  agrees on variant identity, while enum-returning calls remain dynamically evaluated;
- CLI check/run coverage locks parameter, direct-constructor, equality, and inequality
  behavior end to end; and
- the interpreter's boxed payload representation, enum layout, ownership, and ABI remain
  explicitly provisional and unaffected by this equality slice.

Implemented in the tenth Phase 3 slice:

- zero-argument `main` may return `Unit` alongside the existing `Int` and `Bool`
  bootstrap entry-point types;
- `nova run` prints the existing `Value::Unit` representation as `()` rather than
  rejecting an otherwise semantically valid Unit-valued entry point;
- record-, enum-, and function-valued entry points remain outside the bootstrap
  execution contract and continue to fail with `N4001`; and
- interpreter integration tests plus CLI fixture migration lock both the newly
  accepted Unit entry point and the still-narrow aggregate boundary.

Implemented in the eleventh Phase 3 slice:

- every function call validates runtime arguments against resolved parameter
  types before binding them into a frame;
- every function return validates its runtime value against the declared return
  type before the value crosses the call boundary;
- nominal record and enum validation recursively checks declaration identity,
  record slots, selected variant, and payload shape/type instead of trusting an
  outer runtime tag alone;
- function-value validation checks resolved function identity and signature,
  while `Never` and recovery `Error` can never masquerade as runtime values; and
- malformed-HIR regressions lock argument, return, nested-record, and nominal
  identity drift to deterministic invariant diagnostic `N4005` without changing
  valid source behavior.

Implemented in the twelfth Phase 3 slice:

- record construction validates each evaluated runtime field against the declared
  destination slot type before storing it in the aggregate value;
- enum construction validates an evaluated payload against the selected variant's
  declared payload type before creating the runtime enum value;
- validation reuses the recursive runtime/HIR conformance contract, so nested
  nominal record/enum identity and payload drift also fail closed at construction;
- malformed aggregates are rejected with invariant diagnostic `N4005` even when
  their values are discarded locally and never cross a function boundary; and
- adversarial malformed-HIR regressions plus a valid local-aggregate control case
  lock the new construction boundary without changing accepted source behavior.

Implemented in the thirteenth Phase 3 slice:

- runtime frame entries now retain each resolved binding's type, mutability, and
  initialization state instead of storing only an optional untyped runtime value;
- parameters, ordinary locals, delayed `var` declarations, and match payload
  bindings share one fail-closed slot-construction path that rejects non-conforming
  initial values or incompatible type/mutability reuse of one binding identity while
  allowing repeated execution of the same lexical binding;
- assignment verifies that its resolved target exists, remains mutable, and accepts
  the replacement runtime value under the slot's declared type before mutation;
- binding reads verify both HIR expression-type agreement and recursive runtime-value
  conformance with the slot contract; and
- malformed-HIR regressions cover initializer drift, delayed assignment drift,
  immutable retargeting, incompatible binding-identity aliasing, and match-payload
  binding drift, while valid mutation and loop-local re-entry controls lock accepted
  execution unchanged.

Implemented in the fourteenth Phase 3 slice:

- expression evaluation now has one typed-HIR runtime postcondition around the existing
  per-kind evaluator: every ordinary `Flow::Value` must recursively conform to the
  expression's resolved result type before that value can leave the expression boundary;
- the check applies uniformly to primitive literals, function references, aggregates,
  projections, unary/binary operations, calls, blocks, conditionals, and selected matches,
  including values that are immediately discarded and never reach another runtime boundary;
- structured `Return`, `Break`, and `Continue` flows deliberately bypass the value
  postcondition, preserving the existing propagation semantics for `!` expressions;
- the implementation keeps execution order, step accounting, per-kind invariant checks,
  and valid source behavior unchanged by wrapping rather than rewriting expression logic;
  and
- malformed-HIR regressions cover discarded primitive and composed projection result-type
  drift, while return and loop-transfer controls prove non-value flow remains executable.


Implemented in the fifteenth Phase 3 slice:

- the interpreter executes equality and inequality for first-class top-level function
  values by resolved `FunctionId` declaration identity after ordinary left-to-right
  operand evaluation;
- runtime comparison independently verifies that both referenced declarations exist and
  have identical parameter and return types before comparing identity, so malformed HIR
  with cross-signature function operands fails closed as `N4005`;
- inferred local function aliases use the same runtime identity semantics as direct
  references, while no code address, layout, closure environment, or ABI contract is
  introduced; and
- interpreter malformed-HIR tests plus a CLI check/run fixture lock dynamic alias
  equality, inequality, and signature-drift rejection end to end.

Implemented in the sixteenth Phase 3 slice:

- equality execution validates resolved HIR operand types against the shared semantic
  equality-admissibility rule whenever both operands can complete with ordinary values;
- enum equality rechecks declaration-wide payload freedom from the runtime program table,
  closing a malformed-HIR gap where a currently payload-free variant of a payload-bearing
  enum could previously reach the variant-slot comparison path;
- ordinary expression result conformance still validates each produced value, while the
  new operator precondition protects a distinct invariant: whether those types are legal
  operands for equality at all; `Never` operands deliberately bypass that value-only gate
  so structured return/break/continue propagation remains unchanged;
- function equality retains its independent declaration/signature validation as
  defense-in-depth after the shared type gate; and
- malformed payload-bearing-enum regression coverage plus a valid payload-free enum
  control prove the boundary fails closed with `N4005` without changing valid execution.

Implemented in the seventeenth Phase 3 slice:

- record construction verifies that every value-producing initializer's retained field
  spelling still names its resolved declaration-order destination slot before storage;
- field projection preserves structured noncontinuation from its base, then rechecks the
  retained field spelling, nominal record identity, slot, and declared result type before
  returning an ordinary runtime value;
- malformed HIR that swaps two same-typed constructor slots or retargets a projection to a
  same-typed sibling now fails closed as `N4005` instead of silently changing program meaning;
- the checks complement recursive runtime type conformance rather than duplicating it,
  covering semantic identity drift that remains type-correct at runtime; and
- focused malformed-HIR regressions plus normal record execution keep valid evaluation order,
  runtime representation, and source semantics unchanged.

Implemented in the eighteenth Phase 3 slice:

- enum construction evaluates an optional payload first and only revalidates resolved
  enum/variant name-slot identity when that payload completes with an ordinary value;
- exhaustive matching likewise evaluates the scrutinee before validating the complete arm
  identity/exhaustiveness table, so structured return/break/continue cannot be preempted by
  value-only malformed-HIR checks;
- the interpreter rejects same-payload-type constructor retargeting and exhaustive
  same-shape pattern-slot swaps with `N4005`, while independently preserving payload arity,
  payload type, duplicate-arm, and nominal-enum invariants; and
- adversarial runtime regressions prove both corruption rejection and structured-flow
  precedence without changing valid execution, enum runtime representation, layout, or ABI.

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
