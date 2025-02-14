// integration_tests.rs

use tynyptr::dynamic_table::DynamicTinyPointerTable;

/// Test basic allocation, retrieval, and freeing.
/// Verifies that after freeing a pointer, its slot is no longer accessible.
#[test]
fn integration_alloc_free() {
    let mut table = DynamicTinyPointerTable::new(8);

    // Allocate a series of values.
    let mut pointers = Vec::new();
    for i in 0..20 {
        let ptr = table.allocate(i);
        pointers.push(ptr);
    }

    // Verify that all allocated values can be retrieved.
    for (i, ptr) in pointers.iter().enumerate() {
        assert_eq!(table.get(*ptr), Some(&i));
    }

    // Free every even-indexed pointer.
    for (i, ptr) in pointers.iter().enumerate() {
        if i % 2 == 0 {
            let val = table.free(*ptr);
            assert_eq!(val, Some(i));
        }
    }

    // Verify that freed pointers are now invalid and odd-indexed pointers remain valid.
    for (i, ptr) in pointers.iter().enumerate() {
        if i % 2 == 0 {
            assert_eq!(table.get(*ptr), None);
        } else {
            assert_eq!(table.get(*ptr), Some(&i));
        }
    }
}

/// Test that the table correctly resizes when the free list is exhausted.
#[test]
fn integration_resize() {
    let mut table = DynamicTinyPointerTable::new(4);
    let initial_capacity = table.capacity();

    // Force several resizes by allocating more elements than the initial capacity.
    for i in 0..100 {
        table.allocate(i);
    }
    // Verify that the capacity has grown.
    assert!(table.capacity() > initial_capacity);
}

/// Test that mutable access works as expected.
#[test]
fn integration_get_mut() {
    let mut table = DynamicTinyPointerTable::new(4);
    let ptr = table.allocate(42);
    {
        // Get a mutable reference and modify the value.
        let value = table.get_mut(ptr).expect("Value should exist");
        *value = 99;
    }
    // Verify that the update is visible.
    assert_eq!(table.get(ptr), Some(&99));
}

/// Test generational safety:
/// After freeing a pointer, reusing the slot should invalidate the old pointer.
#[test]
fn integration_generation_safety() {
    let mut table = DynamicTinyPointerTable::new(4);
    let ptr = table.allocate(10);
    
    // Free the pointer.
    let freed = table.free(ptr);
    assert_eq!(freed, Some(10));

    // Allocate a new value which might reuse the same slot but with an incremented generation.
    let new_ptr = table.allocate(20);

    // The old pointer should now be invalid.
    assert_eq!(table.get(ptr), None);
    // The new pointer should retrieve the new value.
    assert_eq!(table.get(new_ptr), Some(&20));
}

/// Stress test: perform many allocations and frees.
/// This helps to catch any issues under heavy load and repeated slot reuse.
#[test]
fn integration_many_allocations_and_frees() {
    let mut table = DynamicTinyPointerTable::new(8);
    let mut pointers = Vec::new();

    // Allocate a large number of values.
    for i in 0..1000 {
        let ptr = table.allocate(i);
        pointers.push(ptr);
    }

    // Verify all allocated values.
    for (i, ptr) in pointers.iter().enumerate() {
        assert_eq!(table.get(*ptr), Some(&i));
    }

    // Free all allocated pointers.
    for ptr in pointers.iter() {
        let _ = table.free(*ptr);
    }

    // Ensure that after freeing, no pointer is accessible.
    for ptr in pointers.iter() {
        assert_eq!(table.get(*ptr), None);
    }
}

/// Mixed usage test:
/// Simulate a random allocation/free pattern to mimic real-world usage.
#[test]
fn integration_mixed_usage() {
    let mut table = DynamicTinyPointerTable::new(16);
    let mut pointers = Vec::new();

    // Perform 500 mixed operations.
    for i in 0..500 {
        if i % 3 == 0 && !pointers.is_empty() {
            // Randomly free one pointer from the list.
            let index = i % pointers.len();
            let ptr = pointers.remove(index);
            let val = table.free(ptr);
            // Make sure freeing returns a value.
            assert!(val.is_some(), "Freeing a valid pointer should return a value");
        } else {
            // Allocate a new pointer.
            let ptr = table.allocate(i);
            pointers.push(ptr);
        }
    }

    // Verify that all remaining pointers are valid.
    for ptr in &pointers {
        assert!(table.get(*ptr).is_some());
    }
}
