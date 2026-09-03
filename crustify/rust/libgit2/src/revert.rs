//! Safe wrappers for libgit2 revert APIs.

use crate::api::revert::{GitRevertOptionsMut, GitRevertOptionsRef};

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

/// Wraps: git_revert
/// Reverts `commit` into the repository's index and working directory.
///
/// The commit is borrowed exclusively for the same reason
/// [`git_cherrypick`](crate::cherrypick::git_cherrypick) is: the revert
/// message is built from `git_commit_summary`, which fills the commit's lazy
/// `summary` cache on first use.
pub fn git_revert(
    repository: &mut crate::repository::GitRepositoryMut<'_>,
    commit: &mut crate::commit::GitCommitMut<'_>,
    options: Option<GitRevertOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    // SAFETY: the repository and the commit are both exclusive for the writes
    // this call performs through them; the optional options remain live and
    // read-only for the synchronous operation.
    let status = unsafe {
        crate::ffi::git_revert(
            repository.as_mut_ptr(),
            commit.as_mut_ptr(),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_revert_options_init
/// Initializes revert options for `version`.
pub fn git_revert_options_init(
    options: &mut GitRevertOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage and the initializer retains no pointer.
    let status = unsafe { crate::ffi::git_revert_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    unsafe fn lookup(
        repository: *mut ffi::git_repository,
        spec: &core::ffi::CStr,
    ) -> *mut ffi::git_object {
        let mut object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut object, repository, spec.as_ptr()) },
            0
        );
        object
    }

    unsafe fn raw_paths(repository: *mut ffi::git_repository) -> Vec<Vec<u8>> {
        let commit = unsafe { lookup(repository, c"HEAD") };
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_revert_commit(
                    &mut index,
                    repository,
                    commit.cast(),
                    commit.cast(),
                    0,
                    core::ptr::null(),
                )
            },
            0
        );
        let paths = (0..unsafe { ffi::git_index_entrycount(index) })
            .map(|position| {
                let entry = unsafe { ffi::git_index_get_byindex(index, position) };
                unsafe { core::ffi::CStr::from_ptr((*entry).path) }
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        unsafe {
            ffi::git_index_free(index);
            ffi::git_object_free(commit);
        }
        paths
    }

    fn safe_paths(repository: *mut ffi::git_repository) -> Vec<Vec<u8>> {
        let commit_raw = unsafe { lookup(repository, c"HEAD") };
        let commit = unsafe { GitCommitRef::from_ptr(commit_raw.cast()) }.unwrap();
        let mut view = unsafe { GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut index = git_revert_commit(&mut view, commit, commit, 0, None).unwrap();
        let paths = (0..crate::index::git_index_entrycount(index.as_ref()))
            .map(|position| {
                crate::index::git_index_get_byindex(&mut index.as_mut(), position)
                    .unwrap()
                    .path()
                    .unwrap()
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        drop(index);
        unsafe { ffi::git_object_free(commit_raw) };
        paths
    }

    #[test]
    fn io_equiv_revert_commit_to_index() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("revert-raw");
        let safe = HistoryFixture::new("revert-safe");
        let raw_paths = unsafe { raw_paths(raw.repository.as_ptr()) };
        assert_eq!(raw_paths, safe_paths(safe.repository.as_ptr()));
        assert_eq!(
            raw_paths,
            [
                b"README.md".to_vec(),
                b"src/alpha.c".to_vec(),
                b"src/beta.c".to_vec()
            ]
        );
    }
}

#[cfg(test)]
mod scheduled_wrapper_tests {
    use super::*;
    use crate::api::revert::GitRevertOptions;

    /// The shape of the working-directory revert.
    type WorkdirRevert = fn(
        &mut GitRepositoryMut<'_>,
        &mut crate::commit::GitCommitMut<'_>,
        Option<GitRevertOptionsRef<'_, '_>>,
    ) -> Result<(), i32>;

    #[test]
    fn the_working_directory_revert_borrows_the_commit_exclusively() {
        // `git_revert` builds its message from `git_commit_summary`, which
        // populates `commit->summary` on first use, so the commit cannot be
        // handed over as a shared handle.
        let _: WorkdirRevert = git_revert;
    }

    #[test]
    fn initializer_writes_the_current_version() {
        let mut options = GitRevertOptions::new();
        git_revert_options_init(&mut options.as_mut(), ffi::GIT_REVERT_OPTIONS_VERSION).unwrap();
        assert_eq!(options.as_ref().version(), ffi::GIT_REVERT_OPTIONS_VERSION);
    }
}
