# DX LLVM Lowering Plan

## Purpose

This document defines the first LLVM-oriented lowering step now that:

- MIR exists
- runtime ops exist
- runtime externs exist
- throw boundaries exist
- a low-level codegen skeleton exists

The goal is to make LLVM lowering a translation step, not a semantic design phase.

## Lowering Stack

The current intended backend stack is:

1. MIR
2. runtime ops / externs / throw plans
3. low-level codegen skeleton
4. LLVM-like module lowering
5. real LLVM backend integration

## Current LLVM-Oriented Step

The current step does **not** require LLVM libraries yet.

It should provide:

- LLVM-like extern declarations
- LLVM-like function signatures
- LLVM-like runtime call instructions
- LLVM-like throw-check instructions

This lets the project validate:

- symbol naming
- type translation
- call sequencing
- throw-boundary placement

before introducing an actual LLVM dependency.

## Design Rule

At this stage:

- no semantic recomputation in LLVM lowering
- no new runtime hook inference
- no new closure calling conventions
- no new throw semantics

LLVM lowering should consume already-fixed backend plans.

## Immediate Exit Criteria

This stage is successful when:

- Python runtime calls lower to LLVM-like call instructions
- closure create/call/thunk lower to LLVM-like call instructions
- throw checks lower to explicit LLVM-like calls
- extern declarations come directly from the runtime extern plan

After that, introducing a real LLVM crate becomes mostly a tooling/integration problem rather than a semantic one.
