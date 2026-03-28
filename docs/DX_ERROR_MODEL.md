# DX Error Model

## Status

This document fixes the intended error-handling strategy for `dx`.

It does not yet define all surface syntax for propagation or recovery.
It defines the semantic policy that the compiler and runtime should follow.

## Thesis

`dx` uses a modern, explicit error model:

- `Result[T, E]` for recoverable domain errors
- `!throw` for foreign/runtime exception-style propagation
- `panic` for bugs and invariant violations

This is intentionally not an "exceptions everywhere" design.

## Design Goals

The error model should:

- keep ordinary business/domain failure in the type system
- keep foreign/runtime failure visible in the effect system
- avoid hidden exception paths in normal native code
- preserve a clean boundary for Python interop
- remain straightforward to lower through MIR, runtime ops, and LLVM

## Three Error Classes

### 1. Domain errors

Domain errors are part of the normal return type.

Example:

```dx
fun parse_user(text: Str) -> Result[User, ParseError]:
    ...
.
```

These are:

- expected
- recoverable
- modeled explicitly as values

`dx` should prefer this form for:

- parsing
- validation
- protocol/domain failures
- application-level fallbacks

### 2. Foreign or runtime throws

`!throw` marks computations that may fail by exception-like propagation.

Examples:

```dx
fun load_csv(path: Str) -> PyObj !py !throw:
    read_csv(path)
.
```

```dx
fun call_dynamic(f: PyObj) -> PyObj !py !throw:
    f()
.
```

This class is intended for:

- Python exceptions
- failures in foreign runtimes
- runtime boundary failures that are not naturally typed as `Result`

The rule is:

- if failure is ordinary domain logic, use `Result`
- if failure comes from foreign/runtime exception propagation, use `!throw`

### 3. Panics

`panic` is reserved for bugs and violated compiler/runtime assumptions.

Examples:

- unreachable internal state
- violated invariants
- impossible backend conditions

This is not normal control flow and should not be the public error story of user code.

## Recommended Default Policy

The intended `dx` policy is:

- **Result-first**
- **effect-marked throw**
- **panic only for bugs**

This means:

- native APIs should prefer `Result[T, E]` for recoverable errors
- foreign boundaries may require `!throw`
- `panic` should stay exceptional and non-routine

## Relation Between `Result` and `!throw`

`Result` and `!throw` are deliberately different.

`Result[T, E]` means:

- the function returns an ordinary value that must be handled by the caller

`!throw` means:

- the function may exit abnormally through an exception-style path

These may coexist in one signature when necessary:

```dx
fun decode(path: Str) -> Result[Data, DecodeError] !io !throw:
    ...
.
```

Meaning:

- domain-level decode failure is expressed as `Result`
- separate runtime/foreign failure may still throw

This should be rare in pure native APIs, but is acceptable at boundaries.

## Python Mapping

Python exceptions map to `!throw`.

That means:

- `!py` marks entry into the Python world
- `!throw` marks that Python may propagate an exception back across the boundary

Typical Python-facing signatures therefore look like:

```dx
fun load(path: Str) -> PyObj !py !throw:
    read_csv(path)
.
```

The compiler/runtime should not pretend Python calls are ordinary pure/native calls.

## Native Runtime Mapping

Native runtime code should avoid using `!throw` for ordinary library/domain failure.

Use `!throw` only when there is a real exception-style runtime boundary.

Examples that should generally stay out of `!throw`:

- parse failure
- validation failure
- missing user input
- domain not-found cases

These should prefer:

- `Result[T, E]`
- or `Option[T]`/equivalent when that type exists for non-error absence

## Closure and Effect Interaction

`!throw` is an ordinary effect and composes with closures exactly like `!py` or `!io`.

Examples:

- `lazy read_csv(path)` has type `lazy PyObj !py !throw`
- a closure that calls Python may have type `(Int) -> PyObj !py !throw`

The effect is preserved through:

- typed HIR
- MIR
- runtime ops

It is not erased by `lazy`.

## MIR and Runtime Implications

The backend should distinguish three things:

1. value-level domain failure
   - `Result`
2. foreign/runtime throw path
   - `!throw`
3. fatal bug path
   - `panic`

This implies:

- `Result` lowering is ordinary data lowering
- `!throw` will need an explicit runtime/ABI story
- `panic` can use a separate fatal runtime path

The key rule is:

- do not encode all failure uniformly as one mechanism

## Not Yet Fixed

This document does not yet fix:

- syntax for propagating `Result`
- syntax for catching/handling `!throw`
- whether `dx` exposes `try` syntax in v0.1 or later
- detailed ABI shape of throw paths in LLVM

Those are next-step design tasks.

## Immediate Compiler Direction

The compiler/runtime should proceed assuming:

- `Result` is ordinary typed data
- `!throw` is an explicit effect that survives lowering
- Python boundary ops may eventually grow throw-aware runtime hooks
- `panic` remains separate from both `Result` and `!throw`

## Strategic Rule

Do not let exception-style propagation become the default error model of `dx`.

`dx` should feel like:

- explicit native code with typed failures

not:

- Python exceptions with stronger syntax.
