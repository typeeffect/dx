//! Integration tests for SharedBufferPool — exercises the documented examples.

use dx_memory::{SharedBufferPool, TensorStorage};

#[test]
fn pool_acquire_release_reuse_cycle() {
    let pool = SharedBufferPool::<i64>::new();

    // First cycle: acquire, fill, drop (returns to pool)
    {
        let mut buf = pool.acquire_with_capacity(16);
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        assert_eq!(buf.len(), 5);
        assert!(buf.capacity() >= 16);
    }
    assert_eq!(pool.available_buffers(), 1);

    // Second cycle: reuse with preserved capacity
    {
        let reused = pool.acquire();
        assert!(reused.is_empty());
        assert!(reused.capacity() >= 16);
    }
    assert_eq!(pool.available_buffers(), 1);
}

#[test]
fn pool_freeze_to_shared_buffer() {
    let pool = SharedBufferPool::<f64>::new();

    let mut buf = pool.acquire();
    buf.push(1.0);
    buf.push(2.0);
    buf.push(3.0);

    let shared = buf.freeze();
    assert_eq!(shared.as_slice(), &[1.0, 2.0, 3.0]);
    // Frozen buffer is not returned to pool
    assert_eq!(pool.available_buffers(), 0);
}

#[test]
fn pool_freeze_into_tensor_storage() {
    let pool = SharedBufferPool::<i64>::new();

    let mut buf = pool.acquire();
    buf.extend_from_slice(&[10, 20, 30, 40, 50, 60]);

    let shared = buf.freeze();
    let tensor = TensorStorage::new(shared, vec![2, 3]).expect("valid shape");

    assert_eq!(tensor.shape(), &[2, 3]);
    assert_eq!(tensor.get(&[1, 2]), Some(&60));
}

#[test]
fn pool_multiple_concurrent_buffers() {
    let pool = SharedBufferPool::<i64>::new();

    let mut a = pool.acquire();
    let mut b = pool.acquire();
    a.push(1);
    b.push(2);

    assert_eq!(a.as_slice(), &[1]);
    assert_eq!(b.as_slice(), &[2]);

    drop(a);
    drop(b);
    assert_eq!(pool.available_buffers(), 2);
}
