# DX Core Features v0 Alpha

## Status

This document defines the first `dx` core that can be considered closed at the
`v0-alpha` level.

"Closed" here does not mean "feature complete".
It means the repository already has:

- one real compiler pipeline
- one real backend path
- one enforced executable contract
- one runnable subset that is mechanically proven in the repo

This is the baseline on top of which later milestones D/E/F/G expand semantics,
program model, schema-provider support, and the memory model.

Reference:

- `docs/DX_SMALL_CORE_RULES.md`

## Closure Criteria

The `v0-alpha` core is considered closed when all of the following are true:

1. `cargo test --workspace` is green
2. the backend subset is documented and mechanically exercised
3. the native executable contract is explicit and enforced
4. a runnable executable subset is proven end-to-end
5. active, experimental, and unsupported features are clearly separated

The current repository state satisfies this bar.

## Proven Core Features

### Compiler Pipeline

The active compiler pipeline is:

1. lexer
2. parser
3. AST
4. HIR
5. type/effect checked HIR
6. MIR
7. runtime plans
8. low-level codegen
9. LLVM-like lowering
10. real textual LLVM IR emission
11. native executable build/run path

Reference:

- `docs/DX_IMPLEMENTATION_ROADMAP.md`

### Backend

The backend is active and proven through:

- `dx-codegen`
- `dx-llvm`
- `dx-llvm-ir`

This includes:

- ordinary closure-call lowering
- thunk lowering
- nominal `match` lowering before textual LLVM IR
- runtime extern planning
- backend validation before textual IR emission

Reference:

- `docs/DX_TOOLCHAIN_PROVEN_SUBSET.md`

### Native Executable Path

The repository has an active native executable path through:

- `dx-build-exec`
- `dx-run-exec`

This is no longer just planning:

- `.dx -> .ll`
- `.ll -> .bc`
- `.bc -> .o`
- `.o -> executable`
- executable run with observed exit code

Reference:

- `docs/DX_EXECUTION_WORKFLOW.md`

### Executable Contract

The current executable contract is:

- top-level `main`
- zero arguments
- return type `Int`

This contract is enforced.

Reference:

- `docs/DX_EXECUTABLE_ENTRYPOINT_PLAN.md`

### Runnable Subset

The current runnable subset includes active, mechanically proven executable
fixtures such as:

- `main_returns_zero`
- `main_arithmetic`
- `main_closure_call_int`
- `main_closure_call_multi_capture`
- `main_closure_call_nested`
- `main_closure_call_bool`
- `main_thunk_capture`
- `main_thunk_arithmetic`
- `main_thunk_bool`

The proof workflow is:

```bash
scripts/prove_executable_entry_subset.sh
```

The manifest-driven execution proof now checks that runnable demo manifests
match actual end-to-end exit codes.

### Python Boundary

The Python runtime boundary is active at the compiler/backend level:

- function call
- method call
- dynamic call
- `!throw` visibility

This should be considered part of the active backend/core surface, but not a
fully semantic native runtime feature.

### Schema Artifact Tooling

The first slice of schema-provider infrastructure now exists as an experimental
core-adjacent feature:

- `dx-schema` crate
- `.dxschema` parser
- `.dxschema` validator
- `dx-schema-validate` CLI

What exists today:

- artifact parsing
- format validation
- canonical rendering
- JSON rendering

What does not exist yet:

- `schema` keyword in the language
- compiler integration
- `dx schema refresh`
- `X.Row` type synthesis

Reference:

- `docs/DX_SCHEMA_PROVIDER_PLAN.md`
- `docs/DX_SCHEMA_ARTIFACT_SPEC.md`

### Memory-Model Runtime

The memory model has a real runtime crate (`dx-memory`) with:

- `Arena`, `ArenaRef<T>`, `ArenaBuf<T>` — temporary bulk allocation
- `SharedBuffer<T>`, `BufferView<T>` — long-lived shared storage with views
- `SharedBufferPool<T>`, `PooledBuffer<T>` — reusable buffer allocation
- `TensorStorage<T>`, `TensorView<T>` — shaped tensor storage with coordinate access, row views, reshape
- `ForeignPtr<T>`, `ForeignBuffer<T>` — FFI boundary types

What does not exist yet:

- DX language syntax for arenas/regions/tensors
- compiler/type-system integration
- tensor shape typing / proving

Reference:

- `docs/DX_MEMORY_MODEL_PLAN.md`
- `docs/DX_MEMORY_MODEL_IMPLEMENTATION_PLAN.md`
- `examples/memory/README.md`

## Experimental Features

These features are real and under active development, but should not yet be
treated as closed core guarantees:

- richer closure env shapes beyond the currently proven runnable fixtures
- richer thunk/runtime semantics beyond the currently proven runnable fixtures
- broader executable program model beyond `main() -> Int`
- schema-provider language integration
- region/arena and shared-buffer memory integration

## Unsupported or Not Yet Supported

The following are outside the current closed core:

- `main` with arguments
- `main` returning `Unit`
- effectful `main`
- full non-stub runtime semantics for every backend path
- full ADT payload runtime semantics
- parser/compiler support for `schema ... = provider.schema(...)`
- `dx schema refresh`
- DX language syntax for arenas, regions, and tensors
- float literals in the parser

## Working Definition

The right interpretation of `dx v0-alpha core` is:

- the bootstrap compiler is real
- the backend is real
- the executable path is real
- a runnable subset is real

But:

- the language surface is not yet broad
- the runtime is not yet fully semantic
- schema providers are not yet a language feature

## Next Phase

After this closed `v0-alpha` core, the main follow-on milestones are:

- **D**: expand runnable runtime semantics
- **E**: widen the executable program model
- **F**: integrate compile-time schema providers into the language/toolchain
- **G**: integrate the region-based memory model into the language/runtime
