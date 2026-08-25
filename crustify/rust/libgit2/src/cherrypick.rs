//! Safe wrappers for libgit2 cherrypick APIs.

/// Wraps: git_cherrypick_commit
/// Computes the in-memory index produced by cherry-picking one commit.
pub fn git_cherrypick_commit(
    repo: crate::repository::GitRepositoryRef<'_>,
    cherrypick_commit: crate::commit::GitCommitRef<'_>,
    our_commit: crate::commit::GitCommitRef<'_>,
    mainline: core::ffi::c_uint,
    options: Option<crate::api::merge::GitMergeOptionsRef<'_, '_>>,
) -> Result<crate::index::GitIndexOwned, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the output slot is writable, all borrowed inputs remain live for
    // the synchronous merge, and no input ownership is transferred.
    let status = unsafe {
        crate::ffi::git_cherrypick_commit(
            &mut out,
            repo.as_ptr().cast_mut(),
            cherrypick_commit.as_ptr().cast_mut(),
            our_commit.as_ptr().cast_mut(),
            mainline,
            options,
        )
    };
    // SAFETY: a non-null output is a newly owned complete index. Adopting it
    // before checking status also releases any output published on error.
    let out = unsafe { crate::index::GitIndexOwned::from_raw(out) };
    if status == 0 {
        out.ok_or(crate::ffi::git_error_code_GIT_ERROR)
    } else {
        drop(out);
        Err(status)
    }
}
