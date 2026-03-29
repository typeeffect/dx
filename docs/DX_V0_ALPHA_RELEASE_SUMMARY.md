# DX v0-alpha Release Summary

## What This Is

The `dx` v0-alpha is a working bootstrap compiler that produces real native
executables. It is not a language preview — it is a proven compiler pipeline
with a narrow but real executable subset.

## What Is Closed

### Compiler Pipeline (Milestones A/B/C)

A full 11-stage pipeline from source to native executable:

```
source → lexer → parser → AST → HIR → typed HIR → MIR
→ runtime plans → dx-codegen → dx-llvm → dx-llvm-ir → native executable
```

- Real textual LLVM IR emission
- Verified with `llvm-as` and `opt -passes=verify`
- Built and linked via `dx-build-exec`
- Executed and tested via `dx-run-exec`

### Runnable Executable Subset

12 executable-entry demos proven end-to-end with expected exit codes:

- Pure arithmetic, single/multi-arg closures, multi-capture closures
- Nested closure calls, thunk capture/force, bool closures/thunks
- Manifest-driven execution proof: every listed demo verified against actual output

### Executable Contract

- Top-level `main() -> Int`, zero arguments
- Enforced: wrong return type, wrong arity, missing main, empty source, effectful main — all rejected

### Schema Artifact Tooling

- `dx-schema` crate: parser, validator, canonical/JSON rendering
- `dx-schema-validate` CLI: summary, `--json`, `--canonical`, `--check-canonical`
- `dx-schema-new`, `dx-schema-match` CLIs
- Draft `.dxschema` v0.1.0 artifact format

### Memory Model Runtime

- `dx-memory` crate: Arena, SharedBuffer, BufferView, SharedBufferPool, PooledBuffer
- TensorStorage, TensorView with coordinate access, row views, row ranges, reshape
- ForeignPtr, ForeignBuffer for FFI boundary

## What Is Active

| Area | Status | Reference |
|------|--------|-----------|
| Runnable fixture expansion | Expanding | `scripts/runnable_entry_demos.txt` |
| Schema tooling | Real crate, no language integration | `docs/DX_SCHEMA_PROVIDER_PLAN.md` |
| Memory model | Real crate, no language integration | `docs/DX_MEMORY_MODEL_PLAN.md` |
| Target examples recovery | 18 examples across 6 tranches | `examples/targets/README.md` |

## What Is Not Yet Supported

- Effect/handler syntax in the parser (target examples only)
- `schema` keyword or compiler integration
- Arena/region/tensor language syntax
- AD via effects (target example, not implemented)
- Probabilistic programming (target example, not implemented)
- `main` with arguments, `Unit` return, or effects
- Float literals in the parser
- Full non-stub runtime semantics

## Test Coverage

```
cargo test --workspace    # ~920+ tests, 0 failures
```

Key test layers:

- Parser: 274 tests (operators, precedence, fixtures, error recovery)
- Backend: demo fixtures, IR emission, determinism, symbol coverage
- Integration: black-box `dx-run-exec` and `dx-build-exec` execution
- Manifest proof: exit codes verified against actual execution
- Schema: parse, validate, canonical roundtrip, CLI coverage
- Memory: arena, buffer, pool, tensor storage/access/reshape

## Quick Start

```bash
# Build and run a native executable
cargo run -p dx-llvm-ir --bin dx-run-exec -- --json examples/backend/main_arithmetic.dx

# Prove the runnable subset
scripts/prove_executable_entry_subset.sh

# Validate a schema artifact
cargo run -p dx-schema --bin dx-schema-validate -- examples/schema/customers.dxschema.example

# Run all tests
cargo test --workspace
```

## Documentation Map

| Doc | What it covers |
|-----|----------------|
| `DX_LANGUAGE_VISION.md` | Strategic positioning and thesis |
| `DX_IMPLEMENTATION_ROADMAP.md` | Milestone-by-milestone plan |
| `DX_CORE_FEATURES_v0_ALPHA.md` | What is closed vs experimental |
| `DX_EXECUTION_WORKFLOW.md` | Native executable workflow |
| `DX_TOOLCHAIN_PROVEN_SUBSET.md` | What is mechanically proven |
| `DX_SCHEMA_PROVIDER_PLAN.md` | Schema provider design |
| `DX_SCHEMA_ARTIFACT_SPEC.md` | `.dxschema` format spec |
| `DX_MEMORY_MODEL_PLAN.md` | Memory model direction |
| `DX_FOUNDATIONS_PAPER_TRAIL.md` | Papers → design claims → examples |
| `DX_TARGET_EXAMPLES_RECOVERY.md` | dx-03 recovery tracker |
| `DX_AD_PPL_DIRECTION.md` | AD and probabilistic programming |
