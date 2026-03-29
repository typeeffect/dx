# DX Long-Term Roadmap

## Purpose

This document defines the long-term platform trajectory of `dx` beyond the
current implementation milestones.

It exists to keep two things separate:

- the near-term implementation order in
  `docs/DX_IMPLEMENTATION_ROADMAP.md`
- the broader product/language direction that `dx` is intended to grow into

## Design Rule

`dx` should not try to market every long-term ambition at once.

The right structure is:

1. one foundation
2. one cross-cutting principle
3. a small number of stacked growth layers
4. one adoption bridge

That keeps the language coherent while still preserving the full long-term
scope.

Reference:

- `docs/DX_SMALL_CORE_RULES.md`

## Foundation

The base of the platform is a native deterministic core:

- native compilation
- static types
- explicit effects
- deterministic runtime and memory costs
- explicit foreign boundaries
- schema-aware external data direction
- runtime/library support for tensor- and inference-oriented workloads

This foundation is what makes the later layers credible.

### Target Demo

Flagship workflow:

- native executable build/run of the current runnable subset

Why it matters:

- proves that `dx` is not only a design language but a native compiled one
- anchors every later layer on a real compiler/runtime path

Closest repo artifacts today:

- `docs/DX_CORE_FEATURES_v0_ALPHA.md`
- `docs/DX_EXECUTION_WORKFLOW.md`
- `scripts/prove_executable_entry_subset.sh`

Status:

- runnable today

## Cross-Cutting Principle: LLM-First

`dx` should be designed for both humans and LLMs.

That means:

- regular syntax
- stable AST shapes
- explicit semantics
- formatter-first workflows
- machine-readable compiler output
- structured tool and model interaction over ad-hoc string glue

LLM-first is not a separate vertical.
It is a design principle that should shape every layer of the platform.

### Target Workflow

Flagship workflow:

- schema-backed, tool-oriented model interaction with structured outputs instead
  of ad-hoc prompt strings

Why it matters:

- makes the LLM-first claim concrete in terms of types, schema, and machine-
  readable interfaces
- connects the design principle to both schema providers and future inference
  workflows

Closest repo artifacts today:

- `docs/DX_LANGUAGE_VISION.md`
- `docs/DX_SCHEMA_ARTIFACT_SPEC.md`
- `crates/dx-schema`

Status:

- design/tooling direction exists today
- no dedicated language/runtime feature layer yet

## Layer 1: Systems-Capable Native Runtime

`dx` should grow into a systems-capable native language, but not by starting as
a classic low-level systems language.

Target capabilities:

- deterministic runtime behavior
- explicit memory model
- strong FFI
- future `unsafe` boundary
- runtime/infrastructure implementation viability
- deployment to edge and embedded environments

This layer should support:

- runtime and library implementation
- inference engines
- data and analytics infrastructure
- edge/embedded AI inference workloads

### Target Demo

Flagship workflow:

- pooled tensor allocation plus shaped tensor access in native runtime code

Why it matters:

- is the shortest path from the current runtime crate to real inference-oriented
  workloads
- exercises deterministic allocation, views, reshape, and FFI boundaries

Closest repo artifacts today:

- `crates/dx-memory`
- `examples/memory/README.md`

Status:

- runtime/library slice exists today
- not yet a DX language-level feature

## Layer 2: Typed Data Language

`dx` should become a strong language for typed external data:

- compile-time providers, starting with schema
- typed data ingestion
- typed transforms
- query and orchestration workflows
- explicit explainability of execution and pushdown

This is the first clear product wedge and should remain the first practical
adoption path.

### Target Demo

Flagship workflow:

- compile-time-checked external schema flowing into typed row-oriented analysis

Why it matters:

- is the first product-shaped wedge for `dx`
- turns external data metadata into something native and typed instead of a
  `PyObj` escape hatch

Closest repo artifacts today:

- `crates/dx-schema`
- `docs/DX_COMPILETIME_PROVIDERS_PLAN.md`
- `examples/schema/README.md`
- `scripts/audit_schema_examples.sh`

Status:

- tooling exists today
- language integration is not done yet

## Layer 3: ML Language

Above the typed data layer, `dx` should grow into a native ML/inference
language:

- tensor storage and views
- deterministic runtime allocation strategy
- model-serving and inference workflows
- edge/embedded inference support
- compile-time-defined differentiable primitives
- room for native wrappers over Python-backed ecosystems first, and more native
  execution later

This is where the memory model and the systems-capable runtime start to matter
as product features, not just implementation details.

### Target Demo

Flagship demo:

- `examples/targets/ml_mnist_training.dx.example`

Why it matters:

- is the clearest end-to-end ML target currently preserved from the earlier
  design work
- forces tensor semantics, AD, and model-training workflows to compose

Closest repo artifacts today:

- `examples/targets/ml_mnist_training.dx.example`
- `examples/targets/ml_tensor_ops.dx.example`
- `examples/targets/ml_causal_attention.dx.example`

Status:

- target-only today

## Layer 4: Probabilistic Language

`dx` should preserve and then expand the probabilistic/effect-oriented path:

- probabilistic effects and handlers
- inference as effect interpretation
- composition with AD and query/data workflows
- probabilistic programs that can still live inside a native deterministic host

This should be treated as a deliberate language axis, not an accidental
research side path.

### Target Demo

Flagship demo:

- `examples/targets/prob_hmc_regression.dx.example`

Why it matters:

- is the strongest preserved example of probabilistic programming as a real
  language layer, not just a toy distribution API
- forces composition between probability, effects, and numerical machinery

Closest repo artifacts today:

- `examples/targets/prob_hmc_regression.dx.example`
- `examples/targets/prob_bayesian_regression.dx.example`
- `examples/targets/ad_ppl_combined.dx.example`

Status:

- target-only today

## Bridge: Progressive Python Displacement

`dx` should not begin by claiming to replace Python outright.

The better path is:

1. first-class Python interop
2. typed/native workflow slices that are better than the Python equivalent
3. gradual movement of more data/ML/runtime workflows into `dx`

The end state is not "Python compatibility".
It is progressive displacement of Python in the workflows where `dx` becomes
clearly better.

This sits best as an adoption bridge, not as a fifth semantic layer.

### Target Workflow

Flagship workflow:

- schema-aware and typed-native slices replacing Python incrementally while
  Python remains the foreign ecosystem at the boundary

Why it matters:

- gives `dx` an adoption path that does not require a full ecosystem reset
- keeps the product story practical instead of purely theoretical

Closest repo artifacts today:

- `docs/DX_LANGUAGE_VISION.md`
- `docs/PY_INTEROP_ARCHITECTURE.md`
- the existing `dx-schema` tooling package

Status:

- direction and partial infrastructure exist today
- not yet an end-to-end user-facing migration workflow

## Sequencing

The preferred long-term sequencing is:

1. close and package the native deterministic baseline
2. integrate schema providers into the language
3. integrate the memory model into the language/runtime
4. stabilize the effect/handler surface
5. widen typed data workflows
6. widen ML/inference workflows
7. deepen probabilistic and AD composition
8. use Python interop as a gradual adoption/displacement bridge throughout
9. only then push harder on broader systems-language capabilities

## Communication Rule

Externally, `dx` should not be pitched as:

- a Python clone
- a classic systems language
- an LLM wrapper DSL
- a language that already does everything in this roadmap

Instead, it should be communicated as:

- a native deterministic language
- initially focused on typed effectful data/ML workflows
- with a long-term path toward systems-capable runtime infrastructure,
  probabilistic semantics, and broader Python displacement

## Current Relation To The Near-Term Roadmap

Today, the active implementation tracks are still:

- **F**: schema providers
- **G**: region/shared-buffer memory model

Those are the correct current implementation priorities because they are the
first real steps toward the longer-term stack described here.
