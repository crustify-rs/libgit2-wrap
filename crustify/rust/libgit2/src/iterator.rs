//! Safe wrappers for libgit2 iterator APIs.

use ffibox::{define_ctype, impl_dropped};

use crate::ffi;

define_ctype!(
    /// Wraps: git_iterator
    /// The uniquely owned iterator libgit2 publishes as `git_note_iterator`.
    ///
    /// One header and one allocation serve every variant — empty, tree,
    /// index, workdir and filesystem. It owns only its optional duplicated
    /// `start` and `end` range bounds and its pathlist vector; its callback
    /// table and comparison functions point at statics, and the optional
    /// repository and index a variant was built from are borrowed and are
    /// **not** released with the iterator.
    ///
    /// Owning an iterator therefore does not keep those dependencies alive.
    /// A wrapper that hands one out must carry that borrow itself, as
    /// [`GitNoteIterator`](crate::notes::GitNoteIterator) does for the
    /// repository its notes are read from.
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

// SAFETY: `git_note_iterator_free` is the published destructor for a complete
// `git_note_iterator`, a typedef of `git_iterator`. It returns on null and
// otherwise forwards to `git_iterator_free`, the general destructor for every
// iterator variant: it dispatches `cb->free`, disposes the pathlist, frees the
// owned `start` and `end` ranges, then frees the header. It releases neither
// the borrowed repository nor the borrowed index, so no other owner is
// disturbed. `CDropped` supplies one live non-null allocation exactly once.
impl_dropped!(GitIterator, ffi::git_iterator, ffi::git_note_iterator_free);

#[cfg(test)]
mod unit_tests {
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
