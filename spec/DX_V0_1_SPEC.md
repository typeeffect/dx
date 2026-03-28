# DX v0.1 Spec Draft

## Scope

This document defines the current v0.1 language surface that is stable enough to implement.

It intentionally excludes:

- query IR details
- planner semantics
- full module system
- trait/interface system
- advanced tensor typing
- Python-to-DX export

## 1. Lexical Conventions

### 1.1 Significant structural tokens

- `:` opens a block
- `.` closes a block
- `'` is member access
- `=>` introduces a parameterized lambda
- `->` introduces a result type
- `!` introduces effect annotations

### 1.2 Strings

- strings use `"` only
- single-quoted strings do not exist in v0.1

### 1.3 Reserved keywords

Current reserved words:

- `fun`
- `if`
- `elif`
- `else`
- `type`
- `lazy`
- `val`
- `var`
- `match`
- `from`
- `import`
- `py`
- `me`
- `it`

## 2. Core Grammar

This grammar is intentionally partial and operational rather than fully formal.

### 2.1 Program

```ebnf
program        ::= item*
item           ::= fun_decl | type_decl | stmt
```

### 2.2 Function declarations

```ebnf
fun_decl       ::= "fun" ident "(" param_list? ")" return_ann? effect_ann_list? ":" block "."
return_ann     ::= "->" type_expr
effect_ann_list ::= effect_ann+
effect_ann     ::= "!" ident
```

### 2.3 Anonymous functions

```ebnf
anon_fun_expr  ::= "fun" "(" param_list? ")" return_ann? effect_ann_list? ":" block "."
```

### 2.4 Parameters

```ebnf
param_list     ::= param ("," param)*
param          ::= star_param | normal_param | variadic_param
normal_param   ::= ident ":" type_expr default_value?
default_value  ::= "=" expr
variadic_param ::= ident ":" type_expr "..."
star_param     ::= "*"
```

Notes:

- parameters after `*` are keyword-only
- defaulted parameters should remain in the trailing region of the parameter list in v0.1

### 2.5 Blocks

```ebnf
block          ::= stmt*
```

Operationally:

- `:` starts a block
- `.` terminates it

### 2.6 Conditional expressions

```ebnf
if_expr        ::= "if" expr ":" block elif_clause* else_clause? "."
elif_clause    ::= "elif" expr ":" block
else_clause    ::= "else" ":" block
```

There is only one closing `.` for the whole `if` expression.

### 2.7 Expressions

```ebnf
expr           ::= lambda_expr
                 | lazy_expr
                 | if_expr
                 | assign_expr
```

The final expression grammar will require an operator-precedence table. For v0.1 implementation, the important stable constructs are:

- function call
- member access
- lambda
- lazy thunk
- placeholder expressions

### 2.8 Lambda expressions

```ebnf
lambda_expr    ::= lambda_params "=>" expr
                 | lambda_params "=>" ":" block "."

lambda_params  ::= ident
                 | "(" typed_or_untyped_params? ")"
```

Examples:

```dx
x => x + 1
(a, b) => a + b
(x: Int) => x + 1
x =>:
    val y = x * 2
    y + 1
.
```

### 2.9 Zero-ary thunks

```ebnf
lazy_expr      ::= "lazy" expr
                 | "lazy" ":" block "."
```

This is the only source syntax for zero-ary closures in v0.1.

### 2.10 Member access and calls

```ebnf
postfix_expr   ::= primary_expr (member_access | call_suffix)*
member_access  ::= "'" ident
call_suffix    ::= "(" arg_list? ")"
                 | ":" block "."
```

Trailing closure syntax is permitted only when the final argument position expects a closure.

### 2.11 Call arguments

```ebnf
arg_list       ::= arg ("," arg)*
arg            ::= expr
                 | ident ":" expr
```

Rules:

- positional arguments must precede named arguments
- after the first named argument, all remaining arguments must be named

### 2.12 Placeholder expressions

```ebnf
placeholder_expr ::= "_"
```

`_` is not a normal identifier.
It is a unary placeholder node that can trigger implicit lambda lifting.

### 2.13 Match expressions

```ebnf
match_expr     ::= "match" expr ":" match_arm+ "."
match_arm      ::= pattern ":" block
pattern        ::= ident
                 | ident "(" pattern_list? ")"
                 | "_"
pattern_list   ::= pattern ("," pattern)*
```

## 3. Surface Desugarings

### 3.1 `lazy`

```dx
lazy expr
```

desugars conceptually to a zero-ary closure returning `expr`.

```dx
lazy:
    stmt1
    expr
.
```

desugars conceptually to a zero-ary block closure.

### 3.2 Placeholder lifting

Examples:

```dx
add(1, _)
```

desugars to:

```dx
x => add(1, x)
```

```dx
_'email
```

desugars to:

```dx
x => x'email
```

```dx
_'price * _'qty
```

desugars to:

```dx
x => x'price * x'qty
```

Binding rule:

- all `_` occurrences in one placeholder-lifted expression refer to the same single implicit parameter
- `_` is therefore shorthand for unary lambdas only
- multi-parameter placeholder shorthand is not part of v0.1

### 3.3 Trailing closures

```dx
orders'filter:
    x => x'total > 100
.
```

desugars to:

```dx
orders'filter(x => x'total > 100)
```

## 4. Types

### 4.1 Core built-in types

- `Int`
- `Float`
- `Bool`
- `Str`
- `Unit`
- tuples
- `List[T]`
- `Dict[K, V]`
- `Result[A, E]`
- `PyObj`

### 4.2 Function types

```text
(A) -> B
(A, B) -> C
() -> T
() -> T !io
lazy T
lazy T !io
```

Notes:

- zero-ary closures are typed as ordinary function types
- `lazy T` is source-level shorthand for `() -> T`
- `lazy T !effects` is source-level shorthand for `() -> T !effects`
- normalized internal representation still uses ordinary function types

### 4.3 Type annotations

Required in v0.1:

- function parameters
- top-level function return types
- declared fields
- explicit public APIs

Inference allowed in v0.1:

- local bindings
- obvious literal expressions
- simple closure bodies when context is available

## 5. Bindings

Bindings are explicit in v0.1.

### 5.1 Binding introduction

- `val name = expr` introduces an immutable binding
- `var name = expr` introduces a mutable binding

Bare assignment does not introduce a new binding.

### 5.2 Rebinding

`name = expr` is allowed only if:

- `name` is already in scope
- `name` was introduced with `var`

### 5.3 Parameters

Function parameters are immutable bindings.

### 5.4 V0.1 assignment restriction

V0.1 supports rebinding of `var` names, but does not include field assignment.

## 6. Effects

### 6.1 Core effect names

v0.1 recognizes at least:

- `!io`
- `!py`
- `!throw`
- `!wait` or equivalent suspension effect

### 6.2 Effect placement

Effects appear after the return type:

```dx
fun read(path: Str) -> Str !io:
    ...
.
```

### 6.3 Effect meaning

- `!io`: external I/O interaction
- `!py`: crossing into the Python foreign world
- `!throw`: may fail via exception-like propagation
- `!wait`: may suspend / participate in structured concurrency

Error-model policy:

- `Result[T, E]` is the preferred mechanism for recoverable domain errors
- `!throw` is reserved for foreign/runtime exception-style propagation
- `panic` is reserved for bugs and invariant violations

### 6.4 Effect propagation

Operational rule:

- if an expression invokes a function with effect `!e`, the enclosing function must also admit `!e`, unless the effect is handled by a dedicated construct

### 6.5 `lazy` and effects

`lazy` delays execution but does not erase effects.

Examples:

- `lazy 1 + 2 : () -> Int`
- `lazy read_text(path) : () -> Str !io`
- `lazy py'pandas'read_csv(path) : () -> PyObj !py !throw`
- `msg: lazy Str !io` is surface shorthand for `msg: () -> Str !io`

## 7. Python Interop

### 7.1 Imports

```dx
from py pandas import read_csv
from py builtins import print
```

### 7.2 Foreign values

`PyObj` is the v0.1 foreign object type.

### 7.3 Core rule

Crossing into Python requires `!py`.

Example:

```dx
fun load(path: Str) -> PyObj !py !throw:
    read_csv(path)
.
```

### 7.4 Primitive conversions

Initial automatic conversions are intentionally narrow:

- `Int <-> int`
- `Float <-> float`
- `Bool <-> bool`
- `Str <-> str`

Everything else falls back to `PyObj` unless wrapped explicitly.

## 8. `it`

`it` is a block-scoped implicit temporary.

### Rules

- `it` exists only inside the current block
- it refers to the most recent non-`Unit` expression result in that block
- a later non-`Unit` expression replaces it
- if the immediately preceding expression has type `Unit`, there is no fresh `it` from that step
- `it` does not cross function or lambda boundaries
- `it` is not assignable

## 9. Type Checking Rules

This section is intentionally operational rather than proof-oriented.

### 9.1 Function declarations

To type-check:

```dx
fun f(p1: T1, ..., pn: Tn) -> R !e1 ... !ek:
    body
.
```

the checker must verify:

1. parameters are well-typed
2. body type is assignable to `R`
3. body effects are a subset of the declared effect set

### 9.2 Lambdas

For:

```dx
x => expr
```

the checker:

1. introduces a fresh parameter type from context, or a fresh inference variable if context exists to solve it
2. checks `expr`
3. returns a function type with inferred result and effects

Typed lambdas:

```dx
(x: Int) => expr
```

use `Int` directly rather than contextual parameter inference.

### 9.3 `lazy`

For:

```dx
lazy expr
```

the checker:

1. checks `expr`
2. forms the type `() -> T !effects(expr)`

For source-level type annotations:

```dx
msg: lazy Str !io
```

the checker:

1. parses the surface type as a lazy zero-ary thunk type
2. normalizes it to `() -> Str !io` before ordinary type checking

### 9.4 Placeholder lifting

When `_` appears in an expression:

1. the expression is rewritten into an explicit lambda before normal type checking
2. all `_` occurrences in that expression are replaced by the same fresh parameter
3. the resulting expression is a unary lambda

### 9.5 Calls

To type-check a call:

1. resolve positional and named arguments against the callee parameter list
2. apply default values for omitted defaulted parameters
3. verify keyword-only restrictions
4. verify variadic packing if present
5. accumulate callee effects into the call expression effects

### 9.6 Member access

For:

```dx
obj'name
```

the checker resolves `name` according to the type of `obj`.

The same operator is used uniformly for:

- data fields
- methods
- module exports
- namespace access

### 9.7 Assignment

For:

```dx
name = expr
```

the checker must verify:

1. `name` resolves to an existing binding
2. that binding was introduced as `var`
3. `expr` is assignable to the binding type

Assignments to member access expressions are not part of v0.1.

## 10. Non-Goals for v0.1

- full Python compatibility
- full Python-to-DX interop
- dedicated `scope:` syntax
- field assignment
- advanced trait system
- advanced tensor shape solver
- full query semantics in the core parser spec
- macro system
- proof of full soundness

## 11. Implementation Targets

The minimum implementation target for this spec is:

1. parser
2. AST
3. basic type checker
4. effect checker
5. Python foreign boundary
6. enough lowering to support further query and runtime work
