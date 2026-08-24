//! Safe wrappers for libgit2 object_api APIs.

use crate::ffi;
use crate::oid::OidRef;
use crate::repository::GitRepositoryRef;
use crate::tree::{GitTreeOwned, GitTreeRef, RepositoryTree};

/// Wraps: git_tree_id
/// Borrows the tree's inline object ID.
#[must_use]
pub fn git_tree_id<'tree>(tree: GitTreeRef<'tree>) -> OidRef<'tree> {
    // SAFETY: `tree` is live and libgit2 returns its non-null inline ID.
    let id = unsafe { ffi::git_tree_id(tree.as_ptr()) }.cast_mut();
    // SAFETY: the inline ID remains live for the tree handle's lifetime.
    unsafe { OidRef::from_ptr(id) }.expect("a live tree has an object ID")
}

/// Wraps: git_tree_lookup
/// Looks up a tree and ties its owned cache reference to `repository`.
pub fn git_tree_lookup<'repo>(
    repository: GitRepositoryRef<'repo>,
    id: OidRef<'_>,
) -> Result<RepositoryTree<'repo>, i32> {
    let mut tree = core::ptr::null_mut();
    // SAFETY: the output slot is writable, both inputs are live, and success
    // writes one owned tree cache reference backed by `repository`.
    let status = unsafe {
        ffi::git_tree_lookup(
            core::ptr::addr_of_mut!(tree),
            repository.as_ptr().cast_mut(),
            id.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes one complete non-null owned tree reference.
    let tree = unsafe { GitTreeOwned::from_raw(tree) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryTree::from_owned(tree, repository))
}
