//! Safe wrappers for libgit2 cherrypick APIs.

use crate::api::cherrypick::{GitCherrypickOptionsMut, GitCherrypickOptionsRef};

/// Wraps: git_cherrypick_commit
/// Computes the in-memory index produced by cherry-picking one commit.
///
/// The repository is borrowed exclusively. The cherry-pick runs the same merge
/// machinery [`git_merge_trees`](crate::merge::git_merge_trees) does:
/// `merge_normalize_opts` reaches `git_repository_config__weakptr`, which
/// installs `repo->_config` on first use, and the merge-base walk opens the
/// object database the same way. Neither write is visible to a shared handle.
pub fn git_cherrypick_commit(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    cherrypick_commit: crate::commit::GitCommitRef<'_>,
    our_commit: crate::commit::GitCommitRef<'_>,
    mainline: core::ffi::c_uint,
    options: Option<crate::api::merge::GitMergeOptionsRef<'_, '_>>,
) -> Result<crate::index::GitIndexOwned, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the output slot is writable, the repository is exclusively
    // borrowed for the lazy subsystem writes the merge performs, every other
    // borrowed input remains live for the synchronous merge, and no input
    // ownership is transferred.
    let status = unsafe {
        crate::ffi::git_cherrypick_commit(
            &mut out,
            repo.as_mut_ptr(),
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
///
/// The commit is borrowed exclusively. Building the merge message calls
/// `git_commit_summary`, which computes the first paragraph of the message
/// and stores it in `commit->summary` on first use — the same lazy cache that
/// makes [`git_commit_summary`](crate::commit::git_commit_summary) take the
/// exclusive handle.
pub fn git_cherrypick(
    repository: &mut crate::repository::GitRepositoryMut<'_>,
    commit: &mut crate::commit::GitCommitMut<'_>,
    options: Option<GitCherrypickOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository and the commit are both exclusively borrowed for
    // the writes this call performs through them, and the optional options
    // stay live and read-only for the synchronous call.
    let status = unsafe {
        crate::ffi::git_cherrypick(repository.as_mut_ptr(), commit.as_mut_ptr(), options)
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

    /// The shape of the in-memory cherry-pick.
    type CommitCherrypick = fn(
        &mut crate::repository::GitRepositoryMut<'_>,
        crate::commit::GitCommitRef<'_>,
        crate::commit::GitCommitRef<'_>,
        core::ffi::c_uint,
        Option<crate::api::merge::GitMergeOptionsRef<'_, '_>>,
    ) -> Result<crate::index::GitIndexOwned, i32>;

    #[test]
    fn the_in_memory_cherrypick_borrows_the_repository_exclusively() {
        // The commit form runs the merge machinery, which installs the
        // repository config on first use, so it takes the exclusive handle
        // that the working-directory form already required.
        let _: CommitCherrypick = git_cherrypick_commit;
    }

    /// The shape of the working-directory cherry-pick.
    type WorkdirCherrypick = fn(
        &mut crate::repository::GitRepositoryMut<'_>,
        &mut crate::commit::GitCommitMut<'_>,
        Option<GitCherrypickOptionsRef<'_, '_>>,
    ) -> Result<(), i32>;

    #[test]
    fn the_working_directory_cherrypick_borrows_the_commit_exclusively() {
        // `git_cherrypick` builds its merge message from
        // `git_commit_summary`, which populates `commit->summary` on first
        // use, so the commit cannot be handed over as a shared handle.
        let _: WorkdirCherrypick = git_cherrypick;
    }

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
