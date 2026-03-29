# Language Guide

## Types

```dx
Int       // 64-bit integer
Float     // 64-bit floating point (preview syntax — no literals yet)
Str       // UTF-8 string
Bool      // boolean (produced by comparisons, no true/false literals yet)
Unit      // void / no value
```

**works today**: `Int`, `Str` as type annotations. `Float`, `Bool` as type
annotations on parameters. Integer and string literals.

## Functions

Functions use `fun`, end with `.`, and declare return type after `->`:

```dx
fun double(x: Int) -> Int:
    x + x
.
```

**works today**

## Bindings

```dx
val x = 42          // immutable
var y = 10          // mutable
y = y + 1           // rebind
```

**works today**

## Field Access — Genitivo Sassone

DX uses `'` for field access, not `.`:

```dx
customer'name       // "the customer's name"
sales'revenue'sum   // "the sales' revenue's sum"
```

`.` ends blocks. `'` accesses fields. Two different roles, two different symbols.

**works today** for parser recognition. Field access on typed records is
**preview syntax** (requires schema or type declaration support).

## Lambdas

```dx
val f = (x: Int) => x + 1
val g = (a: Int, b: Int) => a * b
```

Lambdas capture variables from enclosing scope:

```dx
fun main() -> Int:
    val offset = 10
    val add_offset = (x: Int) => x + offset
    add_offset(32)
.
```

**works today** — single and multi-arg, single and multi-capture.

## Lazy / Thunks

`lazy` wraps a value in a zero-arg closure. `t()` forces it:

```dx
val x = 42
val t = lazy x
t()             // 42
```

**works today**

## Operators

```dx
x + y       // addition
x - y       // subtraction
x * y       // multiplication
x > y       // greater than (produces Bool)
x < y       // less than
x >= y      // greater or equal
x <= y      // less or equal
x == y      // equality
```

**works today** — integer arithmetic and comparisons.

## Control Flow

### If / Elif / Else

```dx
if x > 0:
    x
elif x == 0:
    0
else:
    0 - x
.
```

**works today** in parser. Control flow in executable programs is
**preview syntax** for the native backend.

### Match

```dx
match value:
    Pattern1 -> expr1
    Pattern2 -> expr2
.
```

**works today** in parser. Nominal match lowering works in the backend for
simple tag-based patterns.

## Effects

Effects are declared with `!` on function signatures:

```dx
fun read_file(path: Str) -> Str !io:
    ...
.
```

Common effects:

```
!io       // I/O operations
!py       // Python interop boundary
!throw    // may throw an error
!smooth   // differentiable arithmetic (AD)
!prob     // probabilistic sampling
```

**preview syntax** — effect declarations parse, but handler execution is not
yet implemented in the native backend. See [Effects Guide](EFFECTS_GUIDE.md).

## Schema Declarations

```dx
schema Customers = csv.schema("data/customers.csv")
```

**works today** — parses, locked artifact workflow functional.
See [Schema Guide](SCHEMA_GUIDE.md).

## Python Imports

```dx
from py pandas import read_csv
```

**works today** in parser and backend IR emission.
See [Python Bridge](PYTHON_BRIDGE.md).

## Block Terminator

Every block ends with `.` — functions, if/else, match, handlers:

```dx
fun f(x: Int) -> Int:
    x + 1
.                   // <- block end
```

**works today** — `.` terminates blocks in the current parser and toolchain.

This is deliberate: `.` ends a block like a period ends a sentence.
`'` accesses fields like a possessive in English.
