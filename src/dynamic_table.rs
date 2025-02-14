//! # DynamicTinyPointerTable with Generational Tiny Pointers
//!
//! This module implements a dynamic dereference table that maps “tiny pointers”
//! (compact indices augmented with generation counters) to values of type `T`.
//!
//! The table uses a free list to manage available slots and doubles its capacity
//! when full, while preserving the validity of currently allocated indices.
//!
//! With generational pointers, each slot stores a generation counter. When a slot
//! is freed, its generation is incremented. Any pointer holding an old generation
//! will be invalid, preventing accidental use-after-free.
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
//! // Free a value and verify that the pointer is now invalid.
//! let freed = table.free(ptr_a);
//! assert_eq!(freed, Some(10));
//! assert_eq!(table.get(ptr_a), None);
//!
//! // After many allocations the table resizes automatically.
//! for i in 0..100 {
//!     table.allocate(i);
//! }
//! assert!(table.capacity() >= 100);
//! ``` 

use std::fmt;

/// A generational tiny pointer.
///
/// This pointer consists of an index (a compact `u32`) and a generation counter.
/// When a slot is freed and later reused, its generation is incremented,
/// which invalidates any old pointer to that slot.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TinyPointer {
    index: u32,
    generation: u32,
}

impl TinyPointer {
    /// Returns the index associated with this pointer.
    #[inline]
    pub fn index(&self) -> usize {
        self.index as usize
    }
    
    /// Returns the generation stored in this pointer.
    #[inline]
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

impl fmt::Display for TinyPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TinyPointer({}, gen: {})", self.index(), self.generation)
    }
}

/// A slot in the dynamic table.
///
/// Each slot stores an optional value along with a generation counter.
/// The generation counter is used to validate that a `TinyPointer`
/// is not stale.
struct Slot<T> {
    value: Option<Box<T>>,
    generation: u32,
}

/// A dynamic dereference table using generational tiny pointers.
///
/// This table stores values of type `T` in a vector. It uses a free list
/// to keep track of available slots and doubles its capacity when needed.
/// Returned pointers are valid only if their generation matches the current
/// generation of the slot.
pub struct DynamicTinyPointerTable<T: Clone> {
    slots: Vec<Slot<T>>,
    free_list: Vec<usize>,
}

impl<T: Clone> DynamicTinyPointerTable<T> {
    /// Creates a new table with the specified initial capacity.
    ///
    /// # Panics
    ///
    /// Panics if `initial_capacity` is 0.
    pub fn new(initial_capacity: usize) -> Self {
        assert!(initial_capacity > 0, "initial_capacity must be > 0");
        let mut slots = Vec::with_capacity(initial_capacity);
        let mut free_list = Vec::with_capacity(initial_capacity);
        for i in 0..initial_capacity {
            slots.push(Slot { value: None, generation: 0 });
            free_list.push(i);
        }
        Self { slots, free_list }
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
    /// and returns a `TinyPointer` (including the current generation).
    ///
    /// This operation is amortized constant-time.
    pub fn allocate(&mut self, value: T) -> TinyPointer {
        if self.free_list.is_empty() {
            self.resize();
        }
        // Pop a free index.
        let idx = self.free_list.pop().expect("free_list should not be empty");
        let slot = &mut self.slots[idx];
        slot.value = Some(Box::new(value));
        // Return a pointer that includes the current generation.
        TinyPointer { index: idx as u32, generation: slot.generation }
    }

    /// Returns an immutable reference to the value corresponding to `ptr`,
    /// or `None` if the pointer is invalid, the slot is free, or the generation mismatches.
    pub fn get(&self, ptr: TinyPointer) -> Option<&T> {
        self.slots.get(ptr.index()).and_then(|slot| {
            if slot.generation == ptr.generation {
                slot.value.as_ref().map(|boxed| &**boxed)
            } else {
                None
            }
        })
    }

    /// Returns a mutable reference to the value corresponding to `ptr`,
    /// or `None` if the pointer is invalid, the slot is free, or the generation mismatches.
    pub fn get_mut(&mut self, ptr: TinyPointer) -> Option<&mut T> {
        self.slots.get_mut(ptr.index()).and_then(|slot| {
            if slot.generation == ptr.generation {
                slot.value.as_mut().map(|boxed| &mut **boxed)
            } else {
                None
            }
        })
    }

    /// Frees the value at `ptr` and returns it.
    ///
    /// If the pointer's generation matches, the slot is freed and its generation is incremented.
    /// The freed slot is then added back to the free list.
    pub fn free(&mut self, ptr: TinyPointer) -> Option<T> {
        let idx = ptr.index();
        if idx < self.slots.len() {
            let slot = &mut self.slots[idx];
            // Only free if the generation matches.
            if slot.generation == ptr.generation {
                let value = slot.value.take();
                // Increment generation to invalidate any stale pointers.
                slot.generation = slot.generation.wrapping_add(1);
                self.free_list.push(idx);
                return value.map(|boxed| *boxed);
            }
        }
        None
    }

    /// Resizes the table by doubling its capacity.
    ///
    /// This method is automatically called from `allocate()` when no free slots remain.
    pub fn resize(&mut self) {
        let old_capacity = self.capacity();
        let new_capacity = old_capacity * 2;
        self.slots.reserve(new_capacity - old_capacity);
        for i in old_capacity..new_capacity {
            self.slots.push(Slot { value: None, generation: 0 });
            self.free_list.push(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that allocation, retrieval, and freeing work as expected.
    #[test]
    fn test_allocation_get_free() {
        let mut table = DynamicTinyPointerTable::new(4);
        assert_eq!(table.capacity(), 4);
        assert_eq!(table.load_factor(), 0.0);

        // Allocate two values.
        let ptr_a = table.allocate(10);
        let ptr_b = table.allocate(20);
        assert_eq!(table.get(ptr_a), Some(&10));
        assert_eq!(table.get(ptr_b), Some(&20));
        assert_eq!(table.allocated(), 2);
        assert_eq!(table.load_factor(), 0.5);

        // Free one pointer and check that it is no longer accessible.
        let freed = table.free(ptr_a);
        assert_eq!(freed, Some(10));
        assert_eq!(table.get(ptr_a), None);
        assert_eq!(table.allocated(), 1);
    }

    /// Tests that the table resizes correctly when the free list is exhausted.
    #[test]
    fn test_resize_behavior() {
        let mut table = DynamicTinyPointerTable::new(2);
        // Fill the table; next allocation should trigger a resize.
        let ptr1 = table.allocate(1);
        let ptr2 = table.allocate(2);
        assert_eq!(table.capacity(), 2);
        // Free list should now be empty.
        assert!(table.free_list.is_empty());
        // Allocate one more; this triggers resize.
        let ptr3 = table.allocate(3);
        assert!(table.capacity() >= 3);
        // Ensure that all values are accessible.
        assert_eq!(table.get(ptr1), Some(&1));
        assert_eq!(table.get(ptr2), Some(&2));
        assert_eq!(table.get(ptr3), Some(&3));
    }

    /// Tests that a mutable reference can be obtained and modified.
    #[test]
    fn test_get_mut() {
        let mut table = DynamicTinyPointerTable::new(4);
        let ptr = table.allocate(5);
        if let Some(val) = table.get_mut(ptr) {
            *val = 42;
        }
        assert_eq!(table.get(ptr), Some(&42));
    }

    /// Tests many allocations and frees, verifying that all values are stored and freed correctly.
    #[test]
    fn test_many_allocations() {
        let mut table = DynamicTinyPointerTable::new(4);
        let mut pointers = Vec::new();
        for i in 0..100 {
            pointers.push(table.allocate(i));
        }
        // Table should have resized appropriately.
        assert!(table.capacity() >= 100);
        // Check that all allocated values are correct.
        for (i, ptr) in pointers.iter().enumerate() {
            assert_eq!(table.get(*ptr), Some(&i));
        }
        // Free half of the pointers.
        for ptr in pointers.iter().take(50) {
            table.free(*ptr);
        }
        // Load factor should now be at most 50/(current capacity).
        assert!(table.load_factor() <= 0.5);
    }

    /// Tests that a pointer becomes invalid once its slot is freed and reused.
    #[test]
    fn test_generation_safety() {
        let mut table = DynamicTinyPointerTable::new(4);
        let ptr = table.allocate(100);
        // Free the pointer.
        let _ = table.free(ptr);
        // Allocate a new value; it might reuse the same slot with a new generation.
        let new_ptr = table.allocate(200);
        // The old pointer should now be invalid.
        assert_eq!(table.get(ptr), None);
        // The new pointer should access the new value.
        assert_eq!(table.get(new_ptr), Some(&200));
    }
}
