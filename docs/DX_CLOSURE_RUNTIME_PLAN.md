# DX Closure Runtime Plan

## Purpose

This document defines the next backend milestone for `dx`:

- closure runtime representation
- closure environment layout
- thunk calling convention
- how closure values connect MIR to the future LLVM layer

It exists to prevent ad hoc closure lowering during LLVM work.

## Current State

The compiler already has:

- typed closures in typed HIR
- effectful closure types
- capture analysis in typed HIR
- MIR closure nodes that preserve:
  - param types
  - return type
  - effects
  - captured locals

This is enough to define a real runtime model.

The repository has now also proven a narrower runnable executable subset via:

- `scripts/prove_executable_entry_subset.sh`

That runnable subset already covers:

- arithmetic
- captured thunk execution

The main remaining runtime gap is ordinary closure dispatch.

## Design Goal

Closure lowering should preserve three properties:

1. explicit environment data
2. explicit call entry
3. explicit effect/call boundary

The compiler should not invent different closure conventions in different backends.

## Non-Goals

This phase does not yet require:

- inlining
- closure body duplication
- heap optimization
- escape analysis
- borrow/ownership redesign
- LLVM implementation details

Those can come later.

## Runtime Shape

The intended runtime value model is:

```text
ClosureValue {
    code_ptr,
    env_ptr,
    metadata,
}
```

Where:

- `code_ptr` points to the callable entry for the closure body
- `env_ptr` points to captured values
- `metadata` can stay abstract at first, but should be able to encode:
  - arity
  - thunk vs ordinary closure
  - effect metadata if needed later

The exact in-memory layout may remain backend-defined for now, but MIR/runtime must agree on the conceptual model.

The next concrete dispatch step is documented in:

- `docs/DX_CLOSURE_DISPATCH_PLAN.md`

## Environment Model

Each closure environment should be modeled as an ordered list of captures:

- source local id
- capture name
- capture type
- mutability flag

This ordering should be stable and preserved from MIR to runtime lowering.

At this stage:

- captured values are copied into the environment conceptually
- mutability is preserved as metadata
- no aliasing optimization is attempted

## Calling Convention

Two callable families must be supported:

1. ordinary closures
2. zero-arg thunks from `lazy`

Conceptually:

```text
call_closure(closure, arg0, arg1, ...)
call_thunk(closure)
```

The runtime plan should be able to distinguish them from the closure type:

- thunk: zero params
- closure: one or more params

The LLVM layer should reuse this convention rather than inventing a separate one.

## MIR Requirements

MIR should preserve enough information to lower closures without re-analysis.

Required on `Rvalue::Closure`:

- captures
- parameter types
- return type
- effects

MIR does not yet need:

- closure body duplication
- explicit environment allocation instructions

Those belong in the next backend-oriented layer.

## Runtime Boundary Layer Requirements

Before LLVM lowering, `dx-runtime` should define:

- closure runtime hook names or operation kinds
- lowered runtime operations for:
  - closure creation
  - closure invocation
  - thunk invocation
- how closure captures map to environment slots

This should mirror the Python runtime plan approach:

- stable operation kinds
- stable hook naming
- inspectable lowered form

## Proposed Runtime Operation Families

The next runtime-layer step should introduce operations conceptually like:

- `dx_rt_closure_create`
- `dx_rt_closure_call`
- `dx_rt_thunk_call`

These names are placeholders for planning.

The key requirement is consistency, not final naming.

## Constraints

The closure runtime model must remain compatible with:

- effectful closures
- local closure calls
- captured `PyObj`
- future `!throw`
- future native LLVM lowering

It must not depend on:

- Python object model semantics
- parser surface syntax
- future query or AD features

## Immediate Implementation Order

1. keep capture information stable in typed HIR and MIR
2. define runtime-side closure operation types in `dx-runtime`
3. define inspectable lowering of closure creation/invocation
4. add tests for:
   - closure with one capture
   - closure with multiple captures
   - zero-arg thunk
   - captured `PyObj`
5. only then start LLVM lowering for closures

## Exit Criteria

This milestone is complete when:

- closure creation and invocation have one explicit runtime model
- the runtime layer exposes closure operations as inspectable lowered structures
- LLVM lowering can consume those operations without re-deciding semantics

## Strategic Rule

Do not let LLVM lowering become the place where closure semantics are first decided.

Closure semantics must be fixed one layer earlier.

And do not let runtime stubs silently fake semantic success once the executable
subset starts running for real. If a path is runnable, it must be runnable for a
structural reason, not by accident.
