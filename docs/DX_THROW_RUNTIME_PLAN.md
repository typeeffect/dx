# DX Throw Runtime Plan

## Purpose

This document defines how `!throw` becomes explicit in the backend before LLVM lowering.

It exists to avoid two bad outcomes:

- treating `!throw` as only a front-end annotation
- inventing throw semantics ad hoc during LLVM lowering

## Current Situation

The compiler already has:

- effect tracking in typed HIR
- effect-preserving MIR
- Python runtime plans
- closure runtime plans
- unified runtime ops
- unified runtime externs

What is still missing is a dedicated runtime-layer interpretation of `!throw`.

## Core Decision

`!throw` should be modeled only at **immediate execution boundaries**.

That means:

- Python calls with `!throw` are throw boundaries
- closure/thunk invocations with `!throw` are throw boundaries
- closure creation is **not** a throw boundary, even if the closure type carries `!throw`

This distinction is important.

Effects on closure values are latent until the closure is actually invoked.

## Boundary Classes

The current backend should distinguish at least:

1. Python function call throw boundary
2. Python method call throw boundary
3. Python dynamic call throw boundary
4. closure call throw boundary
5. thunk call throw boundary

This is enough to keep the runtime model explicit without overcommitting to final exception ABI details.

## Runtime Hook Direction

The current minimal runtime-hook direction is:

- one generic pending-throw check hook at the runtime layer

Placeholder symbol:

- `dx_rt_throw_check_pending`

The exact low-level ABI may still evolve.
What matters now is:

- throw-capable execution points are explicit
- closure creation is not accidentally treated as throwing

## Relation To Error Policy

This plan assumes the already-fixed error policy:

- `Result[T, E]` for recoverable domain failure
- `!throw` for foreign/runtime exception-style propagation
- `panic` for bugs and invariant violations

So this layer is **not** about all failure.
It is only about explicit throw-style boundaries.

## Immediate Compiler Rule

When building throw runtime plans:

- inspect runtime ops
- keep only ops whose effects include `throw`
- discard latent closure creation sites
- retain only immediate execution points

This gives the backend the right shape for future codegen without inventing behavior too early.

## Exit Criteria

This milestone is complete when:

- throw-capable execution sites are explicit in `dx-runtime`
- closure creation is excluded from immediate throw boundaries
- the future LLVM layer can consume throw-site information directly
