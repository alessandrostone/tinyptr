// use tynyptr::dynamic_table::*;
use tynyptr::dynamic_table::DynamicTinyPointerTable;

#[test]
fn integration_alloc_free() {
    let mut table = DynamicTinyPointerTable::new(8);

    // Allocate a series of values.
    let mut pointers = Vec::new();
    for i in 0..20 {
        let ptr = table.allocate(i);
        pointers.push(ptr);
    }

    // Verify that all values can be retrieved.
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

    // Check that freed pointers return None and others still work.
    for (i, ptr) in pointers.iter().enumerate() {
        if i % 2 == 0 {
            assert_eq!(table.get(*ptr), None);
        } else {
            assert_eq!(table.get(*ptr), Some(&i));
        }
    }
}

#[test]
fn integration_resize() {
    let mut table = DynamicTinyPointerTable::new(4);
    let initial_capacity = table.capacity();

    // Force several resizes.
    for i in 0..100 {
        table.allocate(i);
    }
    assert!(table.capacity() > initial_capacity);
}
