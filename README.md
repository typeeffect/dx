# dx

`dx` is a native, statically typed, effect-aware language for typed data applications.

Current focus:

- small core language
- explicit effects
- Python interop as a foreign boundary
- compile-time providers moving from schema/artifact tooling toward broader language integration
- region-based memory model moving from runtime crate slices toward language/runtime integration
- keeping the closed backend/toolchain baseline stable
- keeping the strategic target examples package aligned with the language direction
- post-bootstrap roadmap after backend/toolchain closure
- keeping the core language small and regular while pushing capability into providers and layers
- long-term platform direction:
  - LLM-first native core
  - typed data
  - ML/inference
  - probabilistic semantics
  - progressive Python displacement

## Layout

- `docs/`
  - language vision
  - design critique
  - Python interop architecture
  - AD / probabilistic programming direction
  - implementation roadmap
  - parallel development plan
  - HIR plan
- `spec/`
  - v0.1 core spec
- `examples/`
  - long-form design validation examples
  - backend executable demos
- `compiler/`
  - parser, AST, checker, lowering work
- `runtime/`
  - runtime support code

## Current Status

This repository has moved well beyond the parser/bootstrap stage.

The current implementation target is:

1. keep the frontend/type/effect core stable
2. keep the backend/toolchain baseline stable
3. move schema-provider tooling toward language integration
4. move the region/shared-buffer memory model toward broader integration
5. keep the recovered target examples package coherent with the roadmap

## First Milestone

The v0.1 core still starts from:

- `spec/DX_V0_1_SPEC.md`

Long-form language validation examples:

- `examples/DX_LONG_EXAMPLES.md`

Executable backend demo inputs:

- `docs/DX_EXECUTABLE_DEMOS.md`
- `docs/DX_EXECUTION_WORKFLOW.md`
- `docs/DX_TOOLCHAIN_PROVEN_SUBSET.md`

Primary design docs:

- `docs/DX_SMALL_CORE_RULES.md`
- `docs/DX_PROVIDER_CONSTRAINTS.md`
- `docs/DX_COMPILETIME_PROVIDERS_PLAN.md`
- `docs/DX_AD_PRIMITIVE_PROVIDER_PLAN.md`
- `docs/PY_INTEROP_ARCHITECTURE.md`
- `docs/DX_SCHEMA_PROVIDER_PLAN.md`
- `docs/DX_MEMORY_MODEL_PLAN.md`
- `docs/DX_MEMORY_MODEL_IMPLEMENTATION_PLAN.md`
- `docs/DX_LONG_TERM_ROADMAP.md`
- `docs/DX_AD_PPL_DIRECTION.md`
- `docs/DX_TARGET_EXAMPLES_RECOVERY.md`
- `docs/DX_IMPLEMENTATION_ROADMAP.md`
- `docs/DX_REAL_LLVM_BACKEND_PLAN.md`
- `docs/DX_LLVM_TOOLCHAIN_PLAN.md`
