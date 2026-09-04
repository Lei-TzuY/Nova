# Generic Functions

Nova's bootstrap generics are function-scoped and erased before runtime HIR execution.

A function declares one or more type parameters after its name:

```nova
fn identity<T>(value: T) -> T { value }
```

Direct named calls support both inference and explicit type arguments:

```nova
identity(42)
identity<Int>(42)
```

Explicit type arguments use `<Type, ...>` immediately before the value argument list. The parser recognizes that form only for a direct named call followed by `(`; otherwise `<` keeps its ordinary comparison meaning.

The number of explicit type arguments must equal the callee's declared type-parameter count (`N3039`). Explicit substitutions participate in the same argument checking as inferred substitutions. If a value argument contradicts an explicit substitution, Nova reports `N3037`. A type parameter that cannot be inferred still reports `N3038`, but an explicit argument may satisfy it, including a parameter unused by value arguments.

Generic calls currently target direct named top-level functions. Generic closures, generic nominal types, bounds/traits, specialization, higher-rank polymorphism, and partial type-argument lists are not implemented.

Type arguments are semantic information and do not add a runtime call payload. The interpreter continues to execute concrete typed HIR. Existing frozen semantic-inspection schemas remain fail-closed when a generic function definition contains type parameters they cannot represent.
