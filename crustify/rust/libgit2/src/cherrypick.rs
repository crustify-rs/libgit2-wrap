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

/// Wraps: git_cherrypick_options_init
/// Initializes cherry-pick options for `version`.
pub fn git_cherrypick_options_init(
    options: &mut GitCherrypickOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage, and the initializer retains no pointer into it.
    let status = unsafe { crate::ffi::git_cherrypick_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct FullCherrypickObservation {
        state: i32,
        readme: Vec<u8>,
        alpha: Vec<u8>,
        beta: Vec<u8>,
        index_paths: Vec<Vec<u8>>,
    }

    unsafe fn lookup(
        repository: *mut crate::ffi::git_repository,
        spec: &core::ffi::CStr,
    ) -> *mut crate::ffi::git_object {
        let mut object = core::ptr::null_mut();
        assert_eq!(
            unsafe { crate::ffi::git_revparse_single(&mut object, repository, spec.as_ptr()) },
            0
        );
        object
    }

    unsafe fn raw_paths(repository: *mut crate::ffi::git_repository) -> Vec<Vec<u8>> {
        let pick = unsafe { lookup(repository, c"HEAD~1") };
        let ours = unsafe { lookup(repository, c"HEAD~2") };
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                crate::ffi::git_cherrypick_commit(
                    &mut index,
                    repository,
                    pick.cast(),
                    ours.cast(),
                    0,
                    core::ptr::null(),
                )
            },
            0
        );
        let paths = (0..unsafe { crate::ffi::git_index_entrycount(index) })
            .map(|position| {
                let entry = unsafe { crate::ffi::git_index_get_byindex(index, position) };
                unsafe { core::ffi::CStr::from_ptr((*entry).path) }
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        unsafe {
            crate::ffi::git_index_free(index);
            crate::ffi::git_object_free(ours);
            crate::ffi::git_object_free(pick);
        }
        paths
    }

    fn safe_paths(repository: *mut crate::ffi::git_repository) -> Vec<Vec<u8>> {
        let pick_raw = unsafe { lookup(repository, c"HEAD~1") };
        let ours_raw = unsafe { lookup(repository, c"HEAD~2") };
        let pick = unsafe { crate::commit::GitCommitRef::from_ptr(pick_raw.cast()) }.unwrap();
        let ours = unsafe { crate::commit::GitCommitRef::from_ptr(ours_raw.cast()) }.unwrap();
        let mut view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut index = git_cherrypick_commit(&mut view, pick, ours, 0, None).unwrap();
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
        unsafe {
            crate::ffi::git_object_free(ours_raw);
            crate::ffi::git_object_free(pick_raw);
        }
        paths
    }

    unsafe fn raw_full_cherrypick(fixture: &HistoryFixture) -> FullCherrypickObservation {
        let repository = fixture.repository.as_ptr();
        let pick = unsafe { lookup(repository, c"HEAD~1") };
        let first = unsafe { lookup(repository, c"HEAD~2") };
        assert_eq!(
            unsafe {
                crate::ffi::git_reset(
                    repository,
                    first,
                    crate::ffi::git_reset_t_GIT_RESET_HARD,
                    core::ptr::null(),
                )
            },
            0
        );
        assert_eq!(
            unsafe { crate::ffi::git_cherrypick(repository, pick.cast(), core::ptr::null()) },
            0
        );
        let observation = unsafe { full_observation(fixture) };
        unsafe {
            crate::ffi::git_object_free(first);
            crate::ffi::git_object_free(pick);
        }
        observation
    }

    fn safe_full_cherrypick(fixture: &HistoryFixture) -> FullCherrypickObservation {
        let repository = fixture.repository.as_ptr();
        let pick = unsafe { lookup(repository, c"HEAD~1") };
        let first = unsafe { lookup(repository, c"HEAD~2") };
        {
            let mut repository_view =
                unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
            let first_view = unsafe { crate::object::GitObjectRef::from_ptr(first) }.unwrap();
            crate::reset::git_reset(
                &mut repository_view,
                first_view,
                crate::reset::ResetType::Hard,
                None,
            )
            .unwrap();
            let mut pick_view =
                unsafe { crate::commit::GitCommitMut::from_ptr(pick.cast()) }.unwrap();
            git_cherrypick(&mut repository_view, &mut pick_view, None).unwrap();
        }
        let observation = unsafe { full_observation(fixture) };
        unsafe {
            crate::ffi::git_object_free(first);
            crate::ffi::git_object_free(pick);
        }
        observation
    }

    unsafe fn full_observation(fixture: &HistoryFixture) -> FullCherrypickObservation {
        let repository = fixture.repository.as_ptr();
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { crate::ffi::git_repository_index(&mut index, repository) },
            0
        );
        let index_paths = (0..unsafe { crate::ffi::git_index_entrycount(index) })
            .map(|position| {
                let entry = unsafe { crate::ffi::git_index_get_byindex(index, position) };
                unsafe { core::ffi::CStr::from_ptr((*entry).path) }
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        unsafe { crate::ffi::git_index_free(index) };
        FullCherrypickObservation {
            state: unsafe { crate::ffi::git_repository_state(repository) },
            readme: std::fs::read(fixture.directory.path().join("README.md")).unwrap(),
            alpha: std::fs::read(fixture.directory.path().join("src/alpha.c")).unwrap(),
            beta: std::fs::read(fixture.directory.path().join("src/beta.c")).unwrap(),
            index_paths,
        }
    }

    #[test]
    fn io_equiv_cherrypick_commit_to_index() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("cherrypick-raw");
        let safe = HistoryFixture::new("cherrypick-safe");
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

    #[test]
    fn io_equiv_full_cherrypick_updates_index_workdir_and_state() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("full-cherrypick-raw");
        let safe = HistoryFixture::new("full-cherrypick-safe");
        let raw_observation = unsafe { raw_full_cherrypick(&raw) };
        assert_eq!(raw_observation, safe_full_cherrypick(&safe));
        assert_eq!(
            raw_observation.state,
            crate::ffi::git_repository_state_t_GIT_REPOSITORY_STATE_CHERRYPICK as i32
        );
        assert_eq!(raw_observation.alpha, b"int alpha(void) { return 2; }\n");
    }
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
    fn the_published_initializer_restores_every_constructed_default() {
        // `GIT_INIT_STRUCTURE_FROM_TEMPLATE` copies the whole template over
        // the caller's storage rather than only stamping the version, so
        // reinitializing deliberately dirtied options both proves that and
        // pins `GitCherrypickOptions::new` to `GIT_CHERRYPICK_OPTIONS_INIT`.
        let mut options = GitCherrypickOptions::new();
        {
            let mut view = options.as_mut();
            view.set_version(0);
            view.set_mainline(2);
            view.merge_options_mut().set_version(0);
            view.merge_options_mut()
                .set_flags(crate::api::merge::GitMergeFlags::NO_RECURSIVE);
            view.checkout_options_mut().set_version(0);
        }

        git_cherrypick_options_init(
            &mut options.as_mut(),
            crate::ffi::GIT_CHERRYPICK_OPTIONS_VERSION,
        )
        .unwrap();

        let view = options.as_ref();
        assert_eq!(view.version(), crate::ffi::GIT_CHERRYPICK_OPTIONS_VERSION);
        assert_eq!(view.mainline(), 0);
        assert_eq!(
            view.merge_options().version(),
            crate::ffi::GIT_MERGE_OPTIONS_VERSION
        );
        assert_eq!(
            view.merge_options().flags(),
            Ok(crate::api::merge::GitMergeFlags::FIND_RENAMES)
        );
        assert_eq!(
            view.checkout_options().version(),
            crate::ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
    }

    #[test]
    fn an_unsupported_version_is_rejected_without_writing() {
        // SAFETY: process-global initialization is reference counted and is
        // balanced below; the rejected version reaches `git_error_set`, which
        // allocates through the allocator only initialization installs.
        assert!(unsafe { crate::ffi::git_libgit2_init() } > 0);
        let mut options = GitCherrypickOptions::new();
        options.as_mut().set_mainline(2);
        assert!(git_cherrypick_options_init(&mut options.as_mut(), 0).is_err());
        assert_eq!(options.as_ref().mainline(), 2);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { crate::ffi::git_libgit2_shutdown() } >= 0);
    }
}
