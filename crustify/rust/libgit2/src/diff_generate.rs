//! Safe wrappers for libgit2 diff_generate APIs.

/// A generated diff tied to the repository pointer stored in its C header.
pub struct RepositoryDiff<'repo> {
    inner: crate::diff::DiffOwned,
    _repository: core::marker::PhantomData<crate::repository::GitRepositoryRef<'repo>>,
}

impl RepositoryDiff<'_> {
    /// Borrows the diff for read-only operations.
    #[must_use]
    pub fn as_ref(&self) -> crate::diff::DiffRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the diff exclusively for transformations and traversal.
    #[must_use]
    pub fn as_mut(&mut self) -> crate::diff::DiffMut<'_> {
        self.inner.as_mut()
    }
}

fn diff_result<'repo>(
    status: i32,
    inner: Option<crate::diff::DiffOwned>,
) -> Result<RepositoryDiff<'repo>, i32> {
    if status == 0 {
        Ok(RepositoryDiff {
            inner: inner.ok_or(crate::ffi::git_error_code_GIT_ERROR)?,
            _repository: core::marker::PhantomData,
        })
    } else {
        drop(inner);
        Err(status)
    }
}

#[cfg(test)]
mod owner_result_tests {
    use super::*;

    #[test]
    fn typed_diff_result_rejects_a_null_success_output() {
        assert!(matches!(
            diff_result::<'static>(0, None),
            Err(crate::ffi::git_error_code_GIT_ERROR)
        ));
    }

    #[test]
    fn typed_diff_result_preserves_a_constructor_error() {
        assert!(matches!(diff_result::<'static>(-7, None), Err(-7)));
    }
}

/// Wraps: git_diff_index_to_index
/// Creates a generated diff between two index snapshots.
pub fn git_diff_index_to_index<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    old_index: crate::index::GitIndexRef<'_>,
    new_index: crate::index::GitIndexRef<'_>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the output is writable and all borrowed inputs remain live for
    // generation. A non-null output transfers one complete count, which is
    // adopted before leaving this FFI seam.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_index_to_index(
            &mut out,
            repo.as_ptr().cast_mut(),
            old_index.as_ptr().cast_mut(),
            new_index.as_ptr().cast_mut(),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
}

/// Wraps: git_diff_index_to_workdir
/// Creates a diff from an index, or the repository index, to the worktree.
pub fn git_diff_index_to_workdir<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    mut index: Option<&mut crate::index::GitIndexMut<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let index = index
        .as_mut()
        .map_or(core::ptr::null_mut(), |index| index.as_mut_ptr());
    // SAFETY: output and optional index are writable, all inputs are live for
    // generation, and a transferred output count is adopted before leaving
    // the seam. The result carries the retained repository lifetime.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_index_to_workdir(
            &mut out,
            repo.as_ptr().cast_mut(),
            index,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
}

/// Wraps: git_diff_tree_to_index
/// Creates a diff from an optional tree to an index or repository index.
pub fn git_diff_tree_to_index<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    old_tree: Option<crate::tree::GitTreeRef<'_>>,
    index: Option<crate::index::GitIndexRef<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let old_tree = old_tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    let index = index.map_or(core::ptr::null_mut(), |index| index.as_ptr().cast_mut());
    // SAFETY: the output is writable, optional inputs are null or live, and a
    // non-null output transfers one complete count. The result retains only
    // the repository pointer whose lifetime it carries.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_tree_to_index(
            &mut out,
            repo.as_ptr().cast_mut(),
            old_tree,
            index,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
}

/// Wraps: git_diff_tree_to_tree
/// Creates a diff between optional trees, where an absent side is empty.
pub fn git_diff_tree_to_tree<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    old_tree: Option<crate::tree::GitTreeRef<'_>>,
    new_tree: Option<crate::tree::GitTreeRef<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let old_tree = old_tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    let new_tree = new_tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    // SAFETY: the output is writable, every non-null borrowed input remains
    // live for generation, and a non-null output transfers one complete count.
    // The result keeps the stored repository live.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_tree_to_tree(
            &mut out,
            repo.as_ptr().cast_mut(),
            old_tree,
            new_tree,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
}

/// Wraps: git_diff_tree_to_workdir
/// Creates a strict tree-to-worktree diff, using an empty tree when absent.
pub fn git_diff_tree_to_workdir<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    old_tree: Option<crate::tree::GitTreeRef<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let old_tree = old_tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    // SAFETY: the output is writable, optional inputs are null or live, and a
    // non-null output transfers one complete count. The result's repository
    // borrow covers every later use of its stored pointer.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_tree_to_workdir(
            &mut out,
            repo.as_ptr().cast_mut(),
            old_tree,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
}

/// Wraps: git_diff_tree_to_workdir_with_index
/// Creates a tree-to-worktree diff blended with repository-index state.
pub fn git_diff_tree_to_workdir_with_index<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    tree: Option<crate::tree::GitTreeRef<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let tree = tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    // SAFETY: the output is writable, optional inputs are null or live, and a
    // non-null output transfers one complete count. The returned wrapper
    // retains the repository lifetime stored by the diff.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_tree_to_workdir_with_index(
            &mut out,
            repo.as_ptr().cast_mut(),
            tree,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
}
