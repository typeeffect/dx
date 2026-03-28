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

## Current Status

The bootstrap compiler pipeline now exists end-to-end up to the pre-LLVM runtime layer.

Current implemented path:

1. lexer
2. parser
3. AST
4. HIR
5. type/effect checked HIR
6. MIR
7. runtime boundary plans
8. unified runtime ops plan

This means the project is no longer in "frontend bootstrap only" mode.
The current implementation focus is backend preparation:

- runtime ABI
- throw/error boundary modeling
- closure environment/runtime shape
- LLVM lowering

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

Status:

- complete

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

Status:

- effectively complete for the v0.1 bootstrap subset

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

Status:

- complete

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

Status:

- complete

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

Status:

- complete for the current v0.1 core

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

Status:

- complete in initial operational form

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

Status:

- complete as a checked compiler feature

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

Status:

- complete as the first backend-oriented IR

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

Status:

- in progress, advanced

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

Current implemented pieces:

- Python runtime hook plan
- closure runtime hook plan
- capture-aware closure lowering
- unified runtime ops plan
- MIR/runtime displays and snapshot coverage

Remaining work in this milestone:

- explicit throw/error runtime boundary model
- extern/runtime symbol table suitable for codegen
- more concrete ABI shape for closure environments
- clearer separation of `Result` data lowering vs `!throw` propagation

## Milestone 10: LLVM Lowering

Status:

- not started as a real backend yet

Goal:

- emit LLVM from MIR plus runtime boundary definitions

This should happen only after the previous milestones exist.

The intended LLVM start point is now:

- MIR
- unified runtime ops plan
- runtime hook signatures
- closure runtime plan
- explicit error-model policy

LLVM lowering should not decide semantics.
It should only translate already-fixed runtime operations and data shapes.

## Milestone 11: Throw and Error Boundary

Goal:

- make the modern error model operational in the backend

Included:

- runtime/ABI representation for `!throw`
- separation of:
  - `Result` data lowering
  - `!throw` propagation
  - `panic` paths
- Python exception mapping into the runtime boundary layer

Exit criteria:

- the compiler/runtime can represent throw-capable calls without ambiguity
- LLVM work does not have to invent exception semantics ad hoc

## Milestone 12: First End-to-End Codegen Skeleton

Goal:

- prove one stable path from typed source to low-level callable form

Included:

- function signature lowering
- runtime hook extern lowering
- lowering of Python runtime ops
- lowering of closure create/call/thunk-call ops
- smoke tests on low-level emitted structure

Exit criteria:

- one narrow but real codegen path exists
- runtime operations survive lowering intact

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

## Immediate Next Task List

Immediate next tasks:

1. define extern/runtime symbol tables for unified runtime ops
2. make `!throw` explicit in the runtime boundary layer
3. separate `Result` lowering from throw-capable call lowering
4. stabilize closure environment ABI shape enough for codegen
5. introduce the first LLVM lowering skeleton for runtime ops
6. add end-to-end tests around runtime ops and low-level lowering

## Success Condition

The current phase is successful when the project has:

- one stable compiler pipeline from parser to unified runtime ops
- one explicit backend story for closures
- one explicit backend story for Python interop
- one explicit backend story for `!throw`
- one narrow but real LLVM/codegen skeleton

At that point, higher-level features can land on a compiler architecture that is already backend-safe.
