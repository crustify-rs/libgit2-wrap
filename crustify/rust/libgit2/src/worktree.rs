//! Safe wrappers for libgit2 worktree APIs.

use core::ptr::{addr_of, addr_of_mut};

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

ffibox::define_ctype!(
    /// Wraps: git_worktree_prune_options
    /// Options controlling whether a linked worktree may be pruned.
    GitWorktreePruneOptions,
    GitWorktreePruneOptionsRef,
    GitWorktreePruneOptionsMut,
    ffi::git_worktree_prune_options
);

impl GitWorktreePruneOptionsRef<'_> {
    /// Wraps: git_worktree_prune_options.flags
    /// Returns the bit set of `git_worktree_prune_t` options.
    #[must_use]
    pub fn flags(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Wraps: git_worktree_prune_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }
}

impl GitWorktreePruneOptionsMut<'_> {
    /// Replaces the bit set of `git_worktree_prune_t` options.
    pub fn set_flags(&mut self, flags: u32) {
        // SAFETY: this exclusive handle permits a raw-place write of the
        // scalar without forming a reference to C-visible memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Sets the ABI version of this options value.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: as `set_flags`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }
}

#[cfg(test)]
mod prune_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn prune_options_preserve_the_c_layout() {
        assert_eq!(
            size_of::<GitWorktreePruneOptions>(),
            size_of::<ffi::git_worktree_prune_options>()
        );
        assert_eq!(
            align_of::<GitWorktreePruneOptions>(),
            align_of::<ffi::git_worktree_prune_options>()
        );
        assert_eq!(
            size_of::<GitWorktreePruneOptionsRef<'_>>(),
            size_of::<*const ffi::git_worktree_prune_options>()
        );
        assert_eq!(
            size_of::<GitWorktreePruneOptionsMut<'_>>(),
            size_of::<*mut ffi::git_worktree_prune_options>()
        );
    }

    #[test]
    fn prune_options_handles_read_and_write_every_field() {
        let mut raw = ffi::git_worktree_prune_options {
            version: 1,
            flags: 0,
        };

        // SAFETY: `raw` is initialized, live for the handle's use, and this
        // exclusive handle is the only access path used during the borrow.
        let mut options = unsafe { GitWorktreePruneOptionsMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(options.as_ref().version(), 1);
        assert_eq!(options.as_ref().flags(), 0);

        options.set_version(2);
        options.set_flags(0b101);
        assert_eq!(options.as_ref().version(), 2);
        assert_eq!(options.as_ref().flags(), 0b101);
    }
}
