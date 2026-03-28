# DX Real LLVM Backend Plan

## Decision

The next backend step uses **real textual LLVM IR emission** before introducing
LLVM Rust bindings.

This is the current best choice for the project because:

- `dx-llvm` already models most backend decisions explicitly
- LLVM bindings would add integration churn before the IR shape is stable
- textual IR keeps the backend close to the LLVM LangRef
- this makes later adoption of `llvm-sys` or direct toolchain integration a
  tooling problem rather than a semantic redesign

## Chosen Path

1. keep `dx-llvm` as the LLVM-like structural model
2. add `dx-llvm-ir` as a **real IR emitter**
3. emit valid textual LLVM IR for the subset that is already semantically solid
4. only after that:
   - introduce LLVM tool invocation / verification
   - optionally introduce Rust LLVM bindings

## Lowering Strategy

The first real IR emitter is intentionally:

- **stack-based**
- **non-SSA at the source level**
- lowered into LLVM IR using:
  - `alloca`
  - `store`
  - `load`

This is the correct bootstrap path because it avoids needing:

- PHI insertion
- mem2reg-like reasoning in the compiler
- full SSA construction too early

LLVM can optimize this later.

## Current Supported Real-IR Subset

The initial `dx-llvm-ir` layer supports:

- globals for string literals
- extern declarations
- plain functions
- plain assignments
- integer binary ops
- `ret`
- `br`
- `condbr`
- `unreachable`

It does **not** yet fully support:

- `match`
- Python placeholder operands like `%py_*`
- full runtime-hook execution paths
- closure/runtime ABI as executable LLVM

Those are still represented in `dx-llvm`, but the real IR emitter may reject
them until the ABI is fully fixed.

## Design Rule

`dx-llvm-ir` must not invent new semantics.

If a construct is not executable yet with enough fidelity, it should:

- fail explicitly
- document the unsupported feature

instead of silently producing misleading IR.

## Next Steps

1. expand real IR emission to more plain computation cases
2. lower more control flow into real IR
3. make runtime-hook operands concrete enough for real IR emission
4. decide final closure env ABI
5. add LLVM verification/tool integration on top of emitted IR
