# DX AD Primitive Provider Plan

## Purpose

This document defines the second concrete compile-time provider kind for `dx`:

- AD primitive providers

Schema is the first provider kind already landing in `dx-04`.
AD primitive definitions should be the next strong recovery target from `dx-03`.

Reference:

- `docs/DX_COMPILETIME_PROVIDERS_PLAN.md`
- `docs/DX_AD_PPL_DIRECTION.md`

## Why This Exists

`dx-03` proved a valuable idea through `smooth_primitive`:

- a user could declare a new differentiable primitive
- the compiler could generate derivative rules for a narrow expression subset
- or the user could provide an explicit fused backward rule
- the result became ordinary compiler-visible code plus tape recording

That should not be recovered as a one-off AD trick.
It should become the second compile-time provider kind after schema.

## Design Goal

The long-term target is:

- compile-time extension written in DX
- explicit declaration in source
- deterministic compiler-visible result
- optional locked artifact when useful
- no hidden host-language registration

This makes AD primitive definition:

- typed
- reviewable
- reproducible
- compatible with future compiler-in-DX work

## What Must Be Recovered From `dx-03`

### 1. User-Declared Primitives

The key capability was:

- define a new primitive in source
- use it like an ordinary function
- have AD understand it without hardcoding every new case into the compiler

### 2. Two Rule Kinds

`dx-03` had two strategically important modes.

#### Autodiscovery

- forward body declared by the user
- derivative generated at compile time for a restricted expression subset

This was backed by `SymDiff.kt`.

Useful recovery:

- keep the idea of narrow compile-time derivative synthesis
- keep the restriction explicit
- fail clearly outside the supported subset

#### Fused Backward

- forward body declared by the user
- explicit backward rule provided by the user

Useful recovery:

- custom numerically stable derivatives
- Jacobian-row style backward for multi-parameter primitives
- no need to hardcode special fused ops in the compiler

### 3. Compiler-Visible Lowering

The old desugaring path mattered:

- primitive declaration did not stay magical
- it lowered into ordinary compiler-visible structure
- tape recording and derivative metadata stayed explicit

`dx-04` should preserve that property.

## Non-Goals

This is not yet a plan for:

- arbitrary symbolic algebra over all DX
- compile-time execution without constraints
- higher-order AD metaprogramming in v0
- general macro facilities

The provider should stay intentionally narrow at first.

Constraint reference:

- `docs/DX_PROVIDER_CONSTRAINTS.md`

## Unified Lifecycle

AD primitive providers should follow the same lifecycle as schema providers.

### 1. Source Declaration

Illustrative direction only:

```dx
ad primitive sigmoid(x: Float) -> Float
    forward:
        1.0 / (1.0 + exp(-x))
    backward(x, dout):
        val s = 1.0 / (1.0 + exp(-x))
        s * (1.0 - s) * dout
.
```

Or autodiscovery:

```dx
ad primitive square(x: Float) -> Float
    forward:
        x * x
.
```

The exact surface is still open.
The important thing is the declaration shape and lifecycle.

### 2. Compile-Time Analysis

The compiler/provider system analyzes the declaration and produces:

- primitive name
- signature
- rule kind
  - generated
  - explicit backward
- validation diagnostics

### 3. Optional Locked Artifact

Unlike schema, an external artifact may not be required for every AD primitive.

Still, the model should allow one when useful:

- reviewable generated derivative metadata
- stable primitive packs
- future distribution of primitive libraries

Possible future artifact direction:

- `.dxprimitive`

This is a later slice, not a v0 requirement.

### 4. Compiler-Facing Typed Result

The provider contributes a typed primitive contract to the compiler.

At minimum:

- primitive symbol
- forward type
- backward type/arity contract
- effect metadata
- lowering/runtime linkage metadata

### 5. Refresh / Generation Policy

If artifacts are generated or locked, refresh should be explicit, not hidden in
normal compilation.

Schema already points in this direction.
AD primitive providers should follow the same rule.

## v0 Provider Model

The first implementation slice should be intentionally narrow.

### Supported shape

- scalar primitives first
- 1 to 3 parameters
- explicit typed parameters
- `Float`-focused start

### Generated mode

Compile-time derivative synthesis should support only a restricted subset,
mirroring the spirit of `dx-03`:

- arithmetic
- unary negation
- selected math builtins
- simple composition

Anything outside that subset should fail with a clear diagnostic telling the
user to provide an explicit backward rule.

### Explicit backward mode

Allow:

- one backward rule block
- `dout` input
- 1 row per parameter for multi-arg primitives

This preserves the most valuable `smooth_primitive` behavior without dragging in
the entire old surface unchanged.

## Compiler Integration Direction

The provider result should eventually feed:

- name resolution
- type checking
- effect checking
- lowering/runtime linkage

At v0 planning level, the important thing is to avoid opaque side channels.

The compiler should know:

- that the primitive exists
- what its type contract is
- what derivative rule kind it uses

## Relation To The Memory Model

AD primitive providers and the memory model solve different layers.

- providers define compile-time primitive semantics
- the memory model governs tensor/storage/runtime behavior

They meet later when:

- tensor-aware primitives appear
- fused backward rules care about layout/buffering
- inference and training workloads share the same runtime substrate

## Relation To Probabilistic Programming

This provider kind is specifically about primitive AD extensibility.

It complements, but does not replace:

- AD as an effect
- `prob` as an effect
- their composition in model/inference code

The strategic value is that the same language can host:

- user-visible gradient APIs
- provider-defined differentiable primitives
- future AD/PPL composition without host-language black boxes

## Immediate Next Steps

1. finish the schema provider integration path already underway
2. keep AD primitive examples visible in `examples/targets/`
3. document provider lifecycle examples side by side for schema and AD
4. later define the first parser/compiler slice for AD primitive declarations

## Judgment

If `dx` wants one compile-time extension machinery instead of several unrelated
systems, AD primitive providers are the decisive second case after schema.
