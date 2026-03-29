# DX Memory Model Plan

## Purpose

This document defines the intended memory-management direction for `dx`.

The goal is:

- safer than C and Nim in normal code
- significantly simpler than Rust to use
- suitable for data systems and ML inference workloads
- deterministic in cost and destruction behavior

This is not a plan for a full borrow-checker language.
It is a plan for a layered memory model with clear runtime costs.

## Design Target

The preferred direction is:

1. plain values on the stack / by move
2. region or arena allocation for temporary bulk lifetimes
3. explicit shared buffers for long-lived or shared data

This means `dx` should not depend on:

- tracing GC
- manual `free` in normal user code
- pervasive raw pointers
- Rust-style borrow checking everywhere

## Core Principles

- no tracing GC in the core runtime
- deterministic destruction
- predictable runtime costs
- safe defaults in normal code
- explicit escape hatches for foreign or low-level code

## Safety Goal

Safe `dx` code should rule out the common C/Nim failure modes:

- use-after-free
- double free
- uninitialized reads
- unchecked pointer arithmetic
- out-of-bounds memory access in safe collection APIs

At the same time, the model should avoid the complexity cliff of:

- explicit lifetime parameters everywhere
- a general borrow checker on arbitrary references

## Three-Layer Model

### 1. Plain Values

Small plain values use stack / move semantics.

Examples:

- `Int`
- `Float`
- `Bool`
- small structs without heap-managed payloads

This layer should stay boring and cheap.

### 2. Regions / Arenas

Regions are the main tool for temporary memory.

Good targets:

- request-scope data
- batch-scope data
- query/intermediate transform results
- inference-time temporaries
- short-lived tensor staging buffers

The important property is bulk release:

- allocate many objects
- free them together at region end

This keeps the model simple and predictable for high-throughput workloads.

### 3. Shared Buffers

Some data must outlive a region or be shared:

- model weights
- reused tensors
- caches
- shared lookup tables
- long-lived buffers crossing component boundaries

For this layer, the preferred direction is explicit shared ownership through
reference-counted or handle-managed buffers.

This should be explicit in the type model, not hidden behind "everything is a
shared object".

## ML / Inference Requirements

Inference-heavy workloads need:

- cheap temporary allocation
- bulk cleanup
- stable long-lived model storage
- cheap views/slices into existing buffers
- predictable behavior under repeated batch execution

So the memory model should support at least:

- `Arena`
- `ArenaBuf[T]`
- `Tensor[T]` on shared storage
- `TensorView[T]` as a non-owning view
- future buffer pools and device-aware storage

Arena support is important, but arena alone is not enough.
Inference needs both:

- temporary regions
- shared long-lived storage

## Simplicity Rule

`dx` should use region lifetimes, not general reference lifetimes, as the main
safe mechanism.

Preferred mental model:

- a value belongs to a scope or region
- a view cannot outlive the region it depends on
- long-lived shared data must be placed in an explicit shared container

This is much easier to teach than a full borrow-checker model.

## Foreign / Unsafe Boundary

The low-level escape hatch should remain explicit.

Future low-level capabilities may include:

- raw pointers
- allocator APIs
- device buffers
- C ABI handles

But these should sit behind:

- explicit foreign types
- explicit unsafe operations
- narrow FFI boundaries

Normal `dx` code should not need them.

## Initial v0 Direction

The first practical slice should stay narrow:

1. keep plain values simple
2. introduce an explicit arena/region abstraction
3. keep shared heap storage explicit
4. delay general low-level ownership features

This is enough to support:

- deterministic native execution
- batch/request-scope allocation
- first ML/inference-oriented runtime APIs

## Relationship To Other Milestones

This plan connects directly to:

- Milestone D: richer runtime semantics
- Milestone E: wider executable/runtime model
- Milestone F: schema providers that may feed typed data pipelines

It should become its own follow-on implementation wave after those baseline
areas stabilize.

## Current Status

The first implementation wave (G1/G2/G3) is now in code.

What exists today in `crates/dx-memory`:

- `Arena` — temporary bulk allocation with `alloc` and `alloc_buf`
- `ArenaRef<T>` — stable reference into arena storage
- `ArenaBuf<T>` — contiguous buffer slice in arena storage
- `SharedBuffer<T>` — reference-counted long-lived shared storage
- `BufferView<T>` — non-owning view/slice over shared storage
- `TensorStorage<T>` — shaped storage backed by `SharedBuffer`, with validation
- `TensorView<T>` — non-owning view over tensor storage with shape metadata
- `TensorShapeError` — shape validation errors (empty, zero-dim, mismatch, overflow)
- `SharedBufferPool<T>` — reusable buffer pool with acquire/release
- `PooledBuffer<T>` — mutable buffer from pool, returns on drop, freezes to `SharedBuffer`
- `SharedBufferPool::acquire_for_shape` — shape-aware capacity reservation
- `PooledBuffer::freeze_into_tensor` — direct transition from pooled buffer to `TensorStorage`
- `ForeignPtr<T>` — explicit raw pointer wrapper at the FFI boundary
- `ForeignBuffer<T>` — explicit foreign slice/buffer wrapper with unsafe slice exposure

What does not exist yet:

- DX language-level syntax for arenas/regions/tensors
- compiler/type-system integration
- region checking rules
- tensor shape typing / proving
- device-specific storage
- richer pool policy (size limits, eviction)
- tensor-aware pooling beyond shape-aware reservation and direct tensor freeze

API examples are at `examples/memory/README.md`.

## Recommended Implementation Order

1. document the safe region model
2. introduce a minimal arena runtime surface
3. add explicit shared-buffer handles
4. add views/slices over shared storage
5. only then consider lower-level ownership or unsafe controls

## Implementation Reference

The concrete milestone breakdown for this direction is in:

- `docs/DX_MEMORY_MODEL_IMPLEMENTATION_PLAN.md`
