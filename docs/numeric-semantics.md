# Nova numeric semantics

Status: **normative for the implemented bootstrap `Int` behavior**

Nova currently has one numeric type, `Int`. It is a signed 64-bit integer with the
closed range `-9223372036854775808` through `9223372036854775807`. This document
promotes that executable behavior from an interpreter accident to the language-level
contract for the current single-`Int` subset; it does not reserve the design of future
numeric families, implicit conversions, suffixes, or backend ABIs.

## Literals and boundaries

Decimal, binary, octal, and hexadecimal integer literals decode to one radix-independent
magnitude. Positive literals are limited to `Int::MAX`. Magnitude `2^63` is accepted only
as the operand of prefix negation, producing `Int::MIN`; larger magnitudes are rejected.

Two payload-free associated constants expose the exact language boundaries:

- `Int::MIN` is `-9223372036854775808`;
- `Int::MAX` is `9223372036854775807`.

`Int` is a reserved primitive type, so these names cannot collide with a user-defined
nominal type. Other payload-free `Int::member` spellings and payload-bearing boundary
forms such as `Int::MAX(1)` are not numeric constants and remain rejected by ordinary
semantic resolution. The parser keeps qualified syntax generic; semantic analysis
canonicalizes only the implemented built-in spellings before ordinary name/type
resolution.

## Arithmetic

Unary `-` and binary `+`, `-`, `*`, `/`, and `%` operate on `Int`. Arithmetic is checked:
a mathematically unrepresentable result is an error rather than wrapping, saturating, or
silently widening. Division truncates toward zero. A non-zero remainder has the dividend's
sign and satisfies `a == (a / b) * b + (a % b)` for representable, non-zero-divisor
operations.

Division or remainder by zero is an error. `Int::MIN / -1` and `Int::MIN % -1` are
overflow because the exact quotient is outside the `Int` range. Semantic constant
preflight and runtime execution share the same `nova-int-semantics` truth table so the
same operation cannot be accepted at compile time and fail differently at runtime.

## Comparison and conversions

`Int` supports `==`, `!=`, `<`, `<=`, `>`, and `>=`.

The bootstrap language has one explicit conversion into `Int`:

- `Int::from(false)` evaluates to `0`;
- `Int::from(true)` evaluates to `1`.

The conversion operand must have type `Bool`, is evaluated exactly once, and retains the
ordinary control-flow and side-effect semantics of any other expression. Semantic
canonicalization lowers the conversion to ordinary typed conditional HIR, so the
interpreter does not carry a parallel conversion opcode or a second source of truth.
A missing payload or a non-`Bool` payload is rejected by normal semantic diagnostics.
There are no implicit conversions.

Literal suffixes, floating-point semantics, additional numeric families,
widening/narrowing policy, and backend representation beyond this signed-64 contract
remain future BIL-5 work and must be specified before syntax is added for them.
