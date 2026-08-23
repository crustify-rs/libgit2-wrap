//! Safe wrappers for libgit2 reflog APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_reflog
    /// An opaque in-memory reference log owned by libgit2.
    ///
    /// Owned pointers use [`GitReflogOwned`]. Dropping an owner releases its
    /// entries, reference name, retained reference-database count, and header.
    GitReflog,
    GitReflogRef,
    GitReflogMut,
    ffi::git_reflog
);

/// An owning handle to a fully formed libgit2 reflog.
pub type GitReflogOwned = CBox<GitReflog>;

// SAFETY: `git_reflog_free` is the public destructor for a fully formed,
// ordinary libgit2 reflog. It accepts null, although `CBox` supplies one live
// non-null allocation exactly once, and releases all fields before the header.
ffibox::impl_dropped!(GitReflog, ffi::git_reflog, ffi::git_reflog_free);

ffibox::define_ctype!(
    /// Wraps: git_reflog_entry
    /// An opaque reflog entry borrowed from its containing [`GitReflog`].
    ///
    /// Libgit2's public API does not transfer ownership of individual entries;
    /// callers receive a shared handle whose lifetime remains tied to the log.
    GitReflogEntry,
    GitReflogEntryRef,
    GitReflogEntryMut,
    ffi::git_reflog_entry
);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn wrappers_preserve_the_opaque_ffi_seams() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}
        fn assert_copy<T: Copy>() {}

        assert_cell::<GitReflog>();
        assert_cell::<GitReflogEntry>();
        assert_dropped::<GitReflog>();
        assert_copy::<GitReflogRef<'_>>();
        assert_copy::<GitReflogEntryRef<'_>>();

        assert_eq!(size_of::<GitReflog>(), size_of::<ffi::git_reflog>());
        assert_eq!(align_of::<GitReflog>(), align_of::<ffi::git_reflog>());
        assert_eq!(
            size_of::<GitReflogRef<'_>>(),
            size_of::<*const ffi::git_reflog>()
        );
        assert_eq!(
            size_of::<GitReflogMut<'_>>(),
            size_of::<*mut ffi::git_reflog>()
        );
        assert_eq!(
            size_of::<GitReflogOwned>(),
            size_of::<*mut ffi::git_reflog>()
        );

        assert_eq!(
            size_of::<GitReflogEntry>(),
            size_of::<ffi::git_reflog_entry>()
        );
        assert_eq!(
            align_of::<GitReflogEntry>(),
            align_of::<ffi::git_reflog_entry>()
        );
        assert_eq!(
            size_of::<GitReflogEntryRef<'_>>(),
            size_of::<*const ffi::git_reflog_entry>()
        );
        assert_eq!(
            size_of::<GitReflogEntryMut<'_>>(),
            size_of::<*mut ffi::git_reflog_entry>()
        );
    }

    #[test]
    fn null_reflog_seams_create_no_handle() {
        // SAFETY: all conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitReflogRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitReflogMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitReflogOwned::from_raw(ptr::null_mut()).is_none());
            assert!(GitReflogEntryRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitReflogEntryMut::from_ptr(ptr::null_mut()).is_none());
        }
    }
}
