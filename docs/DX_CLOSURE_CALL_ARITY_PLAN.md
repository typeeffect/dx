# DX Closure Call Arity Plan

## Purpose

This document fixes the next backend decision for ordinary closure-call externs.

It exists because the current backend state has exposed a real ABI mismatch:

- call sites pass:
  - `closure_handle, arg0, arg1, ...`
- but the current minimal extern declaration does not describe those operands

That mismatch must be resolved structurally, not hidden in tests.

## Decision

Ordinary closure calls should use **per-signature extern symbols**.

Examples:

```text
dx_rt_closure_call_i64_1_i64(ptr, i64) -> i64
dx_rt_closure_call_i64_2_i64_i64(ptr, i64, i64) -> i64
dx_rt_closure_call_ptr_1_ptr(ptr, ptr) -> ptr
dx_rt_closure_call_void_3_i64_ptr_i1(ptr, i64, ptr, i1) -> void
```

This is the correct next step because it keeps:

- the ABI explicit
- the validator honest
- the textual LLVM IR faithful

## Why Not The Other Options

### Not a single `(ptr)` extern

That leaves real arguments outside the declared ABI and makes validation lie.

### Not `(ptr, arg_count)`

That conflicts with the new real-operand call shape already chosen upstream.

### Not varargs

Varargs adds unnecessary backend looseness at this stage and weakens type
checking where we actually want stronger invariants.

## Scope

This decision applies only to:

- ordinary closure calls with one or more arguments

It does not change:

- thunk ABI
- closure creation ABI
- source semantics
- named-argument surface syntax

Thunks remain:

```text
dx_rt_thunk_call_i64(ptr) -> i64
```

## Symbol Scheme

The symbol must encode:

1. return ABI
2. arity
3. argument ABI sequence

Recommended pattern:

```text
dx_rt_closure_call_<ret>_<arity>_<arg0>_<arg1>_...
```

Where `<ret>` is one of:

- `i64`
- `f64`
- `i1`
- `ptr`
- `void`

Examples:

- `dx_rt_closure_call_i64_1_i64`
- `dx_rt_closure_call_ptr_2_ptr_i64`
- `dx_rt_closure_call_void_3_i64_ptr_i1`

## Signature Rule

For arity `N`, the extern signature is:

```text
(ClosureHandle, Arg0, Arg1, ... ArgN-1) -> Ret
```

This means:

- the first parameter is always the closure handle
- all remaining parameters are the actual lowered operands
- the validator can check both:
  - arg count
  - arg type

## Layer Responsibilities

### `dx-runtime`

Must:

- derive ordinary closure runtime hooks from:
  - return ABI
  - arity
  - lowered argument ABI types
- expose distinct symbols/signatures for each supported call signature

### `dx-codegen`

Must:

- preserve actual lowered call arguments as it already does
- carry enough information for later layers to choose the correct symbol

### `dx-llvm`

Must:

- call the correct per-signature extern
- pass operands matching the declared extern signature
- keep validator checks strict

### `dx-llvm-ir`

Must:

- emit exactly the declared extern signature
- remain mechanical

## Non-Goals

This phase does not require:

- generic closure ABI
- varargs support
- runtime argument packing
- named-arg metadata preservation in the ABI

Named args may still lower positionally if that is already the compiler’s
current semantic choice.

## Tests Required

Minimum coverage after implementation:

- `Int -> Int` closure call with one `Int` arg uses `dx_rt_closure_call_i64_1_i64`
- pointer-return closure call with one pointer arg uses `dx_rt_closure_call_ptr_1_ptr`
- mixed-ABI closure call with arity 2 uses a symbol that encodes both arg ABIs
- validator rejects wrong arity against per-arity extern
- validator rejects wrong argument type against per-arity extern
- thunk ABI remains unchanged

## Exit Criteria

This task is complete when:

- ordinary closure-call extern declarations and call sites agree exactly
- validator can enforce closure-call arg count and type coherently
- there is no longer a structural mismatch hidden behind relaxed tests
