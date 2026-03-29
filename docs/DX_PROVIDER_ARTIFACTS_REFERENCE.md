# DX Provider Artifacts Reference

## Purpose

This doc compares the two compile-time provider artifact families side by side.
Both follow the same lifecycle: declare → refresh → lock → build.

## Artifact Comparison

|  | `.dxschema` | `.dxprim` (future) |
|--|-------------|-------------------|
| **What it locks** | External data shape (fields, types, nullability) | Differentiable operation (forward body, backward rule) |
| **Source declaration** | `schema X = csv.schema("path")` | `smooth_primitive f(x) ...` |
| **Refresh command** | `dx schema refresh` | `dx prim refresh` (future) |
| **Normal build reads** | Field names, types, nullability | Backward rule, signature |
| **Normal build queries source** | Never | Never |
| **Compiler result** | Typed `X.Row` with known fields | Typed backward rule for the AD tape |
| **Format version** | `0.1.0` | `0.1.0` (draft) |

## `.dxschema` Artifact Shape

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

**Status**: partially implemented (parser, validator, CLIs working).

Examples: `examples/schema/customers.dxschema.example`,
`examples/schema/sales.dxschema.example`,
`examples/schema/orders.dxschema.example`

Spec: `docs/DX_SCHEMA_ARTIFACT_SPEC.md`

## `.dxprim` Artifact Shape (Future Draft)

```toml
[primitive]
format_version = "0.1.0"
name = "sigmoid"
rule_kind = "explicit_backward"
source_fingerprint = "sha256:..."
generated_at = "2026-03-29T16:00:00Z"

[signature]
params = [{ name = "x", type = "Float" }]
return_type = "Float"

[forward]
body = "1.0 / (1.0 + exp(0.0 - x))"

[backward]
params = [{ name = "x", type = "Float" }, { name = "dout", type = "Float" }]
return_type = "Float"
body = "val s = 1.0 / (1.0 + exp(0.0 - x)); s * (1.0 - s) * dout"
```

**Status**: not yet implemented (draft artifact shape only).

Example: `examples/providers/sigmoid.dxprim.example`

## Common Properties

Both artifact families share:

- **Deterministic**: same source → same artifact
- **Reviewable**: plain text, suitable for version control
- **Offline builds**: normal compilation never touches the source
- **Fingerprinted**: source and content fingerprints detect drift
- **Versioned**: `format_version` tracks the artifact format

## Implementation Status

| Artifact | Parser | Format | Validator | Compiler | Refresh CLI |
|----------|--------|--------|-----------|----------|-------------|
| `.dxschema` | done | v0.1.0 | done | not yet | not yet |
| `.dxprim` | not yet | draft | not yet | not yet | not yet |
