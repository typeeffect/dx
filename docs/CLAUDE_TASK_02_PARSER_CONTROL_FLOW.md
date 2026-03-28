# Claude Task 02: Parser Control Flow

## Goal

Implement parser support for approved control-flow syntax in the v0.1 core.

## Owned Files

- `crates/dx-parser/src/parser.rs`
- `crates/dx-parser/src/ast.rs`

## Files Not To Edit

- `crates/dx-parser/src/token.rs`
- `crates/dx-parser/src/lexer.rs`
- type/effect documents

## Required Work

1. parse `if / elif / else`
2. represent them in the existing AST cleanly
3. add parser tests for nested cases

## Syntax To Support

```dx
if cond:
    ...
elif cond2:
    ...
else:
    ...
.
```

There is exactly one closing `.` for the entire `if`.

## Acceptance Criteria

1. parser supports:
   - plain `if`
   - `if + else`
   - `if + elif + else`
   - nested `if`
2. tests demonstrate block termination behavior clearly
3. no changes to language semantics beyond this syntax
4. `cargo test -p dx-parser` passes

## Non-Goals

- `match`
- operator precedence beyond what is minimally required
- effect checking
- lowering

## Important Constraint

Do not invent new AST concepts unless needed.
Prefer the smallest AST extension that matches the current v0.1 spec.
