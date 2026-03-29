# DX Target Examples Recovery

## Purpose

This document tracks the strategic `dx-03` demos that are being recovered into
`dx-bootstrap` as semantic target examples.

The goal is not to transplant old syntax.
The goal is to keep the strongest `dx` design claims visible in the new repo
while the exact parser-stable surface is still narrowing.

Reference package:

- `examples/targets/`

## Recovery Rule

Recovered target examples should:

- preserve the semantic point of the original `dx-03` demo
- use the newer DX block-based style where possible
- stay honest about provisional surface syntax
- use `.dx.example` when the language surface is not yet parser-stable

## Strategic Tranches

### 1. Effects and Async Decoloring

This tranche protects one of the core `dx` identity claims:

- effects are first-class
- handler composition matters
- local functions still work inside handled regions
- `!io` is explicit without `async`/`await` coloring

Recovered:

- `effects_run.dx`
  -> `examples/targets/effects_run.dx.example`
- `effects_compose.dx`
  -> `examples/targets/effects_compose.dx.example`
- `async_no_coloring.dx`
  -> `examples/targets/async_no_coloring.dx.example`
- `nested_fun_handle.dx`
  -> `examples/targets/nested_fun_handle.dx.example`

Why it matters:

- this is the clearest language-facing justification for explicit effects
- it protects the no-coloring thesis from becoming only a doc claim

### 2. AD and Probabilistic Programming

This tranche protects the longer-term differentiator of `dx`:

- gradient programming through effects
- custom gradient behavior through handlers
- probabilistic models with handler-selectable inference

Recovered:

- `ad.dx`
  -> `examples/targets/ad_scalar_grad.dx.example`
- `dx_custom_grad.dx`
  -> `examples/targets/ad_custom_grad.dx.example`
- `prob_basic.dx`
  -> `examples/targets/prob_basic_inference.dx.example`
- `prob_monty.dx`
  -> `examples/targets/prob_monty_hall.dx.example`

- `dx_grad_full.dx`
  -> `examples/targets/ad_full_smooth_handler.dx.example`
- `prob_bayesian.dx`
  -> `examples/targets/prob_bayesian_regression.dx.example`
- `hmc.dx`
  -> `examples/targets/prob_hmc_regression.dx.example`

Why it matters:

- these examples show that AD and PPL are not separate gimmicks
- they are intended to be native effect-driven subsystems in the language

### 3. Multi-shot Search and Selective CPS

This tranche protects the selective-CPS side of the language thesis:

- `multi` effects are not theoretical decoration
- handlers can branch, aggregate, and backtrack
- search is a first-class design target

Recovered:

- `amb_basic.dx`
  -> `examples/targets/amb_basic.dx.example`
- `amb_collect.dx`
  -> `examples/targets/amb_collect.dx.example`
- `amb_queens.dx`
  -> `examples/targets/amb_queens.dx.example`

Why it matters:

- these are the strongest concrete demos for why `multi` effects justify
  selective CPS in the compiler/runtime architecture
- without them, multi-shot effects risk becoming only a paper claim

### 4. ML / Tensor Workloads

This tranche protects the bridge from effects/query language design toward
native ML and inference workloads.

Recovered:

- `attention_forward.dx`
  -> `examples/targets/ml_causal_attention.dx.example`
- `transformer_ops.dx`
  -> `examples/targets/ml_tensor_ops.dx.example`
- `mnist_dx_grad.dx`
  -> `examples/targets/ml_mnist_training.dx.example`

Why it matters:

- it connects the language to tensor-heavy realistic workloads
- it also informs the future memory-model milestone

## Current State

Recovered target package now covers 26 examples across 10 tranches:

- effects and handler execution (4)
- AD basics and custom gradients (2), plus full smooth handler and d/d syntax (1)
- custom AD primitives and fused backward rules (2)
- probabilistic programming: basic, Monty Hall, Bayesian regression, HMC (4)
- combined AD + PPL: variational inference via gradient ascent on ELBO (1)
- multi-shot search: amb, collect, N-queens (3)
- ML/tensor: primitives, causal attention, MNIST training (3)
- typed data and Python bridge: schema pipeline, incremental migration (2)
- LLM-first: structured output, tool workflow (2)
- edge/embedded inference: sensor inference, quantized pipeline (2)

The package does not yet claim:

- final parser-stable effect syntax
- final parser-stable handler syntax

## Remaining Gaps

All major strategic dx-03 demos are now recovered.

Lower-priority remaining sources:

- `mnist_full.dx` — Kotlin-builtin AD version (superseded by dx-level AD in `ml_mnist_training`)
- `effects_multi_handler.dx` — multi-handler nesting (partially covered by `effects_compose`)
- ~~Combined AD + PPL example~~ → `ad_ppl_combined.dx.example`

## Roadmap Relevance

This recovery work does not change the current closed backend baseline.
It does change the language-facing roadmap by making future milestones concrete.

Most relevant milestones:

- post-baseline effect surface stabilization
- compile-time schema providers
- region/shared-buffer memory model

The target examples package is the current place where those future semantics
remain visible before the parser/compiler catches up.
