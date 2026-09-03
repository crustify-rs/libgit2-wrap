//! Safe wrappers for libgit2 midx APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_midx_writer
    /// An opaque writer for incrementally constructing a multi-pack index.
    GitMidxWriter,
    GitMidxWriterRef,
    GitMidxWriterMut,
    ffi::git_midx_writer
);

/// An exclusively owned multi-pack-index writer.
pub type GitMidxWriterOwned = CBox<GitMidxWriter>;

// SAFETY: `git_midx_writer_free` is the public full destructor for one
// uniquely owned, fully initialized writer. `CBox` supplies a non-null pointer
// and invokes this destructor exactly once, although C also accepts null.
ffibox::impl_dropped!(
    GitMidxWriter,
    ffi::git_midx_writer,
    ffi::git_midx_writer_free
);

#[cfg(test)]
mod unit_tests {
    use core::mem::size_of;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn writer_uses_typed_pointer_sized_handles() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitMidxWriter>();
        assert_dropped::<GitMidxWriter>();
        assert_eq!(
            size_of::<GitMidxWriterRef<'_>>(),
            size_of::<*const ffi::git_midx_writer>()
        );
        assert_eq!(
            size_of::<GitMidxWriterMut<'_>>(),
            size_of::<*mut ffi::git_midx_writer>()
        );
        assert_eq!(
            size_of::<GitMidxWriterOwned>(),
            size_of::<*mut ffi::git_midx_writer>()
        );
    }

    #[test]
    fn null_writer_pointers_do_not_create_handles_or_owners() {
        // SAFETY: all conversion seams explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitMidxWriterRef::from_ptr(core::ptr::null_mut()).is_none());
            assert!(GitMidxWriterMut::from_ptr(core::ptr::null_mut()).is_none());
            assert!(GitMidxWriterOwned::from_raw(core::ptr::null_mut()).is_none());
        }
    }
}
