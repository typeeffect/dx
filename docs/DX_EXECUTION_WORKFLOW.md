# DX Execution Workflow

## Purpose

This document describes the current practical workflow for the executable
backend subset.

It is intentionally narrow:

- emit textual LLVM IR
- optionally verify it with LLVM tools
- inspect executable planning
- inspect runtime-stub packaging

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

The most useful demo inputs live in:

- `examples/backend/arithmetic.dx`
- `examples/backend/thunk.dx`
- `examples/backend/closure_call_int.dx`
- `examples/backend/closure_call_str.dx`
- `examples/backend/closure_call_two_args.dx`
- `examples/backend/match_nominal.dx`

These demos are documented in:

- `docs/DX_EXECUTABLE_DEMOS.md`

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

### Show Executable Planning

```bash
cargo run -q -p dx-llvm-ir --bin dx-plan-exec -- examples/backend/closure_call_int.dx
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

## What Is Still Missing

This workflow is not yet the final execution story.

Major remaining steps:

- broader runtime implementation beyond stubs
- more complete executable coverage for closure call paths
- stronger real-toolchain execution loop
- richer match/value flow beyond nominal tag checks

## Success Criterion For This Phase

This phase is successful when a small stable subset can be:

1. emitted to real textual LLVM IR
2. verified when LLVM tools are present
3. linked against the runtime stub surface
4. extended incrementally without changing backend semantics ad hoc
