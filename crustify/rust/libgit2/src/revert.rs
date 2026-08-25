//! Safe wrappers for libgit2 revert APIs.

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
