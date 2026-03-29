# DX Memory Model Examples

These examples show the implemented `dx-memory` API (Milestone G).
They are Rust snippets illustrating the real API surface, not DX source code.

## Implemented (G1/G2/G3/G4)

### Arena — Temporary Allocation

```rust
use dx_memory::Arena;

let arena = Arena::new();

// Allocate a single value — stable reference until arena drops
let val = arena.alloc(42_i64);
assert_eq!(*val.get(), 42);

// Allocate a buffer — contiguous slice until arena drops
let buf = arena.alloc_buf(&[1_i64, 2, 3, 4, 5]);
assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);

// Multiple allocations, bulk release at arena drop
let a = arena.alloc(100);
let b = arena.alloc(200);
assert_eq!(arena.allocated_blocks(), 3); // val + buf + a (b shares block)
// All released when `arena` drops — no per-object free
```

### SharedBuffer — Long-Lived Shared Storage

```rust
use dx_memory::SharedBuffer;

// Create from owned data
let buffer = SharedBuffer::from_vec(vec![10_i64, 20, 30, 40, 50]);

// Clone shares storage (reference counted, no copy)
let shared = buffer.clone();
assert_eq!(buffer.strong_count(), 2);
assert_eq!(shared.as_slice(), &[10, 20, 30, 40, 50]);

// Deterministic drop — last owner frees
drop(shared);
assert_eq!(buffer.strong_count(), 1);
```

### BufferView — Non-Owning Views

```rust
use dx_memory::SharedBuffer;

let buffer = SharedBuffer::from_vec(vec![10_i64, 20, 30, 40, 50]);

// Full view — does not own storage
let all = buffer.view();
assert_eq!(all.as_slice(), &[10, 20, 30, 40, 50]);

// Sliced view — zero-copy window
let middle = buffer.slice(1..4);
assert_eq!(middle.as_slice(), &[20, 30, 40]);

// Views are Copy — cheap to pass around
let copy = middle;
assert_eq!(copy.as_slice(), &[20, 30, 40]);
```

### TensorStorage — Shaped Storage Layer

```rust
use dx_memory::{SharedBuffer, TensorStorage};

// Wrap a shared buffer with shape metadata
let data = SharedBuffer::from_vec(vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0]);
let tensor = TensorStorage::new(data, vec![2, 3]).expect("valid shape");

assert_eq!(tensor.shape(), &[2, 3]);
assert_eq!(tensor.rank(), 2);
assert_eq!(tensor.len(), 6);
assert_eq!(tensor.as_slice(), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

// Mutable bulk access is available only when storage is uniquely owned
let mut ints = TensorStorage::new(SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]), vec![2, 2])
    .expect("valid shape");
ints.as_mut_slice().expect("unique")[2] = 30;
assert_eq!(ints.get(&[1, 0]), Some(&30));
```

### TensorView — Non-Owning Tensor View

```rust
use dx_memory::{SharedBuffer, TensorStorage};

let data = SharedBuffer::from_vec(vec![10_i64, 20, 30, 40]);
let tensor = TensorStorage::new(data, vec![2, 2]).expect("valid shape");

// Non-owning view with shape
let view = tensor.view();
assert_eq!(view.shape(), &[2, 2]);
assert_eq!(view.rank(), 2);
assert_eq!(view.as_slice(), &[10, 20, 30, 40]);

// Views are cheap to clone and pass around
let copy = view.clone();
assert_eq!(copy.as_slice(), &[10, 20, 30, 40]);

// Views stay read-only; mutable bulk access remains on uniquely owned storage
```

### TensorShapeError — Shape Validation

```rust
use dx_memory::{SharedBuffer, TensorStorage, TensorShapeError};

// Shape must match element count
let data = SharedBuffer::from_vec(vec![1_i64, 2, 3]);
let err = TensorStorage::new(data, vec![2, 2]).unwrap_err();
assert!(matches!(err, TensorShapeError::ElementCountMismatch { expected: 4, actual: 3 }));

// Empty shape rejected
let data = SharedBuffer::from_vec(vec![1_i64]);
let err = TensorStorage::new(data, vec![]).unwrap_err();
assert!(matches!(err, TensorShapeError::EmptyShape));

// Zero dimension rejected
let data = SharedBuffer::from_vec(vec![1_i64]);
let err = TensorStorage::new(data, vec![1, 0]).unwrap_err();
assert!(matches!(err, TensorShapeError::ZeroDimension));
```

### Tensor Coordinate Access — `get` and `offset_of`

```rust
use dx_memory::{SharedBuffer, TensorStorage};

// 2x3 matrix in row-major order
let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
let tensor = TensorStorage::new(data, vec![2, 3]).expect("valid shape");

// Access by coordinate (row-major indexing)
assert_eq!(tensor.get(&[0, 0]), Some(&1));
assert_eq!(tensor.get(&[0, 2]), Some(&3));
assert_eq!(tensor.get(&[1, 1]), Some(&5));

// Flat offset for manual access
assert_eq!(tensor.offset_of(&[1, 2]), Some(5));

// Out of bounds returns None
assert_eq!(tensor.get(&[2, 0]), None);

// Rank mismatch returns None
assert_eq!(tensor.get(&[0]), None);

// Works on views too
let view = tensor.view();
assert_eq!(view.get(&[1, 0]), Some(&4));
```

### Tensor Row Views — `row`

```rust
use dx_memory::{SharedBuffer, TensorStorage};

// 3x4 matrix
let data = SharedBuffer::from_vec(vec![
    10_i64, 11, 12, 13,
    20, 21, 22, 23,
    30, 31, 32, 33,
]);
let tensor = TensorStorage::new(data, vec![3, 4]).expect("valid shape");

// Extract a row as a rank-1 view (zero-copy)
let row0 = tensor.row(0).expect("valid row");
assert_eq!(row0.as_slice(), &[10, 11, 12, 13]);
assert_eq!(row0.shape(), &[4]);
assert_eq!(row0.rank(), 1);

let row2 = tensor.row(2).expect("valid row");
assert_eq!(row2.as_slice(), &[30, 31, 32, 33]);

// Out-of-bounds row returns None
assert!(tensor.row(3).is_none());

// row() only works on rank-2 tensors
let vec = SharedBuffer::from_vec(vec![1_i64, 2, 3]);
let rank1 = TensorStorage::new(vec, vec![3]).expect("valid shape");
assert!(rank1.row(0).is_none()); // not rank-2
```

### Tensor Row Ranges — `rows`

```rust
use dx_memory::{SharedBuffer, TensorStorage};

let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6, 7, 8]);
let tensor = TensorStorage::new(data, vec![4, 2]).expect("valid shape");

// Extract a contiguous rank-2 block of rows
let rows = tensor.rows(1..3).expect("row range");
assert_eq!(rows.shape(), &[2, 2]);
assert_eq!(rows.as_slice(), &[3, 4, 5, 6]);
assert_eq!(rows.get(&[1, 1]), Some(&6));

// Only contiguous row ranges on rank-2 tensors are supported
assert!(tensor.rows(3..5).is_none());
```

### Tensor Row Splitting — `split_rows`

```rust
use dx_memory::{SharedBuffer, TensorStorage};

let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6, 7, 8]);
let tensor = TensorStorage::new(data, vec![4, 2]).expect("valid shape");

// Split a rank-2 tensor into two contiguous row blocks
let (head, tail) = tensor.split_rows(2).expect("split");
assert_eq!(head.shape(), &[2, 2]);
assert_eq!(tail.shape(), &[2, 2]);
assert_eq!(head.as_slice(), &[1, 2, 3, 4]);
assert_eq!(tail.as_slice(), &[5, 6, 7, 8]);

// Still rank-2 only
assert!(tensor.split_rows(5).is_none());
```

### Tensor Reshape — `reshape`

```rust
use dx_memory::{SharedBuffer, TensorStorage};

let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
let tensor = TensorStorage::new(data, vec![2, 3]).expect("valid shape");

// Reinterpret the same contiguous storage with a new valid shape
let reshaped = tensor.reshape(vec![3, 2]).expect("reshape");
assert_eq!(reshaped.shape(), &[3, 2]);
assert_eq!(reshaped.get(&[2, 1]), Some(&6));

// Element-count mismatch is rejected
assert!(tensor.reshape(vec![4, 2]).is_err());
```

### Tensor Flatten — `flatten`

```rust
use dx_memory::{SharedBuffer, TensorStorage};

let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
let tensor = TensorStorage::new(data, vec![2, 3]).expect("valid shape");

// Reinterpret any contiguous tensor/view as rank-1
let flat = tensor.flatten();
assert_eq!(flat.shape(), &[6]);
assert_eq!(flat.as_slice(), &[1, 2, 3, 4, 5, 6]);
assert_eq!(flat.get(&[4]), Some(&5));
```

### SharedBufferPool — Reusable Buffer Allocation

```rust
use dx_memory::SharedBufferPool;

let pool = SharedBufferPool::<i64>::new();

// Acquire a buffer from the pool (or allocate a new one)
{
    let mut buf = pool.acquire_with_capacity(8);
    buf.extend_from_slice(&[1, 2, 3, 4]);
    assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    assert!(buf.capacity() >= 8);
    // Buffer is returned to pool on drop, cleared but capacity preserved
}
assert_eq!(pool.available_buffers(), 1);

// Reuse: acquire gets the returned buffer (capacity preserved, contents cleared)
let reused = pool.acquire();
assert!(reused.is_empty());         // contents cleared
assert!(reused.capacity() >= 8);    // capacity preserved
```

### PooledBuffer.freeze() — Transition to SharedBuffer

```rust
use dx_memory::SharedBufferPool;

let pool = SharedBufferPool::<i64>::new();

// Build data in a pooled buffer
let mut buf = pool.acquire();
buf.push(10);
buf.push(20);
buf.push(30);

// Freeze into an immutable SharedBuffer — detaches from pool
let shared = buf.freeze();
assert_eq!(shared.as_slice(), &[10, 20, 30]);
assert_eq!(pool.available_buffers(), 0); // buffer was consumed, not returned
```

### Tensor-Aware Pool Flow

```rust
use dx_memory::SharedBufferPool;

let pool = SharedBufferPool::<f32>::new();

// Reserve capacity directly from the intended tensor shape
let mut buf = pool.acquire_for_shape(&[2, 3]).expect("shape");
buf.extend_from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

// Freeze straight into shaped storage
let tensor = buf.freeze_into_tensor(vec![2, 3]).expect("tensor");
assert_eq!(tensor.shape(), &[2, 3]);
assert_eq!(tensor.get(&[1, 2]), Some(&6.0));
```

### Foreign / FFI Boundary

```rust
use dx_memory::{ForeignBuffer, ForeignPtr};

let mut data = vec![10_i64, 20, 30];

// Unsafe entry is explicit and narrow
let ptr = unsafe { ForeignPtr::new(data.as_mut_ptr()) }.expect("ptr");
let mut foreign = ForeignBuffer::from_ptr(ptr, data.len());

unsafe {
    let slice = foreign.as_mut_slice();
    slice[1] = 99;
}

assert_eq!(data, vec![10, 99, 30]);
```

## Edge/Embedded Inference Pattern

This section shows how the current `dx-memory` API maps to edge inference
workflows: pool-backed quantized buffers, shaped tensor creation, and
arena-scoped temporaries.

### Pool-Backed Quantized Inference Buffers

```rust
use dx_memory::{SharedBufferPool, TensorStorage};

// Reuse buffers across inference frames — no per-frame allocation
let pool = SharedBufferPool::<i8>::new();

// Frame 1: acquire, fill with sensor/camera data, freeze into tensor
{
    let mut buf = pool.acquire_for_shape(&[1, 28, 28]).expect("shape");
    // In real code: fill from sensor DMA, camera frame, etc.
    buf.extend_from_slice(&vec![0_i8; 784]);

    let input = buf.freeze_into_tensor(vec![1, 28, 28]).expect("tensor");
    assert_eq!(input.shape(), &[1, 28, 28]);
    assert_eq!(input.len(), 784);

    // Process input... (model inference)
    // input is a SharedBuffer-backed tensor — immutable, shareable
}

// Frame 2: pool gives back the same capacity — zero allocation
{
    let buf = pool.acquire();
    assert!(buf.capacity() >= 784); // capacity preserved from frame 1
}
```

### Arena-Scoped Inference Temporaries

```rust
use dx_memory::{Arena, SharedBuffer, TensorStorage};

let arena = Arena::new();

// Model weights: long-lived, shared across all frames
let weights = SharedBuffer::from_vec(vec![0.1_f32; 16 * 4]);
let w = TensorStorage::new(weights, vec![16, 4]).expect("weights");

// Per-frame: arena-allocated temporaries
let activations = arena.alloc_buf(&[0.0_f32; 16]);
let output = arena.alloc_buf(&[0.0_f32; 4]);

// Use activations and output for one inference step...
assert_eq!(activations.len(), 16);
assert_eq!(output.len(), 4);

// arena.reset() would bulk-release all temporaries
// (In practice, drop the arena scope or call reset between frames)
```

### When to Use What

| Memory primitive | Role in edge inference |
|-----------------|----------------------|
| `Arena` | Per-frame temporaries (activations, scratch buffers) |
| `SharedBuffer` | Long-lived model weights, lookup tables |
| `SharedBufferPool` | Reusable I/O buffers across frames (sensor, camera) |
| `TensorStorage` | Shaped access over any buffer (weights, activations, I/O) |
| `ForeignPtr` / `ForeignBuffer` | Hardware DMA buffers, device memory |

Arena handles bulk-release (no per-object free, no fragmentation).
SharedBuffer handles shared ownership (model weights loaded once, used everywhere).
Pool handles reuse (same capacity across frames, no repeated allocation).

## Not Yet Implemented

- General strided slicing (arbitrary subviews beyond row)
- Higher-rank subviews (e.g., extracting a 2D slice from a 3D tensor)
- DX language-level syntax for arenas/regions/tensors
- Compiler/type-system integration
- Tensor shape typing / proving
- Device-specific execution / storage
- Autodiff over tensor operations
- Richer pool policy (size limits, eviction)
- Tensor-aware pooling beyond shape-capacity reservation and direct freeze into tensor storage

## Specification

- `docs/DX_MEMORY_MODEL_PLAN.md` — design direction
- `docs/DX_MEMORY_MODEL_IMPLEMENTATION_PLAN.md` — implementation roadmap (G1-G5)
