//! Safe wrappers for libgit2 revert APIs.

use crate::api::revert::{GitRevertOptionsMut, GitRevertOptionsRef};

use crate::api::merge::GitMergeOptionsRef;
use crate::commit::GitCommitRef;
use crate::ffi;
use crate::repository::GitRepositoryMut;

/// Wraps: git_revert_commit
/// Reverts `revert_commit` against `ours` into a newly owned in-memory index.
pub fn git_revert_commit(
    repository: &mut GitRepositoryMut<'_>,
    revert_commit: GitCommitRef<'_>,
    ours: GitCommitRef<'_>,
    mainline: u32,
    options: Option<GitMergeOptionsRef<'_, '_>>,
) -> Result<crate::index::GitIndexOwned, i32> {
    let mut raw = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: `raw` is a writable owner slot; the repository is exclusively
    // borrowed and both commits and optional options remain live for the
    // synchronous revert. No input pointer is retained.
    let status = unsafe {
        ffi::git_revert_commit(
            core::ptr::addr_of_mut!(raw),
            repository.as_mut_ptr(),
            revert_commit.as_ptr().cast_mut(),
            ours.as_ptr().cast_mut(),
            mainline,
            options,
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success publishes one complete owned index.
    unsafe { crate::index::GitIndexOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_revert
/// Reverts `commit` into the repository's index and working directory.
pub fn git_revert(
    repository: &mut crate::repository::GitRepositoryMut<'_>,
    commit: crate::commit::GitCommitRef<'_>,
    options: Option<GitRevertOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    // SAFETY: the repository is exclusive; the commit and optional options
    // remain live and read-only for the synchronous operation.
    let status = unsafe {
        crate::ffi::git_revert(
            repository.as_mut_ptr(),
            commit.as_ptr().cast_mut(),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_revert_options_init
/// Initializes revert options for `version`.
pub fn git_revert_options_init(
    options: &mut GitRevertOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage and the initializer retains no pointer.
    let status = unsafe { crate::ffi::git_revert_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_wrapper_tests {
    use super::*;
    use crate::api::revert::GitRevertOptions;

    #[test]
    fn initializer_writes_the_current_version() {
        let mut options = GitRevertOptions::new();
        git_revert_options_init(&mut options.as_mut(), ffi::GIT_REVERT_OPTIONS_VERSION).unwrap();
        assert_eq!(options.as_ref().version(), ffi::GIT_REVERT_OPTIONS_VERSION);
    }
}
