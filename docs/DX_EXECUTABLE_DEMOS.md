# DX Executable Demo Inputs

## Purpose

This document defines a small set of source files that exercise the current
backend/runtime subset in a repeatable way.

These demos are intended for:

- `dx-emit-llvm`
- `dx-plan-exec`
- runtime-stub symbol checks
- future end-to-end executable validation

They are deliberately small and backend-oriented.

## Demo Set

### Arithmetic

File:

- `examples/backend/arithmetic.dx`

Behavior:

- simple integer computation
- no runtime hooks beyond the normal function path

### Thunk

File:

- `examples/backend/thunk.dx`

Behavior:

- closure creation
- thunk call ABI

### Ordinary Closure Call: Int

File:

- `examples/backend/closure_call_int.dx`

Behavior:

- closure creation
- ordinary closure call with `Int` argument
- should drive symbols shaped like:
  - `dx_rt_closure_call_i64_1_i64`

### Ordinary Closure Call: Str

File:

- `examples/backend/closure_call_str.dx`

Behavior:

- pointer-return closure call
- pointer argument ABI
- should drive symbols shaped like:
  - `dx_rt_closure_call_ptr_1_ptr`

### Ordinary Closure Call: Two Args

File:

- `examples/backend/closure_call_two_args.dx`

Behavior:

- ordinary closure call with two `Int` args
- should drive a per-signature symbol for arity 2

### Nominal Match

File:

- `examples/backend/match_nominal.dx`

Behavior:

- `match` lowering through `dx_rt_match_tag`
- no raw `MatchBr` should reach `dx-llvm-ir`
- this demo intentionally uses `Unit` arms, because nominal tag checking is
  already in the executable subset while value-producing match merge is still a
  stricter backend case

## Suggested Commands

Or use the top-level `Makefile` shortcuts:

```bash
make demo-plan DEMO=examples/backend/closure_call_int.dx
make demo-emit DEMO=examples/backend/closure_call_two_args.dx
make runtime-stub-info
```

Direct commands:

Emit real textual LLVM IR:

```bash
cargo run -q -p dx-llvm-ir --bin dx-emit-llvm -- examples/backend/closure_call_int.dx
```

Show executable planning:

```bash
cargo run -q -p dx-llvm-ir --bin dx-plan-exec -- examples/backend/closure_call_int.dx
```

Show runtime stub symbol surface:

```bash
cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-symbols
```

Show runtime stub package plan:

```bash
cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-plan -- build/demo.o build/demo
```

Show runtime stub build plan:

```bash
cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-build-plan -- release /tmp/dx-target
```

## Scope Notes

These demos are not meant to cover:

- full runtime semantics
- Python execution
- rich ADT payload extraction
- optimizer behavior

They exist to keep the executable subset concrete while the runtime grows.
