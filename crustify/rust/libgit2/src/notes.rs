//! Safe wrappers for libgit2 notes APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_note
    /// An opaque note allocation owned by libgit2.
    ///
    /// Owned pointers use [`GitNoteOwned`]. Dropping an owner releases the
    /// note's duplicated signatures and message along with the allocation.
    GitNote,
    GitNoteRef,
    GitNoteMut,
    ffi::git_note
);

/// An owning handle to a fully formed libgit2 note.
pub type GitNoteOwned = CBox<GitNote>;

// SAFETY: `git_note_free` is the public destructor for a fully formed
// `git_note`. It accepts null, although `CBox` supplies one live non-null
// allocation exactly once, and releases all owned fields before the header.
ffibox::impl_dropped!(GitNote, ffi::git_note, ffi::git_note_free);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn note_wrapper_preserves_the_opaque_seam_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitNote>();
        assert_dropped::<GitNote>();
        assert_eq!(size_of::<GitNote>(), size_of::<ffi::git_note>());
        assert_eq!(align_of::<GitNote>(), align_of::<ffi::git_note>());
        assert_eq!(
            size_of::<GitNoteRef<'_>>(),
            size_of::<*const ffi::git_note>()
        );
        assert_eq!(size_of::<GitNoteMut<'_>>(), size_of::<*mut ffi::git_note>());
        assert_eq!(size_of::<GitNoteOwned>(), size_of::<*mut ffi::git_note>());
    }

    #[test]
    fn null_note_seams_create_no_handle() {
        // SAFETY: all conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitNoteRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitNoteMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitNoteOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
