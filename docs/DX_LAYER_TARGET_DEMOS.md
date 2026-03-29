# DX Layer → Target Demo Map

This doc maps each layer of the long-term `dx` stack to the strongest
demo families already present in the repo.

## Foundation

Core compiler pipeline, executable contract, native binary output.

| Demo family | Artifacts | Status |
|-------------|-----------|--------|
| Executable-entry fixtures | `examples/backend/main_*.dx` (12 demos) | **runnable** |
| Manifest-driven proof | `scripts/prove_executable_entry_subset.sh` | **runnable** |
| Backend canonical demos | `examples/backend/*.dx` (28 total) | **runnable** (entry) / tooling-only (non-entry) |

Closest reference: `docs/DX_TOOLCHAIN_PROVEN_SUBSET.md`

## Systems-Capable Runtime

Arena allocation, shared buffers, tensor storage, buffer pooling, FFI boundary.

| Demo family | Artifacts | Status |
|-------------|-----------|--------|
| Memory model API examples | `examples/memory/README.md` | **runnable** (Rust API) |
| Pool integration tests | `crates/dx-memory/tests/pool_integration.rs` | **runnable** |
| Tensor access tests | `crates/dx-memory/tests/tensor_access_integration.rs` | **runnable** |

Closest reference: `docs/DX_MEMORY_MODEL_IMPLEMENTATION_PLAN.md`

## Typed Data

Compile-time schema acquisition, locked artifacts, typed field access.

| Demo family | Artifacts | Status |
|-------------|-----------|--------|
| Schema artifact examples | `examples/schema/*.dxschema.example` | **tooling-only** |
| Canonical artifacts | `examples/schema/*.dxschema.canonical` | **tooling-only** |
| Source surface example | `examples/schema/customer_analysis.dx.example` | **target-only** |
| Schema CLI validation | `dx-schema-validate`, `dx-schema-new`, `dx-schema-match` | **tooling-only** |

Closest reference: `docs/DX_SCHEMA_PROVIDER_PLAN.md`

## ML / Inference

Tensor primitives, attention, training loops with effect-based AD.

| Demo family | Artifacts | Status |
|-------------|-----------|--------|
| Tensor ops | `examples/targets/ml_tensor_ops.dx.example` | **target-only** |
| Causal attention | `examples/targets/ml_causal_attention.dx.example` | **target-only** |
| MNIST training | `examples/targets/ml_mnist_training.dx.example` | **target-only** |

Closest reference: `docs/DX_AD_PPL_DIRECTION.md`

## Probabilistic

Effect-based probabilistic models, inference strategies, HMC.

| Demo family | Artifacts | Status |
|-------------|-----------|--------|
| Basic inference | `examples/targets/prob_basic_inference.dx.example` | **target-only** |
| Bayesian regression | `examples/targets/prob_bayesian_regression.dx.example` | **target-only** |
| HMC (3-effect composition) | `examples/targets/prob_hmc_regression.dx.example` | **target-only** |
| Monty Hall | `examples/targets/prob_monty_hall.dx.example` | **target-only** |
| AD + PPL combined | `examples/targets/ad_ppl_combined.dx.example` | **target-only** |

Closest reference: `docs/DX_FOUNDATIONS_PAPER_TRAIL.md`

## Python Bridge

Foreign boundary for Python ecosystem interop.

| Demo family | Artifacts | Status |
|-------------|-----------|--------|
| py_call backend demos | `examples/backend/py_call_*.dx` (4 demos) | **tooling-only** (emit + plan, not executed) |
| Python interop architecture | `docs/PY_INTEROP_ARCHITECTURE.md` | docs only |

Closest reference: `docs/PY_INTEROP_ARCHITECTURE.md`

## Status Key

| Status | Meaning |
|--------|---------|
| **runnable** | Executes end-to-end via `dx-run-exec` or Rust tests |
| **tooling-only** | Real crate/CLI exists but no language integration |
| **target-only** | `.dx.example` semantic target, not parser-stable |
| **future** | Documented direction, no implementation yet |
