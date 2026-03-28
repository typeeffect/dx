# DX LLVM Toolchain Integration Plan

## Purpose

This document defines how `dx` should move from:

- a validated LLVM-like backend (`dx-llvm`)
- a real textual LLVM IR emitter (`dx-llvm-ir`)

to:

- LLVM IR that is checked by real LLVM tools

without introducing LLVM Rust bindings too early.

## Decision

The next backend milestone should use the external LLVM toolchain first.

The chosen sequence is:

1. emit textual `.ll` from `dx-llvm-ir`
2. verify that `.ll` with real LLVM tools:
   - `llvm-as`
   - `opt -verify`
3. only after the emitted IR is stable:
   - consider `llvm-sys`
   - or another binding layer

This keeps the current architecture intact:

- semantics are fixed before tool invocation
- tool integration remains a verification/execution problem
- we avoid binding churn while the backend ABI is still settling

## Why This Is the SOTA Path Here

For this codebase, the modern and pragmatic sequence is:

- textual IR first
- tool verification second
- bindings last

because:

- `dx-llvm` already encodes the backend structure explicitly
- `dx-llvm-ir` already emits real IR for a serious subset
- LLVM bindings would increase integration complexity without improving the
  semantic model yet
- LLVM tools are the best ground truth for whether our IR is actually acceptable

## Scope

This milestone is about verification and readiness.

It is not yet about:

- object emission
- JIT execution
- native runtime completeness
- optimization pipelines beyond basic verification

Those come after the emitted `.ll` is stable on the supported subset.

## Required Preconditions

Before toolchain integration becomes the main line of work, the backend should
already satisfy:

1. supported subset emits deterministic textual LLVM IR
2. runtime extern symbols are explicit and stable
3. closure env packing is explicit
4. unsupported constructs fail explicitly
5. validator catches the obvious structural mistakes before tool invocation

Current major remaining blockers still outside this milestone:

- `match` still needs to disappear before `dx-llvm-ir`
- ordinary closure-call ABI still needs to be fully realized as real operands

## Integration Strategy

### Stage 1: File-Level Emission

Add a narrow path that writes the output of `dx-llvm-ir` to a `.ll` file.

Requirements:

- deterministic output
- one file per module
- no semantic changes in the writer

This should be a thin wrapper over the existing emitter, not a second backend.

### Stage 2: Verification Command Layer

Add an execution path or test harness that, when LLVM tools are present:

1. writes `.ll`
2. runs `llvm-as`
3. optionally runs `opt -verify`

Expected behavior:

- if tools are missing, tests skip cleanly
- if IR is malformed, the failure is surfaced directly

### Stage 3: Supported-Subset Verification Tests

Add end-to-end tests only for the subset already promised by `dx-llvm-ir`.

Good initial cases:

- plain arithmetic
- `if`
- string globals
- `Unit -> ret void`
- thunk path
- Python runtime call path
- ordinary closure call path once ABI work lands

### Stage 4: Toolchain-Driven Tightening

Only after real tool verification is running:

- tighten emitter fidelity where LLVM rejects valid-looking output
- improve type fidelity
- improve global and extern rendering where needed

Do not add new language semantics in response to toolchain failures.
Only fix emission fidelity.

## Design Rules

1. `dx-llvm-ir` remains the single real IR emitter.
2. LLVM tools validate emitted IR; they do not replace our internal lowering.
3. No second textual backend.
4. No premature Rust LLVM binding integration.
5. No backend redesign driven by convenience in the verification harness.

## Test Policy

Two classes of tests should exist:

### Internal backend tests

These continue to run everywhere:

- emitter tests
- validator tests
- cross-layer tests

### LLVM toolchain tests

These are conditional:

- run only when LLVM tools are present
- verify the subset already marked as supported
- must not fail the whole suite just because the machine lacks LLVM

## Failure Classification

When toolchain integration starts, failures should be classified as one of:

1. emitter fidelity bug
2. backend invariant bug missed by `dx-llvm::validate`
3. unsupported feature reached the emitter by mistake
4. environment/tooling missing

This distinction matters because only the first three are compiler issues.

## Non-Goals

This milestone does not require:

- introducing PHI construction
- switching to a direct SSA-producing compiler
- replacing stack-based lowering
- implementing the full runtime
- linking native executables yet

## Exit Criteria

This milestone is complete when:

- `dx-llvm-ir` output for the supported subset can be written as `.ll`
- that `.ll` can be verified by real LLVM tools when available
- toolchain verification failures point to emission issues, not architectural ambiguity
- the project can move to runtime execution work without guessing whether the IR is valid LLVM
