# Schema Guide

## Declare a schema

```dx
schema Customers = csv.schema("data/customers.csv")
```

**works today** — parses in the current compiler.

## What happens

1. The compiler looks for a locked `.dxschema` artifact
2. Without `using "..."`, the default path is `schemas/customers.dxschema`
3. The artifact contains field names, types, and nullability
4. The compiler introduces a typed `Customers.Row` with those fields

## Locked artifacts

A `.dxschema` file looks like:

```toml
[schema]
format_version = "0.1.0"
name = "Customers"
provider = "csv"
source = "data/customers.csv"
source_fingerprint = "sha256:..."
schema_fingerprint = "sha256:..."
generated_at = "2026-03-29T10:00:00Z"

[fields]
id = { type = "Int", nullable = false }
name = { type = "Str", nullable = false }
email = { type = "Str", nullable = true }
```

**works today** — artifact parsing, validation, and CLI tools are functional.

## Nullable fields

Nullable fields become `Option(T)` in the Row type:

```dx
fun greet(c: Customers.Row) -> Str:
    c'name                       // Str (non-nullable)
.

fun contact(c: Customers.Row) -> Option(Str):
    c'email                      // Option(Str) (nullable)
.
```

**works today** — direct field access on schema-backed row values is typed from
the bound artifact, including nullable fields as `Option(T)`.

## Refresh workflow

```bash
# Generate or update the locked artifact for the schema declaration
dx schema refresh input.dx
```

**works today** — first user-facing refresh entrypoint. This is still a
bootstrap slice: field metadata is still passed explicitly under the hood, and
auto-inference from the datasource is not implemented yet.

## Explicit artifact path

```dx
schema Sales = parquet.schema("data/sales.parquet") using "schemas/sales.dxschema"
```

**works today** — `using` overrides the default artifact path.

## Normal builds are offline

Normal builds read the locked artifact. They never query the datasource.
If the artifact is missing or mismatched, the build fails with a clear error.

## End-to-end workflow

```dx
schema Customers = csv.schema("data/customers.csv")

fun contact(c: Customers.Row) -> Option(Str):
    c'email
.
```

**works today** — the declaration parses, `Customers.Row` is materialized from
the bound artifact, and direct field access is typed from schema metadata.

```bash
# 1. Refresh the locked artifact
dx schema refresh input.dx

# 2. Build normally; build stays offline and consumes the locked artifact
cargo run -p dx-llvm-ir --bin dx-build-exec -- input.dx build/
```

**works today** — normal build fails clearly if the locked artifact is missing
or does not match the declaration.

## Supported providers

| Provider | Extension | Status |
|----------|-----------|--------|
| `csv` | `.csv` | works today |
| `parquet` | `.parquet` | works today |
