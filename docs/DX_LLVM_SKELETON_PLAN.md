# DX LLVM Skeleton Plan

## Purpose

This document defines the first codegen-oriented step after MIR and runtime planning.

The goal is not "real LLVM" yet.
The goal is to create one stable low-level lowering shape that LLVM can later consume directly.

## Input Layers

The low-level skeleton should consume:

- MIR
- runtime ops plan
- runtime extern plan
- throw runtime plan

LLVM lowering should not recompute these decisions.

## Immediate Goal

Produce one low-level module shape with:

- low-level extern declarations
- low-level function signatures
- low-level runtime call steps
- low-level throw-check steps

This is enough to prove the backend path is coherent without introducing LLVM-specific complexity too early.

## Design Rule

The low-level skeleton is allowed to be conservative.

For example:

- many high-level `dx` types may lower to pointer-like placeholders initially
- runtime calls may be represented as symbolic low-level call steps
- closure bodies do not need inlining or duplication

What matters is consistency.

## Expected Shape

The first low-level layer should capture:

1. extern declarations
2. function parameter/return types
3. runtime call sites
4. throw-check boundaries

It does not yet need:

- full data-layout decisions
- SSA reconstruction
- LLVM instruction selection
- allocation strategy details

## Exit Criteria

This step is complete when:

- a module containing Python calls lowers to one low-level shape
- a module containing closure create/call lowers to one low-level shape
- throw boundaries appear as explicit low-level steps
- extern declarations are taken from the runtime extern plan, not recomputed ad hoc
