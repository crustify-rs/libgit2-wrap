//! Safe wrappers for libgit2 iterator APIs.

use ffibox::{define_ctype, impl_dropped};

use crate::ffi;

define_ctype!(
    /// Wraps: git_iterator
    /// The opaque iterator underlying libgit2's public note iterator.
    ///
    /// Owned pointers use [`ffibox::CBox<GitIterator>`]. Dropping an owner
    /// releases the iterator through `git_note_iterator_free`; borrowed
    /// access uses [`GitIteratorRef`] or [`GitIteratorMut`] without forming a
    /// Rust reference to storage managed by libgit2.
    GitIterator,
    GitIteratorRef,
    GitIteratorMut,
    ffi::git_iterator
);

// SAFETY: `git_note_iterator_free` is the public destructor for a complete
// `git_note_iterator` (an alias of `git_iterator`) and accepts null, although
// `CDropped` supplies one live non-null allocation exactly once.
impl_dropped!(GitIterator, ffi::git_iterator, ffi::git_note_iterator_free);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CBox, CCell, CDropped};

    use super::*;

    #[test]
    fn iterator_preserves_the_opaque_c_seam_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitIterator>();
        assert_dropped::<GitIterator>();
        assert_eq!(size_of::<GitIterator>(), size_of::<ffi::git_iterator>());
        assert_eq!(align_of::<GitIterator>(), align_of::<ffi::git_iterator>());
        assert_eq!(
            size_of::<GitIteratorRef<'_>>(),
            size_of::<*const ffi::git_iterator>()
        );
        assert_eq!(
            size_of::<GitIteratorMut<'_>>(),
            size_of::<*mut ffi::git_iterator>()
        );
        assert_eq!(
            size_of::<CBox<GitIterator>>(),
            size_of::<*mut ffi::git_iterator>()
        );
    }

    #[test]
    fn null_iterator_seams_create_no_handle() {
        // SAFETY: all three conversions explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitIteratorRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitIteratorMut::from_ptr(ptr::null_mut()).is_none());
            assert!(CBox::<GitIterator>::from_raw(ptr::null_mut()).is_none());
        }
    }
}
