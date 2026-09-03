//! Safe wrappers for libgit2 stash APIs.

use core::ffi::{CStr, c_void};

use crate::api::stash::{
    GitStashApplyOptionsMut, GitStashApplyOptionsRef, GitStashCallback, GitStashFlags,
    GitStashSaveOptionsMut, GitStashSaveOptionsRef,
};
use crate::api::types::GitSignatureRef;
use crate::ffi;
use crate::oid::{Oid, OidRef};
use crate::repository::GitRepositoryMut;

/// Wraps: git_stash_apply_progress_t
/// A stage reported while libgit2 applies a stash.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum StashApplyProgress {
    /// No application work has started yet.
    None = ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_NONE,
    /// The stashed objects are being loaded from the object database.
    LoadingStash = ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_LOADING_STASH,
    /// The stored index is being analyzed.
    AnalyzeIndex = ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_ANALYZE_INDEX,
    /// Modified files are being analyzed.
    AnalyzeModified = ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_ANALYZE_MODIFIED,
    /// Untracked and ignored files are being analyzed.
    AnalyzeUntracked = ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_ANALYZE_UNTRACKED,
    /// Untracked files are being written to the worktree.
    CheckoutUntracked = ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_CHECKOUT_UNTRACKED,
    /// Modified files are being written to the worktree.
    CheckoutModified = ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_CHECKOUT_MODIFIED,
    /// The stash was applied successfully.
    Done = ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_DONE,
}

/// A C value that is not a published [`StashApplyProgress`] stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidStashApplyProgress(ffi::git_stash_apply_progress_t);

impl InvalidStashApplyProgress {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_stash_apply_progress_t {
        self.0
    }
}

impl From<StashApplyProgress> for ffi::git_stash_apply_progress_t {
    fn from(progress: StashApplyProgress) -> Self {
        progress as Self
    }
}

impl TryFrom<ffi::git_stash_apply_progress_t> for StashApplyProgress {
    type Error = InvalidStashApplyProgress;

    fn try_from(progress: ffi::git_stash_apply_progress_t) -> Result<Self, Self::Error> {
        match progress {
            ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_NONE => Ok(Self::None),
            ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_LOADING_STASH => {
                Ok(Self::LoadingStash)
            }
            ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_ANALYZE_INDEX => {
                Ok(Self::AnalyzeIndex)
            }
            ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_ANALYZE_MODIFIED => {
                Ok(Self::AnalyzeModified)
            }
            ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_ANALYZE_UNTRACKED => {
                Ok(Self::AnalyzeUntracked)
            }
            ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_CHECKOUT_UNTRACKED => {
                Ok(Self::CheckoutUntracked)
            }
            ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_CHECKOUT_MODIFIED => {
                Ok(Self::CheckoutModified)
            }
            ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_DONE => Ok(Self::Done),
            value => Err(InvalidStashApplyProgress(value)),
        }
    }
}

/// Wraps: git_stash_drop
/// Removes the stash at `index` from the repository.
pub fn git_stash_drop(repository: &mut GitRepositoryMut<'_>, index: usize) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusively borrowed while libgit2
    // updates its stash reference and reflog; no pointer is retained.
    let status = unsafe { ffi::git_stash_drop(repository.as_mut_ptr(), index) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn progress_stages_round_trip_through_the_c_type() {
        for progress in [
            StashApplyProgress::None,
            StashApplyProgress::LoadingStash,
            StashApplyProgress::AnalyzeIndex,
            StashApplyProgress::AnalyzeModified,
            StashApplyProgress::AnalyzeUntracked,
            StashApplyProgress::CheckoutUntracked,
            StashApplyProgress::CheckoutModified,
            StashApplyProgress::Done,
        ] {
            let raw = ffi::git_stash_apply_progress_t::from(progress);
            assert_eq!(StashApplyProgress::try_from(raw), Ok(progress));
        }
    }

    #[test]
    fn unknown_progress_stage_is_rejected() {
        let invalid = ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_DONE + 1;
        let error = StashApplyProgress::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn progress_stage_matches_the_c_abi_scalar() {
        assert_eq!(
            size_of::<StashApplyProgress>(),
            size_of::<ffi::git_stash_apply_progress_t>()
        );
        assert_eq!(
            align_of::<StashApplyProgress>(),
            align_of::<ffi::git_stash_apply_progress_t>()
        );
    }

    /// A refcounted hold on the process-global libgit2 initialization.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and refcounted;
            // `Drop` below balances this successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard; every repository opened under it is dropped first.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A hand-built bare repository whose stash reflog this test controls.
    struct StashRepo(std::path::PathBuf);

    impl StashRepo {
        /// Writes a bare repository holding a `refs/stash` reflog with one
        /// entry that carries a message and one older entry that does not.
        fn create(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-stash-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("objects")).expect("a private temporary directory");
            std::fs::create_dir_all(path.join("refs")).expect("a refs directory");
            std::fs::create_dir_all(path.join("logs/refs")).expect("a reflog directory");
            std::fs::write(path.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                path.join("config"),
                b"[core]\n\trepositoryformatversion = 0\n\tbare = true\n",
            )
            .expect("a config file");

            let zero = "0".repeat(40);
            let one = "1".repeat(40);
            let two = "2".repeat(40);
            std::fs::write(path.join("refs/stash"), format!("{two}\n")).expect("a stash ref");
            // The reflog file is oldest-first, and `git_stash_foreach` reports
            // entries newest-first. The first line deliberately omits the tab
            // that introduces a message, which leaves `git_reflog_entry::msg`
            // null and drives the callback's `None` case.
            let signature = "A U Thor <author@example.com> 1500000000 +0000";
            std::fs::write(
                path.join("logs/refs/stash"),
                format!(
                    "{zero} {one} {signature}\n\
                     {one} {two} {signature}\tWIP on main: 1111111 message\n"
                ),
            )
            .expect("a stash reflog");
            Self(path)
        }

        fn c_path(&self) -> std::ffi::CString {
            std::ffi::CString::new(self.0.to_str().expect("a UTF-8 temporary path"))
                .expect("a path without interior NUL")
        }
    }

    impl Drop for StashRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn stash_iteration_reports_an_entry_whose_reflog_line_has_no_message() {
        let _init = Libgit2Init::acquire();
        let repo_dir = StashRepo::create("no-message");
        let mut repository = crate::repository::git_repository_open(&repo_dir.c_path())
            .expect("the hand-built bare repository opens");

        let mut seen: Vec<(usize, Option<std::ffi::CString>)> = Vec::new();
        let mut collect = |index: usize, message: Option<&CStr>, _: OidRef<'_>| {
            seen.push((index, message.map(CStr::to_owned)));
            0
        };
        assert_eq!(
            git_stash_foreach(&mut repository.as_mut(), &mut collect),
            Ok(())
        );

        assert_eq!(seen.len(), 2, "both reflog entries are visited");
        assert_eq!(seen[0].0, 0);
        assert_eq!(
            seen[0].1.as_deref(),
            Some(c"WIP on main: 1111111 message"),
            "the newest entry carries its reflog message"
        );
        assert_eq!(seen[1].0, 1);
        assert_eq!(
            seen[1].1, None,
            "a reflog line without a message part reaches the callback as `None`"
        );
    }

    #[test]
    fn stash_save_takes_a_checked_flag_set() {
        let _init = Libgit2Init::acquire();
        let repo_dir = StashRepo::create("save-flags");
        let mut repository = crate::repository::git_repository_open(&repo_dir.c_path())
            .expect("the hand-built bare repository opens");
        let stasher = crate::signature::git_signature_now(c"A U Thor", c"author@example.com")
            .expect("a signature stamped from the current time");

        // The fixture is bare, so C rejects the save before it consults the
        // flags. What this exercises is that a checked flag set reaches the
        // `uint32_t` parameter, replacing a raw bit set at the safe boundary.
        let saved = git_stash_save(
            &mut repository.as_mut(),
            stasher.as_ref(),
            Some(c"WIP"),
            GitStashFlags::KEEP_INDEX | GitStashFlags::INCLUDE_UNTRACKED,
        );
        assert_eq!(saved.err(), Some(ffi::git_error_code_GIT_EBAREREPO));
        assert_eq!(
            crate::util::errors::git_error_last().message.as_deref(),
            Some(c"cannot stash save. This operation is not allowed against bare repositories.")
        );

        let save: fn(
            &mut GitRepositoryMut<'static>,
            GitSignatureRef<'static>,
            Option<&CStr>,
            GitStashFlags,
        ) -> Result<Oid, i32> = git_stash_save;
        let _ = save;
    }
}

/// Wraps: git_stash_foreach
/// Visits each transient stash entry, newest first.
pub fn git_stash_foreach<C>(
    repository: &mut GitRepositoryMut<'_>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitStashCallback,
{
    unsafe extern "C" fn trampoline<C: GitStashCallback>(
        index: usize,
        message: *const core::ffi::c_char,
        oid: *const ffi::git_oid,
        payload: *mut c_void,
    ) -> i32 {
        if oid.is_null() || payload.is_null() {
            return -1;
        }
        // SAFETY: the wrapper supplies this live exclusive callback payload
        // throughout the synchronous traversal.
        let callback = unsafe { &mut *payload.cast::<C>() };
        // A stash reflog entry whose line carries no message part leaves
        // `git_reflog_entry::msg` null, so libgit2 passes a null message here.
        let message = if message.is_null() {
            None
        } else {
            // SAFETY: a non-null message is the reflog-owned NUL-terminated
            // string, live for the duration of this callback invocation.
            Some(unsafe { CStr::from_ptr(message) })
        };
        // SAFETY: the non-null OID remains live for this invocation.
        let oid = unsafe { OidRef::from_ptr(oid.cast_mut()) }.expect("checked non-null");
        callback.call(index, message, oid)
    }

    // SAFETY: the exclusive repository and callback remain live until this
    // synchronous traversal returns; no callback pointer is retained.
    let status = unsafe {
        ffi::git_stash_foreach(
            repository.as_mut_ptr(),
            Some(trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_stash_save
/// Saves the worktree and returns the created stash commit ID.
pub fn git_stash_save(
    repository: &mut GitRepositoryMut<'_>,
    stasher: GitSignatureRef<'_>,
    message: Option<&CStr>,
    flags: GitStashFlags,
) -> Result<Oid, i32> {
    let mut output = Oid::zeroed();
    // SAFETY: the OID is writable, repository is exclusive, and libgit2 only
    // borrows the signature and optional message for this synchronous save.
    let status = unsafe {
        ffi::git_stash_save(
            core::ptr::addr_of_mut!(output).cast(),
            repository.as_mut_ptr(),
            stasher.as_ptr(),
            message.map_or(core::ptr::null(), CStr::as_ptr),
            flags.bits(),
        )
    };
    if status == 0 { Ok(output) } else { Err(status) }
}

/// Wraps: git_stash_save_options_init
/// Initializes stash-save options for the requested ABI version.
pub fn git_stash_save_options_init(
    options: &mut GitStashSaveOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle exposes writable layout-compatible options
    // storage, and initialization retains no pointer.
    let status = unsafe { ffi::git_stash_save_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_stash_save_with_opts
/// Saves the selected worktree changes and returns the stash commit ID.
pub fn git_stash_save_with_opts(
    repository: &mut GitRepositoryMut<'_>,
    options: GitStashSaveOptionsRef<'_, '_>,
) -> Result<Oid, i32> {
    let mut output = Oid::zeroed();
    // SAFETY: the output is writable, the repository is exclusively borrowed,
    // and the options type retains every nested borrow for this call.
    let status = unsafe {
        ffi::git_stash_save_with_opts(
            core::ptr::addr_of_mut!(output).cast(),
            repository.as_mut_ptr(),
            options.as_ptr(),
        )
    };
    if status == 0 { Ok(output) } else { Err(status) }
}

#[cfg(test)]
mod save_options_init_tests {
    use super::*;
    use crate::api::stash::{GitStashFlags, GitStashSaveOptions};

    #[test]
    fn initializer_writes_the_published_defaults() {
        let mut storage = GitStashSaveOptions::<'static>::new();
        let mut options = storage.as_mut();
        git_stash_save_options_init(&mut options, ffi::GIT_STASH_SAVE_OPTIONS_VERSION).unwrap();
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_STASH_SAVE_OPTIONS_VERSION
        );
        assert_eq!(options.as_ref().flags(), Ok(GitStashFlags::NONE));
        assert!(options.as_ref().stasher().is_none());
    }
}

/// Wraps: git_stash_apply
/// Applies the stash at `index` to the repository.
pub fn git_stash_apply(
    repository: &mut GitRepositoryMut<'_>,
    index: usize,
    options: Option<GitStashApplyOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    // SAFETY: the repository is exclusively borrowed and the optional options
    // and callback payload remain live for this synchronous operation.
    let status = unsafe {
        ffi::git_stash_apply(
            repository.as_mut_ptr(),
            index,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_stash_apply_init_options
/// Initializes deprecated stash-apply options for `version`.
pub fn git_stash_apply_init_options(
    options: &mut GitStashApplyOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage and initialization retains no pointer into it.
    let status = unsafe { ffi::git_stash_apply_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_stash_apply_options_init
/// Initializes stash-application options for `version`.
pub fn git_stash_apply_options_init(
    options: &mut GitStashApplyOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage, and the initializer retains no pointer into it.
    let status = unsafe { ffi::git_stash_apply_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_stash_pop
/// Applies and then removes the stash at `index`.
pub fn git_stash_pop(
    repository: &mut GitRepositoryMut<'_>,
    index: usize,
    options: Option<GitStashApplyOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    // SAFETY: the repository is exclusive and the optional options remain
    // live while C synchronously applies and removes the stash.
    let status = unsafe {
        ffi::git_stash_pop(
            repository.as_mut_ptr(),
            index,
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    fn edit(fixture: &HistoryFixture) {
        std::fs::write(
            fixture.directory.path().join("README.md"),
            b"fixture\nstashed worktree contents\n",
        )
        .unwrap();
    }

    fn edit_index_untracked_and_ignored(fixture: &HistoryFixture) {
        std::fs::write(
            fixture.directory.path().join("README.md"),
            b"fixture\nstaged stash contents\n",
        )
        .unwrap();
        std::fs::write(fixture.directory.path().join(".gitignore"), b"*.ignored\n").unwrap();
        std::fs::write(
            fixture.directory.path().join("untracked.txt"),
            b"untracked stash contents\n",
        )
        .unwrap();
        std::fs::write(
            fixture.directory.path().join("private.ignored"),
            b"ignored stash contents\n",
        )
        .unwrap();
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, fixture.repository.as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_index_add_bypath(index, c"README.md".as_ptr()) },
            0
        );
        assert_eq!(unsafe { ffi::git_index_write(index) }, 0);
        unsafe { ffi::git_index_free(index) };
    }

    unsafe fn raw_stash(repository: *mut ffi::git_repository) -> (Vec<u8>, Vec<Vec<u8>>) {
        let mut signature = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_signature_new(
                    &mut signature,
                    c"Crustify".as_ptr(),
                    c"crustify@example.com".as_ptr(),
                    1_700_000_200,
                    0,
                )
            },
            0
        );
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_stash_save(
                    &mut id,
                    repository,
                    signature,
                    c"equivalence stash".as_ptr(),
                    ffi::git_stash_flags_GIT_STASH_DEFAULT,
                )
            },
            0
        );
        unsafe extern "C" fn collect(
            _index: usize,
            message: *const core::ffi::c_char,
            _id: *const ffi::git_oid,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            unsafe { &mut *payload.cast::<Vec<Vec<u8>>>() }
                .push(unsafe { CStr::from_ptr(message) }.to_bytes().to_vec());
            0
        }
        let mut messages = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_stash_foreach(
                    repository,
                    Some(collect),
                    core::ptr::from_mut(&mut messages).cast(),
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_stash_pop(repository, 0, core::ptr::null()) },
            0
        );
        unsafe { ffi::git_signature_free(signature) };
        (id.id.to_vec(), messages)
    }

    fn safe_stash(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
    ) -> (Vec<u8>, Vec<Vec<u8>>) {
        let signature = crate::signature::git_signature_new(
            c"Crustify",
            c"crustify@example.com",
            1_700_000_200,
            0,
        )
        .unwrap();
        let mut id = git_stash_save(
            repository,
            signature.as_ref(),
            Some(c"equivalence stash"),
            GitStashFlags::NONE,
        )
        .unwrap();
        let mut messages = Vec::new();
        git_stash_foreach(
            repository,
            &mut |_index: usize, message: Option<&CStr>, _id: OidRef<'_>| {
                messages.push(message.unwrap().to_bytes().to_vec());
                0
            },
        )
        .unwrap();
        git_stash_pop(repository, 0, None).unwrap();
        let id = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(id).cast()) }.unwrap();
        (id.raw_bytes().elems().collect(), messages)
    }

    unsafe fn raw_stash_options(
        repository: *mut ffi::git_repository,
    ) -> (Vec<u8>, Vec<u32>, Vec<Vec<u8>>, u32) {
        let mut signature = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_signature_new(
                    &mut signature,
                    c"Crustify".as_ptr(),
                    c"crustify@example.com".as_ptr(),
                    1_700_000_300,
                    0,
                )
            },
            0
        );
        let mut save = unsafe { core::mem::zeroed::<ffi::git_stash_save_options>() };
        assert_eq!(
            unsafe {
                ffi::git_stash_save_options_init(&mut save, ffi::GIT_STASH_SAVE_OPTIONS_VERSION)
            },
            0
        );
        save.stasher = signature;
        save.message = c"options stash".as_ptr();
        save.flags = ffi::git_stash_flags_GIT_STASH_INCLUDE_UNTRACKED
            | ffi::git_stash_flags_GIT_STASH_INCLUDE_IGNORED;
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_stash_save_with_opts(&mut id, repository, &save) },
            0
        );

        unsafe extern "C" fn progress(
            stage: ffi::git_stash_apply_progress_t,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            unsafe { &mut *payload.cast::<Vec<u32>>() }.push(stage);
            0
        }
        let mut stages = Vec::new();
        let mut apply = unsafe { core::mem::zeroed::<ffi::git_stash_apply_options>() };
        assert_eq!(
            unsafe {
                ffi::git_stash_apply_options_init(&mut apply, ffi::GIT_STASH_APPLY_OPTIONS_VERSION)
            },
            0
        );
        apply.flags = ffi::git_stash_apply_flags_GIT_STASH_APPLY_REINSTATE_INDEX;
        apply.progress_cb = Some(progress);
        apply.progress_payload = core::ptr::from_mut(&mut stages).cast();
        assert_eq!(unsafe { ffi::git_stash_apply(repository, 0, &apply) }, 0);

        let mut status = 0;
        assert_eq!(
            unsafe { ffi::git_status_file(&mut status, repository, c"README.md".as_ptr()) },
            0
        );
        let workdir = unsafe { CStr::from_ptr(ffi::git_repository_workdir(repository)) }
            .to_str()
            .unwrap();
        let contents = [
            "README.md",
            ".gitignore",
            "untracked.txt",
            "private.ignored",
        ]
        .map(|path| std::fs::read(std::path::Path::new(workdir).join(path)).unwrap())
        .to_vec();
        assert_eq!(unsafe { ffi::git_stash_drop(repository, 0) }, 0);
        unsafe { ffi::git_signature_free(signature) };
        (id.id.to_vec(), stages, contents, status)
    }

    fn safe_stash_options(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
    ) -> (Vec<u8>, Vec<u32>, Vec<Vec<u8>>, u32) {
        use crate::api::stash::{GitStashApplyFlags, GitStashApplyOptions, GitStashSaveOptions};

        let signature = crate::signature::git_signature_new(
            c"Crustify",
            c"crustify@example.com",
            1_700_000_300,
            0,
        )
        .unwrap();
        let mut save = GitStashSaveOptions::new();
        git_stash_save_options_init(&mut save.as_mut(), ffi::GIT_STASH_SAVE_OPTIONS_VERSION)
            .unwrap();
        {
            let mut options = save.as_mut();
            options.set_stasher(Some(signature.as_ref()));
            options.set_message(Some(c"options stash"));
            options.set_flags(GitStashFlags::INCLUDE_UNTRACKED | GitStashFlags::INCLUDE_IGNORED);
        }
        let mut id = git_stash_save_with_opts(repository, save.as_ref()).unwrap();

        let mut stages = Vec::new();
        let mut progress = |stage: StashApplyProgress| {
            stages.push(stage as u32);
            0
        };
        let mut apply = GitStashApplyOptions::new();
        git_stash_apply_options_init(&mut apply.as_mut(), ffi::GIT_STASH_APPLY_OPTIONS_VERSION)
            .unwrap();
        {
            let mut options = apply.as_mut();
            options.set_flags(GitStashApplyFlags::REINSTATE_INDEX);
            options.set_progress_callback(&mut progress);
        }
        git_stash_apply(repository, 0, Some(apply.as_ref())).unwrap();
        drop(apply);
        drop(progress);

        let status = crate::status::git_status_file(repository, c"README.md")
            .unwrap()
            .bits();
        let workdir = crate::repository::git_repository_workdir(repository.as_ref()).unwrap();
        let contents = [
            "README.md",
            ".gitignore",
            "untracked.txt",
            "private.ignored",
        ]
        .map(|path| {
            std::fs::read(std::path::Path::new(workdir.to_str().unwrap()).join(path)).unwrap()
        })
        .to_vec();
        git_stash_drop(repository, 0).unwrap();
        let id = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(id).cast()) }.unwrap();
        (id.raw_bytes().elems().collect(), stages, contents, status)
    }

    #[test]
    fn io_equiv_stash_save_list_and_pop() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("stash-raw");
        let safe = HistoryFixture::new("stash-safe");
        edit(&raw);
        edit(&safe);
        let raw_observation = unsafe { raw_stash(raw.repository.as_ptr()) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(raw_observation, safe_stash(&mut safe_repository));
        assert_eq!(
            std::fs::read(raw.directory.path().join("README.md")).unwrap(),
            std::fs::read(safe.directory.path().join("README.md")).unwrap()
        );
        assert_eq!(
            std::fs::read(raw.directory.path().join("README.md")).unwrap(),
            b"fixture\nstashed worktree contents\n"
        );
    }

    #[test]
    fn io_equiv_stash_options_untracked_ignored_reinstate_index_progress_and_drop() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("stash-options-raw");
        let safe = HistoryFixture::new("stash-options-safe");
        edit_index_untracked_and_ignored(&raw);
        edit_index_untracked_and_ignored(&safe);
        let raw_observation = unsafe { raw_stash_options(raw.repository.as_ptr()) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        let safe_observation = safe_stash_options(&mut safe_repository);
        assert_eq!(raw_observation, safe_observation);
        assert_eq!(
            raw_observation.3,
            ffi::git_status_t_GIT_STATUS_INDEX_MODIFIED
        );
        assert_eq!(
            raw_observation.1.last(),
            Some(&ffi::git_stash_apply_progress_t_GIT_STASH_APPLY_PROGRESS_DONE)
        );
    }
}

#[cfg(test)]
mod scheduled_apply_tests {
    use super::*;
    use crate::api::stash::GitStashApplyOptions;

    #[test]
    fn the_published_initializer_restores_every_constructed_default() {
        // The C initializer copies `GIT_STASH_APPLY_OPTIONS_INIT` over the
        // whole record, so reinitializing deliberately dirtied options pins
        // `GitStashApplyOptions::new` to that template.
        let mut options = GitStashApplyOptions::new();
        {
            let mut view = options.as_mut();
            view.set_version(0);
            view.set_flags(crate::api::stash::GitStashApplyFlags::REINSTATE_INDEX);
            view.checkout_options_mut().set_version(0);
        }

        git_stash_apply_options_init(&mut options.as_mut(), ffi::GIT_STASH_APPLY_OPTIONS_VERSION)
            .unwrap();

        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_STASH_APPLY_OPTIONS_VERSION);
        assert_eq!(
            view.flags(),
            Ok(crate::api::stash::GitStashApplyFlags::NONE)
        );
        assert!(!view.has_progress_callback());
        assert!(!view.has_progress_payload());
        assert_eq!(
            view.checkout_options().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
    }

    #[test]
    fn an_unsupported_version_is_rejected_without_writing() {
        // SAFETY: process-global initialization is reference counted and is
        // balanced below; the rejected version reaches `git_error_set`, which
        // allocates through the allocator only initialization installs.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut options = GitStashApplyOptions::new();
        options
            .as_mut()
            .set_flags(crate::api::stash::GitStashApplyFlags::REINSTATE_INDEX);
        assert!(git_stash_apply_options_init(&mut options.as_mut(), 0).is_err());
        assert_eq!(
            options.as_ref().flags(),
            Ok(crate::api::stash::GitStashApplyFlags::REINSTATE_INDEX)
        );
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
