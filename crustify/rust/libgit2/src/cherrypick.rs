//! Safe wrappers for libgit2 cherrypick APIs.

use crate::api::cherrypick::{GitCherrypickOptionsMut, GitCherrypickOptionsRef};

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

/// Wraps: git_cherrypick
/// Cherry-picks `commit` into the repository's index and working directory.
pub fn git_cherrypick(
    repository: &mut crate::repository::GitRepositoryMut<'_>,
    commit: crate::commit::GitCommitRef<'_>,
    options: Option<GitCherrypickOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository is exclusively borrowed and the commit and
    // optional options are live, read-only inputs for this synchronous call.
    let status = unsafe {
        crate::ffi::git_cherrypick(repository.as_mut_ptr(), commit.as_ptr().cast_mut(), options)
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_cherrypick_init_options
/// Initializes deprecated cherry-pick options for `version`.
pub fn git_cherrypick_init_options(
    options: &mut GitCherrypickOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable options storage and the
    // initializer retains none of its pointers.
    let status = unsafe { crate::ffi::git_cherrypick_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_wrapper_tests {
    use super::*;
    use crate::api::cherrypick::GitCherrypickOptions;

    #[test]
    fn deprecated_initializer_writes_the_current_version() {
        let mut options = GitCherrypickOptions::new();
        git_cherrypick_init_options(
            &mut options.as_mut(),
            crate::ffi::GIT_CHERRYPICK_OPTIONS_VERSION,
        )
        .unwrap();
        assert_eq!(
            options.as_ref().version(),
            crate::ffi::GIT_CHERRYPICK_OPTIONS_VERSION
        );
    }
}
