//! Safe wrappers for libgit2 worktree APIs.

use core::ffi::CStr;
use core::ptr::{addr_of, addr_of_mut};

use ffibox::{CBox, CVal};

use crate::api::buffer::GitBufMut;
use crate::api::worktree::GitWorktreePruneFlags;
use crate::ffi;
use crate::repository::GitRepositoryRef;
use crate::strarray::GitStrArray;

ffibox::define_ctype!(
    /// Wraps: git_worktree
    /// An opaque worktree managed by libgit2.
    ///
    /// An owned handle represents one libgit2-allocated worktree and releases
    /// it with `git_worktree_free`. That destructor frees the six separately
    /// allocated path and name strings before the header, and tolerates the
    /// null fields a partially constructed worktree still carries, so a handle
    /// adopted from a failed constructor is released correctly too.
    GitWorktree,
    GitWorktreeRef,
    GitWorktreeMut,
    ffi::git_worktree
);

/// An owned libgit2 worktree allocation.
pub type GitWorktreeOwned = CBox<GitWorktree>;

// SAFETY: `git_worktree_free` is the public destructor for a libgit2-allocated
// `git_worktree`. It releases the six owned strings and the header exactly
// once, treating a null string field as a no-op. It accepts a null worktree
// too, although `CBox` supplies one live, non-null allocation.
ffibox::impl_dropped!(GitWorktree, ffi::git_worktree, ffi::git_worktree_free);

/// Wraps: git_worktree_is_locked
/// Reports whether a worktree is locked and optionally fills its reason.
pub fn git_worktree_is_locked(
    worktree: GitWorktreeRef<'_>,
    reason: Option<&mut GitBufMut<'_>>,
) -> Result<bool, i32> {
    let reason = reason.map_or(core::ptr::null_mut(), GitBufMut::as_mut_ptr);
    // SAFETY: the worktree is live and shared. `reason` is null or an
    // exclusively borrowed valid buffer header that libgit2 may replace.
    let status = unsafe { ffi::git_worktree_is_locked(reason, worktree.as_ptr()) };
    if status < 0 {
        Err(status)
    } else {
        Ok(status != 0)
    }
}

/// Wraps: git_worktree_is_prunable
/// Reports whether a worktree may be pruned under `options`.
pub fn git_worktree_is_prunable(
    worktree: GitWorktreeRef<'_>,
    options: Option<GitWorktreePruneOptionsRef<'_>>,
) -> Result<bool, i32> {
    let options = options.map_or(core::ptr::null_mut(), |options| options.as_ptr().cast_mut());
    // SAFETY: the worktree and optional validated options are live shared
    // borrows; the C body only reads them while inspecting the filesystem.
    let status = unsafe { ffi::git_worktree_is_prunable(worktree.as_ptr().cast_mut(), options) };
    if status < 0 {
        Err(status)
    } else {
        Ok(status != 0)
    }
}

/// Wraps: git_worktree_list
/// Returns the linked worktree names for a repository.
pub fn git_worktree_list(repository: GitRepositoryRef<'_>) -> Result<CVal<GitStrArray>, i32> {
    let mut names = GitStrArray::new();
    let status = {
        let mut output = names.as_mut();
        // SAFETY: `output` is an empty exclusive output header and the live
        // repository is only read while its worktree directory is scanned.
        unsafe { ffi::git_worktree_list(output.as_mut_ptr(), repository.as_ptr().cast_mut()) }
    };
    if status == 0 { Ok(names) } else { Err(status) }
}

/// Wraps: git_worktree_lock
/// Locks a worktree, optionally recording a reason.
pub fn git_worktree_lock(
    worktree: &mut GitWorktreeMut<'_>,
    reason: Option<&CStr>,
) -> Result<(), i32> {
    let reason = reason.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the worktree is live and exclusive and the optional reason is a
    // live C string read synchronously before the worktree's state is updated.
    let status = unsafe { ffi::git_worktree_lock(worktree.as_mut_ptr(), reason) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_worktree_lookup
/// Looks up and adopts a worktree by name.
pub fn git_worktree_lookup(
    repository: GitRepositoryRef<'_>,
    name: &CStr,
) -> Result<GitWorktreeOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable and both inputs are live for the
    // call; success produces a self-contained caller-owned worktree.
    let status = unsafe {
        ffi::git_worktree_lookup(
            core::ptr::addr_of_mut!(raw),
            repository.as_ptr().cast_mut(),
            name.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes a non-null complete worktree allocation.
    unsafe { GitWorktreeOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_worktree_name
/// Borrows the worktree's name.
#[must_use]
pub fn git_worktree_name<'a>(worktree: GitWorktreeRef<'a>) -> &'a CStr {
    // SAFETY: the live worktree owns the returned non-null NUL string.
    let name = unsafe { ffi::git_worktree_name(worktree.as_ptr()) };
    // SAFETY: the string remains live for the worktree handle lifetime.
    unsafe { CStr::from_ptr(name) }
}

/// Wraps: git_worktree_open_from_repository
/// Opens and adopts the worktree described by a linked repository.
pub fn git_worktree_open_from_repository(
    repository: GitRepositoryRef<'_>,
) -> Result<GitWorktreeOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable and the repository remains live for
    // the call; success returns a self-contained caller-owned worktree.
    let status = unsafe {
        ffi::git_worktree_open_from_repository(
            core::ptr::addr_of_mut!(raw),
            repository.as_ptr().cast_mut(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes a non-null complete worktree allocation.
    unsafe { GitWorktreeOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_worktree_path
/// Borrows the worktree's filesystem path.
#[must_use]
pub fn git_worktree_path<'a>(worktree: GitWorktreeRef<'a>) -> &'a CStr {
    // SAFETY: the live worktree owns the returned non-null NUL string.
    let path = unsafe { ffi::git_worktree_path(worktree.as_ptr()) };
    // SAFETY: the string remains live for the worktree handle lifetime.
    unsafe { CStr::from_ptr(path) }
}

/// Wraps: git_worktree_prune
/// Removes the worktree's administrative data under `options`.
pub fn git_worktree_prune(
    worktree: &mut GitWorktreeMut<'_>,
    options: Option<GitWorktreePruneOptionsRef<'_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null_mut(), |options| options.as_ptr().cast_mut());
    // SAFETY: the worktree is live and exclusive and optional options are live
    // and read-only for the filesystem operation.
    let status = unsafe { ffi::git_worktree_prune(worktree.as_mut_ptr(), options) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_worktree_prune_options_init
/// Constructs prune options for the current public ABI version.
pub fn git_worktree_prune_options_init() -> Result<GitWorktreePruneOptions, i32> {
    let mut options = GitWorktreePruneOptions::zeroed();
    // SAFETY: `options` is writable layout-compatible storage and the version
    // constant was generated from the same public headers as the function.
    let status = unsafe {
        ffi::git_worktree_prune_options_init(
            core::ptr::addr_of_mut!(options).cast(),
            ffi::GIT_WORKTREE_PRUNE_OPTIONS_VERSION,
        )
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_worktree_unlock
/// Unlocks a worktree, returning `false` when it was already unlocked.
pub fn git_worktree_unlock(worktree: &mut GitWorktreeMut<'_>) -> Result<bool, i32> {
    // SAFETY: the worktree is live and exclusive while libgit2 updates its
    // lockfile and cached lock flag.
    match unsafe { ffi::git_worktree_unlock(worktree.as_mut_ptr()) } {
        0 => Ok(true),
        1 => Ok(false),
        error => Err(error),
    }
}

/// Wraps: git_worktree_validate
/// Checks that both sides of a linked worktree still exist and agree.
pub fn git_worktree_validate(worktree: GitWorktreeRef<'_>) -> Result<(), i32> {
    // SAFETY: the live worktree is shared and the C function only reads its
    // paths while validating the filesystem.
    let status = unsafe { ffi::git_worktree_validate(worktree.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

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
    /// Field: git_worktree_prune_options.flags
    /// Returns the validated worktree-pruning overrides.
    pub fn flags(&self) -> Result<GitWorktreePruneFlags, ffi::git_worktree_prune_t> {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        let flags = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitWorktreePruneFlags::from_bits(flags).ok_or(flags)
    }

    /// Field: git_worktree_prune_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }
}

impl GitWorktreePruneOptionsMut<'_> {
    /// Sets the worktree-pruning overrides.
    pub fn set_flags(&mut self, flags: GitWorktreePruneFlags) {
        // SAFETY: this exclusive handle permits a raw-place write of the
        // scalar without forming a reference to C-visible memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
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
        assert_eq!(options.as_ref().flags(), Ok(GitWorktreePruneFlags::NONE));

        options.set_version(2);
        options.set_flags(GitWorktreePruneFlags::VALID | GitWorktreePruneFlags::WORKING_TREE);
        assert_eq!(options.as_ref().version(), 2);
        assert_eq!(
            options.as_ref().flags(),
            Ok(GitWorktreePruneFlags::VALID | GitWorktreePruneFlags::WORKING_TREE)
        );
    }

    #[test]
    fn prune_options_constructor_uses_the_current_version_and_defaults() {
        let mut options = git_worktree_prune_options_init().expect("current version is supported");
        // SAFETY: `options` is initialized, remains live for the handle, and
        // this scope uses only this shared access path.
        let options = unsafe {
            GitWorktreePruneOptionsRef::from_ptr(
                core::ptr::addr_of_mut!(options).cast::<ffi::git_worktree_prune_options>(),
            )
        }
        .expect("the address of a stack value is non-null");
        assert_eq!(options.version(), ffi::GIT_WORKTREE_PRUNE_OPTIONS_VERSION);
        assert_eq!(options.flags(), Ok(GitWorktreePruneFlags::NONE));
    }

    #[test]
    fn prune_options_reject_unknown_override_bits() {
        let unknown = GitWorktreePruneFlags::ALL.bits() << 1;
        let mut raw = ffi::git_worktree_prune_options {
            version: 1,
            flags: unknown,
        };

        // SAFETY: `raw` is initialized and live for the shared handle's use.
        let options = unsafe { GitWorktreePruneOptionsRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(options.flags(), Err(unknown));
    }
}
