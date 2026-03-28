# Claude Task 03: Parser Match and Import Cleanup

## Goal

Complete parser support for:

- `match`
- `from py ... import ...`

within the current v0.1 scope.

## Owned Files

- `crates/dx-parser/src/parser.rs`
- `crates/dx-parser/src/ast.rs`

## Files Not To Edit

- `crates/dx-parser/src/lexer.rs`
- `crates/dx-parser/src/token.rs`
- runtime or checker plans

## Required Work

1. parse `match` with the currently approved minimal pattern surface
2. keep `from py ... import ...` support solid and tested
3. add parser tests for both

## Syntax To Support

```dx
match x:
    Ok(v):
        v
    Err(_):
        0
.
```

and:

```dx
from py pandas import read_csv
from py builtins import print
```

## Acceptance Criteria

1. `match` parses with:
   - simple constructor-like arms
   - `_` wildcard
   - multiple arms
2. import parsing remains stable
3. tests cover both features
4. `cargo test -p dx-parser` passes

## Non-Goals

- advanced pattern matching
- exhaustiveness checking
- effect semantics
- desugaring

## Important Constraint

If a pattern case is not already in the v0.1 spec, do not add it.
Keep the implementation narrow.
