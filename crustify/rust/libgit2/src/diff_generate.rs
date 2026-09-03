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
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::ffi;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, RawBuf, safe_buf_bytes};

    fn mutate_workdir(fixture: &HistoryFixture) {
        std::fs::write(
            fixture.directory.path().join("src/alpha.c"),
            b"int alpha(void) { return 42; }\n",
        )
        .unwrap();
        std::fs::remove_file(fixture.directory.path().join("src/beta.c")).unwrap();
        std::fs::write(
            fixture.directory.path().join("untracked.txt"),
            b"untracked\n",
        )
        .unwrap();
    }

    unsafe fn raw_patch(repository: *mut crate::ffi::git_repository) -> Vec<u8> {
        let mut tree_object = core::ptr::null_mut();
        // SAFETY: repository, spec and output are live.
        assert_eq!(
            unsafe {
                crate::ffi::git_revparse_single(
                    &mut tree_object,
                    repository,
                    c"HEAD^{tree}".as_ptr(),
                )
            },
            0
        );
        let mut diff = core::ptr::null_mut();
        // SAFETY: all inputs and the output slot are live.
        assert_eq!(
            unsafe {
                crate::ffi::git_diff_tree_to_workdir_with_index(
                    &mut diff,
                    repository,
                    tree_object.cast(),
                    core::ptr::null(),
                )
            },
            0
        );
        let mut buffer = RawBuf::new();
        // SAFETY: diff and output buffer are live and exclusive.
        assert_eq!(
            unsafe {
                crate::ffi::git_diff_to_buf(
                    &mut buffer.0,
                    diff,
                    crate::ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH,
                )
            },
            0
        );
        let bytes = buffer.bytes();
        // SAFETY: successful constructors transferred these owners.
        unsafe {
            crate::ffi::git_diff_free(diff);
            crate::ffi::git_object_free(tree_object);
        }
        bytes
    }

    unsafe fn raw_statuses(repository: *mut crate::ffi::git_repository) -> Vec<(Vec<u8>, u32)> {
        unsafe extern "C" fn callback(
            path: *const core::ffi::c_char,
            status: u32,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            // SAFETY: the caller supplies the exact vector payload and C
            // supplies a transient NUL-terminated path.
            let output = unsafe { &mut *payload.cast::<Vec<(Vec<u8>, u32)>>() };
            output.push((
                unsafe { core::ffi::CStr::from_ptr(path) }
                    .to_bytes()
                    .to_vec(),
                status,
            ));
            0
        }
        let mut output = Vec::new();
        // SAFETY: repository and payload remain live for the traversal.
        assert_eq!(
            unsafe {
                crate::ffi::git_status_foreach(
                    repository,
                    Some(callback),
                    core::ptr::from_mut(&mut output).cast(),
                )
            },
            0
        );
        output.sort();
        output
    }

    fn safe_patch(repository: &mut crate::repository::GitRepositoryMut<'_>) -> Vec<u8> {
        let mut tree_object = core::ptr::null_mut();
        // Use the raw lookup only to obtain a borrowed tree input. The value
        // under comparison is generated and formatted through safe wrappers.
        assert_eq!(
            unsafe {
                crate::ffi::git_revparse_single(
                    &mut tree_object,
                    repository.as_mut_ptr(),
                    c"HEAD^{tree}".as_ptr(),
                )
            },
            0
        );
        // SAFETY: the successful object lookup returned a live tree object.
        let tree = unsafe { crate::tree::GitTreeRef::from_ptr(tree_object.cast()) }.unwrap();
        let mut diff = git_diff_tree_to_workdir_with_index(repository, Some(tree), None).unwrap();
        let output =
            crate::diff_print::git_diff_to_buf(&mut diff.as_mut(), crate::diff::DiffFormat::Patch)
                .unwrap();
        let bytes = safe_buf_bytes(output.as_ref());
        drop(output);
        drop(diff);
        // SAFETY: the raw lookup transferred this sole object owner.
        unsafe { crate::ffi::git_object_free(tree_object) };
        bytes
    }

    fn safe_statuses(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
    ) -> Vec<(Vec<u8>, u32)> {
        let mut output = Vec::new();
        crate::status::git_status_foreach(
            repository,
            &mut |path: &core::ffi::CStr, status: crate::status::Status| {
                output.push((path.to_bytes().to_vec(), status.bits()));
                0
            },
        )
        .unwrap();
        output.sort();
        output
    }

    #[test]
    fn io_equiv_workdir_diff_and_status() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("diff-raw");
        let safe = HistoryFixture::new("diff-safe");
        mutate_workdir(&raw);
        mutate_workdir(&safe);

        // SAFETY: fixture is live and not concurrently accessed.
        let raw_patch = unsafe { raw_patch(raw.repository.as_ptr()) };
        // SAFETY: fixture is live and not concurrently accessed.
        let raw_statuses = unsafe { raw_statuses(raw.repository.as_ptr()) };
        // SAFETY: this is the only handle used for the safe fixture here.
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(raw_patch, safe_patch(&mut safe_repository));
        assert_eq!(raw_statuses, safe_statuses(&mut safe_repository));
        assert!(
            raw_patch
                .windows(b"alpha".len())
                .any(|part| part == b"alpha")
        );
        assert!(raw_patch.windows(b"beta".len()).any(|part| part == b"beta"));
    }

    fn prepare_matrix(fixture: &HistoryFixture) {
        std::fs::write(
            fixture.directory.path().join("src/alpha.c"),
            b"int alpha(void) { return 314; }\n",
        )
        .unwrap();
        std::fs::write(
            fixture.directory.path().join("staged.txt"),
            b"staged matrix\n",
        )
        .unwrap();
        let status = std::process::Command::new("git")
            .current_dir(fixture.directory.path())
            .args(["add", "staged.txt"])
            .status()
            .unwrap();
        assert!(status.success());
        std::fs::create_dir_all(fixture.directory.path().join("untracked/deep")).unwrap();
        std::fs::write(
            fixture.directory.path().join("untracked/deep/value.txt"),
            b"untracked matrix\n",
        )
        .unwrap();
    }

    unsafe fn raw_diff_bytes(diff: *mut ffi::git_diff) -> Vec<u8> {
        let mut output = RawBuf::new();
        assert_eq!(
            unsafe {
                ffi::git_diff_to_buf(
                    &mut output.0,
                    diff,
                    ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH,
                )
            },
            0
        );
        let bytes = output.bytes();
        unsafe { ffi::git_diff_free(diff) };
        bytes
    }

    unsafe fn raw_matrix(repository: *mut ffi::git_repository) -> Vec<Vec<u8>> {
        let mut old_object = core::ptr::null_mut();
        let mut new_object = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_revparse_single(&mut old_object, repository, c"HEAD~1^{tree}".as_ptr())
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_revparse_single(&mut new_object, repository, c"HEAD^{tree}".as_ptr())
            },
            0
        );
        let old_tree = old_object.cast::<ffi::git_tree>();
        let new_tree = new_object.cast::<ffi::git_tree>();
        let index_path = std::ffi::CString::new(
            std::path::Path::new(
                unsafe { core::ffi::CStr::from_ptr(ffi::git_repository_path(repository)) }
                    .to_str()
                    .unwrap(),
            )
            .join("index")
            .to_str()
            .unwrap(),
        )
        .unwrap();
        let mut old_index = core::ptr::null_mut();
        let mut new_index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_index_open(&mut old_index, index_path.as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_index_open(&mut new_index, index_path.as_ptr()) },
            0
        );
        let mut output = Vec::new();
        let mut diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_tree_to_tree(
                    &mut diff,
                    repository,
                    old_tree,
                    new_tree,
                    core::ptr::null(),
                )
            },
            0
        );
        output.push(unsafe { raw_diff_bytes(diff) });
        diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_tree_to_index(
                    &mut diff,
                    repository,
                    old_tree,
                    old_index,
                    core::ptr::null(),
                )
            },
            0
        );
        output.push(unsafe { raw_diff_bytes(diff) });
        diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_index_to_workdir(&mut diff, repository, old_index, core::ptr::null())
            },
            0
        );
        output.push(unsafe { raw_diff_bytes(diff) });
        diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_tree_to_workdir(&mut diff, repository, old_tree, core::ptr::null())
            },
            0
        );
        output.push(unsafe { raw_diff_bytes(diff) });
        diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_tree_to_workdir_with_index(
                    &mut diff,
                    repository,
                    old_tree,
                    core::ptr::null(),
                )
            },
            0
        );
        output.push(unsafe { raw_diff_bytes(diff) });
        diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_index_to_index(
                    &mut diff,
                    repository,
                    old_index,
                    new_index,
                    core::ptr::null(),
                )
            },
            0
        );
        output.push(unsafe { raw_diff_bytes(diff) });
        unsafe {
            ffi::git_index_free(new_index);
            ffi::git_index_free(old_index);
            ffi::git_object_free(new_object);
            ffi::git_object_free(old_object);
        }
        output
    }

    fn safe_diff_bytes(mut diff: RepositoryDiff<'_>) -> Vec<u8> {
        let output =
            crate::diff_print::git_diff_to_buf(&mut diff.as_mut(), crate::diff::DiffFormat::Patch)
                .unwrap();
        safe_buf_bytes(output.as_ref())
    }

    fn safe_matrix(repository: *mut ffi::git_repository) -> Vec<Vec<u8>> {
        let mut old_object = core::ptr::null_mut();
        let mut new_object = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_revparse_single(&mut old_object, repository, c"HEAD~1^{tree}".as_ptr())
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_revparse_single(&mut new_object, repository, c"HEAD^{tree}".as_ptr())
            },
            0
        );
        let old_tree = unsafe { crate::tree::GitTreeRef::from_ptr(old_object.cast()) }.unwrap();
        let new_tree = unsafe { crate::tree::GitTreeRef::from_ptr(new_object.cast()) }.unwrap();
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let index_path = std::ffi::CString::new(
            std::path::Path::new(
                crate::repository::git_repository_path(repository_view.as_ref())
                    .unwrap()
                    .to_str()
                    .unwrap(),
            )
            .join("index")
            .to_str()
            .unwrap(),
        )
        .unwrap();
        let mut old_index = crate::index::git_index_open(&index_path).unwrap();
        let mut new_index = crate::index::git_index_open(&index_path).unwrap();
        let output = vec![
            safe_diff_bytes(
                git_diff_tree_to_tree(&mut repository_view, Some(old_tree), Some(new_tree), None)
                    .unwrap(),
            ),
            safe_diff_bytes(
                git_diff_tree_to_index(
                    &mut repository_view,
                    Some(old_tree),
                    Some(&mut old_index.as_mut()),
                    None,
                )
                .unwrap(),
            ),
            safe_diff_bytes(
                git_diff_index_to_workdir(
                    &mut repository_view,
                    Some(&mut old_index.as_mut()),
                    None,
                )
                .unwrap(),
            ),
            safe_diff_bytes(
                git_diff_tree_to_workdir(&mut repository_view, Some(old_tree), None).unwrap(),
            ),
            safe_diff_bytes(
                git_diff_tree_to_workdir_with_index(&mut repository_view, Some(old_tree), None)
                    .unwrap(),
            ),
            safe_diff_bytes(
                git_diff_index_to_index(
                    &mut repository_view,
                    &mut old_index.as_mut(),
                    &mut new_index.as_mut(),
                    None,
                )
                .unwrap(),
            ),
        ];
        unsafe {
            ffi::git_object_free(new_object);
            ffi::git_object_free(old_object);
        }
        output
    }

    #[test]
    fn io_equiv_diff_matrix_across_trees_index_and_workdir() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("diff-matrix-raw");
        let safe = HistoryFixture::new("diff-matrix-safe");
        prepare_matrix(&raw);
        prepare_matrix(&safe);
        let raw_output = unsafe { raw_matrix(raw.repository.as_ptr()) };
        assert_eq!(raw_output, safe_matrix(safe.repository.as_ptr()));
        assert!(raw_output.iter().filter(|patch| !patch.is_empty()).count() >= 4);
    }

    fn prepare_diff_drivers(fixture: &HistoryFixture) {
        let path = fixture.directory.path();
        std::fs::write(
            path.join(".gitattributes"),
            b"*.bin binary\n*.word diff=word\n",
        )
        .unwrap();
        std::fs::write(path.join("payload.bin"), [0, 1, 2, 0, 4, 5]).unwrap();
        std::fs::write(
            path.join("sections.word"),
            b"section alpha\nfirst original sentence\nsection beta\nsecond sentence\n",
        )
        .unwrap();
        let run = |arguments: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(path)
                .args(arguments)
                .env("GIT_AUTHOR_DATE", "2023-11-14T22:32:00Z")
                .env("GIT_COMMITTER_DATE", "2023-11-14T22:32:00Z")
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {arguments:?}");
        };
        run(&["config", "diff.word.xfuncname", "^section .*"]);
        run(&["config", "diff.word.wordRegex", "[A-Za-z]+"]);
        run(&["add", ".gitattributes", "payload.bin", "sections.word"]);
        run(&[
            "-c",
            "user.name=Crustify",
            "-c",
            "user.email=crustify@example.com",
            "commit",
            "-q",
            "-m",
            "diff driver baseline",
        ]);
        std::fs::write(path.join("payload.bin"), [0, 1, 9, 0, 8, 5]).unwrap();
        std::fs::write(
            path.join("sections.word"),
            b"section alpha\nfirst changed phrase\nsection beta\nsecond sentence\n",
        )
        .unwrap();
    }

    unsafe fn raw_driver_diff(repository: *mut ffi::git_repository) -> Vec<u8> {
        let mut diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_index_to_workdir(
                    &mut diff,
                    repository,
                    core::ptr::null_mut(),
                    core::ptr::null(),
                )
            },
            0
        );
        unsafe { raw_diff_bytes(diff) }
    }

    fn safe_driver_diff(repository: *mut ffi::git_repository) -> Vec<u8> {
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let diff = git_diff_index_to_workdir(&mut repository, None, None).unwrap();
        safe_diff_bytes(diff)
    }

    #[test]
    fn io_equiv_generated_diff_binary_and_custom_word_driver() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("diff-driver-raw");
        let safe = HistoryFixture::new("diff-driver-safe");
        prepare_diff_drivers(&raw);
        prepare_diff_drivers(&safe);
        let raw_output = unsafe { raw_driver_diff(raw.repository.as_ptr()) };
        let safe_output = safe_driver_diff(safe.repository.as_ptr());
        assert_eq!(raw_output, safe_output);
        assert!(
            raw_output
                .windows(b"Binary files".len())
                .any(|w| w == b"Binary files")
        );
        assert!(
            raw_output
                .windows(b"section alpha".len())
                .any(|w| w == b"section alpha")
        );
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
