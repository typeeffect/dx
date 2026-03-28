Work in the repository root.

Read these files first:
- docs/DX_CLOSURE_CALL_ABI_PLAN.md
- docs/DX_MATCH_LOWERING_PLAN.md
- docs/DX_REAL_LLVM_BACKEND_PLAN.md
- crates/dx-runtime/src/closure.rs
- crates/dx-codegen/src/low.rs
- crates/dx-llvm/src/lower.rs
- crates/dx-llvm-ir/src/emit.rs

Important current status:
- backend architecture is already fixed through:
  - `dx-runtime`
  - `dx-codegen`
  - `dx-llvm`
  - `dx-llvm-ir`
- frontend / HIR / MIR semantics are off-limits
- do not redesign closure semantics
- do not redesign pattern semantics

Task:
Implement the next backend convergence work across two tightly scoped areas:

1. ordinary closure-call ABI with real call arguments
2. lowering `match` before `dx-llvm-ir`

This is implementation work, not just tests.
Stay conservative and follow the two mini-specs exactly.

You may edit:
- crates/dx-runtime/src/closure.rs
- crates/dx-runtime/src/externs.rs
- crates/dx-runtime/src/ops.rs
- crates/dx-runtime/src/display.rs
- crates/dx-codegen/src/low.rs
- crates/dx-codegen/src/lower.rs
- crates/dx-codegen/src/display.rs
- crates/dx-llvm/src/llvm.rs
- crates/dx-llvm/src/lower.rs
- crates/dx-llvm/src/validate.rs
- crates/dx-llvm/src/display.rs
- crates/dx-llvm-ir/src/emit.rs
- crates/dx-llvm/tests/*
- crates/dx-llvm-ir/tests/*

Do not edit:
- crates/dx-parser/*
- crates/dx-hir/*
- crates/dx-mir/*
- docs/* except if a tiny follow-up note is absolutely necessary

Primary goals:

1. Ordinary closure calls must stop being comment-only at the backend ABI level.
   - preserve actual call operands through lowering
   - use them in the closure-call extern shape
   - keep thunk ABI separate

2. `match` must stop reaching `dx-llvm-ir`.
   - lower it in `dx-llvm`
   - `dx-llvm-ir` should keep rejecting raw `MatchBr`
   - supported match cases should now emit real textual LLVM IR

Closure-call ABI rules to preserve:
- first extern arg is always the closure handle
- following args are the already-lowered call operands
- return ABI remains specialized:
  - `dx_rt_closure_call_i64`
  - `dx_rt_closure_call_f64`
  - `dx_rt_closure_call_i1`
  - `dx_rt_closure_call_ptr`
  - `dx_rt_closure_call_void`
- do not redesign thunk calls

Match-lowering rules to preserve:
- lower `MatchBr` before `dx-llvm-ir`
- `dx-llvm-ir` remains mechanical
- no new pattern semantics
- prefer simple branch lowering over fake sophistication

Non-goals:
- no LLVM toolchain integration yet
- no runtime implementation of hooks
- no parser or type-system redesign
- no new match features
- no new closure semantics

Required coverage:

Closure-call path:
- closure call with one `Int` arg
- closure call with one `Str` arg
- closure call with multiple positional args if supported by the current frontend
- closure call mixed with arithmetic and string globals
- validator catches closure-call extern arg mismatch

Match path:
- a supported match case now emits real textual LLVM IR instead of failing at `dx-llvm-ir`
- `dx-llvm-ir` still rejects manually-constructed raw `MatchBr`
- mixed arithmetic + match case emits supported real IR

Acceptance criteria:
- `cargo test -q` passes
- ordinary closure-call args are used as real ABI operands through the backend path
- supported match no longer blocks real textual IR emission
- summarize exactly what changed
- list touched files
- explicitly note any backend blind spots still left after this pass
