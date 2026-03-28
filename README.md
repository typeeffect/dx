# dx

`dx` is a native, statically typed, effect-aware language for typed data applications.

Current focus:

- small core language
- explicit effects
- deterministic memory direction
- Python interop as a foreign boundary
- query/transform/orchestration as the killer-app direction

## Layout

- `docs/`
  - language vision
  - design critique
  - Python interop architecture
  - AD / probabilistic programming direction
  - implementation roadmap
- `spec/`
  - v0.1 core spec
- `examples/`
  - long-form design validation examples
- `compiler/`
  - parser, AST, checker, lowering work
- `runtime/`
  - runtime support code

## Current Status

This repository is a clean bootstrap for serious `dx` language development.

The immediate implementation target is:

1. parser
2. AST
3. basic type checker
4. effect checker
5. Python foreign boundary
6. then query/runtime work on top of the stabilized core

## First Milestone

Implement the v0.1 core described in:

- `spec/DX_V0_1_SPEC.md`

and validate it against:

- `examples/DX_LONG_EXAMPLES.md`

See also:

- `docs/PY_INTEROP_ARCHITECTURE.md`
- `docs/DX_AD_PPL_DIRECTION.md`
- `docs/DX_IMPLEMENTATION_ROADMAP.md`
