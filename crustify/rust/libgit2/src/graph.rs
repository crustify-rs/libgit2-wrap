//! Safe wrappers for libgit2 graph APIs.

use crate::ffi;
use crate::oid::OidRef;
use crate::repository::GitRepositoryRef;

/// Wraps: git_graph_ahead_behind
/// Counts commits unique to the local and upstream histories.
pub fn git_graph_ahead_behind(
    repository: GitRepositoryRef<'_>,
    local: OidRef<'_>,
    upstream: OidRef<'_>,
) -> Result<(usize, usize), i32> {
    let (mut ahead, mut behind) = (0, 0);
    // SAFETY: both outputs are writable and all three borrowed handles remain
    // live for this non-retaining graph walk.
    let status = unsafe {
        ffi::git_graph_ahead_behind(
            &mut ahead,
            &mut behind,
            repository.as_ptr().cast_mut(),
            local.as_ptr(),
            upstream.as_ptr(),
        )
    };
    if status == 0 {
        Ok((ahead, behind))
    } else {
        Err(status)
    }
}

/// Wraps: git_graph_descendant_of
/// Tests strict ancestry; a commit is not considered its own descendant.
pub fn git_graph_descendant_of(
    repository: GitRepositoryRef<'_>,
    commit: OidRef<'_>,
    ancestor: OidRef<'_>,
) -> Result<bool, i32> {
    // SAFETY: all borrowed handles remain live and C retains none of them.
    let status = unsafe {
        ffi::git_graph_descendant_of(
            repository.as_ptr().cast_mut(),
            commit.as_ptr(),
            ancestor.as_ptr(),
        )
    };
    match status {
        0 => Ok(false),
        1 => Ok(true),
        error => Err(error),
    }
}
