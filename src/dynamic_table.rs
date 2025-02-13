//! # DynamicTinyPointerTable
//!
//! This module implements a dynamic dereference table that maps “tiny pointers” (compact indices)
//! to values of type `T`. When the table is full, it doubles its capacity while keeping the already
//! allocated indices valid (amortized constant time). Note that references returned by `get()` or
//! `get_mut()` are valid only until the next call that might trigger a resize.
//!
//! ## Example
//!
//! ```rust
//! use tynyptr::dynamic_table::*;
//!
//! // Create a table with an initial capacity of 4.
//! let mut table = DynamicTinyPointerTable::new(4);
//!
//! // Allocate some values.
//! let ptr_a = table.allocate(10);
//! let ptr_b = table.allocate(20);
//! assert_eq!(table.get(ptr_a), Some(&10));
//! assert_eq!(table.get(ptr_b), Some(&20));
//!
//! // Free a value.
//! let freed = table.free(ptr_a);
//! assert_eq!(freed, Some(10));
//!
//! // After many allocations the table resizes automatically.
//! for i in 0..100 {
//!     table.allocate(i);
//! }
//! assert!(table.capacity() >= 100);
//! ``` 

use std::fmt;

/// A tiny pointer is represented as a compact index (here as a `u32`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TinyPointer(u32);

impl TinyPointer {
    /// Returns the index represented by this tiny pointer.
    #[inline]
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for TinyPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TinyPointer({})", self.index())
    }
}



/// A dynamic dereference table that stores values of type `T` in a vector and
/// uses a free list to track available slots. When the free list is empty,
/// the table is resized (capacity is doubled).
pub struct DynamicTinyPointerTable<T: Clone> {
    slots: Vec<Option<Box<T>>>,
    free_list: Vec<usize>,
}

impl<T: Clone> DynamicTinyPointerTable<T> {
    /// Creates a new table with the specified initial capacity.
    ///
    /// # Panics
    ///
    /// Panics if `initial_capacity` is 0.
    pub fn new(initial_capacity: usize) -> Self {
        let mut free_list = Vec::with_capacity(initial_capacity);
        for i in 0..initial_capacity {
            free_list.push(i);
        }
        Self {
            slots: vec![None; initial_capacity],
            free_list,
        }
    }

    /// Returns the current total capacity of the table.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Returns the number of allocated (non-free) entries.
    #[inline]
    pub fn allocated(&self) -> usize {
        self.slots.len() - self.free_list.len()
    }

    /// Returns the current load factor (allocated slots divided by capacity).
    #[inline]
    pub fn load_factor(&self) -> f64 {
        self.allocated() as f64 / self.capacity() as f64
    }

    /// Allocates a slot for `value`, resizing the table if necessary,
    /// and returns a `TinyPointer` to the allocated slot.
    ///
    /// Amortized constant-time.
    pub fn allocate(&mut self, value: T) -> TinyPointer {
        if self.free_list.is_empty() {
            self.resize();
        }
        let idx = self.free_list.pop().expect("free_list should not be empty");
        self.slots[idx] = Some(Box::new(value)) ;
        TinyPointer(idx as u32)
    }

    /// Returns an immutable reference to the value corresponding to `ptr`,
    /// or `None` if the pointer is invalid or the slot is free.
    pub fn get(&self, ptr: TinyPointer) -> Option<&T> {
        self.slots
            .get(ptr.index())
            .and_then(|slot| slot.as_ref().map(|boxed| &**boxed))
}

    /// Returns a mutable reference to the value corresponding to `ptr`,
    /// or `None` if the pointer is invalid or the slot is free.
    pub fn get_mut(&mut self, ptr: TinyPointer) -> Option<&mut T> {
        self.slots
            .get_mut(ptr.index())
            .and_then(|slot| slot.as_mut().map(|boxed| &mut **boxed))
    }

    /// Frees the value at `ptr` and returns it.
    ///
    /// After freeing, the slot is added back to the free list.
    pub fn free(&mut self, ptr: TinyPointer) -> Option<T> {
        let idx = ptr.index();
        if idx < self.slots.len() {
            let value = self.slots[idx].take();
            if value.is_some() {
                self.free_list.push(idx);
            }
            value.map(|boxed| *boxed) // Dereference the Box to get T
        } else {
            None
        }
    }

    /// Resizes the table by doubling its capacity.
    ///
    /// This method is called automatically from `allocate()` when there are no free slots.
    pub fn resize(&mut self) {
        let old_capacity = self.capacity();
        let new_capacity = old_capacity * 2;
        self.slots.resize(new_capacity, None);
        // Add new indices (in reverse order) to the free list.
        for i in (old_capacity..new_capacity).rev() {
            self.free_list.push(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_get_free() {
        let mut table = DynamicTinyPointerTable::new(4);
        assert_eq!(table.capacity(), 4);
        assert_eq!(table.load_factor(), 0.0);

        let ptr_a = table.allocate(10);
        let ptr_b = table.allocate(20);
        assert_eq!(table.get(ptr_a), Some(&10));
        assert_eq!(table.get(ptr_b), Some(&20));
        assert_eq!(table.allocated(), 2);
        assert_eq!(table.load_factor(), 0.5);

        // Free ptr_a and check.
        let freed = table.free(ptr_a);
        assert_eq!(freed, Some(10));
        assert_eq!(table.get(ptr_a), None);
        assert_eq!(table.allocated(), 1);
    }

    #[test]
    fn test_resize_behavior() {
        let mut table = DynamicTinyPointerTable::new(2);
        // Fill the table; next allocation should trigger a resize.
        let ptr1 = table.allocate(1);
        let ptr2 = table.allocate(2);
        assert_eq!(table.capacity(), 2);
        // Free list is now empty.
        assert!(table.free_list.is_empty());
        // Allocate one more; this triggers resize.
        let ptr3 = table.allocate(3);
        assert!(table.capacity() >= 3);
        // Ensure all values are still accessible.
        assert_eq!(table.get(ptr1), Some(&1));
        assert_eq!(table.get(ptr2), Some(&2));
        assert_eq!(table.get(ptr3), Some(&3));
    }

    #[test]
    fn test_get_mut() {
        let mut table = DynamicTinyPointerTable::new(4);
        let ptr = table.allocate(5);
        if let Some(val) = table.get_mut(ptr) {
            *val = 42;
        }
        assert_eq!(table.get(ptr), Some(&42));
    }

    #[test]
    fn test_many_allocations() {
        let mut table = DynamicTinyPointerTable::new(4);
        let mut pointers = Vec::new();
        for i in 0..100 {
            pointers.push(table.allocate(i));
        }
        // Ensure that the table has resized enough.
        assert!(table.capacity() >= 100);
        // Check that all allocated values are correct.
        for (i, ptr) in pointers.iter().enumerate() {
            assert_eq!(table.get(*ptr), Some(&i));
        }
        // Free half the pointers.
        for ptr in pointers.iter().take(50) {
            table.free(*ptr);
        }
        // Load factor should now be at most 50/ (current capacity)
        assert!(table.load_factor() <= 0.5);
    }
}
