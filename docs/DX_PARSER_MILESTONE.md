# DX Parser Milestone

## Goal

Bootstrap the smallest parser and AST needed for the v0.1 core.

## Included

- identifiers
- literals
- `fun`
- `val` / `var`
- `if` / `elif` / `else`
- `match`
- member access with `'`
- calls
- `=>`
- `lazy`
- block delimiters `:` and `.`
- `from py ... import ...`
- effect markers `!name`

## Excluded

- query syntax
- `scope:`
- field assignment
- advanced pattern matching
- full operator precedence coverage on day one

## Deliverables

1. token model
2. AST model
3. lexer skeleton
4. parser skeleton
5. smoke tests
