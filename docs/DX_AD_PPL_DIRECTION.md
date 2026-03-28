# DX AD and Probabilistic Programming Direction

## Purpose

This document records what should be intentionally recovered from `dx-03` for the new `dx` language.

The goal is not to copy old syntax wholesale.
The goal is to recover the successful semantic ideas.

## What Should Be Recovered

## 1. Automatic Differentiation as an Effect

This is worth preserving as a core long-term thesis.

The valuable idea from `dx-03` is:

- differentiation is not a magical compiler-only side channel
- differentiation is modeled through an effect boundary
- user-level gradient APIs remain expressible in the language

From `dx-03`:

- `smooth` behaved as the AD effect family
- `grad()` was a user-visible function, not just a hidden builtin
- derivative syntax desugared to `grad(...)`
- custom handlers enabled tracing, clipping, scaling, and alternative behaviors

This is a strong idea and should be preserved conceptually.

## 2. Probabilistic Programming via Effects

This should also be preserved.

From `dx-03`:

- `prob` modeled operations like `sample` and `observe`
- different handlers implemented different inference strategies
- demos already existed for:
  - basic probabilistic programming
  - Bayesian regression
  - Monty Hall
  - HMC-based Bayesian linear regression

The important reusable thesis is:

- model code stays stable
- inference strategy varies by handler/effect interpretation

That remains a very good direction for `dx`.

## 3. Composition of Effects

One of the strongest results from `dx-03` was not just AD or PPL individually.
It was their composition.

The long-term target should preserve:

- `smooth` / AD-related effects
- `prob`
- randomness
- state
- eventually `io` and `wait`

The HMC demo is especially valuable because it shows:

- gradients
- randomness
- stateful chain updates

all interacting in one program.

## What Should Not Be Recovered Blindly

The old surface syntax and old implementation strategy should not be copied mechanically.

Examples of things to reconsider rather than inherit:

- exact `effect` declaration syntax from `dx-03`
- `handle ... with ... -> resume(...)` surface form as-is
- old parser and statement grammar
- old mutation and scope conventions
- old record and assignment syntax

The recovery target is semantic, not syntactic.

## What Was Actually Found in `dx-03`

The strongest concrete artifacts found were:

- AD examples:
  - `src/test/resources/programs/ad.dx`
  - `src/test/resources/programs/dx_grad_full.dx`
  - `src/test/resources/programs/dx_custom_grad.dx`
  - `src/test/resources/programs/smooth_primitive.dx`
- probabilistic programming:
  - `src/test/resources/programs/prob_basic.dx`
  - `src/test/resources/programs/prob_bayesian.dx`
  - `src/test/resources/programs/prob_monty.dx`
- HMC / Bayesian linear regression:
  - `src/test/resources/programs/hmc.dx`

Note:

- an explicit HMM demo was not found in the inspected files
- HMC definitely exists and is strong

## Recommended Recovery Plan

## Phase A: Preserve the Design Slot

Do this now:

- explicitly reserve AD and PPL as future first-class subsystems
- avoid type/effect/core IR decisions that would make them awkward later

This means:

- effects must remain first-class in the language design
- HIR/MIR must have a place for effectful operations
- the runtime boundary must be able to host effect handlers later

## Phase B: Recover the Semantics, Not the Full Surface

After the v0.1 core parser/checker exists:

1. define effect declarations for `dx`
2. define effect operations
3. define handler syntax
4. define effect elimination / interpretation points

Only after that:

5. recover `smooth`
6. recover `prob`
7. recover combined demos

## Phase C: Rebuild the Demos in the New Language

The demos worth reintroducing in the new `dx` repo are:

1. scalar AD
2. custom gradient transforms
3. Monty Hall
4. Bayesian linear regression
5. HMC

These should be rewritten in the new syntax, not transplanted unchanged.

## Why This Matters Strategically

The combination of:

- effects
- no async coloring
- AD
- probabilistic programming
- query/data applications

is a real differentiator.

It gives `dx` a stronger identity than:

- "typed Python"
- or "query language only"

## Near-Term Rule

Do not implement AD or PPL before the core parser/checker and core effect machinery exist.

But do treat them as committed design targets, not vague future possibilities.
