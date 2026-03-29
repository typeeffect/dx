# DX Compile-Time Providers Plan

## Purpose

This document generalizes the current schema-provider direction into a single
compile-time extension machinery for `dx`.

The goal is not to add arbitrary compile-time execution.
The goal is to support a small number of explicit, deterministic, artifact-backed
compile-time facilities written in `dx` itself.

## Core Thesis

`dx` should not grow a separate mechanism for every compile-time feature.

Instead, it should have one compile-time provider core that can host multiple
provider kinds:

1. schema acquisition
2. new AD primitive definitions
3. future ML/runtime primitive packs
4. future typed external interface descriptions

Schema is the first provider kind, not the whole category.

## Why This Matters

`dx-03` already contained a second strong compile-time direction besides schema:

- `smooth_primitive`
- symbolic derivative generation via `SymDiff.kt`
- fused user-defined backward rules
- desugaring into compiler-visible normal functions plus tape recording

That was not just an AD trick.
It was already a narrow compile-time extension mechanism:

- explicit declaration in source
- compile-time transformation
- deterministic compiler-visible output
- reusable semantic surface

`dx-04` should recover that value without rebuilding another ad hoc subsystem.

## Non-Goals

This is not a plan for:

- arbitrary compile-time I/O during normal builds
- unconstrained macros
- compiler plugins in Rust only
- hidden host-language side channels

The intended direction is narrow and explicit.

Hard constraint reference:

- `docs/DX_PROVIDER_CONSTRAINTS.md`

## Design Rules

Compile-time providers in `dx` should be:

- written in `dx` itself
- deterministic
- explicit in source
- artifact-backed when external metadata is involved
- reviewable in version control
- capability-limited
- diagnosable by the compiler as normal language constructs

They should also remain constrained enough that they cannot create dialects of
`dx`.

The normal build should remain reproducible and offline-capable where possible.

## Unified Provider Model

Every provider kind should fit the same lifecycle.

### 1. Source Declaration

The user writes an explicit declaration in DX source.

Examples:

```dx
schema Customers = csv.schema("data/customers.csv") using "schemas/customers.dxschema"
```

```dx
ad primitive sigmoid(x: Float) -> Float using "primitives/sigmoid.dxprimitive"
```

The exact AD surface is still open, but the lifecycle should match.

### 2. Compile-Time Request

The declaration creates a compile-time request that the compiler can analyze.

The request is:

- typed
- provider-specific
- explicit about refresh/build policy

### 3. Locked Artifact

When the provider depends on external or generated metadata, the result should be
locked in a stable artifact.

Examples:

- `.dxschema`
- future `.dxprimitive`

This keeps normal builds reproducible and reviewable.

### 4. Compiler-Facing Typed Result

The provider contributes something typed to the compiler.

Schema example:

- `Customers.Row`
- known field names
- nullability

AD primitive example:

- primitive symbol
- declared signature
- derivative rule metadata
- lowering/runtime linkage metadata

### 5. Refresh / Generation Policy

Refresh or regeneration must be explicit.

Examples:

- `dx schema refresh`
- future `dx provider refresh`
- future `dx ad refresh`

Normal compilation should consume locked artifacts and report mismatches clearly.

## Provider Kind 1: Schema

This is the first implemented provider slice in `dx-04`.

Today the repo already has:

- `.dxschema` artifacts
- parser/validator/canonical render
- source-vs-artifact contract checking
- parser/HIR pass-through for `schema ...`

What remains is language integration:

- compiler loading of locked artifacts
- `X.Row` as a real semantic type
- explicit refresh workflow

Reference:

- `docs/DX_SCHEMA_PROVIDER_PLAN.md`
- `docs/DX_SCHEMA_ARTIFACT_SPEC.md`
- `docs/DX_AD_PRIMITIVE_PROVIDER_PLAN.md`

## Provider Kind 2: AD Primitives

This is the second strategic provider kind to recover from `dx-03`.

The most valuable parts of the old design were:

- user-declared differentiable primitives
- autodiscovery mode
  - compile-time derivative generation for a restricted expression subset
- fused mode
  - user-supplied backward rule
- compiler-visible lowering to ordinary functions plus tape recording

The long-term direction in `dx` should preserve those ideas while avoiding the
old ad hoc split between language surface and host implementation.

The provider result should eventually carry:

- primitive name
- forward signature
- effect contract
- derivative rule kind
  - generated
  - explicit backward
- runtime/lowering linkage
- validation diagnostics

## Why Providers Must Be Written in DX

If compile-time extensions live only in the host implementation, `dx` gets a
split architecture:

- one language for users
- another language for meta-features

That weakens the project.

The stronger direction is:

- compile-time extensions written in DX
- narrow compile-time capabilities
- artifact-backed workflows
- typed compiler-visible outputs

This also keeps the door open to future self-hosting and compiler-in-DX work
without requiring arbitrary compile-time evaluation from the start.

## v0 Direction

The first version should stay intentionally narrow.

### v0.1

- schema provider flow continues to land
- provider core stays implicit but should be documented now
- AD primitive recovery stays at target-example and plan level

### v0.2 direction

- make provider core explicit in compiler/tooling planning
- keep schema as the first fully integrated provider
- add a draft artifact model for AD primitives

### Later

- DX-in-DX compile-time provider authoring
- refresh/generation commands generalized across provider kinds
- controlled compile-time capability surface

## Roadmap Impact

Milestone F should now be read as:

- compile-time providers

with schema as the first concrete provider and AD primitive definitions as the
next strong recovery target.

This avoids:

- one-off schema machinery
- one-off AD metaprogramming
- repeated redesign every time a new compile-time feature appears

## Immediate Next Steps

1. keep integrating schema declarations into the compiler
2. introduce artifact loading and refresh policy for schema
3. document the AD recovery target in terms of provider machinery
4. preserve AD target examples that exercise:
   - custom primitives
   - fused backward rules
   - custom gradient behavior
