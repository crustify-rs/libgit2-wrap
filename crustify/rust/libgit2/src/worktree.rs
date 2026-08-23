//! Safe wrappers for libgit2 worktree APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_worktree
    /// An opaque worktree managed by libgit2.
    ///
    /// An owned handle represents one fully constructed worktree allocation and
    /// releases it with `git_worktree_free`.
    GitWorktree,
    GitWorktreeRef,
    GitWorktreeMut,
    ffi::git_worktree
);

/// An owned libgit2 worktree allocation.
pub type GitWorktreeOwned = CBox<GitWorktree>;

// SAFETY: `git_worktree_free` is the public destructor for a fully constructed
// libgit2-allocated `git_worktree`. It releases all owned strings and the
// header exactly once. It accepts null, although `CBox` supplies one live,
// non-null allocation.
ffibox::impl_dropped!(GitWorktree, ffi::git_worktree, ffi::git_worktree_free);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_worktree_preserves_the_c_layout_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitWorktree>();
        assert_dropped::<GitWorktree>();
        assert_eq!(size_of::<GitWorktree>(), size_of::<ffi::git_worktree>());
        assert_eq!(align_of::<GitWorktree>(), align_of::<ffi::git_worktree>());
        assert_eq!(
            size_of::<GitWorktreeRef<'_>>(),
            size_of::<*const ffi::git_worktree>()
        );
        assert_eq!(
            size_of::<GitWorktreeMut<'_>>(),
            size_of::<*mut ffi::git_worktree>()
        );
        assert_eq!(
            size_of::<GitWorktreeOwned>(),
            size_of::<*mut ffi::git_worktree>()
        );
    }

    #[test]
    fn null_worktree_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitWorktreeRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitWorktreeMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitWorktreeOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
