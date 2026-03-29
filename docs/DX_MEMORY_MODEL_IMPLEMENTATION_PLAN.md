# DX Memory Model Implementation Plan

## Purpose

This document turns the memory-model direction into an implementation roadmap.

It is intentionally narrower than `DX_MEMORY_MODEL_PLAN.md`.
The goal here is to define:

- the first concrete runtime/library slices
- the order they should land in
- the invariants each slice must preserve

## Scope

This plan is for Post-Baseline Milestone G.

It does not attempt to solve:

- a full borrow checker
- arbitrary raw-pointer programming in safe code
- device-runtime integration in the first slice
- a complete tensor type system

## Design Rule

The first implementation wave should support three concepts explicitly:

1. `Arena`
2. `SharedBuffer[T]`
3. `View`/slice types over owned storage

Those concepts are enough to support:

- deterministic temporary allocation
- long-lived shared storage
- future tensor and inference APIs

## Implementation Order

### Slice G1: Arena Runtime Surface

**Status: implemented in `crates/dx-memory`.**

Goal:

- introduce a minimal arena abstraction for temporary memory

Minimum surface:

- `Arena`
- `ArenaBuf[T]`
- explicit arena scope creation/destruction

Good first use cases:

- request-local buffers
- query/intermediate lists
- inference temporary storage

Required invariants:

- arena-owned values cannot outlive the arena
- arena release is bulk, not per-object free
- safe APIs never expose raw arena pointers

Non-goals for G1:

- cross-thread arena sharing
- custom allocator plugins
- tensor semantics

### Slice G2: Shared Buffer Surface

**Status: implemented in `crates/dx-memory`.**

Goal:

- introduce explicit long-lived shared storage

Minimum surface:

- `SharedBuffer[T]`
- cheap clone/share semantics
- deterministic drop

Good first use cases:

- model weights
- reusable immutable lookup tables
- long-lived host buffers
- future tensor backing storage

Required invariants:

- shared ownership is explicit in the type
- safe APIs preserve bounds and initialization
- no manual free in normal user code

Non-goals for G2:

- lock-free tricks
- pervasive atomics by default
- device memory

### Slice G2b: Reusable Buffer Pool

**Status: implemented in `crates/dx-memory`.**

Implemented: `SharedBufferPool<T>`, `PooledBuffer<T>`, acquire/release reuse,
`freeze()` transition to `SharedBuffer<T>`.

Goal:

- reduce allocation pressure in batch/request-scope workloads

The pool preserves allocated capacity across reuse cycles. `freeze()` detaches
a buffer from the pool and converts it to an immutable `SharedBuffer`.

### Slice G3: Views and Slices

**Status: implemented in `crates/dx-memory`.**

Goal:

- add non-owning views over arena/shared storage

Minimum surface:

- `Slice[T]`
- `ArenaView[T]`
- `BufferView[T]`

Required invariants:

- views do not own storage
- safe indexing is bounds-checked
- a view cannot outlive the arena/buffer it depends on

This is the first point where region-style lifetime rules become visible in the
safe model, but they should remain tied to region/storage scopes, not arbitrary
reference graphs.

### Slice G4: Tensor-Oriented Storage Layer

**Status: storage layer and access ergonomics implemented in `crates/dx-memory`.**

Implemented: `TensorStorage<T>`, `TensorView<T>`, `TensorShapeError`,
coordinate access (`get`, `offset_of`), rank-2 row views (`row`).
Not yet implemented: general strided slicing, higher-rank subviews, tensor typing, shape proving, autodiff, device execution.

Goal:

- add the first storage model that ML/inference code can build on

Minimum surface:

- `TensorStorage[T]`
- `TensorView[T]`
- shape metadata kept separate from raw storage ownership

Required invariants:

- storage ownership is explicit
- views remain non-owning
- runtime cost is visible

Non-goals for G4:

- advanced tensor typing
- shape proving
- autodiff
- device-specific execution

### Slice G5: Unsafe / FFI Boundary

**Status: minimally implemented in `crates/dx-memory`.**

Implemented: `ForeignPtr<T>`, `ForeignBuffer<T>`, null rejection, explicit
unsafe constructors, explicit slice exposure.

Goal:

- expose narrow low-level escape hatches without polluting normal code

Minimum surface:

- explicit foreign pointer/handle wrappers
- explicit unsafe allocation/buffer interop points
- C ABI-compatible handle passing where needed

Required invariants:

- safe code still has no raw pointer arithmetic
- unsafe entry points are narrow and named
- arena/shared-buffer invariants are not silently bypassed by default APIs

## Recommended Concrete Types

The exact names can still move, but the conceptual set should stay close to:

- `Arena`
- `ArenaBuf[T]`
- `SharedBuffer[T]`
- `Slice[T]`
- `ArenaView[T]`
- `BufferView[T]`
- `TensorStorage[T]`
- `TensorView[T]`

## Runtime / Compiler Split

This milestone should start runtime/library-first.

Order:

1. runtime/library model
2. explicit safe APIs
3. only then compiler/type-system surface where needed

Reason:

- the ownership/storage model must be real before syntax or checker sugar grows
- this reduces design drift

## Safety Bar

The implementation should remain:

- safer than C/Nim in ordinary code
- simpler than Rust to explain and use

That means the first implementation should prefer:

- scope/region rules
- typed handles
- checked views

over:

- generic lifetime parameters everywhere
- advanced ownership inference
- many overlapping pointer categories

## ML / Inference Fit

The first memory model must explicitly support:

- request/batch arenas for temporaries
- long-lived shared buffers for weights and caches
- cheap non-owning views for tensor-like APIs

This is the minimum viable substrate for inference workloads.

## Exit Criteria For Milestone G

Milestone G is meaningfully started:

- `Arena` exists and is tested (**done**)
- `SharedBuffer[T]` exists and is tested (**done**)
- `BufferView[T]` exists and is tested (**done**)
- the unsafe/FFI boundary exists as a narrow surface (**done**)

Milestone G will be substantially complete when:

- tensor-oriented storage layer exists (**G4 storage done**)
- unsafe/FFI boundary types exist (G5)
- the storage model is good enough to host future tensor APIs (**storage substrate done**)
- tensor typing and shape proving exist in the compiler (not yet)

## References

- `docs/DX_MEMORY_MODEL_PLAN.md`
- `docs/DX_LANGUAGE_VISION.md`
- `docs/DX_IMPLEMENTATION_ROADMAP.md`
