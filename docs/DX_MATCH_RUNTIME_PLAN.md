# DX Match Runtime Plan

## Purpose

This document fixes the runtime contract for the current backend implementation
of `match`.

The goal is to prevent the new `match` lowering from depending on an ambiguous
extern symbol.

## Current State

`match` is now lowered before `dx-llvm-ir`.

The backend path currently uses:

- `dx_rt_match_tag(scrutinee, pattern_name) -> i1`

inside `dx-llvm` as part of the generated check blocks.

This is the correct short-term strategy, but the runtime contract must be made
explicit now so later runtime work does not reinvent it.

## Decision

For the current compiler subset, `dx_rt_match_tag` is the canonical runtime
helper for nominal tag checks.

Its contract is:

```text
dx_rt_match_tag(value_handle: ptr, pattern_tag_name: ptr) -> i1
```

Where:

- `value_handle` is the runtime handle of the scrutinee
- `pattern_tag_name` is a UTF-8 zero-terminated string global naming the tag
- the result is:
  - `true` if the scrutinee has the requested nominal tag
  - `false` otherwise

## Scope

This helper only covers the currently implemented backend subset:

- nominal tag equality
- wildcard/default behavior handled by control-flow lowering
- constructor-name matching at the tag level

It does not yet cover:

- payload extraction
- nested patterns
- guards
- structural matching
- exhaustiveness reasoning

## Semantics

The semantics are intentionally narrow:

1. `dx_rt_match_tag` performs **only a tag comparison**
2. it does not allocate
3. it does not bind payload locals
4. it does not perform fallback logic

That means the branch chain remains responsible for:

- trying arms in order
- selecting the default arm
- preserving existing match ordering semantics

## Wildcard Rule

Wildcard/default does **not** need a runtime helper.

The preferred lowering remains:

- explicit final fallback branch in control flow

not:

- calling `dx_rt_match_tag(_, "_")`

If the current branch chain still materializes `"_"` for implementation
convenience, that should be treated as a temporary backend detail, not the
desired long-term runtime contract.

## Constructor Pattern Rule

For the currently supported subset, constructor patterns such as `Ok(v)` are
treated nominally at this runtime boundary:

- runtime checks only whether the scrutinee tag is `Ok`
- payload extraction is outside the scope of `dx_rt_match_tag`

This is acceptable because the current backend milestone is about control-flow
correctness first, not full ADT payload lowering.

## ABI Shape

The extern declaration should remain:

```text
dx_rt_match_tag(ptr, ptr) -> i1
```

This keeps the helper:

- simple
- toolchain-friendly
- easy to stub in the first runtime

## Runtime Implementation Expectations

The first runtime implementation may be minimal:

- inspect the discriminant/tag of a runtime value
- compare it against the provided UTF-8 tag string
- return `i1`

It should not:

- materialize payload objects
- mutate runtime state
- throw on ordinary mismatch

If invalid foreign/runtime values can reach it later, that policy belongs in
runtime validation, not in match semantics.

## Design Rules

1. `dx_rt_match_tag` stays nominal-only in this phase.
2. Do not overload it with payload extraction.
3. Do not make wildcard matching depend on it.
4. Do not add hidden control-flow semantics to the runtime helper.
5. Keep the helper compatible with future real runtime implementation.

## Tests To Require Later

When runtime implementation work starts, minimum tests should cover:

- tag match success
- tag mismatch
- constructor tag names like `Ok` / `Err`
- fallback arm reached without calling a wildcard-specific helper

## Exit Criteria

This plan is in force when:

- backend lowering, docs, and future runtime work all agree that
  `dx_rt_match_tag` is only a nominal tag checker
- the compiler no longer treats the helper as semantically open-ended
