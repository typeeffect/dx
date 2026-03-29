# Quickstart

## Prerequisites

- Rust toolchain (`cargo`)
- LLVM tools: `llvm-as`, `llc`, `cc`

## Build the toolchain

```bash
cargo build --workspace
```

This produces the DX compiler and CLI tools.

## Write your first program

Create `hello.dx`:

```dx
fun main() -> Int:
    42
.
```

**works today** — `main() -> Int` is the current executable contract.

## Build and run

```bash
# Build a native executable
cargo run -p dx-llvm-ir --bin dx-build-exec -- hello.dx build/

# Build and run, see the exit code as JSON
cargo run -p dx-llvm-ir --bin dx-run-exec -- --json hello.dx build/
```

Output:

```json
{"executable":"build/hello","exit_code":42}
```

## A program with closures

```dx
fun main() -> Int:
    val add = (x: Int, y: Int) => x + y
    add(20, 22)
.
```

**works today** — closures with multiple arguments and captures work end-to-end.

## A program with a thunk

```dx
fun main() -> Int:
    val x = 40 + 2
    val t = lazy x
    t()
.
```

**works today** — `lazy` creates a thunk; `t()` forces it.

## Verify with LLVM tools

```bash
cargo run -p dx-llvm-ir --bin dx-run-exec -- --verify --json hello.dx build/
```

The `--verify` flag runs `llvm-as` and `opt -passes=verify` on the emitted IR
before building.

## Run the full test suite

```bash
cargo test --workspace
```

## What works today

- `fun`, `val`, `var`, lambdas, `lazy`
- Integer arithmetic: `+ - * > < >= <= ==`
- Closures: single-arg, multi-arg, multi-capture, nested calls
- Thunks: capture and force
- Bool via comparison operators
- Native LLVM compilation to binary
- `--json`, `--verify`, `--runtime-archive` CLI flags

## What does not work yet

- Float literals (no `3.14` in the parser)
- Effect/handler syntax (`handle...with`)
- Schema declarations in executable programs
- General `if`/`elif`/`else` and `match` as normal executable-program constructs
- `main` with arguments or `Unit` return
