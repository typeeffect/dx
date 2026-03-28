# Claude Task 01: Lexer and Tokens

## Goal

Complete and harden the v0.1 lexer for the currently approved syntax without changing language semantics.

## Owned Files

- `crates/dx-parser/src/token.rs`
- `crates/dx-parser/src/lexer.rs`

## Files Not To Edit

- `crates/dx-parser/src/ast.rs`
- `crates/dx-parser/src/parser.rs`
- any docs except this task file if strictly necessary

## Required Work

1. complete token coverage for the approved v0.1 syntax surface
2. improve tokenization tests
3. keep token behavior aligned with `spec/DX_V0_1_SPEC.md`

## Important Syntax To Cover

- `:`
- `.`
- `'`
- `=>`
- `->`
- `!name`
- `_`
- `...`
- `(`
- `)`
- `,`
- `=`
- string literals with `"` only
- identifiers
- keywords
- integer literals

## Acceptance Criteria

1. lexer tests cover:
   - member access chains
   - `lazy`
   - function signatures with effects
   - `from py ... import ...`
   - placeholder `_`
   - named arguments
2. no parser or AST semantics are changed
3. `cargo test -p dx-parser` passes

## Non-Goals

- parsing
- precedence
- desugaring
- semantic validation

## Notes

If a syntax case seems underspecified, do not invent semantics.
Leave a clear TODO and keep the implementation conservative.
