# Nova v0.1 Implemented Frontend Grammar

Status: **normative for the syntax accepted by the Phase 1 frontend**

This document describes only implemented syntax. It does not reserve syntax for
planned ownership, effects, concurrency, generics, macros, modules, FFI, or
backends.

## Lexical rules

Source files must be valid UTF-8. The current identifier grammar is deliberately
ASCII-only:

```ebnf
letter          = "A" … "Z" | "a" … "z" ;
digit           = "0" … "9" ;
identifier      = (letter | "_") , { letter | digit | "_" } ;
integer         = digit , { [ "_" ] , digit } ;
```

Keywords are `fn`, `let`, `var`, `if`, `else`, `return`, `true`, and `false`.
A keyword cannot be used as an identifier.

Integer separators cannot lead, trail, or repeat. The frontend checks decimal
conversion and rejects magnitudes above `9223372036854775807`; it never wraps or
truncates a literal.

Spaces, tabs, carriage returns, and newlines separate tokens. Newline has no
statement-ending meaning. `//` begins a line comment. `/*` and `*/` delimit a
nested block comment. An unterminated block comment is an error.

## Grammar

```ebnf
program             = { function } , end_of_file ;

function            = "fn" , identifier , "(" , [ parameters ] , ")" ,
                      "->" , type_name , block ;
parameters          = parameter , { "," , parameter } , [ "," ] ;
parameter           = identifier , ":" , type_name ;
type_name           = identifier ;

block               = "{" , { statement } , [ expression ] , "}" ;
statement           = binding_statement
                    | return_statement
                    | expression_statement ;
binding_statement   = ("let" | "var") , identifier ,
                      [ ":" , type_name ] , "=" , expression , ";" ;
return_statement    = "return" , expression , ";" ;
expression_statement = expression , ";" ;

expression          = logical_or ;
logical_or          = logical_and , { "||" , logical_and } ;
logical_and         = equality , { "&&" , equality } ;
equality            = comparison , { ("==" | "!=") , comparison } ;
comparison          = additive , { ("<" | "<=" | ">" | ">=") , additive } ;
additive            = multiplicative , { ("+" | "-") , multiplicative } ;
multiplicative      = unary , { ("*" | "/" | "%") , unary } ;
unary               = ("!" | "-") , unary | call ;
call                = primary , { "(" , [ arguments ] , ")" } ;
arguments           = expression , { "," , expression } , [ "," ] ;
primary             = integer
                    | "true"
                    | "false"
                    | identifier
                    | "(" , expression , ")"
                    | block
                    | if_expression ;
if_expression       = "if" , expression , block , "else" ,
                      (block | if_expression) ;
```

A block may end in one expression without a semicolon; that expression is its
tail value. Any earlier expression must end in `;`. Bindings and returns always
end in `;`. Top-level statements are not accepted.

An `if` is an expression and therefore always has an `else` branch in this
subset. `else if` is represented by the recursive `if_expression` production.

## Precedence and associativity

From tightest to loosest:

| Level | Forms | Associativity |
|---|---|---|
| 1 | call `f(...)` | left/postfix |
| 2 | prefix `!`, `-` | right |
| 3 | `*`, `/`, `%` | left |
| 4 | `+`, `-` | left |
| 5 | `<`, `<=`, `>`, `>=` | left |
| 6 | `==`, `!=` | left |
| 7 | `&&` | left |
| 8 | `||` | left |

The parser enforces a finite nesting budget and emits diagnostic `N2008` rather
than continuing unbounded recursive descent. This budget is an implementation
limit, not a promise that deeply nested source will remain portable unchanged.

## Deliberate limitations

The implemented grammar has no strings, floating-point literals, arrays,
records, enums, pattern matching, assignment, loops, methods, modules, imports,
generics, traits, effects, async syntax, ownership syntax, unsafe blocks, or
attributes. Encountering such syntax is an error, not an approximation.
