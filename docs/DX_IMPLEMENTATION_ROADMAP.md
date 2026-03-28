# DX Implementation Roadmap

## Purpose

This document defines the near-term implementation order for the first `dx` compiler components.

It exists to prevent:

- syntax-first drift
- premature backend work
- multiple parallel representations of the same concepts
- repeated rewrites caused by missing intermediate milestones

## Development Rule

At this stage, development should be driven by:

1. a stable v0.1 core
2. one parser path
3. one desugaring path
4. one checker path
5. one lowering path

No parallel experimental frontends should be introduced unless the current plan proves insufficient.

## Compiler Pipeline

The intended pipeline is:

1. lexer
2. parser
3. AST
4. HIR
5. type/effect checked HIR
6. MIR
7. runtime boundary layer
8. LLVM lowering

## Representation Roles

### AST

Purpose:

- preserve source structure closely
- keep syntax choices visible
- support diagnostics and formatting later

Must still contain:

- `lazy`
- placeholder `_`
- trailing closure surface form if preserved in parsing
- direct syntactic forms like `from py ... import ...`

### HIR

Purpose:

- remove sugar
- normalize callable forms
- normalize imports
- normalize block-level convenience features

HIR should desugar:

- `lazy`
- `_`
- `it`
- trailing closure forms

After HIR, the compiler should no longer need to reason about these surface conveniences directly.

### Type/Effect Checked HIR

Purpose:

- attach resolved names
- attach types
- attach effects
- attach mutability/binding class

This is the semantic source of truth before control-flow lowering.

### MIR

Purpose:

- explicit control flow
- explicit block/branch structure
- explicit call boundaries
- explicit closure/thunk representation
- effect-relevant execution boundaries visible

MIR should be the first representation that is truly codegen-oriented.

## Milestones

## Milestone 1: Lexer and Core Parser

Goal:

- parse the stable v0.1 core surface

Included:

- `fun`
- `val` / `var`
- rebinding of plain names
- `from py ... import ...`
- member access `'`
- calls
- lambdas
- `lazy`
- `if`
- `match`
- named args
- defaultable parameter syntax

Exit criteria:

- parser tests cover these forms
- no placeholder-only parser shells remain for included syntax

## Milestone 2: AST Freeze

Goal:

- stabilize the AST enough that downstream work can begin

Required decisions:

- final AST shape for imports
- final AST shape for `if`
- final AST shape for `match`
- final AST shape for lambda and lazy bodies
- final AST shape for rebinding vs declaration

Exit criteria:

- AST churn becomes the exception, not the norm

## Milestone 3: HIR and Desugaring

Goal:

- create a normalized semantic surface

HIR work:

- desugar `lazy` to zero-ary closure form
- desugar `_` to explicit unary lambda form
- desugar `it` to explicit temporaries
- normalize named-argument call representation

Exit criteria:

- checker runs only on HIR, not raw AST

## Milestone 4: Name Resolution and Bindings

Goal:

- resolve names and binding classes cleanly

Included:

- locals
- function params
- `me`
- `it` after desugaring
- imported Python names

Exit criteria:

- errors for unknown names
- clear symbol table model
- `val` vs `var` tracked semantically

## Milestone 5: Initial Type Checker

Goal:

- type-check the minimal non-effectful core

Included:

- literals
- names
- function calls
- lambdas
- member access shape
- rebinds of `var`
- function return checking

Exit criteria:

- typed AST/HIR or side tables exist
- binding mutability rules enforced

## Milestone 6: Effect Checker

Goal:

- make effects operational in the compiler

Included:

- effect collection on expressions
- function effect declaration checking
- propagation of `!py`
- propagation of `!throw`
- propagation of initial `!io` / `!wait` hooks where needed

Exit criteria:

- effect mismatches become diagnosable
- `!py` boundary is enforced

## Milestone 7: Python Boundary

Goal:

- make `dx -> py` a real checked feature

Included:

- `PyObj`
- import representation for `from py ... import ...`
- placeholder runtime call nodes for Python entry
- clear semantic boundary in HIR/MIR

Exit criteria:

- compiler distinguishes native calls from Python foreign calls
- boundary survives lowering without becoming ad hoc

## Milestone 8: MIR

Goal:

- lower typed/effectful HIR to explicit control-flow form

Included:

- function bodies
- branches
- returns
- closure/thunk representation
- explicit foreign boundary nodes

Exit criteria:

- MIR can become the single source for runtime and LLVM work

## Milestone 9: Runtime Boundary Layer

Goal:

- define the minimal runtime surface before LLVM lowering gets large

Included:

- primitive call ABI
- Python boundary hooks
- error/throw boundary hooks
- closure/thunk calling convention
- distinction between `Result` data lowering, `!throw` propagation, and panic paths

Exit criteria:

- codegen does not invent runtime calls ad hoc

## Milestone 10: LLVM Lowering

Goal:

- emit LLVM from MIR plus runtime boundary definitions

This should happen only after the previous milestones exist.

## What To Defer

Until the above milestones are stable, defer:

- query syntax
- effect handler syntax
- AD implementation
- probabilistic programming implementation
- concurrency syntax beyond effect slots
- advanced tensor typing
- traits / type classes

These remain important, but they must land on top of a stable pipeline.

## First Concrete Task List

Immediate next tasks:

1. finish parsing `if`
2. finish parsing `match`
3. add minimal expression precedence for basic operators
4. freeze AST for Milestone 2
5. create an HIR crate or module
6. implement desugaring for `lazy`, `_`, and `it`

## Success Condition

The first phase is successful when the project has:

- a parser that handles the real v0.1 core
- an AST stable enough to lower
- a real HIR
- a basic type/effect checker
- one explicit path toward MIR and LLVM

At that point, the project can grow safely without multiplying representations or compiler variants.
