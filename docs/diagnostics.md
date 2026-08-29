# Bootstrap Diagnostics Contract

This document specifies Nova's implemented diagnostic severity and command-line
behavior. It is a bootstrap tooling contract, not a promise that every current
diagnostic code or warning policy is stable through Nova 1.0.

## Structured model

Every diagnostic carries:

- severity `error` or `warning`;
- a compiler-version-scoped code;
- a summary;
- ordered primary and secondary labels with source-qualified UTF-8 byte spans;
  and
- zero or more notes.

Human rendering and JSON Lines are presentations of the same data. JSON output
uses the lowercase severity spelling and emits one complete object per line.
Diagnostics are ordered by primary source span for deterministic output.

An `error` rejects the requested compile operation. A `warning` describes an
accepted program and does not by itself prevent HIR inspection or execution.
`AnalysisOutput::has_errors()` and `AnalysisOutput::is_success()` therefore test
severity rather than whether the diagnostic vector is empty.

## CLI behavior

Semantic diagnostics are written to standard error before command-specific
output. Their effect is:

| Result | Exit status | `check` | `run` | `inspect` |
| --- | ---: | --- | --- | --- |
| no diagnostics | `0` | no output | value on stdout | document on stdout |
| warnings only | `0` | warnings on stderr | warnings on stderr, value on stdout | warnings on stderr, document on stdout |
| any error | `1` | diagnostics on stderr | no execution | no document |
| invalid command line | `2` | usage error | usage error | usage error |

`--message-format human|json` changes diagnostic presentation only. It does not
change acceptance, exit status, runtime values, or semantic-inspection JSON.

## `N3033` unreachable code

The first implemented warning is deliberately narrow. After a function CFG has
passed verification and the complete semantic analysis contains no errors, the
warning pass computes nodes reachable from the entry without crossing a
`Diagnostic` edge. For each executable `return`, `break`, or `continue` transfer
that has diagnostic-only successors, it reports the earliest successor span:

- primary label: the first source region that cannot execute;
- secondary label: the controlling transfer and why it prevents continuation;
- at most one warning per transfer; and
- exact duplicate primary spans are emitted once.

Source after a transfer remains fully lowered and statically checked. If that
source produces an error, the program is rejected and the warning pass is
suppressed, preventing an unreachable warning from obscuring the real error.

This slice does not warn for constant-selected `if` or `match` branches,
short-circuit operands, a statically skipped `while` body, or code after a proven
nonterminating loop. Those paths may also use diagnostic CFG edges, but expanding
the warning policy requires separate noise, usefulness, and suppression design.

## Deliberate limits

Nova has no warning configuration, lint groups, source attributes, command-line
allow/deny switches, warnings-as-errors mode, cap-lints policy, or cross-package
diagnostic aggregation yet. `N3033` is not a claim that CFG reachability is a
general-purpose linter or that the current warning set is complete.
