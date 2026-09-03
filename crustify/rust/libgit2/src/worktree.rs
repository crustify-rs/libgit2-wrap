//! Safe wrappers for libgit2 worktree APIs.

use core::ffi::CStr;
use core::ptr::{addr_of, addr_of_mut};

use ffibox::{CBox, CVal};

use crate::api::buffer::GitBufMut;
use crate::api::worktree::{
    GitWorktreeAddOptionsMut, GitWorktreeAddOptionsRef, GitWorktreePruneFlags,
};
use crate::ffi;
use crate::repository::{GitRepositoryMut, GitRepositoryRef};
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
mod unit_tests {
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

/// Wraps: git_worktree_add
/// Creates and checks out a linked worktree.
pub fn git_worktree_add(
    repository: &mut GitRepositoryMut<'_>,
    name: &CStr,
    path: &CStr,
    options: Option<GitWorktreeAddOptionsRef<'_, '_>>,
) -> Result<GitWorktreeOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable, the repository is exclusive, and both names
    // plus optional options remain live for the synchronous filesystem work.
    let status = unsafe {
        ffi::git_worktree_add(
            &mut out,
            repository.as_mut_ptr(),
            name.as_ptr(),
            path.as_ptr(),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    if status == 0 {
        // SAFETY: success transfers one complete libgit2 worktree allocation.
        unsafe { GitWorktreeOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
    } else {
        Err(status)
    }
}

/// Wraps: git_worktree_add_options_init
/// Initializes worktree-add options for `version`.
pub fn git_worktree_add_options_init(
    options: &mut GitWorktreeAddOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage and the initializer retains no pointer.
    let status = unsafe { ffi::git_worktree_add_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, RawBuf, TempDir, safe_buf_bytes};

    unsafe fn raw_add(
        repository: *mut ffi::git_repository,
        path: &CStr,
    ) -> (Vec<u8>, Vec<Vec<u8>>, Vec<u8>) {
        let mut worktree = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_worktree_add(
                    &mut worktree,
                    repository,
                    c"linked".as_ptr(),
                    path.as_ptr(),
                    core::ptr::null(),
                )
            },
            0
        );
        assert_eq!(unsafe { ffi::git_worktree_validate(worktree) }, 0);
        assert_eq!(
            unsafe { ffi::git_worktree_lock(worktree, c"equivalence lock".as_ptr()) },
            0
        );
        let mut reason = RawBuf::new();
        assert_eq!(
            unsafe { ffi::git_worktree_is_locked(&mut reason.0, worktree) },
            1
        );
        assert_eq!(unsafe { ffi::git_worktree_unlock(worktree) }, 0);
        let name = unsafe { CStr::from_ptr(ffi::git_worktree_name(worktree)) }
            .to_bytes()
            .to_vec();
        let mut list = ffi::git_strarray {
            strings: core::ptr::null_mut(),
            count: 0,
        };
        assert_eq!(unsafe { ffi::git_worktree_list(&mut list, repository) }, 0);
        let names = (0..list.count)
            .map(|index| {
                unsafe { CStr::from_ptr(*list.strings.add(index)) }
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        unsafe {
            ffi::git_strarray_dispose(&mut list);
            ffi::git_worktree_free(worktree);
        }
        (name, names, reason.bytes())
    }

    fn safe_add(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
        path: &CStr,
    ) -> (Vec<u8>, Vec<Vec<u8>>, Vec<u8>) {
        let mut worktree = git_worktree_add(repository, c"linked", path, None).unwrap();
        git_worktree_validate(worktree.as_ref()).unwrap();
        git_worktree_lock(&mut worktree.as_mut(), Some(c"equivalence lock")).unwrap();
        let mut reason = crate::api::buffer::GitBuf::new();
        assert!(git_worktree_is_locked(worktree.as_ref(), Some(&mut reason.as_mut())).unwrap());
        assert!(git_worktree_unlock(&mut worktree.as_mut()).unwrap());
        let name = git_worktree_name(worktree.as_ref()).to_bytes().to_vec();
        let list = git_worktree_list(repository.as_ref()).unwrap();
        let strings = list.as_ref().strings().unwrap();
        let names = (0..strings.len())
            .map(|index| strings.get(index).unwrap().to_bytes().to_vec())
            .collect();
        (name, names, safe_buf_bytes(reason.as_ref()))
    }

    unsafe fn raw_open_and_prune(
        repository: *mut ffi::git_repository,
        path: &CStr,
    ) -> (Vec<u8>, Vec<u8>, bool, bool, bool, usize) {
        let mut worktree = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_worktree_add(
                    &mut worktree,
                    repository,
                    c"ephemeral".as_ptr(),
                    path.as_ptr(),
                    core::ptr::null(),
                )
            },
            0
        );
        let mut looked_up = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_worktree_lookup(&mut looked_up, repository, c"ephemeral".as_ptr(),) },
            0
        );
        let mut linked_repository = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_open_from_worktree(&mut linked_repository, looked_up) },
            0
        );
        let mut reopened = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_worktree_open_from_repository(&mut reopened, linked_repository) },
            0
        );
        let reopened_name = unsafe { CStr::from_ptr(ffi::git_worktree_name(reopened)) }
            .to_bytes()
            .to_vec();
        let reopened_path = unsafe { CStr::from_ptr(ffi::git_worktree_path(reopened)) }
            .to_bytes()
            .to_vec();
        let mut head = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_repository_head_for_worktree(&mut head, repository, c"ephemeral".as_ptr())
            },
            0
        );
        let detached = unsafe {
            ffi::git_repository_head_detached_for_worktree(repository, c"ephemeral".as_ptr())
        } != 0;
        let prunable =
            unsafe { ffi::git_worktree_is_prunable(worktree, core::ptr::null_mut()) } != 0;
        let already_unlocked = unsafe { ffi::git_worktree_unlock(worktree) } == 1;

        unsafe {
            ffi::git_reference_free(head);
            ffi::git_worktree_free(reopened);
            ffi::git_repository_free(linked_repository);
            ffi::git_worktree_free(looked_up);
        }
        let mut prune = unsafe { core::mem::zeroed::<ffi::git_worktree_prune_options>() };
        assert_eq!(
            unsafe {
                ffi::git_worktree_prune_options_init(
                    &mut prune,
                    ffi::GIT_WORKTREE_PRUNE_OPTIONS_VERSION,
                )
            },
            0
        );
        prune.flags = ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_VALID
            | ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_WORKING_TREE;
        assert_eq!(unsafe { ffi::git_worktree_prune(worktree, &mut prune) }, 0);
        let checkout_exists = std::path::Path::new(path.to_str().unwrap()).exists();
        let mut list = ffi::git_strarray {
            strings: core::ptr::null_mut(),
            count: 0,
        };
        assert_eq!(unsafe { ffi::git_worktree_list(&mut list, repository) }, 0);
        let remaining = list.count;
        unsafe {
            ffi::git_strarray_dispose(&mut list);
            ffi::git_worktree_free(worktree);
        }
        (
            reopened_name,
            reopened_path
                .rsplit(|byte| *byte == b'/')
                .next()
                .unwrap()
                .to_vec(),
            detached,
            prunable,
            already_unlocked && !checkout_exists,
            remaining,
        )
    }

    fn safe_open_and_prune(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
        path: &CStr,
    ) -> (Vec<u8>, Vec<u8>, bool, bool, bool, usize) {
        let mut worktree = git_worktree_add(repository, c"ephemeral", path, None).unwrap();
        let looked_up = git_worktree_lookup(repository.as_ref(), c"ephemeral").unwrap();
        let linked_repository =
            crate::repository::git_repository_open_from_worktree(looked_up.as_ref()).unwrap();
        let reopened = git_worktree_open_from_repository(linked_repository.as_ref()).unwrap();
        let reopened_name = git_worktree_name(reopened.as_ref()).to_bytes().to_vec();
        let reopened_path = git_worktree_path(reopened.as_ref()).to_bytes().to_vec();
        let head =
            crate::repository::git_repository_head_for_worktree(repository, c"ephemeral").unwrap();
        drop(head);
        let detached =
            crate::repository::git_repository_head_detached_for_worktree(repository, c"ephemeral")
                .unwrap();
        let prunable = git_worktree_is_prunable(worktree.as_ref(), None).unwrap();
        let already_unlocked = !git_worktree_unlock(&mut worktree.as_mut()).unwrap();
        drop(reopened);
        drop(linked_repository);
        drop(looked_up);

        let mut prune = git_worktree_prune_options_init().unwrap();
        let mut prune =
            unsafe { GitWorktreePruneOptionsMut::from_ptr(core::ptr::addr_of_mut!(prune).cast()) }
                .unwrap();
        prune.set_flags(GitWorktreePruneFlags::VALID | GitWorktreePruneFlags::WORKING_TREE);
        git_worktree_prune(&mut worktree.as_mut(), Some(prune.as_ref())).unwrap();
        let checkout_exists = std::path::Path::new(path.to_str().unwrap()).exists();
        let remaining = git_worktree_list(repository.as_ref())
            .unwrap()
            .as_ref()
            .count();
        (
            reopened_name,
            reopened_path
                .rsplit(|byte| *byte == b'/')
                .next()
                .unwrap()
                .to_vec(),
            detached,
            prunable,
            already_unlocked && !checkout_exists,
            remaining,
        )
    }

    #[test]
    fn io_equiv_linked_worktree_add_lock_and_list() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("worktree-parent-raw");
        let safe = HistoryFixture::new("worktree-parent-safe");
        let raw_checkout = TempDir::new("worktree-raw");
        let safe_checkout = TempDir::new("worktree-safe");
        let raw_checkout_path = raw_checkout.path().join("checkout");
        let safe_checkout_path = safe_checkout.path().join("checkout");
        let raw_path = std::ffi::CString::new(raw_checkout_path.to_str().unwrap()).unwrap();
        let safe_path = std::ffi::CString::new(safe_checkout_path.to_str().unwrap()).unwrap();
        let raw_observation = unsafe { raw_add(raw.repository.as_ptr(), &raw_path) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(raw_observation, safe_add(&mut safe_repository, &safe_path));
        assert_eq!(raw_observation.0, b"linked");
        assert_eq!(raw_observation.2, b"equivalence lock");
        assert_eq!(
            std::fs::read(raw_checkout_path.join("README.md")).unwrap(),
            std::fs::read(safe_checkout_path.join("README.md")).unwrap()
        );
    }

    #[test]
    fn io_equiv_worktree_lookup_repository_roundtrip_and_forced_prune() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("worktree-roundtrip-parent-raw");
        let safe = HistoryFixture::new("worktree-roundtrip-parent-safe");
        let raw_checkout = TempDir::new("worktree-roundtrip-raw");
        let safe_checkout = TempDir::new("worktree-roundtrip-safe");
        let raw_path =
            std::ffi::CString::new(raw_checkout.path().join("checkout").to_str().unwrap()).unwrap();
        let safe_path =
            std::ffi::CString::new(safe_checkout.path().join("checkout").to_str().unwrap())
                .unwrap();
        let raw_observation = unsafe { raw_open_and_prune(raw.repository.as_ptr(), &raw_path) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(
            raw_observation,
            safe_open_and_prune(&mut safe_repository, &safe_path)
        );
        assert_eq!(
            raw_observation,
            (
                b"ephemeral".to_vec(),
                b"checkout".to_vec(),
                true,
                false,
                true,
                0
            )
        );
    }
}

#[cfg(test)]
mod scheduled_add_tests {
    use super::*;
    use crate::api::worktree::GitWorktreeAddOptions;

    #[test]
    fn initializer_writes_the_current_version() {
        let mut options = GitWorktreeAddOptions::new();
        git_worktree_add_options_init(&mut options.as_mut(), ffi::GIT_WORKTREE_ADD_OPTIONS_VERSION)
            .unwrap();
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_WORKTREE_ADD_OPTIONS_VERSION
        );
    }
}
