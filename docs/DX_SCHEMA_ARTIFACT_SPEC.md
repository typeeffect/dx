# DX Schema Artifact Specification (Draft)

## Purpose

This document specifies the `.dxschema` artifact format used by the DX
compile-time schema provider system.

A `.dxschema` file is a locked, deterministic representation of an external
data schema. It is produced by `dx schema refresh` and consumed by normal
builds.

## Status

Draft. Not yet implemented.

Partial tooling exists in `crates/dx-schema`:

- `dx-schema-validate`
- `dx-schema-match`
- `dx-schema-new`

These commands validate, match, and bootstrap canonical artifacts from explicit
metadata. They do not replace the future `dx schema refresh` command.

## Format

The artifact uses a TOML-like plain text format with two sections:

### `[schema]` Section

Required fields:

| Field | Type | Description |
|-------|------|-------------|
| `format_version` | string | Artifact format version (currently "0.1.0") |
| `name` | string | Schema name matching the source declaration |
| `provider` | string | Provider kind (`csv`, `parquet`, etc.) |
| `source` | string | Original datasource path or URI |
| `source_fingerprint` | string | Hash of the datasource definition |
| `schema_fingerprint` | string | Hash of the actual schema content |
| `generated_at` | string | ISO 8601 timestamp of generation |

### `[fields]` Section

Each field is a key-value pair:

```toml
field_name = { type = "Type", nullable = bool }
```

Supported types:

| DX Type | Description |
|---------|-------------|
| `Int` | 64-bit integer |
| `Float` | 64-bit floating point |
| `Str` | UTF-8 string |
| `Bool` | Boolean |

Nullable fields generate `Option(T)` in the corresponding `X.Row` type.

## Fingerprints

Two fingerprints serve different purposes:

- **`source_fingerprint`**: Hash of the datasource definition (path, query, etc.).
  Changes when the source declaration in DX code changes.
- **`schema_fingerprint`**: Hash of the actual schema content (fields, types,
  nullability). Changes when the external data shape changes.

Normal builds check both fingerprints. If either is stale, the build warns.

## Build Behavior

### Normal Build (`dx build`)

- Reads the locked `.dxschema` artifact
- Does NOT query the datasource
- Fails if the artifact is missing, invalid, or has a provider mismatch
- Rejects artifacts with an unsupported `format_version`
- Warns if fingerprints suggest the artifact may be stale

### Refresh (`dx schema refresh`)

- Queries the actual datasource
- Regenerates the `.dxschema` artifact
- Updates both fingerprints
- Fails if the datasource is inaccessible or the schema is unsupported

## Versioning

The `format_version` field in `[schema]` tracks the artifact format version. The current version is `0.1.0`. Future format changes will increment this version.

## Examples

Concrete examples of the artifact format:

- `examples/schema/customers.dxschema.example` (CSV provider)
- `examples/schema/sales.dxschema.example` (Parquet provider)

## Relationship to Source Declarations

A source declaration like:

```dx
schema Customers = csv.schema("data/customers.csv")
```

corresponds 1:1 to a `.dxschema` artifact. The `name` field in the artifact
must match the schema name in the source declaration. The `provider` and
`source` fields must match the provider call.

The `using` clause makes the artifact path explicit:

```dx
schema Customers = csv.schema("data/customers.csv") using "schemas/customers.dxschema"
```

Without `using`, the compiler uses a default artifact path derived from the
schema name.
