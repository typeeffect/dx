//! Integration tests for tensor coordinate access, row views, row ranges, and reshape.

use dx_memory::{SharedBuffer, TensorStorage};

// ── coordinate access ───────────────────────────────────────────

#[test]
fn get_2x3_matrix_elements() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
    let tensor = TensorStorage::new(data, vec![2, 3]).expect("valid");

    assert_eq!(tensor.get(&[0, 0]), Some(&1));
    assert_eq!(tensor.get(&[0, 1]), Some(&2));
    assert_eq!(tensor.get(&[0, 2]), Some(&3));
    assert_eq!(tensor.get(&[1, 0]), Some(&4));
    assert_eq!(tensor.get(&[1, 1]), Some(&5));
    assert_eq!(tensor.get(&[1, 2]), Some(&6));
}

#[test]
fn get_returns_none_for_out_of_bounds() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
    let tensor = TensorStorage::new(data, vec![2, 2]).expect("valid");

    assert_eq!(tensor.get(&[2, 0]), None);
    assert_eq!(tensor.get(&[0, 2]), None);
}

#[test]
fn get_returns_none_for_rank_mismatch() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
    let tensor = TensorStorage::new(data, vec![2, 2]).expect("valid");

    assert_eq!(tensor.get(&[0]), None);
    assert_eq!(tensor.get(&[0, 0, 0]), None);
}

#[test]
fn view_get_matches_storage_get() {
    let data = SharedBuffer::from_vec(vec![10_i64, 20, 30, 40, 50, 60]);
    let tensor = TensorStorage::new(data, vec![2, 3]).expect("valid");
    let view = tensor.view();

    for r in 0..2 {
        for c in 0..3 {
            assert_eq!(tensor.get(&[r, c]), view.get(&[r, c]));
        }
    }
}

// ── row views ───────────────────────────────────────────────────

#[test]
fn row_extracts_rank1_view() {
    let data = SharedBuffer::from_vec(vec![10_i64, 11, 12, 20, 21, 22, 30, 31, 32]);
    let tensor = TensorStorage::new(data, vec![3, 3]).expect("valid");

    let row0 = tensor.row(0).expect("row 0");
    assert_eq!(row0.as_slice(), &[10, 11, 12]);
    assert_eq!(row0.shape(), &[3]);
    assert_eq!(row0.rank(), 1);

    let row2 = tensor.row(2).expect("row 2");
    assert_eq!(row2.as_slice(), &[30, 31, 32]);
}

#[test]
fn row_returns_none_for_out_of_bounds() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
    let tensor = TensorStorage::new(data, vec![2, 2]).expect("valid");

    assert!(tensor.row(2).is_none());
}

#[test]
fn row_returns_none_for_non_rank2() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3]);
    let rank1 = TensorStorage::new(data, vec![3]).expect("valid");
    assert!(rank1.row(0).is_none());

    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6, 7, 8]);
    let rank3 = TensorStorage::new(data, vec![2, 2, 2]).expect("valid");
    assert!(rank3.row(0).is_none());
}

#[test]
fn view_row_matches_storage_row() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
    let tensor = TensorStorage::new(data, vec![2, 3]).expect("valid");
    let view = tensor.view();

    let sr = tensor.row(1).expect("storage row");
    let vr = view.row(1).expect("view row");
    assert_eq!(sr.as_slice(), vr.as_slice());
}

#[test]
fn rows_extracts_rank2_subview() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6, 7, 8]);
    let tensor = TensorStorage::new(data, vec![4, 2]).expect("valid");

    let rows = tensor.rows(1..3).expect("rows");
    assert_eq!(rows.shape(), &[2, 2]);
    assert_eq!(rows.as_slice(), &[3, 4, 5, 6]);
    assert_eq!(rows.get(&[1, 0]), Some(&5));
}

#[test]
fn rows_returns_none_for_wrong_rank_or_invalid_range() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
    let matrix = TensorStorage::new(data, vec![2, 2]).expect("valid");
    assert!(matrix.rows(1..3).is_none());
    assert!(matrix.rows(2..1).is_none());

    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3]);
    let vector = TensorStorage::new(data, vec![3]).expect("valid");
    assert!(vector.rows(0..1).is_none());
}

#[test]
fn split_rows_partitions_matrix_into_two_contiguous_blocks() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6, 7, 8]);
    let tensor = TensorStorage::new(data, vec![4, 2]).expect("valid");

    let (head, tail) = tensor.split_rows(2).expect("split");
    assert_eq!(head.shape(), &[2, 2]);
    assert_eq!(tail.shape(), &[2, 2]);
    assert_eq!(head.as_slice(), &[1, 2, 3, 4]);
    assert_eq!(tail.as_slice(), &[5, 6, 7, 8]);
    assert_eq!(tail.get(&[1, 1]), Some(&8));
}

// ── reshape ─────────────────────────────────────────────────────

#[test]
fn reshape_reinterprets_contiguous_tensor_shape() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
    let tensor = TensorStorage::new(data, vec![2, 3]).expect("valid");

    let reshaped = tensor.reshape(vec![3, 2]).expect("reshape");

    assert_eq!(reshaped.shape(), &[3, 2]);
    assert_eq!(reshaped.get(&[0, 1]), Some(&2));
    assert_eq!(reshaped.get(&[2, 1]), Some(&6));
}

#[test]
fn reshape_rejects_element_count_mismatch() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4]);
    let tensor = TensorStorage::new(data, vec![2, 2]).expect("valid");

    let err = tensor.reshape(vec![3, 2]).expect_err("mismatch");
    assert_eq!(
        err.to_string(),
        "tensor element count mismatch: expected 6, got 4"
    );
}

#[test]
fn flatten_reinterprets_matrix_as_rank1_view() {
    let data = SharedBuffer::from_vec(vec![1_i64, 2, 3, 4, 5, 6]);
    let tensor = TensorStorage::new(data, vec![2, 3]).expect("valid");

    let flat = tensor.flatten();
    assert_eq!(flat.shape(), &[6]);
    assert_eq!(flat.as_slice(), &[1, 2, 3, 4, 5, 6]);
    assert_eq!(flat.get(&[5]), Some(&6));
}
