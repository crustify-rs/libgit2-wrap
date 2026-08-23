//! Safe wrappers for libgit2 index APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_index_time
    /// A seconds-and-nanoseconds timestamp stored in an index entry.
    IndexTime,
    IndexTimeRef,
    IndexTimeMut,
    ffi::git_index_time
);

impl IndexTimeRef<'_> {
    /// Wraps: git_index_time.seconds
    /// Returns the whole-second component of this timestamp.
    #[inline]
    #[must_use]
    pub fn seconds(&self) -> i32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).seconds).read() }
    }

    /// Wraps: git_index_time.nanoseconds
    /// Returns the subsecond nanosecond component of this timestamp.
    #[inline]
    #[must_use]
    pub fn nanoseconds(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).nanoseconds).read() }
    }
}

impl IndexTimeMut<'_> {
    /// Sets the whole-second component of this timestamp.
    #[inline]
    pub fn set_seconds(&mut self, value: i32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).seconds).write(value) }
    }

    /// Sets the subsecond nanosecond component of this timestamp.
    #[inline]
    pub fn set_nanoseconds(&mut self, value: u32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).nanoseconds).write(value) }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_index_conflict_iterator
    /// Opaque storage for an iterator over conflicts in a libgit2 index.
    ///
    /// The iterator borrows the index supplied to its constructor. Safe
    /// constructor and iteration wrappers must therefore keep that exclusive
    /// index borrow alive for as long as the owner exists.
    GitIndexConflictIterator,
    GitIndexConflictIteratorRef,
    GitIndexConflictIteratorMut,
    ffi::git_index_conflict_iterator
);

/// An owning conflict-iterator allocation.
///
/// This raw ownership building block does not by itself carry the iterator's
/// borrow of its source index. Safe constructors must wrap it in a
/// lifetime-carrying handle before returning it.
pub type GitIndexConflictIteratorOwned = CBox<GitIndexConflictIterator>;

// SAFETY: `git_index_conflict_iterator_free` is the public destructor for a
// fully formed conflict-iterator allocation and accepts null, although `CBox`
// always supplies a live non-null allocation exactly once. It does not access
// the borrowed index stored by the iterator.
ffibox::impl_dropped!(
    GitIndexConflictIterator,
    ffi::git_index_conflict_iterator,
    ffi::git_index_conflict_iterator_free
);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn index_time_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<IndexTime>(), size_of::<ffi::git_index_time>());
        assert_eq!(align_of::<IndexTime>(), align_of::<ffi::git_index_time>());
        assert_eq!(
            size_of::<IndexTimeRef<'_>>(),
            size_of::<*const ffi::git_index_time>()
        );
        assert_eq!(
            size_of::<IndexTimeMut<'_>>(),
            size_of::<*mut ffi::git_index_time>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_both_components() {
        let mut raw = ffi::git_index_time {
            seconds: -2,
            nanoseconds: 3,
        };

        // SAFETY: `raw` is initialized, non-null, exclusively borrowed for the
        // handle's lifetime, and remains live until the handle is last used.
        let mut time = unsafe { IndexTimeMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(time.as_ref().seconds(), -2);
        assert_eq!(time.as_ref().nanoseconds(), 3);

        time.set_seconds(4);
        time.set_nanoseconds(5);
        assert_eq!(time.as_ref().seconds(), 4);
        assert_eq!(time.as_ref().nanoseconds(), 5);
    }

    #[test]
    fn conflict_iterator_preserves_the_ffi_layout_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitIndexConflictIterator>();
        assert_dropped::<GitIndexConflictIterator>();
        assert_eq!(
            size_of::<GitIndexConflictIterator>(),
            size_of::<ffi::git_index_conflict_iterator>()
        );
        assert_eq!(
            align_of::<GitIndexConflictIterator>(),
            align_of::<ffi::git_index_conflict_iterator>()
        );
        assert_eq!(
            size_of::<GitIndexConflictIteratorRef<'_>>(),
            size_of::<*const ffi::git_index_conflict_iterator>()
        );
        assert_eq!(
            size_of::<GitIndexConflictIteratorMut<'_>>(),
            size_of::<*mut ffi::git_index_conflict_iterator>()
        );
        assert_eq!(
            size_of::<GitIndexConflictIteratorOwned>(),
            size_of::<*mut ffi::git_index_conflict_iterator>()
        );
    }

    #[test]
    fn null_conflict_iterator_seams_create_no_handle() {
        // SAFETY: each conversion accepts null and returns `None` without
        // borrowing or adopting an object.
        unsafe {
            assert!(GitIndexConflictIteratorRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitIndexConflictIteratorMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitIndexConflictIteratorOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
