# DX HIR Plan

## Purpose

This document defines the first high-level intermediate representation for `dx`.

HIR is the boundary between:

- syntax-oriented parsing
- semantics-oriented checking and lowering

The main purpose of HIR is to remove surface sugar early and make the rest of the compiler reason about one normalized form.

## Design Rule

After HIR lowering, the compiler should no longer need to reason directly about:

- `lazy` source syntax
- placeholder `_` syntax
- trailing closure surface syntax
- `it` as an implicit temporary

Those are source conveniences, not semantic core constructs.

## Pipeline Position

The planned path is:

1. source text
2. tokens
3. AST
4. HIR
5. typed/effect-checked HIR
6. MIR

HIR should be introduced before full type/effect checking.

## HIR Goals

HIR should:

- normalize declarations
- normalize expressions
- preserve source spans where practical
- make binding structure explicit
- make callable forms uniform

HIR should not yet:

- lower to explicit control-flow graphs
- decide runtime layout
- decide LLVM-level calling conventions

## Core Normalizations

## 1. `lazy`

Source:

```dx
lazy expr
```

HIR:

- explicit zero-parameter closure node

Source:

```dx
lazy:
    stmt1
    expr
.
```

HIR:

- explicit zero-parameter closure with block body

Rule:

- HIR does not preserve `lazy` as a special syntax form
- it becomes the same callable family as other closures, but with zero parameters

## 2. Placeholder `_`

Source:

```dx
_'email
```

AST may preserve placeholder syntax.

HIR must rewrite it to:

```text
lambda(x) => x'email
```

Rule:

- `_` never survives into HIR
- placeholder lifting happens before type checking
- all `_` occurrences in a placeholder-lifted expression become the same unary parameter

## 3. `it`

Source:

```dx
block:
    produce()
    it'use()
.
```

HIR should rewrite this into explicit temporary bindings.

Conceptually:

```text
let $it0 = produce()
$it0'use()
```

Rules:

- `it` never survives into HIR
- each non-`Unit` producer creates an explicit temporary if needed
- scope remains block-local

## 4. Trailing closures

Source:

```dx
orders'filter:
    x => x'active
.
```

HIR:

- ordinary call with the closure in the final argument position

Rule:

- trailing syntax is parser sugar only
- HIR sees ordinary calls only

## HIR Syntax Shape

The first HIR should stay small.

Suggested forms:

- module
- item
- function declaration
- import declaration
- explicit local binding
- explicit rebind
- literal expressions
- name expressions
- member access
- calls
- closures
- `if`
- `match`

## Binding Model in HIR

HIR should already distinguish:

- immutable local binding
- mutable local binding
- rebind operation
- parameter binding

This is important because `val` / `var` semantics should stop being just source syntax at this stage.

## Python Boundary in HIR

HIR should preserve a dedicated representation for Python imports and Python-facing calls.

At minimum:

- imported Python symbol declarations
- call sites that target imported Python symbols

This is needed so later effect checking and lowering can treat `!py` as a real boundary instead of reconstructing it heuristically.

## Open Question Deliberately Left Open

Do imported Python names become:

- ordinary resolved symbols with Python metadata
- or dedicated HIR node variants

This can remain open briefly, but it must be resolved before typed HIR is frozen.

## Immediate Follow-Up

The next concrete compiler work after the current parser milestone should be:

1. finish `if`
2. finish `match`
3. add minimal operator precedence
4. add an HIR module
5. implement desugaring of:
   - `lazy`
   - `_`
   - `it`

## Constraint

Do not add a second normalization layer in parallel.

There should be:

- one AST
- one HIR
- one place where each sugar lowers
