# Python Bridge

DX treats Python as a foreign ecosystem, not as the semantic host.

## Import Python libraries

```dx
from py pandas import read_csv
from py sklearn.linear_model import LinearRegression
```

**works today** — parses and emits backend IR for Python calls.

## PyObj boundary

Python values enter DX as `PyObj` — an opaque type:

```dx
fun load_data(path: Str) -> PyObj !py:
    read_csv(path)
.
```

- `PyObj` field access is dynamic: `df'columns` works but is not type-checked
- `!py` marks functions that touch Python — the effect system tracks it

**works today** in parser and backend IR. Runtime execution of Python calls
requires a Python-capable runtime (not yet in the native LLVM backend).

## Why `!py` matters

Every Python call carries the `!py` effect:

```dx
fun native_analysis(data: List(Int)) -> Int:
    // pure DX — no !py needed
    data'sum
.

fun mixed_pipeline(path: Str) -> Int !py !io:
    val df = load_data(path)       // !py: Python call
    val result = native_analysis(extract(df))
    result
.
```

The effect signature tells you exactly where Python is used.
Pure DX functions have no `!py` — they compile to native code with no Python
dependency.

## Incremental migration

The intended adoption path:

1. **Import** — use Python libraries you already know
2. **Wrap** — DX functions call Python at the boundary
3. **Orchestrate** — DX controls the workflow, Python does the heavy lifting
4. **Replace** — move hot paths to native DX as features land

```dx
// Stage 1-2: Python does the work
fun train_model(X: PyObj, y: PyObj) -> PyObj !py:
    val model = LinearRegression()
    model'fit(X, y)
    model
.

// Stage 4 (future): native DX replaces the Python call
// fun train_model(X: Tensor(Float), y: Tensor(Float)) -> Model:
//     least_squares(X, y)
// .
```

**preview syntax** — the migration stages are a design target.
The Python bridge parses today; full runtime interop is not yet in the
native backend.

## What works today

- `from py ... import ...` syntax
- `PyObj` as a type
- `!py` effect annotation
- Backend IR emission for `py_call_function`, `py_call_method`, `py_call_dynamic`

## What is preview syntax

- Runtime Python execution in native binaries
- `PyObj` field access type checking
- Automatic type conversion at the boundary
