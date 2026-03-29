# DX Closure Dispatch Plan

## Purpose

This document fixes the next concrete backend/runtime step needed to make
ordinary closure calls actually runnable.

It exists because the current executable-entry milestone has exposed a precise
gap:

- the backend can lower ordinary closure calls faithfully
- the runtime ABI surface exists and links
- but the runtime stub still cannot execute the closure body

The missing piece is explicit closure dispatch identity.

## Current Executable Reality

Today the repository already supports:

- stable executable entrypoints via `main() -> Int`
- a runnable subset with:
  - pure arithmetic
  - captured thunk execution

The main remaining executable blocker is:

- ordinary `closure_call_*`

For example, `main_closure_call_int.dx` emits:

```llvm
declare i64 @dx_rt_closure_call_i64_1_i64(ptr, i64)
declare ptr @dx_rt_closure_create(ptr, i64)
```

and calls:

```llvm
%t1 = call ptr @dx_rt_closure_create(ptr %env, i64 1)
%t3 = call i64 @dx_rt_closure_call_i64_1_i64(ptr %t1, i64 41)
```

That shape preserves:

- env pointer
- arity
- runtime call args

But it still loses:

- which closure body should run

So the runtime can preserve ABI shape but cannot dispatch semantically.

## Root Cause

The current `dx_rt_closure_create` ABI:

```text
dx_rt_closure_create(env_ptr, arity) -> closure_handle
```

does not carry:

- code pointer
- closure-entry symbol identity
- dispatch tag

As a result:

- thunk calls only work when the stub can recover the answer from captured env
- ordinary closure calls cannot execute the closure body at all

This is why `main_thunk_capture.dx` is runnable but
`main_closure_call_int.dx` is not.

## Decision

The next runtime step should make closure dispatch explicit.

Conceptually, a closure value must carry:

```text
ClosureValue {
    code_ptr,
    env_ptr,
    arity,
}
```

This means the runtime creation ABI should evolve from:

```text
closure_create(env_ptr, arity)
```

to something conceptually like:

```text
closure_create(code_ptr, env_ptr, arity)
```

The exact parameter order can be decided later, but `code_ptr` must become
explicit.

## Dispatch Model

Each closure creation site should lower to a dedicated entry stub whose symbol
is stable and backend-generated.

Examples conceptually:

```text
dx_closure_entry_main_bb0_stmt0
dx_closure_entry_force_bb0_stmt0
```

Those entry stubs should have ABI-specialized signatures.

### Ordinary closure entry

```text
ret closure_entry(env_ptr, arg0, arg1, ...)
```

### Thunk entry

```text
ret thunk_entry(env_ptr)
```

The runtime `dx_rt_closure_call_*` / `dx_rt_thunk_call_*` hooks then become
simple dispatchers:

1. unpack `code_ptr`
2. unpack `env_ptr`
3. call the stored entry stub

## Why This Is The Right Next Step

It preserves all the good parts of the current backend:

- per-signature ordinary closure-call externs stay intact
- validator honesty stays intact
- textual LLVM IR remains explicit

And it fixes the one thing the current stub cannot invent:

- body identity

## Non-Goals

This step does not require:

- changing source semantics
- redesigning thunk vs ordinary closure distinction
- GC
- optimizing closure layout
- production runtime design

It is purely the minimum dispatch identity needed for execution.

## Layer Responsibilities

### `dx-runtime`

Must:

- assign a stable closure-entry identity per creation site
- expose creation metadata that includes the future callable entry
- keep ordinary closure and thunk families explicit

### `dx-codegen`

Must:

- preserve enough information to emit the dedicated closure-entry stubs
- keep call args explicit

### `dx-llvm`

Must:

- lower closure-entry stubs explicitly
- pass the callable entry into closure creation
- keep ordinary `closure_call_*` extern signatures unchanged unless a narrow ABI
  tweak is required by the dispatch design

### `dx-llvm-ir`

Must:

- emit the closure-entry functions as real textual LLVM IR
- emit creation/call sites faithfully

### `dx-runtime-stub`

Must:

- store `code_ptr`
- dispatch through it for:
  - `dx_rt_closure_call_*`
  - `dx_rt_thunk_call_*`

## Recommended Implementation Order

1. freeze the dispatch ABI shape
2. thread closure-entry identity through runtime planning
3. emit dedicated closure-entry stubs in backend lowering
4. update `dx_rt_closure_create` to store `code_ptr`
5. make `dx_rt_closure_call_*` dispatch semantically
6. widen the runnable executable subset once ordinary closure calls return real values

## Exit Criteria

This step is complete when:

- `main_closure_call_int.dx` exits with `42`
- ordinary closure-call runtime hooks no longer return default placeholder values
- the runnable executable subset includes at least one ordinary closure-call demo
- backend IR, runtime ABI, and runtime stub all agree on one explicit dispatch model
