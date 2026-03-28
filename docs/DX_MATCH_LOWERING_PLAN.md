# DX Match Lowering Plan

## Purpose

This document fixes where `match` should stop existing as a first-class backend
construct.

The goal is to keep `dx-llvm-ir` simple and honest:

- real emitter translates already-decided low-level control flow
- it does not become the place where pattern semantics are first implemented

## Decision

`match` must be lowered **before** `dx-llvm-ir`.

The correct boundary is:

- `dx-llvm-ir` supports:
  - `ret`
  - `br`
  - `condbr`
  - `unreachable`
- `dx-llvm-ir` does **not** support `MatchBr`

So the lowering of `MatchBr` must happen in `dx-llvm`, not in the real textual
emitter.

## Why This Layer

This is the right place because `dx-llvm` already owns:

- explicit blocks
- explicit operands
- validator rules
- backend control-flow invariants

It is late enough that:

- pattern decisions are already semantically known

But still early enough that:

- `dx-llvm-ir` can remain a mechanical emitter

## Scope

This task is not about extending pattern semantics.

It only covers the currently supported `match` subset already parsed and typed:

- name pattern
- wildcard `_`
- constructor patterns already represented in current lowering

No new surface syntax or new pattern semantics should be introduced.

## Lowering Rule

`MatchBr` must lower into ordinary blocks and branches.

Conceptually:

1. evaluate or use the existing scrutinee operand
2. produce the branch structure needed for the already-known arms
3. jump to the correct arm body block
4. jump to the fallback block if no arm matches

The result after `dx-llvm` lowering must be only:

- `Br`
- `CondBr`
- `Ret`
- `Unreachable`

If the current backend model needs a temporary internal helper structure inside
`dx-llvm`, that is acceptable, but `dx-llvm-ir` must never see it.

## Preferred Strategy

Prefer the simplest lowering that is faithful to current semantics:

- for the currently minimal subset, a cascade of comparisons and branches is fine
- do not force a fake LLVM `switch` if the pattern model is not switch-like

In other words:

- correctness first
- branch form first
- optimization later

## Design Rules

1. No new pattern semantics.
2. No backend-only reinterpretation of constructor patterns.
3. No support for richer pattern matching than the frontend already guarantees.
4. No emission of fake or misleading LLVM IR from `dx-llvm-ir`.

## Layer Responsibilities

### `dx-mir`

No redesign required.

`MIR` may keep explicit `match` structure if that remains useful.

### `dx-codegen`

May continue to preserve `Match` if helpful for clarity.

No semantic redesign required here either.

### `dx-llvm`

This is the implementation point for the lowering decision.

It must:

- remove `MatchBr` before the module reaches `dx-llvm-ir`
- emit only ordinary control-flow terminators afterward
- keep validator coverage aligned with the transformed form

### `dx-llvm-ir`

Must continue to reject `MatchBr` if it ever appears.

That rejection stays useful as a guardrail.

## Tests Required

Minimum coverage after implementation:

- simple wildcard-vs-name match lowered to branch-only LLVM-like control flow
- constructor match lowered without leaving `MatchBr` in the final `dx-llvm` module
- mixed function with arithmetic plus match now emits real IR instead of failing with:
  - `UnsupportedTerminator("match")`
- regression test that `dx-llvm-ir` still rejects raw `MatchBr` if one is constructed manually

## Exit Criteria

This task is complete when:

- supported `match` cases no longer block real textual LLVM emission
- `dx-llvm-ir` receives only supported terminators
- no new match semantics were invented to make the lowering work
