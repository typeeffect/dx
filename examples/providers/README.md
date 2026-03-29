# DX Compile-Time Providers

This package shows the unified compile-time provider lifecycle that DX uses
for both schema acquisition and AD primitive discovery.

The core pattern is the same:

1. **Declare** — a top-level declaration names the provider and its source
2. **Lock** — a `.dx*` artifact captures the acquired metadata
3. **Build** — normal builds consume the locked artifact (offline, deterministic)
4. **Refresh** — an explicit command re-acquires from the source

## Provider Families

### Schema Providers (partially implemented)

Schema providers acquire typed data shapes at compile time.

```
schema Customers = csv.schema("data/customers.csv") using "schemas/customers.dxschema"
```

- **Implemented**: parser support, `.dxschema` artifact format, validator CLIs,
  `dx schema refresh` top-level command, bound schema catalog in HIR,
  nullable fields → `Option(T)` in `X.Row`
- **Default artifact path**: without `using`, derived from schema name
  (`schema Customers` → `schemas/customers.dxschema`)
- **Normal build**: reads locked artifact, validates source ↔ artifact contract,
  fails on missing/mismatched artifacts
- **Not yet**: auto-infer fields from datasource during refresh,
  full `X.Row` field-access type checking in the compiler

Full examples: `examples/schema/`

### AD Primitive Providers (planned)

AD primitive providers acquire differentiable operation definitions at compile
time — either by auto-discovery (symbolic differentiation of the body) or from
explicit backward rules.

```
smooth_primitive sigmoid(x: Float) -> Float:
    1.0 / (1.0 + exp(0.0 - x))
backward(x: Float, dout: Float) -> Float:
    val s = 1.0 / (1.0 + exp(0.0 - x))
    s * (1.0 - s) * dout
.
```

- **Not yet implemented**: compiler integration, artifact format
- **Draft artifacts**:
  - `sigmoid.dxprim.example` — `rule_kind = "explicit_backward"`: user-provided
    backward rule for numerical stability
  - `generated_square.dxprim.example` — `rule_kind = "generated"`: compiler
    auto-derived backward by symbolic differentiation of the forward body
- **Target examples**: `examples/targets/ad_custom_primitive.dx.example`,
  `examples/targets/ad_fused_backward.dx.example`

**Rule kinds**:
- `generated` — the compiler derives the backward rule from the forward body.
  The user writes only the forward; the compiler does the rest during refresh.
- `explicit_backward` — the user provides a hand-written backward rule, typically
  for numerical stability or efficiency. The compiler trusts it.

## Common Lifecycle

```
┌──────────────┐     ┌───────────────┐     ┌──────────────┐
│  Source decl  │────>│  dx refresh   │────>│  .dx* artifact│
│  in .dx file  │     │  (explicit)   │     │  (locked)     │
└──────────────┘     └───────────────┘     └──────┬───────┘
                                                   │
                                           ┌───────▼───────┐
                                           │  dx build     │
                                           │  (offline)    │
                                           │  reads artifact│
                                           └───────────────┘
```

| Step | Schema Provider | AD Primitive Provider |
|------|----------------|----------------------|
| Declaration | `schema X = provider.schema(...)` | `smooth_primitive f(x) ...` |
| Artifact | `.dxschema` (TOML-like) | `.dxprim` (future) |
| Refresh | `dx schema refresh` | `dx prim refresh` (future) |
| Compiler result | Typed `X.Row` with known fields | Typed backward rule for the tape |
| Normal build | Reads `.dxschema`, never queries source | Reads `.dxprim`, never re-derives |

## Implementation Status

| Provider | Parser | Artifact | Validator | Compiler Integration |
|----------|--------|----------|-----------|---------------------|
| Schema | done | done (v0.1.0) | done | not yet |
| AD Primitive | not yet | not yet | not yet | not yet |

## Failure Modes

Providers are designed to fail clearly, not silently.

### Normal Build Failures

| Condition | Result |
|-----------|--------|
| Locked artifact missing | Build error: "artifact not found at `path`" |
| Artifact name ≠ declaration name | Build error: "name mismatch" |
| Artifact provider ≠ declaration provider | Build error: "provider mismatch" |
| Artifact source ≠ declaration source | Build error: "source mismatch" |
| Unsupported `format_version` | Build error: "unsupported format_version" |
| Corrupt or unparseable artifact | Build error with parse diagnostic |

Normal builds **never** query the datasource. If the artifact is missing, the
build fails — it does not silently acquire one.

### Refresh Failures

| Condition | Result |
|-----------|--------|
| Datasource inaccessible | Refresh error: "cannot read source" |
| Schema surface unsupported | Refresh error: "unsupported type" or "unsupported provider" |
| Network/credentials unavailable | Refresh error (schema providers only) |

Refresh is **always explicit** (`dx schema refresh` / `dx prim refresh`).
It is never triggered as a side effect of normal compilation.

### Stale Artifact Warnings

If the source fingerprint in the artifact does not match the current source
declaration, the build warns that the artifact may be stale. This is a
warning, not an error — the build still uses the locked artifact.

To resolve: run refresh, review the diff, commit the updated artifact.

## Design Principle

Both providers follow the same rule: **compile-time metadata, not runtime data.**

- The compiler acquires metadata once (refresh)
- The locked artifact is deterministic and reviewable
- Normal builds are offline — no network, no credentials, no datasource access
- The type system sees the provider result as a concrete type, not a dynamic object
- Providers are deterministic tooling, not dynamic plugin magic
