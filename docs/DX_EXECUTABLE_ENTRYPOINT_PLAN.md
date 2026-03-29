# DX Executable Entrypoint Plan

## Purpose

This document fixes the current minimal entrypoint contract for building a real
native executable from `dx` source.

It exists because the backend and toolchain path are now mature enough to build
real binaries, but the language still does not have a fully specified
executable-program model.

## Current Reality

With the local LLVM toolchain installed, the current backend can already build
a real executable for a module like:

```dx
fun main() -> Int:
    0
.
```

This succeeds through:

- `dx-build-exec`
- `dx-emit-llvm`
- `llvm-as`
- `llc`
- `cc`

and the resulting program exits with code `0`.

The emitted IR is currently:

```llvm
define i64 @main() {
entry:
  br label %bb0
bb0:
  ret i64 0
}
```

That is enough to prove the first real end-to-end executable path.

There is now also a canonical proof workflow for the currently runnable subset:

```bash
scripts/prove_executable_entry_subset.sh
```

At the moment, that runnable subset is narrower than the full executable-entry
fixture set. Pure `main() -> Int` demos already run end-to-end; executable
fixtures that rely on ordinary closure-call or thunk runtime semantics still
depend on a richer runtime than the current stub.

## Important Constraint

A module like:

```dx
fun main() -> Unit:
    0
.
```

currently emits:

```llvm
define void @main() {
entry:
  br label %bb0
bb0:
  ret void
}
```

This links, but it is **not** a stable executable contract.

On the current platform it can exit with a garbage status code, because the C
runtime expects `main` to return an integer-compatible exit status.

So:

- `main -> Int` is currently acceptable as the minimal executable entrypoint
- `main -> Unit` is not yet a valid stable executable contract

## Minimal Entrypoint Contract

The short-term executable contract should be:

1. the program entrypoint is a top-level function named `main`
2. `main` must currently be zero-argument
3. `main` must currently return `Int`

This is intentionally narrow.

It is enough to unlock:

- the first real executable demos
- toolchain-backed end-to-end verification
- a stable bootstrap for future executable semantics

## Non-Goals For This Phase

This phase should not yet try to solve:

- `argc` / `argv`
- `main -> Unit` implicit exit mapping
- structured process exit APIs
- full runtime startup/shutdown protocol
- richer top-level executable modules

Those are real design questions, but they should come after the first stable
native executable contract is in place.

## Next Step

The next backend-executable milestone should use this narrow contract:

- prove `fun main() -> Int` end-to-end
- add canonical executable-entry demos outside the current backend subset only
  when the workflow around `dx-build-exec` is stable
- distinguish clearly between:
  - executable-entry fixtures that satisfy the entrypoint contract
  - runnable executable-entry fixtures that already execute correctly with the
    current runtime stub
- later decide how to widen entrypoint semantics

## Widening Options Later

Once the first executable contract is stable, the likely next choices are:

1. allow `main() -> Unit` and lower it to a defined integer exit code
2. allow a runtime wrapper around a `dx` entry function instead of exposing
   `@main` directly
3. introduce an explicit executable/module entry concept if the language needs
   it

But none of these should block the current milestone.
