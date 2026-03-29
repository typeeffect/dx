use std::any::Any;
use std::borrow::Cow;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::{Range, RangeBounds};
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Default)]
pub struct Arena {
    storage: RefCell<Vec<Box<dyn Any>>>,
}

impl Arena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc<T: 'static>(&self, value: T) -> ArenaRef<'_, T> {
        let boxed = Box::new(value);
        let ptr = NonNull::from(boxed.as_ref());
        self.storage.borrow_mut().push(boxed);
        ArenaRef {
            ptr,
            _marker: PhantomData,
        }
    }

    pub fn alloc_buf<T: Clone + 'static>(&self, values: &[T]) -> ArenaBuf<'_, T> {
        let mut vec = values.to_vec();
        let ptr = if vec.is_empty() {
            NonNull::dangling()
        } else {
            NonNull::new(vec.as_mut_ptr()).expect("vec ptr")
        };
        let len = vec.len();
        self.storage.borrow_mut().push(Box::new(vec));
        ArenaBuf {
            ptr,
            len,
            _marker: PhantomData,
        }
    }

    pub fn allocated_blocks(&self) -> usize {
        self.storage.borrow().len()
    }
}

#[derive(Clone, Copy)]
pub struct ArenaRef<'a, T> {
    ptr: NonNull<T>,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> ArenaRef<'a, T> {
    pub fn get(self) -> &'a T {
        unsafe { self.ptr.as_ref() }
    }
}

#[derive(Clone, Copy)]
pub struct ArenaBuf<'a, T> {
    ptr: NonNull<T>,
    len: usize,
    _marker: PhantomData<&'a [T]>,
}

impl<'a, T> ArenaBuf<'a, T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &'a [T] {
        if self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedBuffer<T> {
    data: Arc<[T]>,
}

impl<T> SharedBuffer<T> {
    pub fn from_vec(values: Vec<T>) -> Self {
        Self {
            data: Arc::from(values),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> Option<&mut [T]> {
        Arc::get_mut(&mut self.data).map(|slice| &mut slice[..])
    }

    pub fn view(&self) -> BufferView<'_, T> {
        BufferView { slice: &self.data }
    }

    pub fn slice(&self, range: Range<usize>) -> BufferView<'_, T> {
        BufferView {
            slice: &self.data[range],
        }
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.data)
    }
}

#[derive(Clone)]
pub struct SharedBufferPool<T> {
    storage: Rc<RefCell<Vec<Vec<T>>>>,
}

impl<T> SharedBufferPool<T> {
    pub fn new() -> Self {
        Self {
            storage: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn acquire(&self) -> PooledBuffer<T> {
        let vec = self.storage.borrow_mut().pop().unwrap_or_default();
        PooledBuffer {
            storage: Rc::clone(&self.storage),
            data: Some(vec),
        }
    }

    pub fn acquire_with_capacity(&self, min_capacity: usize) -> PooledBuffer<T> {
        let mut vec = self.storage.borrow_mut().pop().unwrap_or_default();
        if vec.capacity() < min_capacity {
            vec.reserve(min_capacity - vec.capacity());
        }
        PooledBuffer {
            storage: Rc::clone(&self.storage),
            data: Some(vec),
        }
    }

    pub fn available_buffers(&self) -> usize {
        self.storage.borrow().len()
    }

    pub fn acquire_for_shape(&self, shape: &[usize]) -> Result<PooledBuffer<T>, TensorShapeError> {
        let capacity = element_count(shape)?;
        Ok(self.acquire_with_capacity(capacity))
    }
}

impl<T> Default for SharedBufferPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PooledBuffer<T> {
    storage: Rc<RefCell<Vec<Vec<T>>>>,
    data: Option<Vec<T>>,
}

impl<T> PooledBuffer<T> {
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.data.as_ref().expect("pooled buffer data").capacity()
    }

    pub fn as_slice(&self) -> &[T] {
        self.data.as_ref().expect("pooled buffer data").as_slice()
    }

    pub fn push(&mut self, value: T) {
        self.data.as_mut().expect("pooled buffer data").push(value);
    }

    pub fn clear(&mut self) {
        self.data.as_mut().expect("pooled buffer data").clear();
    }

    pub fn freeze(mut self) -> SharedBuffer<T> {
        SharedBuffer::from_vec(self.data.take().expect("pooled buffer data"))
    }

    pub fn freeze_into_tensor(self, shape: Vec<usize>) -> Result<TensorStorage<T>, TensorShapeError> {
        let expected = element_count(&shape)?;
        let actual = self.len();
        if expected != actual {
            return Err(TensorShapeError::ElementCountMismatch { expected, actual });
        }
        Ok(TensorStorage {
            buffer: self.freeze(),
            shape,
        })
    }
}

impl<T: Clone> PooledBuffer<T> {
    pub fn extend_from_slice(&mut self, values: &[T]) {
        self.data
            .as_mut()
            .expect("pooled buffer data")
            .extend_from_slice(values);
    }
}

impl<T> Drop for PooledBuffer<T> {
    fn drop(&mut self) {
        if let Some(mut vec) = self.data.take() {
            vec.clear();
            self.storage.borrow_mut().push(vec);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferView<'a, T> {
    slice: &'a [T],
}

impl<'a, T> BufferView<'a, T> {
    pub fn len(&self) -> usize {
        self.slice.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    pub fn as_slice(&self) -> &'a [T] {
        self.slice
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignPtr<T> {
    ptr: NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T> ForeignPtr<T> {
    pub unsafe fn new(ptr: *mut T) -> Result<Self, ForeignBufferError> {
        let ptr = NonNull::new(ptr).ok_or(ForeignBufferError::NullPointer)?;
        Ok(Self {
            ptr,
            _marker: PhantomData,
        })
    }

    pub fn as_ptr(self) -> *mut T {
        self.ptr.as_ptr()
    }

    pub fn cast<U>(self) -> ForeignPtr<U> {
        ForeignPtr {
            ptr: self.ptr.cast(),
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignBuffer<T> {
    ptr: NonNull<T>,
    len: usize,
    _marker: PhantomData<T>,
}

impl<T> ForeignBuffer<T> {
    pub unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> Result<Self, ForeignBufferError> {
        let ptr = if len == 0 {
            NonNull::dangling()
        } else {
            NonNull::new(ptr).ok_or(ForeignBufferError::NullPointer)?
        };
        Ok(Self {
            ptr,
            len,
            _marker: PhantomData,
        })
    }

    pub fn from_ptr(ptr: ForeignPtr<T>, len: usize) -> Self {
        Self {
            ptr: ptr.ptr,
            len,
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    pub unsafe fn as_slice<'a>(&self) -> &'a [T] {
        if self.len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(self.ptr.as_ptr(), self.len)
        }
    }

    pub unsafe fn as_mut_slice<'a>(&mut self) -> &'a mut [T] {
        if self.len == 0 {
            &mut []
        } else {
            std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TensorStorage<T> {
    buffer: SharedBuffer<T>,
    shape: Vec<usize>,
}

impl<T> TensorStorage<T> {
    pub fn new(buffer: SharedBuffer<T>, shape: Vec<usize>) -> Result<Self, TensorShapeError> {
        let expected = element_count(&shape)?;
        if expected != buffer.len() {
            return Err(TensorShapeError::ElementCountMismatch {
                expected,
                actual: buffer.len(),
            });
        }
        Ok(Self { buffer, shape })
    }

    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn as_slice(&self) -> &[T] {
        self.buffer.as_slice()
    }

    pub fn as_mut_slice(&mut self) -> Option<&mut [T]> {
        self.buffer.as_mut_slice()
    }

    pub fn offset_of(&self, indices: &[usize]) -> Option<usize> {
        row_major_offset(&self.shape, indices)
    }

    pub fn get(&self, indices: &[usize]) -> Option<&T> {
        self.offset_of(indices).map(|offset| &self.buffer.as_slice()[offset])
    }

    pub fn row(&self, row_index: usize) -> Option<TensorView<'_, T>> {
        self.view().row(row_index)
    }

    pub fn rows<R>(&self, range: R) -> Option<TensorView<'_, T>>
    where
        R: RangeBounds<usize>,
    {
        self.view().rows(range)
    }

    pub fn split_rows(&self, at: usize) -> Option<(TensorView<'_, T>, TensorView<'_, T>)> {
        self.view().split_rows(at)
    }

    pub fn flatten(&self) -> TensorView<'_, T> {
        self.view().flatten()
    }

    pub fn reshape(&self, shape: Vec<usize>) -> Result<TensorView<'_, T>, TensorShapeError> {
        self.view().reshape(shape)
    }

    pub fn view(&self) -> TensorView<'_, T> {
        TensorView {
            slice: self.buffer.as_slice(),
            shape: Cow::Borrowed(&self.shape),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TensorView<'a, T> {
    slice: &'a [T],
    shape: Cow<'a, [usize]>,
}

impl<'a, T> TensorView<'a, T> {
    pub fn shape(&self) -> &[usize] {
        self.shape.as_ref()
    }

    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    pub fn len(&self) -> usize {
        self.slice.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    pub fn as_slice(&self) -> &'a [T] {
        self.slice
    }

    pub fn offset_of(&self, indices: &[usize]) -> Option<usize> {
        row_major_offset(self.shape.as_ref(), indices)
    }

    pub fn get(&self, indices: &[usize]) -> Option<&'a T> {
        self.offset_of(indices).map(|offset| &self.slice[offset])
    }

    pub fn row(&self, row_index: usize) -> Option<TensorView<'a, T>> {
        if self.shape.len() != 2 {
            return None;
        }
        let cols = self.shape[1];
        if row_index >= self.shape[0] {
            return None;
        }
        let start = row_index.checked_mul(cols)?;
        let end = start.checked_add(cols)?;
        Some(TensorView {
            slice: &self.slice[start..end],
            shape: Cow::Owned(vec![cols]),
        })
    }

    pub fn rows<R>(&self, range: R) -> Option<TensorView<'a, T>>
    where
        R: RangeBounds<usize>,
    {
        if self.shape.len() != 2 {
            return None;
        }
        let start = match range.start_bound() {
            std::ops::Bound::Included(&value) => value,
            std::ops::Bound::Excluded(&value) => value.checked_add(1)?,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&value) => value.checked_add(1)?,
            std::ops::Bound::Excluded(&value) => value,
            std::ops::Bound::Unbounded => self.shape[0],
        };
        if start > end || end > self.shape[0] {
            return None;
        }
        let cols = self.shape[1];
        let start_offset = start.checked_mul(cols)?;
        let end_offset = end.checked_mul(cols)?;
        Some(TensorView {
            slice: &self.slice[start_offset..end_offset],
            shape: Cow::Owned(vec![end - start, cols]),
        })
    }

    pub fn split_rows(&self, at: usize) -> Option<(TensorView<'a, T>, TensorView<'a, T>)> {
        if self.shape.len() != 2 || at > self.shape[0] {
            return None;
        }
        let cols = self.shape[1];
        let split = at.checked_mul(cols)?;
        let (left, right) = self.slice.split_at(split);
        Some((
            TensorView {
                slice: left,
                shape: Cow::Owned(vec![at, cols]),
            },
            TensorView {
                slice: right,
                shape: Cow::Owned(vec![self.shape[0] - at, cols]),
            },
        ))
    }

    pub fn flatten(&self) -> TensorView<'a, T> {
        TensorView {
            slice: self.slice,
            shape: Cow::Owned(vec![self.slice.len()]),
        }
    }

    pub fn reshape(&self, shape: Vec<usize>) -> Result<TensorView<'a, T>, TensorShapeError> {
        let expected = element_count(&shape)?;
        let actual = self.slice.len();
        if expected != actual {
            return Err(TensorShapeError::ElementCountMismatch { expected, actual });
        }
        Ok(TensorView {
            slice: self.slice,
            shape: Cow::Owned(shape),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TensorShapeError {
    EmptyShape,
    ZeroDimension,
    ElementCountMismatch { expected: usize, actual: usize },
    SizeOverflow,
}

impl std::fmt::Display for TensorShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TensorShapeError::EmptyShape => write!(f, "tensor shape must not be empty"),
            TensorShapeError::ZeroDimension => write!(f, "tensor shape dimensions must be non-zero"),
            TensorShapeError::ElementCountMismatch { expected, actual } => {
                write!(f, "tensor element count mismatch: expected {expected}, got {actual}")
            }
            TensorShapeError::SizeOverflow => write!(f, "tensor shape size overflow"),
        }
    }
}

impl std::error::Error for TensorShapeError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForeignBufferError {
    NullPointer,
}

impl std::fmt::Display for ForeignBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForeignBufferError::NullPointer => write!(f, "foreign buffer pointer must not be null"),
        }
    }
}

impl std::error::Error for ForeignBufferError {}

fn element_count(shape: &[usize]) -> Result<usize, TensorShapeError> {
    if shape.is_empty() {
        return Err(TensorShapeError::EmptyShape);
    }
    let mut total = 1usize;
    for dim in shape {
        if *dim == 0 {
            return Err(TensorShapeError::ZeroDimension);
        }
        total = total.checked_mul(*dim).ok_or(TensorShapeError::SizeOverflow)?;
    }
    Ok(total)
}

fn row_major_offset(shape: &[usize], indices: &[usize]) -> Option<usize> {
    if shape.len() != indices.len() {
        return None;
    }

    let mut stride = 1usize;
    let mut offset = 0usize;

    for (&dim, &index) in shape.iter().rev().zip(indices.iter().rev()) {
        if index >= dim {
            return None;
        }
        offset = offset.checked_add(index.checked_mul(stride)?)?;
        stride = stride.checked_mul(dim)?;
    }

    Some(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_alloc_returns_stable_ref() {
        let arena = Arena::new();
        let value = arena.alloc(42_i64);

        assert_eq!(*value.get(), 42);
        assert_eq!(arena.allocated_blocks(), 1);
    }

    #[test]
    fn arena_alloc_buf_exposes_slice_contents() {
        let arena = Arena::new();
        let buf = arena.alloc_buf(&[1_i64, 2, 3]);

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.as_slice(), &[1, 2, 3]);
        assert_eq!(arena.allocated_blocks(), 1);
    }

    #[test]
    fn arena_alloc_buf_handles_empty_slice() {
        let arena = Arena::new();
        let buf = arena.alloc_buf::<i64>(&[]);

        assert!(buf.is_empty());
        assert_eq!(buf.as_slice(), &[]);
    }

    #[test]
    fn shared_buffer_clones_share_storage() {
        let a = SharedBuffer::from_vec(vec![1_i64, 2, 3]);
        let b = a.clone();

        assert_eq!(a.as_slice(), &[1, 2, 3]);
        assert_eq!(b.as_slice(), &[1, 2, 3]);
        assert_eq!(a.strong_count(), 2);
        assert_eq!(b.strong_count(), 2);
    }

    #[test]
    fn shared_buffer_view_and_slice_are_non_owning() {
        let buffer = SharedBuffer::from_vec(vec![10_i64, 20, 30, 40]);
        let all = buffer.view();
        let part = buffer.slice(1..3);

        assert_eq!(all.as_slice(), &[10, 20, 30, 40]);
        assert_eq!(part.as_slice(), &[20, 30]);
        assert_eq!(buffer.strong_count(), 1);
    }

    #[test]
    fn shared_buffer_as_mut_slice_requires_unique_ownership() {
        let mut unique = SharedBuffer::from_vec(vec![1_i64, 2, 3]);
        unique.as_mut_slice().expect("unique")[1] = 20;
        assert_eq!(unique.as_slice(), &[1, 20, 3]);

        let mut shared = unique.clone();
        assert!(shared.as_mut_slice().is_none());
    }

    #[test]
    fn shared_buffer_pool_reuses_released_capacity() {
        let pool = SharedBufferPool::<i64>::new();
        {
            let mut buf = pool.acquire_with_capacity(8);
            buf.extend_from_slice(&[1, 2, 3, 4]);
            assert_eq!(buf.len(), 4);
            assert!(buf.capacity() >= 8);
        }

        assert_eq!(pool.available_buffers(), 1);

        let buf = pool.acquire();
        assert!(buf.is_empty());
        assert!(buf.capacity() >= 8);
        assert_eq!(pool.available_buffers(), 0);
    }

    #[test]
    fn shared_buffer_pool_freeze_detaches_buffer_from_pool() {
        let pool = SharedBufferPool::<i64>::new();
        let mut buf = pool.acquire();
        buf.extend_from_slice(&[10, 20, 30]);

        let shared = buf.freeze();

        assert_eq!(shared.as_slice(), &[10, 20, 30]);
        assert_eq!(pool.available_buffers(), 0);
    }

    #[test]
    fn shared_buffer_pool_clears_contents_before_reuse() {
        let pool = SharedBufferPool::<i64>::new();
        {
            let mut buf = pool.acquire();
            buf.extend_from_slice(&[5, 6, 7]);
        }

        let mut reused = pool.acquire();
        assert!(reused.is_empty());
        reused.push(42);
        assert_eq!(reused.as_slice(), &[42]);
    }

    #[test]
    fn shared_buffer_pool_acquire_for_shape_reserves_tensor_capacity() {
        let pool = SharedBufferPool::<i64>::new();
        let buf = pool.acquire_for_shape(&[2, 3]).expect("shape");

        assert!(buf.is_empty());
        assert!(buf.capacity() >= 6);
    }

    #[test]
    fn pooled_buffer_freeze_into_tensor_creates_storage() {
        let pool = SharedBufferPool::<i64>::new();
        let mut buf = pool.acquire_for_shape(&[2, 2]).expect("shape");
        buf.extend_from_slice(&[1, 2, 3, 4]);

        let tensor = buf.freeze_into_tensor(vec![2, 2]).expect("tensor");

        assert_eq!(tensor.shape(), &[2, 2]);
        assert_eq!(tensor.get(&[1, 1]), Some(&4));
    }

    #[test]
    fn pooled_buffer_freeze_into_tensor_rejects_shape_mismatch() {
        let pool = SharedBufferPool::<i64>::new();
        let mut buf = pool.acquire_for_shape(&[2, 2]).expect("shape");
        buf.extend_from_slice(&[1, 2, 3]);

        let err = buf.freeze_into_tensor(vec![2, 2]).expect_err("mismatch");

        assert_eq!(
            err,
            TensorShapeError::ElementCountMismatch {
                expected: 4,
                actual: 3,
            }
        );
        assert_eq!(pool.available_buffers(), 1);
    }

    #[test]
    fn tensor_storage_wraps_shared_buffer_with_shape() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
        let tensor = TensorStorage::new(buffer, vec![2, 2]).expect("tensor");

        assert_eq!(tensor.shape(), &[2, 2]);
        assert_eq!(tensor.rank(), 2);
        assert_eq!(tensor.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn tensor_storage_as_mut_slice_allows_unique_bulk_update() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
        let mut tensor = TensorStorage::new(buffer, vec![2, 2]).expect("tensor");

        let slice = tensor.as_mut_slice().expect("unique tensor storage");
        slice[2] = 30;

        assert_eq!(tensor.as_slice(), &[1, 2, 30, 4]);
        assert_eq!(tensor.get(&[1, 0]), Some(&30));
    }

    #[test]
    fn tensor_storage_as_mut_slice_rejects_shared_storage() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
        let tensor = TensorStorage::new(buffer.clone(), vec![2, 2]).expect("tensor");
        let mut shared_tensor = TensorStorage::new(buffer, vec![2, 2]).expect("tensor");

        assert_eq!(tensor.as_slice(), &[1, 2, 3, 4]);
        assert!(shared_tensor.as_mut_slice().is_none());
    }

    #[test]
    fn tensor_view_is_non_owning_and_reports_shape() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
        let tensor = TensorStorage::new(buffer, vec![2, 3]).expect("tensor");
        let view = tensor.view();

        assert_eq!(view.shape(), &[2, 3]);
        assert_eq!(view.rank(), 2);
        assert_eq!(view.as_slice(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn tensor_storage_get_uses_row_major_indices() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
        let tensor = TensorStorage::new(buffer, vec![2, 3]).expect("tensor");

        assert_eq!(tensor.offset_of(&[0, 0]), Some(0));
        assert_eq!(tensor.offset_of(&[1, 2]), Some(5));
        assert_eq!(tensor.get(&[1, 1]), Some(&5));
    }

    #[test]
    fn tensor_view_get_rejects_rank_mismatch_and_out_of_bounds() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
        let tensor = TensorStorage::new(buffer, vec![2, 2]).expect("tensor");
        let view = tensor.view();

        assert_eq!(view.get(&[0]), None);
        assert_eq!(view.get(&[0, 2]), None);
        assert_eq!(view.offset_of(&[2, 0]), None);
    }

    #[test]
    fn tensor_storage_row_returns_contiguous_2d_subview() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
        let tensor = TensorStorage::new(buffer, vec![2, 3]).expect("tensor");

        let row = tensor.row(1).expect("row");

        assert_eq!(row.shape(), &[3]);
        assert_eq!(row.as_slice(), &[4, 5, 6]);
        assert_eq!(row.get(&[2]), Some(&6));
    }

    #[test]
    fn tensor_row_rejects_wrong_rank_and_out_of_bounds() {
        let matrix = TensorStorage::new(SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]), vec![2, 2])
            .expect("matrix");
        let vector =
            TensorStorage::new(SharedBuffer::from_vec(vec![1_i64, 2, 3]), vec![3]).expect("vector");

        assert!(matrix.row(2).is_none());
        assert!(vector.row(0).is_none());
    }

    #[test]
    fn tensor_storage_rows_returns_contiguous_rank2_subview() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6, 7, 8]);
        let tensor = TensorStorage::new(buffer, vec![4, 2]).expect("tensor");

        let rows = tensor.rows(1..3).expect("rows");

        assert_eq!(rows.shape(), &[2, 2]);
        assert_eq!(rows.as_slice(), &[3, 4, 5, 6]);
        assert_eq!(rows.get(&[1, 1]), Some(&6));
    }

    #[test]
    fn tensor_storage_split_rows_returns_two_contiguous_rank2_subviews() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6, 7, 8]);
        let tensor = TensorStorage::new(buffer, vec![4, 2]).expect("tensor");

        let (head, tail) = tensor.split_rows(1).expect("split");

        assert_eq!(head.shape(), &[1, 2]);
        assert_eq!(tail.shape(), &[3, 2]);
        assert_eq!(head.as_slice(), &[1, 2]);
        assert_eq!(tail.as_slice(), &[3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn tensor_rows_reject_wrong_rank_and_out_of_bounds() {
        let matrix = TensorStorage::new(SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]), vec![2, 2])
            .expect("matrix");
        let vector =
            TensorStorage::new(SharedBuffer::from_vec(vec![1_i64, 2, 3]), vec![3]).expect("vector");

        assert!(matrix.rows(1..3).is_none());
        assert!(matrix.rows(2..1).is_none());
        assert!(matrix.split_rows(3).is_none());
        assert!(vector.rows(0..1).is_none());
        assert!(vector.split_rows(0).is_none());
    }

    #[test]
    fn tensor_storage_reshape_reinterprets_contiguous_shape() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
        let tensor = TensorStorage::new(buffer, vec![2, 3]).expect("tensor");

        let reshaped = tensor.reshape(vec![3, 2]).expect("reshape");

        assert_eq!(reshaped.shape(), &[3, 2]);
        assert_eq!(reshaped.get(&[2, 1]), Some(&6));
    }

    #[test]
    fn tensor_storage_flatten_returns_rank1_view() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
        let tensor = TensorStorage::new(buffer, vec![2, 3]).expect("tensor");

        let flat = tensor.flatten();

        assert_eq!(flat.shape(), &[6]);
        assert_eq!(flat.as_slice(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(flat.get(&[4]), Some(&5));
    }

    #[test]
    fn tensor_view_flatten_preserves_slice_contents() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
        let tensor = TensorStorage::new(buffer, vec![2, 2]).expect("tensor");

        let flat = tensor.view().flatten();

        assert_eq!(flat.shape(), &[4]);
        assert_eq!(flat.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn tensor_view_reshape_rejects_mismatched_element_count() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
        let tensor = TensorStorage::new(buffer, vec![2, 2]).expect("tensor");

        let err = tensor.view().reshape(vec![3, 2]).expect_err("mismatch");

        assert_eq!(
            err,
            TensorShapeError::ElementCountMismatch {
                expected: 6,
                actual: 4,
            }
        );
    }

    #[test]
    fn tensor_storage_rejects_mismatched_element_count() {
        let buffer = SharedBuffer::from_vec(vec![1_i64, 2, 3]);
        let err = TensorStorage::new(buffer, vec![2, 2]).expect_err("mismatch");

        assert_eq!(
            err,
            TensorShapeError::ElementCountMismatch {
                expected: 4,
                actual: 3,
            }
        );
    }

    #[test]
    fn tensor_storage_rejects_empty_shape() {
        let buffer = SharedBuffer::from_vec(vec![1_i64]);
        let err = TensorStorage::new(buffer, vec![]).expect_err("empty shape");

        assert_eq!(err, TensorShapeError::EmptyShape);
    }

    #[test]
    fn tensor_storage_rejects_zero_dimension() {
        let buffer = SharedBuffer::from_vec(vec![1_i64]);
        let err = TensorStorage::new(buffer, vec![1, 0]).expect_err("zero dimension");

        assert_eq!(err, TensorShapeError::ZeroDimension);
    }

    #[test]
    fn foreign_ptr_rejects_null() {
        let err = unsafe { ForeignPtr::<i64>::new(std::ptr::null_mut()) }.expect_err("null");

        assert_eq!(err, ForeignBufferError::NullPointer);
    }

    #[test]
    fn foreign_buffer_exposes_slice_from_vec_pointer() {
        let mut data = vec![1_i64, 2, 3];
        let buffer = unsafe { ForeignBuffer::from_raw_parts(data.as_mut_ptr(), data.len()) }.expect("buffer");

        let slice = unsafe { buffer.as_slice() };
        assert_eq!(slice, &[1, 2, 3]);
    }

    #[test]
    fn foreign_buffer_mut_slice_updates_backing_storage() {
        let mut data = vec![1_i64, 2, 3];
        let mut buffer =
            unsafe { ForeignBuffer::from_raw_parts(data.as_mut_ptr(), data.len()) }.expect("buffer");

        let slice = unsafe { buffer.as_mut_slice() };
        slice[1] = 42;

        assert_eq!(data, vec![1, 42, 3]);
    }

    #[test]
    fn foreign_buffer_allows_empty_null_buffer() {
        let buffer = unsafe { ForeignBuffer::<i64>::from_raw_parts(std::ptr::null_mut(), 0) }
            .expect("empty buffer");

        assert!(buffer.is_empty());
        let slice = unsafe { buffer.as_slice() };
        assert_eq!(slice, &[]);
    }
}
