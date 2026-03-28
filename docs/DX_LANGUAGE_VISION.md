# DX Language Vision

## Status

This document fixes the current design direction discussed so far.

It is not a formal spec.
It is a strategic and technical baseline for:

- language identity
- relation with Python
- checker split
- core semantics
- killer app
- near-term roadmap

## Thesis

`dx` is not "Python with a few fixes".

`dx` is a new native language with:

- LLVM compilation
- static types
- explicit effects
- deterministic memory management
- structured concurrency
- no `async`/`await` coloring
- first-class query and transform semantics
- explicit interoperability with the Python ecosystem

Python remains important, but as a foreign ecosystem, not as the semantic foundation of the language.

## Positioning

`dx` should not be positioned as a general-purpose Python replacement.

Its initial niche is:

- typed data applications
- query and transform pipelines
- orchestration
- semantic analytics foundations
- native infrastructure around the Python data ecosystem

Short version:

`dx` aims to be the best language for typed data applications.

## Language Identity

### Core principles

- Small core language
- Regular grammar
- Explicit semantics
- Deterministic runtime costs
- LLM-friendly syntax and tooling
- No source compatibility promise with Python
- Strong interop with Python where useful
- Result-first error handling with explicit `!throw` at foreign/runtime boundaries

### What `dx` keeps from Python

- readable indentation-oriented layout
- familiar control-flow vocabulary
- overall accessibility of expression syntax
- ecosystem access through Python interop

### What `dx` does not inherit from Python

- source compatibility as a hard goal
- implicit mutability as the only model
- `async`/`await` coloring
- Python member access syntax
- Python object model as the native model
- Python runtime as the host runtime

## Syntax Decisions

These are the currently fixed syntax choices.

### Block syntax

- `:` opens a block
- `.` closes a block
- `else:` and `elif:` are part of the same `if`
- no extra block close before `else`

Example:

```dx
fun fact(n: Int) -> Int:
    if n <= 1:
        1
    else:
        n * fact(n - 1)
    .
.
```

### Member access

- `.` is not used for member access
- `'` is the universal member-access operator

Example:

```dx
user'name
user'address'city
orders'filter(pred)
math'max(a, b)
```

Rationale:

- frees `.` for structure
- avoids historical syntax inertia
- reads naturally as possessive access
- is regular across fields, methods, and namespace/module access

### Strings

- strings use `"` only
- `'` is reserved for member access
- no single-quoted strings in the core language

### Member access scope

`'` is used uniformly for:

- field access
- property access
- method access
- module or namespace access

### Receiver keyword

The preferred receiver keyword is:

- `me`

This is preferred over `this` because it carries less historical OOP baggage and fits the language tone better.

Example:

```dx
fun full_name() -> Str:
    me'first + " " + me'last
.
```

### Lambda syntax

Parameterized anonymous functions use `=>`.

Examples:

```dx
x => x + 1
(a, b) => a + b
(x: Int) => x + 1
```

For block-bodied lambdas:

```dx
x =>:
    val gross = x'price * x'qty
    gross - x'discount
.
```

Rules:

- inline lambda bodies do not end with `.`
- block lambda bodies use the same `:` / `.` structure as all other blocks
- single-parameter lambdas may omit parentheses
- typed parameters use parentheses

### Zero-ary closures

Zero-argument deferred computations use `lazy`.

This is the only surface syntax for zero-ary closures.

Examples:

```dx
lazy expensive_message()

lazy:
    val x = compute()
    x + 1
.
```

Conceptually:

- `lazy expr` desugars to a zero-ary closure returning `expr`
- `lazy: ... .` desugars to a zero-ary block closure

Important rule:

- `lazy` defers evaluation
- it does not erase effects

Examples:

- `lazy 1 + 2` has type `() -> Int`
- `lazy read_text(path)` has type `() -> Str !io`
- `lazy py'pandas'read_csv(path)` has type `() -> PyObj !py !throw`

### Anonymous full functions

When a full explicit anonymous function is needed, `fun` is used.

Example:

```dx
fun(x: Int, y: Int) -> Int:
    x + y
.
```

With effects:

```dx
fun(path: Str) -> Str !io:
    read_text(path)
.
```

Design split:

- `=>` for lightweight anonymous functions
- `fun(...) -> ...:` for fully explicit anonymous functions
- `lazy` for zero-ary deferred computations

### Partial application

Partial application uses `_` placeholders.

Examples:

```dx
add(1, _)
_'email
_'price * _'qty
```

Desugarings:

- `add(1, _)` -> `x => add(1, x)`
- `_'email` -> `x => x'email`
- `_'price * _'qty` -> `x => x'price * x'qty`

Rules:

- all `_` occurrences inside one placeholder-lifted expression refer to the same single implicit parameter
- `_` is therefore unary shorthand only
- if more than one parameter is needed, an explicit lambda must be written
- `_1`, `_2`, etc. are not part of the current design
- `_` is a placeholder expression, not a normal identifier

### Parameters, named arguments, and defaults

Named arguments are supported at call sites.

Examples:

```dx
connect(host: "db.local", port: 6432, ssl: false)
plot(x: xs, y: ys, color: "red")
```

Default arguments are declared in function signatures:

```dx
fun connect(host: Str, port: Int = 5432, ssl: Bool = true) -> Conn !io:
    ...
.
```

Rules:

- `:` is used for named arguments at call sites
- `=` is used only in parameter declarations for default values
- positional arguments come before named arguments
- after the first named argument, all following arguments must also be named
- defaulted parameters should stay in the trailing portion of the parameter list in v1

### Keyword-only parameters

Keyword-only parameters are supported using `*` in function signatures.

Example:

```dx
fun connect(host: Str, *, port: Int = 5432, ssl: Bool = true) -> Conn !io:
    ...
.
```

Call:

```dx
connect("db.local", port: 6432, ssl: false)
```

### Variadics

Variadic parameters are supported in a simple form.

Example:

```dx
fun sum(xs: Int...) -> Int:
    ...
.
```

Inside the function, `xs` is treated as a sequence value.

### Trailing closure syntax

If the final argument of a call is a closure, it may be passed in trailing form.

Examples:

```dx
orders'filter:
    x => x'total > 100
.

orders'map:
    x =>:
        val gross = x'price * x'qty
        gross - x'discount
    .
.
```

This is equivalent to passing the final closure argument inside parentheses.

### `it` implicit temporary

`it` is supported as a block-scoped implicit temporary.

Its purpose is:

- reduce boilerplate in local pipelines
- improve token efficiency
- give the language a stronger identity

Example:

```dx
data:
    read_csv("users.csv")
    it'filter(_'active)
    it'map(_'email)
.
```

Rules:

- `it` exists only inside a block
- `it` refers to the most recent non-`Unit` expression result in the current block
- a new non-`Unit` expression replaces the previous `it`
- if the immediately preceding expression has type `Unit`, `it` is not available from that step
- `it` does not cross function boundaries
- `it` does not cross lambda boundaries
- `it` is not assignable

Design constraint:

- `it` is a local pipeline convenience
- it is not a general ambient variable model

## Type System

## Goals

- static type checking
- local inference
- strong explicit interfaces
- predictable diagnostics
- future extension toward shape-aware tensor typing

### Local inference policy

Inference is supported, but only in a disciplined way.

Allowed:

- local variable inference from obvious RHS values
- literal inference
- simple expression inference
- small closure return inference where unambiguous

Required annotations:

- function parameters
- public/top-level function return types
- fields and type declarations
- most Python boundary types unless intentionally `PyObj`

Design rule:

- local inference
- explicit interfaces

### Core types

Initial core types should include:

- `Int`
- `Float`
- `Bool`
- `Str`
- `Unit`
- tuples
- `List[T]`
- `Dict[K, V]`
- algebraic data types
- `Result[A, E]`

### Function and thunk types

Function types are real types in the language.

Examples:

- `(Int) -> Int`
- `(Int, Int) -> Int`
- `() -> Str`
- `() -> Str !io`
- `lazy Str`
- `lazy Str !io`

Important design rule:

- zero-ary closures are written with `lazy`
- zero-ary parameter types may also be written as `lazy T`
- `lazy T` is source sugar for `() -> T`
- `lazy T !effects` is source sugar for `() -> T !effects`
- normalized internal representation still uses ordinary function types

### Tensor direction

The type system should leave room for tensor shape checking.

Planned direction:

1. `Tensor[DType, Rank]`
2. `Tensor[DType, [Dims...]]` with symbolic dimensions
3. richer shape constraints and broadcasting rules later

This is important for future ML applications, but should not explode the v1 core.

## Mutability and Bindings

`val` and `var` are fixed for v0.1.

### Binding forms

- `val` introduces an immutable binding
- `var` introduces a mutable binding
- bare `=` does not declare a new binding

Examples:

```dx
val name = "dx"
var total = 0
total = total + 1
```

### Rules

- local bindings must be introduced with either `val` or `var`
- function parameters are immutable by default
- rebinding with `=` is allowed only for names introduced with `var`
- `val` bindings cannot be rebound
- v0.1 does not include implicit declaration-by-assignment

### V0.1 restriction

Assignment in v0.1 is restricted to rebinding existing `var` names.

Field assignment is not part of v0.1.

## Effects

Effects are part of the core language, not an add-on.

Initial effects:

- `!io`
- `!py`
- `!throw`
- a suspension/concurrency effect such as `!wait` or equivalent

### Effect rules

- effects are explicit in function signatures
- effect propagation is automatic
- no hidden side effects
- pure code and effectful code are clearly separated

Example:

```dx
fun read(path: Str) -> Str !io:
    ...
.

fun load_csv(path: Str) -> PyObj !py !throw:
    ...
.
```

## Concurrency

Structured concurrency belongs in the core design.

The language direction is:

- no user-facing `async`/`await` coloring
- suspension modeled through effects
- structured concurrency as the default execution model
- no orphan task model as the default

This means `dx` should be designed around:

- effect-based suspension
- scoped task lifetimes
- cancellation propagation
- concurrency constructs that lower cleanly to runtime state machines

## Memory Model

## Principles

- no tracing GC
- deterministic destruction
- predictable costs
- future compatibility with more systems-level use cases

### Initial direction

- value types use value/copy semantics
- heap objects are reference-counted by default
- non-atomic RC unless cross-thread/shared semantics require otherwise

### Important constraints

The core must not lock the language into "everything is shared refcounted forever".

The design must leave room for:

- uniqueness-based optimization
- RC elimination
- escape analysis
- arenas/regions
- future lower-level ownership models

### Systems-language compatibility rule

Even if `dx` is not initially positioned as a systems language, the core must avoid decisions that would permanently block that path.

That means:

- clear type layout
- explicit runtime costs
- no pervasive hidden allocations
- future `unsafe` model possible
- future C ABI and FFI stability possible

## Python Interoperability

Python interop is first-class, but explicit.

### Design rule

- CPython is not the semantic host of `dx`
- Python is a privileged foreign ecosystem

### Core v1 interop

Must exist in the core:

- `!py`
- `PyObj`
- `from py ... import ...`
- calls from `dx` into Python
- limited automatic conversions for primitive values

Example:

```dx
from py pandas import read_csv

fun load(path: Str) -> PyObj !py !throw:
    read_csv(path)
.
```

### Conversion policy

Initial automatic conversions only for a narrow set:

- `Int <-> int`
- `Float <-> float`
- `Bool <-> bool`
- `Str <-> str`

Fallback:

- `PyObj`

### What is not in the core v1

- full `python -> dx` interop/export
- Python as host runtime
- full bidirectional object-model interop
- full Python source compatibility

`python -> dx` should come later, after:

- core language stability
- runtime stability
- ABI clarity
- `dx -> py` boundary already working

## Wrappers Strategy

The preferred ecosystem strategy is official wrappers rather than semantic dependence on Python modules.

Examples:

- `dx.numpy`
- `dx.torch`
- `dx.jax`
- `dx.arrow`
- `dx.sql`

These wrappers can initially use Python-backed implementations and later evolve toward more native backends while keeping the `dx` API stable.

## Query and Data Model

This is the chosen killer-app direction for the language core.

### Core killer app

`dx` should be the best language for:

- typed cross-source queries
- typed transforms
- orchestration of data pipelines
- pushdown-aware execution
- mixed remote/local execution
- explainable plans

### Core requirement

Queries must be composable and queryable.

That means:

- queries can query other queries
- queries can run over in-memory collections
- queries can run over arrays and object lists
- queries can run over remote sources
- the same surface query language should work across all of them

### Required conceptual model

At minimum, the design should support concepts like:

- `Queryable[T]`
- `Seq[T]` or in-memory enumerable collections
- materialized results
- optional columnar/tabular representations

### Query execution model

The planner decides among:

- full pushdown
- local execution
- mixed execution

### Explain

`explain` is not optional decoration.
It is part of the product thesis.

It should show:

- which fragments are pushed down
- which fragments execute locally
- where materialization happens
- which clauses break pushdown

## BI / Semantic Layer Direction

This is not v1 core, but it remains part of the long-term platform direction.

Planned layers:

- LINQ-like query syntax for developers
- PowerQuery-like transform layer
- semantic tabular model
- DAX-like measure layer
- Perspective-based analytics UI

Important rule:

- multiple surface languages
- one engine underneath

There must not be separate disconnected engines for LINQ, PowerQuery, and measures.

## AD and Probabilistic Programming Direction

`dx` should intentionally preserve a path toward:

- automatic differentiation as an effect-oriented subsystem
- probabilistic programming via effect interpretation

This direction comes from proven `dx-03` experiments and should be treated as a deliberate future axis, not an accidental extra.

See:

- `DX_AD_PPL_DIRECTION.md`

## LLM-Friendliness

This is a first-class design goal.

`dx` should optimize for:

- regular syntax
- one canonical way to express things
- explicit semantics
- stable AST shapes
- formatter-first workflow
- excellent diagnostics
- machine-readable compiler output

Guiding principle:

`dx` should be clear, not clever.

## Checker Strategy

The current Python-oriented checker should not be merged semantically with the future `dx` checker.

### Required split

- `py-checker`
- `dx-checker`

### `py-checker` role

The current checker remains valuable for:

- Python interop analysis
- wrapper generation
- migration tooling
- Python API extraction
- Python semantic bridge work

### `dx-checker` role

A new checker should be built for the actual `dx` language semantics:

- `:` / `.`
- `'`
- effect system
- `val` / `var`
- deterministic native semantics

### Shared infrastructure

These can be shared across both:

- diagnostics
- symbol machinery
- type-interning infrastructure
- some generic constraint machinery
- benchmarking harnesses

But the parsers, ASTs, and language rules must remain separate.

## Migration Strategy

Source compatibility with Python is not the main strategy.

The better strategy is:

- explicit `dx` language
- explicit `py` world
- migration tooling between them

This suggests a future source-to-source translator:

- Python -> `dx`

Its role would be:

- onboarding
- partial migration
- making `val` / `var` and type information explicit
- isolating code that must remain in `py`

## Systems-Language Compatibility

`dx` should not be marketed initially as a systems language.

However, the core should avoid decisions that permanently block that future.

Initial practical position:

- strong native application language
- infrastructure language
- data/query/orchestration language

Possible future expansion:

- stronger FFI
- `unsafe`
- ABI commitments
- lower-level memory control
- systems-grade runtime and library capabilities

## Growth Path

The strategic goal is not to keep `dx` permanently narrow.

The correct sequencing is:

1. win in a sharp initial niche
2. expand into a broader native application language
3. later grow into a more general-purpose language
4. only then push further toward stronger systems-language capabilities

### Phase 1

Initial identity:

- typed data applications
- query
- transform pipelines
- orchestration
- native infrastructure around the Python data ecosystem

### Phase 2

Broader native application language:

- backend services
- CLI tools
- developer tooling
- infrastructure software
- data-intensive application backends

### Phase 3

More general-purpose language:

- wider application programming
- broader library surface
- richer abstractions beyond the initial killer app

### Phase 4

Stronger systems-language direction:

- stronger FFI
- `unsafe`
- stable ABI expectations
- lower-level memory control
- more explicit layout and allocator controls

### Strategic rule

The v1 should stay focused, but the core should be designed so that later expansion remains possible.

That means:

- do not pitch everything at once
- do not design the core so narrowly that later expansion is blocked
- do not make Python interop central enough to prevent a stronger native identity

## V1 Scope

The v1 language should stay small.

### In scope

- core syntax
- functions
- `if` / `else`
- `match`
- types and ADTs
- explicit effects
- `dx -> py` interop
- LLVM backend

### Out of scope

- full Python source compatibility
- full `py -> dx` runtime interop
- dedicated `scope:` syntax
- field assignment
- query syntax in the initial parser/checker bootstrap
- full DAX engine
- full PowerBI-like product
- deep JAX/PyTorch semantics
- macro system
- general-purpose language maximalism

### Product note

Query and transform remain the killer-app direction, but the language bootstrap should first ship a stable non-query core.

That means:

- query syntax and planning can arrive immediately after the core parser/checker
- they do not need to be part of the first minimal language implementation

## Immediate Next Steps

1. Freeze a proper checker split:
   - `py-checker`
   - `dx-checker`

2. Write a minimal formal `dx` core spec:
   - grammar
   - names
   - blocks
   - member access
   - function signatures
   - effect annotations

3. Build a `dx` parser and AST with no Python compromises.

4. Build the minimum `dx` checker:
   - names
   - local bindings
   - function typing
   - basic effects
   - `PyObj` boundary

5. Build the query IR and a minimal planner.

6. Add one local backend and one remote backend.

7. Add `explain`.

## Final Direction

`dx` should become:

- a small native language
- strongly typed
- effect-aware
- deterministically managed
- query-native
- structured-concurrent
- Python-interoperable at explicit boundaries
- designed for both humans and LLMs

This is the design center.
It is intentionally not "Python, but stricter".

## Implementation Follow-Up

The concrete near-term build order is tracked in:

- `DX_IMPLEMENTATION_ROADMAP.md`
