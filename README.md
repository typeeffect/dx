# dx

`dx` is a native, statically typed, effect-aware language for typed data applications.

Current focus:

- small core language
- explicit effects
- Python interop as a foreign boundary
- backend/toolchain convergence
- first executable subset via `dx-llvm-ir` + `dx-runtime-stub`

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
2. complete the real LLVM IR backend subset
3. connect that backend to LLVM tool verification
4. grow the executable path around `dx-runtime-stub`

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

- `docs/PY_INTEROP_ARCHITECTURE.md`
- `docs/DX_AD_PPL_DIRECTION.md`
- `docs/DX_IMPLEMENTATION_ROADMAP.md`
- `docs/DX_REAL_LLVM_BACKEND_PLAN.md`
- `docs/DX_LLVM_TOOLCHAIN_PLAN.md`
