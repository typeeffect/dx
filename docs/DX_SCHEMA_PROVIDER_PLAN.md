# DX Schema Provider Plan

## Purpose

This document defines the direction for compile-time schema acquisition in `dx`.

Schema should be treated as the **first concrete compile-time provider kind**,
not as a standalone one-off subsystem.

Reference:

- `docs/DX_COMPILETIME_PROVIDERS_PLAN.md`

The goal is:

- typesafe access to external data shapes
- explicit compile-time metadata acquisition
- reproducible builds
- clear separation between compile-time schema and runtime data

This is not a plan for arbitrary compile-time data loading.

## Core Rule

`dx` may acquire **schema** at compile time.

`dx` should not implicitly acquire arbitrary **data** at compile time as part of a normal build.

The intended split is:

- compile time: schema and metadata
- runtime: actual data

## Motivation

For typed data applications, the language should eventually support:

- statically known column names
- statically known field types
- nullable information
- early diagnostics for broken projections or field access

Without that, many data-oriented flows fall back to `PyObj` or dynamic wrappers too early.

## Surface Direction

The intended surface is explicit and declarative.

Examples:

```dx
schema Customers = csv.schema("data/customers.csv")
schema Events = parquet.schema("data/events.parquet")
schema Sales = postgres.schema("postgres://...", "select * from sales")
```

Cached artifact form:

```dx
schema Customers = csv.schema("data/customers.csv") using "schemas/customers.dxschema"
```

Refresh should be explicit, not part of normal compilation.

Example direction:

```dx
schema Customers = csv.schema("data/customers.csv") refresh
```

## Type System Direction

Each schema declaration should introduce a nominal typed surface.

Minimal model:

- `Customers.Row`
- typed field access
- nullable information preserved in types

Example:

```dx
fun customer_name(c: Customers.Row) -> Str:
    c'name
.
```

If a field is nullable, the generated type should reflect that explicitly.

## Artifact Model

Compile-time schema acquisition should resolve to a stable artifact, for example:

- `schemas/customers.dxschema`

That artifact should contain:

- schema name
- provider kind
- source fingerprint
- schema fingerprint
- fields
- field types
- field nullability
- provider metadata when needed

The normal build should consume the artifact, not re-query the datasource by default.

## Build Model

Recommended workflow:

1. declare schema dependency in source
2. refresh or generate `.dxschema` explicitly
3. compile using the locked artifact

This keeps the build:

- offline-capable
- reproducible
- reviewable in version control

## Provider Model

Initial provider direction should stay narrow.

Good early targets:

- `csv.schema(path)`
- `parquet.schema(path)`

Deferred targets:

- `postgres.schema(...)`
- remote object storage
- richer database providers

The compiler core should not grow a general network/data access surface just to support this feature.

## Tooling Direction

This should eventually be driven by explicit commands, not hidden compiler behavior.

Example direction:

```bash
dx schema refresh
dx schema refresh schemas/customers.dxschema
```

Normal build should fail clearly if:

- the schema artifact is missing
- the artifact is invalid
- the source declaration and artifact disagree

## Failure Modes

Build-time failures:

- missing artifact
- corrupted artifact
- provider mismatch
- stale or incompatible fingerprint

Refresh-time failures:

- inaccessible file
- missing credentials
- unavailable network
- unsupported schema surface

Runtime failures:

- real datasource no longer matches the locked schema

Optional runtime compatibility checks can be layered later, but they should not weaken compile-time determinism.

## Relation To Python Interop

Schema providers and Python interop solve different problems.

- Python interop gives access to foreign libraries and dynamic objects
- schema providers give `dx` a native typed view of external data shape

The long-term direction should prefer schema providers for typesafe data access, while keeping Python interop available as a foreign escape hatch.

## Relation To The Memory Model

Schema providers and the memory model solve different layers of the system.

- schema providers give `dx` a typed description of external data shape
- the memory model governs how rows, buffers, and future tensors live at runtime

The intended direction is compatible with:

- regions / arenas for temporary query and inference intermediates
- explicit shared buffers for long-lived dataset/model-backed storage

Reference:

- `docs/DX_MEMORY_MODEL_PLAN.md`

## v0 Direction

The first practical slice should be intentionally small:

1. `csv.schema(path)`
2. `.dxschema` artifact
3. nominal `X.Row` type
4. typed field access
5. explicit refresh command

This is enough to validate the architecture without overcommitting to a large provider ecosystem too early.

## Design Reference

Design-validation examples:

- `examples/schema/customer_analysis.dx.example` — intended DX source surface
- `examples/schema/customers.dxschema.example` — draft locked artifact format

### Source Surface Example

The source example shows:

- schema declaration syntax (`schema X = csv.schema(...)`)
- nominal `X.Row` type usage
- field access with genitivo sassone (`it'field`)
- cached artifact form (`using "path.dxschema"`)
- explicit refresh semantics

### Artifact Format Example

The `.dxschema` artifact example shows:

- schema name, provider kind, and source path
- source fingerprint (detects datasource definition changes)
- schema fingerprint (detects actual schema changes)
- field list with types and nullability
- generation timestamp

The artifact is:

- **deterministic**: same source → same artifact
- **reviewable**: plain text, suitable for version control
- **offline-capable**: normal builds consume the artifact, never query the datasource

### Build / Refresh Lifecycle

```
dx schema refresh             # re-query all datasources, update .dxschema artifacts
dx schema refresh path.dxschema  # refresh a single artifact
dx build                      # consume locked artifacts, never query datasources
```

Normal builds fail clearly if:

- the `.dxschema` artifact is missing
- the artifact is invalid or corrupted
- the source declaration and artifact disagree (provider mismatch, name mismatch)

Refresh fails clearly if:

- the datasource is inaccessible
- the schema surface is unsupported

## Current Implementation Status

Schema providers are planned, not implemented.

No parser, type-system, or compiler support exists yet.
The current implementation priority is documented in `docs/DX_IMPLEMENTATION_ROADMAP.md`
under Post-Baseline Milestone F.

## Specification

The draft artifact format specification is at:

- `docs/DX_SCHEMA_ARTIFACT_SPEC.md`
