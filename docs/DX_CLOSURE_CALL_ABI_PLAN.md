# DX Closure Call ABI Plan

## Purpose

This document fixes the next backend decision for ordinary closure calls:

- how non-thunk closure arguments flow through backend layers
- which runtime hook family is responsible for invocation
- what information must remain explicit before real LLVM IR emission

It exists to prevent the closure-call path from diverging from the thunk path
or inventing backend-only calling semantics in `dx-llvm-ir`.

## Current State

The compiler already preserves ordinary closure-call information through most of
the backend:

- typed HIR distinguishes local closure calls
- MIR preserves call target, args, effects, and result type
- `dx-runtime` preserves `runtime_args` for closure invoke
- `dx-codegen` preserves lowered call args on `LowRuntimeCallKind::ClosureInvoke`
- `dx-llvm` preserves those args in comments for observability

What is still missing is the **actual ABI use** of those args in the runtime
call shape. Today the path remains partly observational.

## Decision

Ordinary closure calls use the same conceptual family already chosen in the
runtime plan:

- `dx_rt_closure_call_i64`
- `dx_rt_closure_call_f64`
- `dx_rt_closure_call_i1`
- `dx_rt_closure_call_ptr`
- `dx_rt_closure_call_void`

But unlike the current bootstrap ABI, these hooks are defined as consuming the
call arguments explicitly, not only metadata.

The closure-call ABI is:

```text
closure_call_ret(closure_handle, arg0, arg1, ...)
```

where:

- the first argument is always the closure handle
- the remaining arguments are the already-lowered call operands
- the return ABI remains specialized by return type

This means:

- thunk calls stay separate:
  - `thunk_call_ret(closure_handle)`
- ordinary closure calls do not use `arg_count` as their main ABI contract
- the actual call arguments must survive to `dx-llvm-ir`

## Design Rules

1. Do not redesign source semantics.
2. Do not change thunk ABI as part of this task.
3. Do not add a varargs-style backend convention.
4. Do not collapse named arguments into new semantics.
5. Do not invent backend-only evaluation behavior.

Evaluation order remains what the language already decided:

- closure operand resolved first
- call arguments already evaluated by existing lowering rules
- backend ABI only transports values that are already semantically fixed

## Layer Responsibilities

### `dx-runtime`

Must:

- preserve call arguments in `LoweredClosureInvocation`
- derive runtime hook requirements from:
  - return ABI
  - ordinary closure vs thunk
- define extern signatures for ordinary closure calls that include:
  - closure handle
  - one parameter per lowered argument

Must not:

- erase the real argument list back into only `arg_count`

### `dx-codegen`

Must:

- lower closure call args into `LowCallArg`
- keep positional/named structure where already present
- provide enough information for the next layer to emit the concrete ABI call

May:

- flatten named args if and only if current semantics already do so earlier

Must not:

- invent packing semantics for ordinary call arguments

### `dx-llvm`

Must:

- lower ordinary closure calls to `CallExtern` with real operands
- keep the closure handle as the first extern arg
- map the remaining closure call args to concrete LLVM-like operands
- validate type coherence against the specialized extern signature

Must not:

- fall back to placeholder `%closure` or comment-only metadata

### `dx-llvm-ir`

Must:

- emit real textual LLVM IR for the supported ordinary closure-call subset
- show the concrete extern call arguments
- stay faithful to the extern signature already decided upstream

Must not:

- infer missing call args from comments
- invent hidden packing for ordinary closure call arguments

## Initial ABI Scope

This milestone covers:

- zero or more ordinary closure call arguments
- return ABI specialization by result type
- local closure calls already recognized by current lowering

This milestone does not require:

- by-reference argument passing
- generic closure ABI
- optimized env/capture layout
- varargs
- foreign closure ABI

## Current Preferred Shape

The preferred current implementation is now the per-arity ABI described in:

- `docs/DX_CLOSURE_CALL_ARITY_PLAN.md`

So ordinary closure calls should use symbols such as:

```text
dx_rt_closure_call_i64_1_i64
dx_rt_closure_call_i64_2_i64_i64
dx_rt_closure_call_ptr_1_ptr
```

This keeps the backend explicit and validator-friendly while still preserving
the chosen real-operand call shape.

## Tests Required

Minimum coverage after implementation:

- ordinary closure call with one `Int` arg
- ordinary closure call with one `Str` arg
- ordinary closure call with multiple positional args
- ordinary closure call in a mixed function with:
  - string globals
  - arithmetic
  - closure env creation
- validator rejects closure extern arg-type mismatch

## Exit Criteria

This task is complete when:

- ordinary closure call args are preserved and used as real ABI operands
- `dx-runtime`, `dx-codegen`, `dx-llvm`, and `dx-llvm-ir` agree on one closure-call ABI
- the path no longer relies on comments to preserve essential closure-call information
