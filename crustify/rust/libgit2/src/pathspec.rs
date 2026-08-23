//! Safe wrappers for libgit2 pathspec APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_pathspec
    /// An opaque compiled pathspec managed by libgit2.
    ///
    /// Each owner represents one reference count. Dropping it calls
    /// `git_pathspec_free`; libgit2 does not publish an operation for acquiring
    /// another count, so the owner intentionally does not implement `Clone`.
    GitPathspec,
    GitPathspecRef,
    GitPathspecMut,
    ffi::git_pathspec
);

/// An owned reference count to a compiled pathspec.
pub type GitPathspecOwned = CBox<GitPathspec>;

// SAFETY: `git_pathspec_free` consumes exactly one reference to a complete
// `git_pathspec`, releasing its owned fields and allocation when the final
// count is dropped. It accepts null, although `CBox` supplies a live non-null
// object exactly once.
ffibox::impl_dropped!(GitPathspec, ffi::git_pathspec, ffi::git_pathspec_free);

ffibox::define_ctype!(
    /// Wraps: git_pathspec_match_list
    /// An opaque list of pathspec matches owned by libgit2.
    ///
    /// Dropping an owner releases the list, its retained pathspec reference,
    /// and its private arrays and pool. Accessors for diff-backed match lists
    /// must additionally preserve their source diff's lifetime.
    GitPathspecMatchList,
    GitPathspecMatchListRef,
    GitPathspecMatchListMut,
    ffi::git_pathspec_match_list
);

/// An owning handle to a complete pathspec match list.
pub type GitPathspecMatchListOwned = CBox<GitPathspecMatchList>;

// SAFETY: `git_pathspec_match_list_free` is the public destructor for a
// complete match-list allocation. It accepts null, although `CBox` supplies a
// live non-null object exactly once, and releases every owned resource before
// freeing the header.
ffibox::impl_dropped!(
    GitPathspecMatchList,
    ffi::git_pathspec_match_list,
    ffi::git_pathspec_match_list_free
);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_types_preserve_the_c_seam_and_lifecycle_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitPathspec>();
        assert_dropped::<GitPathspec>();
        assert_eq!(size_of::<GitPathspec>(), size_of::<ffi::git_pathspec>());
        assert_eq!(align_of::<GitPathspec>(), align_of::<ffi::git_pathspec>());
        assert_eq!(
            size_of::<GitPathspecRef<'_>>(),
            size_of::<*const ffi::git_pathspec>()
        );
        assert_eq!(
            size_of::<GitPathspecMut<'_>>(),
            size_of::<*mut ffi::git_pathspec>()
        );
        assert_eq!(
            size_of::<GitPathspecOwned>(),
            size_of::<*mut ffi::git_pathspec>()
        );

        assert_cell::<GitPathspecMatchList>();
        assert_dropped::<GitPathspecMatchList>();
        assert_eq!(
            size_of::<GitPathspecMatchList>(),
            size_of::<ffi::git_pathspec_match_list>()
        );
        assert_eq!(
            align_of::<GitPathspecMatchList>(),
            align_of::<ffi::git_pathspec_match_list>()
        );
        assert_eq!(
            size_of::<GitPathspecMatchListRef<'_>>(),
            size_of::<*const ffi::git_pathspec_match_list>()
        );
        assert_eq!(
            size_of::<GitPathspecMatchListMut<'_>>(),
            size_of::<*mut ffi::git_pathspec_match_list>()
        );
        assert_eq!(
            size_of::<GitPathspecMatchListOwned>(),
            size_of::<*mut ffi::git_pathspec_match_list>()
        );
    }

    #[test]
    fn null_seams_create_no_pathspec_handles() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting any object.
        unsafe {
            assert!(GitPathspecRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPathspecMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPathspecOwned::from_raw(ptr::null_mut()).is_none());
            assert!(GitPathspecMatchListRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPathspecMatchListMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPathspecMatchListOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
