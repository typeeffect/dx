# DX Toolchain-Proven Backend Subset

## Purpose

This document fixes the current backend subset that is considered mechanically
proven at the repository level.

"Proven" here means:

- the source file exists as a canonical backend demo
- `dx-emit-llvm` can emit real textual LLVM IR for it
- `dx-plan-exec` can build an executable plan for it
- the emitted/runtime-facing symbol surface is covered by `dx-runtime-stub`
- the subset can be audited in one pass with:
  - `scripts/audit_backend_demos.sh`

It does **not** mean:

- the full language is executable
- the runtime behavior is fully semantic
- every emitted program is already linked and run through a real LLVM toolchain

## Current Canonical Demo Set

The current canonical demos live in:

- `examples/backend/arithmetic.dx`
- `examples/backend/thunk.dx`
- `examples/backend/closure_call_int.dx`
- `examples/backend/closure_call_str.dx`
- `examples/backend/closure_call_two_args.dx`
- `examples/backend/match_nominal.dx`

## Proven Operations

### Arithmetic

Demo:

- `examples/backend/arithmetic.dx`

Proves:

- plain integer computation
- real textual LLVM IR emission for non-runtime computation
- executable planning without extra runtime symbols

### Thunk Path

Demo:

- `examples/backend/thunk.dx`

Proves:

- closure environment packing
- `dx_rt_closure_create`
- `dx_rt_thunk_call_i64`

### Ordinary Closure Call: Int

Demo:

- `examples/backend/closure_call_int.dx`

Proves:

- ordinary closure call lowering
- per-signature closure symbol emission
- runtime-stub coverage for:
  - `dx_rt_closure_call_i64_1_i64`

### Ordinary Closure Call: Str

Demo:

- `examples/backend/closure_call_str.dx`

Proves:

- pointer-ABI closure argument lowering
- runtime-stub coverage for:
  - `dx_rt_closure_call_ptr_1_ptr`

### Ordinary Closure Call: Two Int Args

Demo:

- `examples/backend/closure_call_two_args.dx`

Proves:

- arity-2 closure call symbol emission
- runtime-stub coverage for:
  - `dx_rt_closure_call_i64_2_i64_i64`

### Nominal Match

Demo:

- `examples/backend/match_nominal.dx`

Proves:

- `match` is lowered before `dx-llvm-ir`
- `dx_rt_match_tag` is visible in emitted IR
- the executable subset supports nominal tag checking for this narrow case

## Audit Command

The canonical audit entrypoint is:

```bash
scripts/audit_backend_demos.sh
```

When local LLVM tools are available:

```bash
scripts/audit_backend_demos.sh --verify
```

This audit checks:

- emission success
- executable planning success
- expected symbols in emitted IR
- expected symbol presence in `dx-runtime-stub`

## Current Limits

This subset is intentionally narrow.

It still does not prove:

- rich runtime semantics for ordinary closure calls
- Python execution
- full ADT payload extraction
- general executable correctness under a real LLVM toolchain
- full non-stub runtime behavior

## Why This Matters

The repository is no longer in a pure compiler-bootstrap phase.

This subset provides a concrete contract for:

- backend development
- toolchain-readiness work
- runtime-stub growth
- future end-to-end executable demos
