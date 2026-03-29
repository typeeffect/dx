# Python Interop Architecture

## Purpose

This document fixes the implementation split between:

- `typeeffect/dx`
- `typeeffect/py-bridge`

The goal is:

- Python interop as a first-class language feature
- without polluting the `dx` compiler with Python language semantics

This should stay distinct from compile-time schema providers for typed data sources.
Python interop is the foreign boundary.
Schema providers are a separate native typing feature.

## Product Model

For the user, Python is first-class in `dx`.

That means `dx` should support:

- `from py ... import ...`
- `PyObj`
- `!py`
- Python-backed official wrappers
- diagnostics around Python boundary usage

For the implementation, Python support is a dedicated subsystem, not the semantic core of the language.

That means Python interop should not become the only path for typed datasource access.
Where `dx` needs compile-time knowledge of external data shape, the preferred long-term direction is explicit schema artifacts, not hidden Python execution during compilation.

## Repository Split

## `typeeffect/dx`

Owns:

- `dx` syntax and grammar
- `dx` AST
- `dx` type system
- `dx` effect system
- `dx` parser and checker
- `dx` lowering and runtime
- the surface Python boundary inside the language

Examples:

- parsing `from py pandas import read_csv`
- type-checking `!py`
- typing `PyObj`
- lowering a Python call boundary

## `typeeffect/py-bridge`

Owns:

- Python parsing and semantic analysis
- Python-oriented checker logic
- `.pyi` and stub handling
- extraction of Python API metadata
- wrapper generation inputs
- migration support from Python toward `dx`

Examples:

- inspect Python module exports
- resolve Python signatures and attributes
- emit structured metadata for wrappers
- support future Python-to-DX migration tooling

## Boundary Contract

`py-bridge` should expose structured data that `dx` can consume.

The contract should stay small.

Initial output categories:

- module exports
- callable signatures
- class and attribute metadata
- stub-derived type information
- diagnostics about unsupported or dynamic surfaces

## Integration Modes

The preferred order is:

1. file-based structured output
2. library embedding only if clearly worth it later

That means the first stable integration should be something like:

- `py-bridge inspect module ...`
- `py-bridge inspect package ...`
- output as structured JSON

Then `dx` can:

- consume those results
- generate wrappers
- validate Python import boundaries

## Design Rule

Python is first-class at the language level.

Python is not first-class inside the semantic core implementation of `dx`.

That distinction is intentional and should be preserved.

## Near-Term Plan

### In `dx`

- define `PyObj`
- define `!py`
- define `from py ... import ...`
- define lowering stubs for Python calls

### In `py-bridge`

- stabilize Python checker outputs
- document a machine-readable inspection format
- isolate wrapper-generation-friendly metadata

## Non-Goals

- merging the two compilers into one semantic engine
- making Python the host runtime of `dx`
- forcing a monorepo just because the product feature is first-class

Related planned feature:

- `docs/DX_SCHEMA_PROVIDER_PLAN.md`
