# DX Runtime Stub Plan

## Purpose

This document defines the first executable runtime scope for `dx`.

It exists to prevent the next backend phase from stalling between:

- valid textual LLVM IR
- real runtime symbols that do not exist yet

The goal is not a full runtime.
The goal is a minimal, honest runtime that can support the currently stable
backend subset.

## Position In The Roadmap

This plan belongs after:

- closure-call ABI convergence
- `match` lowering before `dx-llvm-ir`
- toolchain-ready textual LLVM IR verification

It is the bridge from:

- "backend can emit/verify `.ll`"

to:

- "small programs can actually link and run"

## Scope

The first runtime stub layer should cover only the symbols already hard-coded as
the current stable backend surface:

- `dx_rt_closure_create`
- `dx_rt_closure_call_*`
- `dx_rt_thunk_call_*`
- `dx_rt_match_tag`
- `dx_rt_throw_check_pending`
- initial Python call hooks:
  - `dx_rt_py_call_function`
  - `dx_rt_py_call_method`
  - `dx_rt_py_call_dynamic`

## Design Rule

The first runtime implementation should be:

- minimal
- explicit
- testable
- ABI-faithful

It should not try to be:

- optimized
- generic over future language features
- a complete production runtime

## Runtime Layers

The minimal runtime should be thought of in three layers.

### Layer 1: Pure ABI Stubs

Purpose:

- ensure symbols exist
- ensure call signatures link
- ensure backend ABI can actually be exercised

Allowed behavior:

- fixed placeholder values
- trivial struct allocation
- explicit `panic` / abort for unsupported paths

This is enough to unblock end-to-end linking tests.

### Layer 2: Semantic Mini-Runtime

Purpose:

- make the stable subset actually work, not only link

Examples:

- `dx_rt_match_tag` performs nominal tag comparison
- `dx_rt_closure_create` stores env/code metadata
- `dx_rt_thunk_call_*` dispatches into the stored code pointer

This is the first layer that should execute meaningful behavior.

### Layer 3: Foreign Boundary Runtime

Purpose:

- make Python hooks operational
- define how `!throw` and foreign failure propagate

This may still begin with stubs if Python embedding is not yet ready.

## Immediate Execution Target

The first end-to-end executable subset should be limited to:

1. arithmetic
2. string literals/globals
3. `if`
4. closure creation
5. thunk call
6. ordinary closure call
7. nominal `match` tag dispatch

Python hooks may initially remain stubbed if the non-Python subset can already
link and run.

## Stub Policy Per Symbol

### `dx_rt_closure_create`

Required first behavior:

- accept env pointer and metadata inputs
- return a closure handle with enough information for later call/thunk dispatch

Current status:

- partially met for thunk env transport
- not yet sufficient for ordinary closure dispatch, because the callable entry
  identity is still missing from the creation ABI

### `dx_rt_closure_call_*`

Required first behavior:

- accept closure handle plus real call operands
- dispatch to the stored callable entry
- return the ABI-specialized result

Current status:

- ABI shape exists
- semantic dispatch is not implemented yet

The next required step is documented in:

- `docs/DX_CLOSURE_DISPATCH_PLAN.md`

### `dx_rt_thunk_call_*`

Required first behavior:

- accept closure handle
- call the stored zero-arg entry
- return the ABI-specialized result

Current status:

- partially semantic for simple captured-env cases in `dx-runtime-stub`
- still not based on a first-class stored callable entry

### `dx_rt_match_tag`

Required first behavior:

- inspect a nominal tag/discriminant
- compare to UTF-8 tag name
- return `i1`

This helper remains nominal-only in this milestone.

### `dx_rt_throw_check_pending`

Required first behavior:

- either no-op in the non-throwing subset
- or abort/panic if the runtime models a pending throw state

The first implementation may be conservative.

### Python hooks

Initial acceptable modes:

1. real embedding if available
2. explicit stub that fails loudly and predictably

What matters is that the ABI is exercised honestly.

## Recommended Implementation Order

1. `dx_rt_closure_create`
2. `dx_rt_thunk_call_*`
3. `dx_rt_closure_call_*`
4. `dx_rt_match_tag`
5. `dx_rt_throw_check_pending`
6. Python hooks

This order maximizes runnable non-foreign programs first.

## Packaging Recommendation

Do not embed the first runtime directly into compiler crates.

Prefer a separate runtime implementation unit, for example:

- a new Rust crate
- or a tiny C runtime linked by tests

The key requirement is a clean ABI seam.

## Testing Strategy

### Phase 1: ABI linkage tests

Verify:

- emitted `.ll` references runtime symbols
- those symbols exist in the runtime object/library
- linking succeeds

### Phase 2: subset execution tests

Verify:

- arithmetic function runs
- thunk executes and returns value
- closure call executes with real args
- nominal `match` dispatch selects the right arm

### Phase 3: foreign boundary tests

Verify:

- Python hook either works or fails explicitly by design
- throw-check path is observable

## Non-Goals

This milestone does not require:

- GC
- ownership/borrowing
- optimized closure env layout
- full ADT payload runtime
- production-grade Python integration
- optimizer passes

## Exit Criteria

This milestone is complete when:

- the stable non-foreign subset can be lowered, linked, and executed end to end
- runtime symbols match the backend ABI exactly
- Python/throw hooks are either operational or explicitly stubbed
- future runtime work can evolve from a real executable baseline instead of a planning-only backend
