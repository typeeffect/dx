# DX Small Core Rules

## Purpose

This document defines what should count as part of the `dx` language core and
what should stay outside it.

The goal is not to make `dx` small by making it weak.
The goal is to keep the core:

- small
- regular
- composable
- strong enough that larger capabilities can be built above it

## Core Thesis

`dx` should have a **small, powerful core** and should grow through:

- effects
- compile-time providers
- runtime/library layers
- explicit foreign boundaries

not by turning every successful feature into a new special case in the grammar
or type system.

## What May Enter The Core

A feature belongs in the core only if at least one of these is true:

1. it is required to express the basic execution model of the language
2. it is required to preserve regularity across the language
3. it cannot be expressed cleanly through providers, libraries, effects, or
   foreign/runtime layers
4. making it non-core would create opaque side channels or semantic fractures

In practice, the core should stay close to:

- functions and closures
- values, blocks, and control flow
- nominal data definitions
- explicit effects and handlers
- type/effect checking
- explicit foreign boundaries
- a minimal compile-time provider mechanism

## What Should Normally Stay Out Of The Core

The following should default to **not** entering the core:

- datasource-specific features
- AD primitive registries
- probabilistic inference strategies
- tensor/storage special cases
- Python ecosystem surface details
- LLM workflow conveniences
- backend- or runtime-specific optimizations

These are important, but they should usually live in:

- providers
- runtime/library layers
- target-specific packages
- foreign boundaries

## Providers As A Pressure Valve

Providers are one of the main ways `dx` should stay small without becoming weak.

They let the language support compile-time specializations such as:

- schema acquisition
- future AD primitive definitions

without hardcoding each one into the core syntax or compiler pipeline as an
isolated subsystem.

Reference:

- `docs/DX_COMPILETIME_PROVIDERS_PLAN.md`
- `docs/DX_AD_PRIMITIVE_PROVIDER_PLAN.md`
- `docs/DX_PROVIDER_CONSTRAINTS.md`

## Effects As A Pressure Valve

Effects are the other major pressure valve.

They let `dx` host:

- structured I/O
- no-coloring async/wait
- AD as an effect boundary
- probabilistic programming

without turning each domain into a separate sublanguage.

Reference:

- `docs/DX_FOUNDATIONS_PAPER_TRAIL.md`
- `docs/DX_AD_PPL_DIRECTION.md`

## Runtime Layers As A Pressure Valve

The runtime and library substrate should carry complexity that does not belong
in the language core.

Examples:

- arena/shared-buffer mechanics
- tensor storage and views
- pool behavior
- FFI buffer wrappers

The core should expose enough structure to integrate these later, but should not
grow tensor- or allocator-specific syntax too early.

Reference:

- `docs/DX_MEMORY_MODEL_PLAN.md`
- `docs/DX_MEMORY_MODEL_IMPLEMENTATION_PLAN.md`

## The Rule For New Features

When a new feature is proposed, ask these questions in order:

1. Can this be expressed as a library/runtime layer?
2. If not, can it be expressed as a compile-time provider?
3. If not, can it be expressed through effects/handlers?
4. If not, can it remain behind a foreign boundary?
5. Only then: does it need a new core feature?

If the answer is "yes" at any earlier step, it should usually stay out of the
core.

## What This Means For Current DX Directions

### Schema

Should not become a giant schema-specific sublanguage.
The right direction is:

- small declaration surface
- provider machinery
- typed result such as a future row type

### AD Primitive Definitions

Should not become a bag of compiler hardcoded exceptions.
The right direction is:

- provider-based primitive definition
- generated or explicit backward rules
- compiler-visible typed metadata

### Probabilistic Programming

Should not become a separate language track.
The right direction is:

- effect operations
- handler-selected inference

### Python Interop

Should not define the core.
The right direction is:

- explicit foreign boundary
- adoption bridge
- escape hatch when native layers are not yet enough

## Anti-Patterns

These are warning signs that the core is growing the wrong way:

- a new keyword for every major feature family
- compile-time magic that bypasses the provider model
- domain-specific syntax before a generic effect/provider/library path is tried
- runtime artifacts hidden behind implicit build behavior
- host-language-only extension points that the language itself cannot model

## Strategic Rule

`dx` should become more capable by:

- strengthening a few general mechanisms

not by:

- adding many unrelated special mechanisms

That is how the language can stay small and still reach:

- typed data
- ML/inference
- probabilistic programming
- systems-capable runtime use
- LLM-first workflows

This only works if providers stay narrow enough that they do not become a
dialect backdoor.

## Related Docs

- `docs/DX_LANGUAGE_VISION.md`
- `docs/DX_LONG_TERM_ROADMAP.md`
- `docs/DX_IMPLEMENTATION_ROADMAP.md`
- `docs/DX_COMPILETIME_PROVIDERS_PLAN.md`
