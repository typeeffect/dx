# DX Execution Workflow

## Purpose

This document describes the current practical workflow for the executable
backend subset.

It is intentionally narrow:

- emit textual LLVM IR
- optionally verify it with LLVM tools
- inspect executable planning
- inspect runtime-stub packaging
- prove the currently runnable executable-entry subset

It does not claim that the whole language is executable yet.

## Current Pipeline

Today the backend path is:

```text
source
-> parser
-> HIR
-> typed/effect HIR
-> MIR
-> runtime plans
-> dx-codegen
-> dx-llvm
-> dx-llvm-ir
```

The runtime side already includes:

- `dx-runtime-stub`
- runtime stub symbol manifest
- runtime stub build planning
- runtime stub link planning

## Current Executable Subset

The current canonical demo set lives in `examples/backend/` and currently includes:

- `arithmetic.dx`
- `thunk.dx`
- `closure_call_int.dx`
- `closure_call_str.dx`
- `closure_call_two_args.dx`
- `closure_call_ptr_ret_int_arg.dx`
- `closure_call_ptr_ret_str_int_args.dx`
- `closure_call_void_ret_three_args.dx`
- `closure_call_float.dx`
- `closure_call_bool.dx`
- `match_nominal.dx`
- `match_with_closure_call.dx`
- `py_call_function.dx`
- `py_call_method.dx`
- `py_call_dynamic.dx`
- `py_call_throw.dx`

Within that broader backend set, the current executable-entry fixtures are:

- `main_returns_zero.dx`
- `main_arithmetic.dx`
- `main_closure_call_int.dx`
- `main_closure_call_subtract.dx`
- `main_closure_call_two_args.dx`
- `main_thunk_arithmetic.dx`
- `main_thunk_capture.dx`

All executable-entry demos are now runnable with the current runtime stub:

- `main_returns_zero.dx` (exit code 0)
- `main_arithmetic.dx` (exit code 42)
- `main_closure_call_int.dx` (exit code 42)
- `main_closure_call_subtract.dx` (exit code 42)
- `main_closure_call_two_args.dx` (exit code 42)
- `main_thunk_arithmetic.dx` (exit code 42)
- `main_thunk_capture.dx` (exit code 42)

These demos are documented in:

- `docs/DX_EXECUTABLE_DEMOS.md`
- `docs/DX_TOOLCHAIN_PROVEN_SUBSET.md`

## Canonical Commands

### Audit The Demo Subset

```bash
scripts/audit_backend_demos.sh
```

When local LLVM tools are available, the same audit can force verification:

```bash
scripts/audit_backend_demos.sh --verify
```

### Emit LLVM IR

```bash
cargo run -q -p dx-llvm-ir --bin dx-emit-llvm -- examples/backend/closure_call_int.dx
```

### Emit And Verify With LLVM Tools

```bash
cargo run -q -p dx-llvm-ir --bin dx-emit-llvm -- --verify examples/backend/closure_call_int.dx
```

This only works when LLVM tools are available locally.
The verify path is compatible with both legacy LLVM (`opt -verify`) and
LLVM 16+ (`opt -passes=verify`).

### Show Executable Planning

```bash
cargo run -q -p dx-llvm-ir --bin dx-plan-exec -- examples/backend/closure_call_int.dx
```

### Build And Run A Native Executable

```bash
cargo run -q -p dx-llvm-ir --bin dx-run-exec -- --json examples/backend/main_arithmetic.dx
```

### Prove The Runnable Executable-Entry Subset

```bash
scripts/prove_executable_entry_subset.sh
```

When local LLVM tools are available, the same proof can force verification:

```bash
scripts/prove_executable_entry_subset.sh --verify
```

### Show Consolidated Backend Status

```bash
scripts/report_backend_status.sh
scripts/report_backend_status.sh --json
```

### Show Runtime Stub Symbol Surface

```bash
cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-symbols
```

### Show Runtime Stub Package Plan

```bash
cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-plan -- build/demo.o build/demo
```

### Show Runtime Stub Build Plan

```bash
cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-build-plan -- release /tmp/dx-target
```

## Makefile Shortcuts

The repository root also exposes:

```bash
make demo-plan DEMO=examples/backend/closure_call_int.dx
make demo-emit DEMO=examples/backend/closure_call_two_args.dx
make demo-verify DEMO=examples/backend/closure_call_int.dx
make runtime-stub-info
make runtime-stub-plan
make runtime-stub-build-plan
```

The audit script is the fastest way to check that the current demo subset still
has coherent:

- LLVM IR emission
- executable planning
- runtime-stub symbol coverage

The executable-entry proof script is the fastest way to check that the current
runnable `main() -> Int` subset still:

- builds through the native CLI
- links against the runtime stub
- exits with the expected integer status

The status script is the fastest way to export the same operational state as:

- human-readable Markdown
- machine-readable JSON

## Milestone Status

### Milestone B: Make the Output LLVM-Toolchain-Ready

**Status: closed.**

The current backend subset is:

- emitted as real textual LLVM IR
- verified with real LLVM tools (`llvm-as`, `opt -passes=verify`)
- built and linked through `dx-build-exec` (black-box CLI coverage)
- tested through `dx-run-exec` (black-box execution coverage)

Verify compatibility covers both legacy LLVM and LLVM 16+.

### Milestone C: Execute Through a Real Runtime

**Status: closed for the current executable-entry subset.**

All executable-entry demos are now runnable:

- `main_returns_zero.dx` (exit code 0)
- `main_arithmetic.dx` (exit code 42)
- `main_closure_call_int.dx` (exit code 42)
- `main_closure_call_subtract.dx` (exit code 42)
- `main_closure_call_two_args.dx` (exit code 42)
- `main_thunk_arithmetic.dx` (exit code 42)
- `main_thunk_capture.dx` (exit code 42)

The runnable subset now equals the full executable-entry subset.

## What Is Still Missing

Major remaining steps:

- broader runtime implementation beyond the current stub
- richer closure env/runtime shapes beyond the current runnable subset
- richer match/value flow beyond nominal tag checks
- deciding how to widen executable-program semantics beyond `main() -> Int`

## Success Criterion For This Phase

This phase is successful when a small stable subset can be:

1. emitted to real textual LLVM IR
2. verified when LLVM tools are present
3. linked against the runtime stub surface
4. extended incrementally without changing backend semantics ad hoc
