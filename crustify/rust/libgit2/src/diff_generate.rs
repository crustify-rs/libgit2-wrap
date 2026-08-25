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
    out: *mut crate::ffi::git_diff,
) -> Result<RepositoryDiff<'repo>, i32> {
    // SAFETY: a non-null constructor output transfers one complete diff count.
    let inner = unsafe { crate::diff::DiffOwned::from_raw(out) };
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

fn options_ptr(
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> *const crate::ffi::git_diff_options {
    options.map_or(core::ptr::null(), |options| options.as_ptr())
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
    // generation; the returned wrapper keeps the stored repository live.
    let status = unsafe {
        crate::ffi::git_diff_index_to_index(
            &mut out,
            repo.as_ptr().cast_mut(),
            old_index.as_ptr().cast_mut(),
            new_index.as_ptr().cast_mut(),
            options_ptr(options),
        )
    };
    diff_result(status, out)
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
    // generation, and the result carries the retained repository lifetime.
    let status = unsafe {
        crate::ffi::git_diff_index_to_workdir(
            &mut out,
            repo.as_ptr().cast_mut(),
            index,
            options_ptr(options),
        )
    };
    diff_result(status, out)
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
    // SAFETY: the output is writable, optional inputs are null or live, and
    // the result retains only the repository pointer whose lifetime it carries.
    let status = unsafe {
        crate::ffi::git_diff_tree_to_index(
            &mut out,
            repo.as_ptr().cast_mut(),
            old_tree,
            index,
            options_ptr(options),
        )
    };
    diff_result(status, out)
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
    // SAFETY: the output is writable and every non-null borrowed input remains
    // live for generation; the result keeps the stored repository live.
    let status = unsafe {
        crate::ffi::git_diff_tree_to_tree(
            &mut out,
            repo.as_ptr().cast_mut(),
            old_tree,
            new_tree,
            options_ptr(options),
        )
    };
    diff_result(status, out)
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
    // SAFETY: the output is writable and optional inputs are null or live; the
    // result's repository borrow covers every later use of its stored pointer.
    let status = unsafe {
        crate::ffi::git_diff_tree_to_workdir(
            &mut out,
            repo.as_ptr().cast_mut(),
            old_tree,
            options_ptr(options),
        )
    };
    diff_result(status, out)
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
    // SAFETY: the output is writable and optional inputs are null or live; the
    // returned wrapper retains the repository lifetime stored by the diff.
    let status = unsafe {
        crate::ffi::git_diff_tree_to_workdir_with_index(
            &mut out,
            repo.as_ptr().cast_mut(),
            tree,
            options_ptr(options),
        )
    };
    diff_result(status, out)
}
