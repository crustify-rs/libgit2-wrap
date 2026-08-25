//! Safe wrappers for libgit2 diff_generate APIs.

/// A generated diff tied to the repository pointer stored in its C header.
///
/// `diff_generated_alloc` stores `diff->base.repo = repo` without a reference
/// count, and the generated diff dereferences it again whenever a delta is
/// loaded, so the repository must outlive every diff produced from it. Each
/// constructor takes the repository exclusively, because generating a diff
/// writes through that pointer, and hands the result back bound to the
/// repository object's own lifetime rather than to the exclusive reborrow, so
/// several diffs of one repository can coexist exactly as they do in C.
///
/// Outliving the repository is therefore rejected:
///
/// ```compile_fail
/// use libgit2::diff_generate::{git_diff_tree_to_tree, RepositoryDiff};
/// use libgit2::repository::git_repository_open_bare;
///
/// fn escape(path: &core::ffi::CStr) -> RepositoryDiff<'static> {
///     let mut owner = git_repository_open_bare(path).unwrap();
///     git_diff_tree_to_tree(&mut owner.as_mut(), None, None, None).unwrap()
/// }
/// ```
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

/// Wraps: git_diff_index_to_index
/// Creates a generated diff between two index snapshots.
///
/// Both indexes are borrowed exclusively: each is handed to
/// `git_iterator_for_index`, whose `git_index_snapshot_new` sorts
/// `index->entries` in place and bumps the reader count before copying the
/// vector. That reordering is the storage
/// [`git_index_get_byindex`](crate::index::git_index_get_byindex) addresses by
/// position, which is why that lookup takes the exclusive handle too.
pub fn git_diff_index_to_index<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    old_index: &mut crate::index::GitIndexMut<'_>,
    new_index: &mut crate::index::GitIndexMut<'_>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the output is writable, the repository and both indexes are
    // exclusively borrowed for the writes generation performs through them,
    // and every other borrowed input remains live for the call. A non-null
    // output transfers one complete count, which is adopted before leaving
    // this FFI seam.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_index_to_index(
            &mut out,
            repo.as_mut_ptr(),
            old_index.as_mut_ptr(),
            new_index.as_mut_ptr(),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
}

/// Wraps: git_diff_index_to_workdir
/// Creates a diff from an index, or the repository index, to the worktree.
///
/// Without an index this diffs the repository's own index, which
/// `diff_load_index` first re-reads with `git_index_read`; if the file changed
/// on disk that clears the index, freeing every entry a previously obtained
/// index handle may still be lending out. The repository is therefore borrowed
/// exclusively.
pub fn git_diff_index_to_workdir<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    mut index: Option<&mut crate::index::GitIndexMut<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let index = index
        .as_mut()
        .map_or(core::ptr::null_mut(), |index| index.as_mut_ptr());
    // SAFETY: output and optional index are writable, the repository is
    // exclusively borrowed for the index reload and lazy subsystem writes
    // generation performs, all inputs are live for generation, and a
    // transferred output count is adopted before leaving the seam. The result
    // carries the retained repository lifetime.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_index_to_workdir(
            &mut out,
            repo.as_mut_ptr(),
            index,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
}

/// Wraps: git_diff_tree_to_index
/// Creates a diff from an optional tree to an index or repository index.
///
/// The index is borrowed exclusively because `git_iterator_for_index` snapshots
/// it through `git_index_snapshot_new`, which sorts `index->entries` in place.
/// Passing `None` diffs the repository's own index, which `diff_load_index`
/// re-reads from disk first, so the repository is exclusive as well.
pub fn git_diff_tree_to_index<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    old_tree: Option<crate::tree::GitTreeRef<'_>>,
    mut index: Option<&mut crate::index::GitIndexMut<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let old_tree = old_tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    let index = index
        .as_mut()
        .map_or(core::ptr::null_mut(), |index| index.as_mut_ptr());
    // SAFETY: the output is writable, the repository and optional index are
    // exclusively borrowed for the writes generation performs through them,
    // the remaining optional inputs are null or live, and a non-null output
    // transfers one complete count. The result retains only the repository
    // pointer whose lifetime it carries.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_tree_to_index(
            &mut out,
            repo.as_mut_ptr(),
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
///
/// The repository is borrowed exclusively: applying the diff options runs
/// `git_repository_config_snapshot`, which installs `repo->_config` through
/// `git_repository_config__weakptr` on first use.
pub fn git_diff_tree_to_tree<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    old_tree: Option<crate::tree::GitTreeRef<'_>>,
    new_tree: Option<crate::tree::GitTreeRef<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let old_tree = old_tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    let new_tree = new_tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    // SAFETY: the output is writable, the repository is exclusively borrowed
    // for the lazy config installation generation performs, every non-null
    // borrowed input remains live for generation, and a non-null output
    // transfers one complete count. The result keeps the stored repository
    // live.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_tree_to_tree(
            &mut out,
            repo.as_mut_ptr(),
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
///
/// The repository is borrowed exclusively: this form reaches its index through
/// `git_repository_index__weakptr` and its diff settings through
/// `git_repository_config_snapshot`, each of which installs the subsystem into
/// the repository on first use.
pub fn git_diff_tree_to_workdir<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    old_tree: Option<crate::tree::GitTreeRef<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let old_tree = old_tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    // SAFETY: the output is writable, the repository is exclusively borrowed
    // for the lazy index and config installation generation performs, optional
    // inputs are null or live, and a non-null output transfers one complete
    // count. The result's repository borrow covers every later use of its
    // stored pointer.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_tree_to_workdir(
            &mut out,
            repo.as_mut_ptr(),
            old_tree,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
}

/// Wraps: git_diff_tree_to_workdir_with_index
/// Creates a tree-to-worktree diff blended with repository-index state.
///
/// This form always diffs the repository's own index, which `diff_load_index`
/// re-reads with `git_index_read` before use — clearing it, and freeing every
/// entry it is lending out, whenever the file changed on disk. The repository
/// is therefore borrowed exclusively.
pub fn git_diff_tree_to_workdir_with_index<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    tree: Option<crate::tree::GitTreeRef<'_>>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
) -> Result<RepositoryDiff<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let tree = tree.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    // SAFETY: the output is writable, the repository is exclusively borrowed
    // for the index reload this constructor always performs, optional inputs
    // are null or live, and a non-null output transfers one complete count.
    // The returned wrapper retains the repository lifetime stored by the diff.
    let (status, inner) = unsafe {
        let status = crate::ffi::git_diff_tree_to_workdir_with_index(
            &mut out,
            repo.as_mut_ptr(),
            tree,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        (status, crate::diff::DiffOwned::from_raw(out))
    };
    diff_result(status, inner)
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

#[cfg(test)]
mod scheduled_constructor_tests {
    use super::*;
    use crate::diff::git_diff_num_deltas;
    use crate::ffi;
    use crate::index::git_index_new;
    use crate::repository::git_repository_open_bare;

    /// The smallest on-disk bare repository `git_repository_open_bare`
    /// accepts, so a generated diff can be produced without a working tree.
    struct BareRepo(std::path::PathBuf);

    impl BareRepo {
        fn create(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "crustify-diff-generate-{}-{tag}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("objects")).expect("a private temporary directory");
            std::fs::create_dir_all(path.join("refs/heads")).expect("a refs directory");
            std::fs::write(path.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                path.join("config"),
                b"[core]\n\trepositoryformatversion = 0\n\tbare = true\n",
            )
            .expect("a config file");
            Self(path)
        }

        fn c_path(&self) -> std::ffi::CString {
            std::ffi::CString::new(self.0.to_str().expect("a UTF-8 temporary path"))
                .expect("a path without interior NUL")
        }
    }

    impl Drop for BareRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn generated_diffs_share_one_repository_across_exclusive_calls() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after every owner created here has been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let directory = BareRepo::create("share");
        let mut owner = git_repository_open_bare(&directory.c_path())
            .expect("the hand-built directory is a bare repository");
        {
            let mut repository = owner.as_mut();
            let mut old_index = git_index_new().expect("an empty in-memory index");
            let mut new_index = git_index_new().expect("an empty in-memory index");

            // Each constructor reborrows the exclusive repository handle for
            // the call only, so the diffs it produces coexist afterwards, as
            // the C API allows.
            let indexes = git_diff_index_to_index(
                &mut repository,
                &mut old_index.as_mut(),
                &mut new_index.as_mut(),
                None,
            )
            .expect("two empty indexes still diff");
            let trees = git_diff_tree_to_tree(&mut repository, None, None, None)
                .expect("two absent trees still diff");

            assert_eq!(git_diff_num_deltas(indexes.as_ref()), 0);
            assert_eq!(git_diff_num_deltas(trees.as_ref()), 0);
            drop(trees);
            drop(indexes);
        }
        drop(owner);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
