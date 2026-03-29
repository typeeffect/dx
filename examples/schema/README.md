# DX Schema Provider Examples

These files are design references for the compile-time schema provider system
(Milestone F). None of them are runnable code — they exist to validate the
design before implementation begins.

## Files

### Source Surface

- **`customer_analysis.dx.example`** — Single-schema analysis (Customers CSV).
- **`sales_analysis.dx.example`** — Mixed CSV + Parquet analysis (Customers + Sales + Orders).
  Uses `using "..."` for locked artifacts.
- **`events_refresh.dx.example`** — Refresh workflow: `schema ... refresh` declaration,
  explicit `dx schema refresh` step, stale detection, mixed locked + refresh mode.

### Locked Artifacts (human-readable, with comments)

- **`customers.dxschema.example`** — CSV provider (Int/Str, nullable). **Not canonical.**
- **`sales.dxschema.example`** — Parquet provider (Int/Float/Str, nullable). **Not canonical.**
- **`orders.dxschema.example`** — Parquet provider (orders table, join-friendly). **Not canonical.**

### Canonical Artifacts (machine-generated, no comments)

- **`customers.dxschema.canonical`** — Canonical form of Customers.
- **`sales.dxschema.canonical`** — Canonical form of Sales.
- **`orders.dxschema.canonical`** — Canonical form of Orders.

The `.example` files are design references with inline documentation.
The `.canonical` files are the machine-generated canonical form that
`dx-schema-validate --check-canonical` accepts.

User-facing command:

- `dx schema refresh` — refresh locked artifacts from source declarations

Internal/bootstrap tooling:

- `dx-schema-new` — bootstrap a canonical artifact from explicit metadata
- `dx-schema-refresh` — low-level source-driven refresh (fields explicit on CLI)
- `dx-schema-validate` — validate or render an artifact
- `dx-schema-match` — check artifact vs expected name/provider/source contract
- `dx-schema-check-source` — check source declaration vs locked artifact
- `scripts/audit_schema_examples.sh` — audit the example package end-to-end

Default artifact path: without `using "..."`, the artifact path is derived from
the schema name: `schema Customers` → `schemas/customers.dxschema`.

The package audit checks that source declarations in `customer_analysis.dx.example`
match the locked `customers` and `sales` artifacts mechanically.

## Specification

The artifact format is specified in:

- `docs/DX_SCHEMA_ARTIFACT_SPEC.md` (draft v0.1.0)

The overall provider design is documented in:

- `docs/DX_SCHEMA_PROVIDER_PLAN.md`

## What `X.Row` Means

When you write `schema Customers = csv.schema("data/customers.csv")`, the
compiler introduces a nominal type `Customers.Row`. This type has:

- **Fields**: one per column in the locked `.dxschema` artifact
- **Types**: `Int`, `Float`, `Str`, `Bool` — fixed by the artifact
- **Nullability**: nullable fields become `Option(T)` in the Row type

`Customers.Row` is **not a dynamic record**. It is a concrete compile-time type
whose shape is fully determined by the locked artifact. If the artifact says
`email` is `Str` and nullable, then `customer'email` has type `Option(Str)` —
the compiler knows this at build time.

```dx
// The compiler sees this as a concrete typed record:
fun greet(c: Customers.Row) -> Str:
    c'name                       // Str (non-nullable)
.

fun contact(c: Customers.Row) -> Option(Str):
    c'email                      // Option(Str) (nullable)
.
```

If the artifact is missing, the build fails. If the artifact's fields don't
match the code, the build fails. The type is only as fresh as the last
`dx schema refresh`.

This is not just a documentation convention — the compiler reads the locked
`.dxschema` artifact and synthesizes `Option(T)` for nullable fields at
build time. Accessing a nullable field without handling `None` is a
compile-time error.

## User-Facing Workflow

```bash
# 1. Write schema declaration in your .dx source
#    schema Customers = csv.schema("data/customers.csv")

# 2. Refresh the locked artifact
dx schema refresh input.dx

# 3. Normal build reads the locked artifact (offline, no datasource access)
dx-build-exec input.dx build/

# 4. Validate the artifact independently
dx-schema-validate schemas/customers.dxschema
```

The `?` suffix on a field type (e.g., `Str?`) means nullable → `Option(Str)`
in the Row type.

## Key Design Points

- **Schema = compile-time metadata**, not runtime data.
- **`.dxschema` = locked artifact**, reviewable in version control.
- **Refresh = explicit** (`dx schema refresh`), not part of normal builds.
- **Field access = genitivo sassone** (`it'field`), not dot notation.
- **Normal builds are offline** — they consume artifacts, never query datasources.
- **`X.Row` = concrete compile-time type**, not a dynamic record.
