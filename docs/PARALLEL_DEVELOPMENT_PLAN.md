# Parallel Development Plan

## Purpose

This document defines how to split early `dx` compiler work between:

- high-risk semantic work
- lower-risk implementation work

The goal is to let external coding agents such as Claude Code contribute in parallel without destabilizing the core language design.

## Rule of Separation

The delicate parts stay local and tightly controlled.

These include:

- language semantics
- type system rules
- effect system rules
- HIR design
- MIR design
- Python boundary semantics
- runtime boundary semantics
- LLVM lowering strategy

The lower-risk parts can be delegated if they have:

- a stable spec
- a bounded file ownership area
- clear acceptance criteria
- low risk of semantic drift

## Safe Areas for Parallel Work

At the current stage, these are good candidates for delegated work:

1. lexer completion and lexer tests
2. parser expansion for already-approved syntax
3. AST utility code
4. parser error reporting improvements
5. fixture-based parser tests
6. documentation extraction and consistency checks
7. non-semantic cleanup in `py-bridge`

## Areas That Must Stay Centralized

These should not be delegated broadly yet:

1. final `val` / `var` semantics
2. effect inference and checking
3. `!py` semantics
4. desugaring policy for `_`, `lazy`, and `it`
5. HIR design
6. MIR design
7. runtime calling conventions
8. future effect handler syntax
9. AD and probabilistic programming semantics

## Delegation Pattern

Each delegated task should include:

- exact goal
- owned files
- files that must not be edited
- tests to add
- acceptance criteria
- explicit non-goals

## Initial Delegable Tasks

The following tasks are now prepared for parallel implementation:

1. `CLAUDE_TASK_01_LEXER_AND_TOKENS.md`
2. `CLAUDE_TASK_02_PARSER_CONTROL_FLOW.md`
3. `CLAUDE_TASK_03_PARSER_MATCH_AND_IMPORTS.md`

## Merge Rule

Delegated work should land only if:

- it matches the current spec
- tests pass
- it does not change language semantics implicitly
- it does not expand scope beyond the task spec

## Strategic Rule

Parallelism is good for implementation throughput.

Semantic fragmentation is not.

So the model is:

- centralized design
- parallelized bounded implementation
