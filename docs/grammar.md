# Nova v0.1 Implemented Frontend Grammar

Status: **normative for the syntax accepted by the bootstrap frontend**

This document describes only implemented syntax. It does not reserve syntax for
planned ownership, effects, concurrency, generics, macros, modules, FFI, or
backends.

## Lexical rules

Source files must be valid UTF-8. The current identifier grammar is deliberately
ASCII-only:

```ebnf
letter          = "A" … "Z" | "a" … "z" ;
digit           = "0" … "9" ;
binary_digit    = "0" | "1" ;
octal_digit     = "0" … "7" ;
hex_digit       = digit | "A" … "F" | "a" … "f" ;
identifier      = (letter | "_") , { letter | digit | "_" } ;
decimal_integer = digit , { [ "_" ] , digit } ;
binary_integer  = ("0b" | "0B") , binary_digit , { [ "_" ] , binary_digit } ;
octal_integer   = ("0o" | "0O") , octal_digit , { [ "_" ] , octal_digit } ;
hex_integer     = ("0x" | "0X") , hex_digit , { [ "_" ] , hex_digit } ;
integer         = decimal_integer | binary_integer | octal_integer | hex_integer ;
```

Keywords are `fn`, `record`, `enum`, `new`, `let`, `var`, `if`, `else`,
`match`, `while`, `break`, `continue`, `return`, `true`, and `false`. A keyword
cannot be used as an identifier.

Integer separators cannot lead, trail, repeat, or immediately follow a radix prefix.
Decimal literals have no prefix; binary, octal, and hexadecimal literals use
`0b`/`0B`, `0o`/`0O`, and `0x`/`0X`. A prefixed literal must use digits valid for
that radix. Lexing erases source radix after decoding and preserves one checked
magnitude, rejecting values above `2^63` without wrapping or truncation. Semantic
lowering interprets that magnitude as signed `Int`: positive literals end at
`2^63 - 1`, while magnitude `2^63` in any supported radix is accepted only under
prefix `-`, giving the exact minimum value `-9223372036854775808`.

Spaces, tabs, carriage returns, and newlines separate tokens. Newline has no
statement-ending meaning. `//` begins a line comment. `/*` and `*/` delimit a
nested block comment. An unterminated block comment is an error.

## Grammar

```ebnf
program             = { declaration } , end_of_file ;
declaration         = record_declaration | enum_declaration | function ;

record_declaration  = "record" , identifier , "{" , [ record_fields ] , "}" ;
record_fields       = record_field , { "," , record_field } , [ "," ] ;
record_field        = identifier , ":" , type_ref ;

enum_declaration    = "enum" , identifier , "{" , enum_variants , "}" ;
enum_variants       = enum_variant , { "," , enum_variant } , [ "," ] ;
enum_variant        = identifier , [ "(" , type_ref , ")" ] ;

function            = "fn" , identifier , "(" , [ parameters ] , ")" ,
                      "->" , type_ref , block ;
parameters          = parameter , { "," , parameter } , [ "," ] ;
parameter           = identifier , ":" , type_ref ;
type_ref            = identifier | "!" | function_type ;
function_type       = "fn" , "(" , [ type_ref_list ] , ")" , "->" , type_ref ;
type_ref_list       = type_ref , { "," , type_ref } , [ "," ] ;

block               = "{" , { statement } , [ expression ] , "}" ;
statement           = binding_statement
                    | uninitialized_var_statement
                    | assignment_statement
                    | while_statement
                    | break_statement
                    | continue_statement
                    | return_statement
                    | expression_statement ;
binding_statement   = ("let" | "var") , identifier ,
                      [ ":" , type_ref ] , "=" , expression , ";" ;
uninitialized_var_statement = "var" , identifier , ":" , type_ref , ";" ;
assignment_statement = identifier , "=" , expression , ";" ;
while_statement     = "while" , expression , block ;
break_statement     = "break" , ";" ;
continue_statement  = "continue" , ";" ;
return_statement    = "return" , expression , ";" ;
expression_statement = expression , ";" ;

expression          = logical_or ;
logical_or          = logical_and , { "||" , logical_and } ;
logical_and         = equality , { "&&" , equality } ;
equality            = comparison , { ("==" | "!=") , comparison } ;
comparison          = additive , { ("<" | "<=" | ">" | ">=") , additive } ;
additive            = multiplicative , { ("+" | "-") , multiplicative } ;
multiplicative      = unary , { ("*" | "/" | "%") , unary } ;
unary               = ("!" | "-") , unary | postfix ;
postfix             = primary , { call_suffix | field_suffix } ;
call_suffix         = "(" , [ arguments ] , ")" ;
field_suffix        = "." , identifier ;
arguments           = expression , { "," , expression } , [ "," ] ;
primary             = integer
                    | "true"
                    | "false"
                    | identifier
                    | record_literal
                    | enum_constructor
                    | unit_literal
                    | "(" , expression , ")"
                    | block
                    | if_expression
                    | match_expression ;
record_literal      = "new" , identifier , "{" , [ record_initializers ] , "}" ;
record_initializers = record_initializer , { "," , record_initializer } , [ "," ] ;
record_initializer  = identifier , ":" , expression ;
unit_literal        = "(" , ")" ;
enum_constructor    = identifier , "::" , identifier ,
                      [ "(" , expression , [ "," ] , ")" ] ;
if_expression       = "if" , expression , block , "else" ,
                      (block | if_expression) ;
match_expression    = "match" , expression , "{" , [ match_arms ] , "}" ;
match_arms          = match_arm , { "," , match_arm } , [ "," ] ;
match_arm           = enum_pattern , "=>" , expression ;
enum_pattern        = identifier , "::" , identifier ,
                      [ "(" , payload_pattern , ")" ] ;
payload_pattern     = identifier | "_" ;
```

A block may end in one expression without a semicolon; that expression is its
tail value. Any earlier expression must end in `;`. Bindings, assignments,
`break`, `continue`, and returns always end in `;`. A `while` statement ends
with its body block and does not take a trailing semicolon. Top-level statements
are not accepted.

`Unit` is a built-in surface type and `()` is its sole literal. A block with no
tail expression also has type `Unit`. A function declared `-> Unit` may therefore
fall through a value-less body; `return ();` is the explicit equivalent. Other
return types still require a compatible tail or an explicit return on every
continuing path. Parenthesized non-empty expressions retain ordinary grouping, so
`(value)` is not a Unit literal.

Function types use the recursive surface form `fn(T1, T2) -> U`; zero parameters and
a trailing comma are allowed, and parameter/return positions may themselves be function
types. The form is accepted anywhere a type reference is accepted, including function
signatures, local annotations, record fields, and enum payloads. This enables named
top-level function values to be passed, returned, stored, and invoked through explicit
signatures. It does not introduce lambdas, closures, captured environments, methods, or
implicit callable coercions. Recursive type parsing has its own finite nesting budget and
reports `N2009` rather than recursing without bound.

`!` is the surface spelling of Nova's uninhabited bottom type. It is accepted anywhere a
type reference is accepted, including nested function signatures. A continuing expression
can never produce a value of type `!`; a call to a declared `fn() -> !` is instead
non-continuing and is compatible with any expected result position. A function declared
`-> !` must itself be proven non-continuing on every reachable path, for example by a
guaranteed loop with no reachable `break`; ordinary fallthrough or a continuing tail is
rejected. `!` introduces no runtime value, layout, allocation, or ABI representation.

Records are nominal top-level types. Each field has an explicit type and field
names must be unique within a record. `new Type { ... }` constructs a record by
naming every declared field exactly once. Initializers may appear in any order;
they are evaluated left to right in the written source order. Semantic analysis
rejects unknown, duplicate, missing, or mistyped fields. `value.field` projects
a field from a value of the matching nominal record type. Projection is
read-only in this slice: assignment targets remain plain local identifiers, not
field accesses. Record equality is not implemented.

The explicit `new` keyword is intentional. A bare `Type { ... }` constructor
would be syntactically ambiguous with the block that follows conditions such as
`if value { ... }` and `while value { ... }`; the bootstrap grammar keeps that
boundary deterministic rather than relying on capitalization or semantic
feedback during parsing.

Enums are nominal top-level types and must declare at least one variant. A
variant carries either no payload or exactly one explicitly typed payload.
`Enum::Variant` constructs a payload-free variant; `Enum::Variant(expression)`
constructs a payload variant. `()` is the Unit literal, so `Enum::Variant(())`
supplies one Unit payload and is valid only for a variant declared with payload
type `Unit`; it is not an alternate spelling for a payload-free constructor.
Declared enum names and record names share one type
namespace, and recursive enum payload types are accepted. This does not define a
stable runtime layout, size, allocation strategy, or ABI for enums.

`match` accepts only qualified enum-variant patterns in this slice. A
payload-carrying variant must bind exactly one immutable name, while a
payload-free variant must not bind one. Every arm is a separate lexical scope.
All variants of the scrutinee's nominal enum must appear exactly once; missing,
duplicate, unknown, or differently qualified variants are errors. Wildcards,
guards, nested patterns, alternatives, literals, and record destructuring are
not accepted. Continuing arm values must have one compatible type; arms that
cannot continue because they return, break, or continue do not constrain that
result type.

The scrutinee is evaluated once before arm selection, and only the selected arm
is evaluated. Definite-assignment state after a valid exhaustive match is the
intersection of every arm that can continue. A non-continuing arm does not
constrain the surviving paths, and an invalid or non-exhaustive match contributes
no initialization evidence.

`var name: Type;` declares a mutable binding whose first value is supplied by a
later assignment. The explicit type is mandatory, while uninitialized `let`
bindings and untyped `var name;` declarations are rejected. Semantic analysis
rejects every read that is not definitely preceded by a type-correct assignment
on every control-flow path that can reach that read.

Assignment is intentionally a statement, not an expression. Its target is one
plain identifier, so chained assignment and assignment inside larger
expressions are not accepted by this grammar. Semantic checking further
requires the target to resolve to a mutable `var` binding and the assigned value
to match that binding's type.

A `while` condition must have type `Bool` and is evaluated before every
iteration. Because the first condition test always occurs but the body may run
zero times, definite-assignment facts created while evaluating the condition may
flow after the loop, while facts established only inside the body do not.
The body block's value, if any, is discarded.

`break;` and `continue;` are legal only inside the body of an enclosing `while`.
The loop condition itself is deliberately outside that control-transfer scope.
`break;` exits the nearest enclosing `while`; `continue;` abandons the remainder
of the current iteration and re-evaluates that same loop's condition. Both are
statement-only, carry no value, and make their current control-flow path
non-continuing for branch and match definite-assignment merging. Unreachable
source after either statement is still analyzed for deterministic diagnostics,
but it cannot manufacture initialization facts on the reachable path. Labelled
loops, value-carrying breaks, and `for` are not implemented.

An `if` is an expression and therefore always has an `else` branch in this
subset. `else if` is represented by the recursive `if_expression` production.
Definite-assignment state is merged across both branches; a branch that cannot
continue because it returns, breaks, or continues does not constrain
initialization on the surviving path.

## Precedence and associativity

From tightest to loosest:

| Level | Forms | Associativity |
|---|---|---|
| 1 | call `f(...)`, field access `value.field` | left/postfix |
| 2 | prefix `!`, `-` | right |
| 3 | `*`, `/`, `%` | left |
| 4 | `+`, `-` | left |
| 5 | `<`, `<=`, `>`, `>=` | left |
| 6 | `==`, `!=` | left |
| 7 | `&&` | left |
| 8 | `||` | left |

The parser enforces finite nesting budgets. Expression recursion emits `N2008`,
and recursive type syntax emits `N2009`, rather than continuing unbounded recursive
descent. These budgets are implementation limits, not promises that pathologically
deep source will remain portable unchanged.

## Deliberate limitations

The implemented grammar has no strings, floating-point literals, arrays,
wildcard or guarded patterns, multi-payload variants, record destructuring,
field assignment, `for`, labelled loops, value-carrying loop control, methods,
modules, imports, generics, traits, effects, async syntax, ownership syntax,
unsafe blocks, or attributes. Encountering such syntax is an error, not an
approximation.
