//! Safe wrappers for libgit2 stash APIs.

use core::ffi::{CStr, c_void};

use crate::api::stash::{GitStashCallback, GitStashSaveOptionsMut, GitStashSaveOptionsRef};
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
mod tests {
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
        if message.is_null() || oid.is_null() || payload.is_null() {
            return -1;
        }
        // SAFETY: the wrapper supplies this live exclusive callback payload
        // throughout the synchronous traversal.
        let callback = unsafe { &mut *payload.cast::<C>() };
        // SAFETY: both non-null values are transient live inputs documented by
        // libgit2 for the duration of this callback.
        let message = unsafe { CStr::from_ptr(message) };
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
    flags: u32,
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
            flags,
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
    use crate::api::stash::GitStashSaveOptions;

    #[test]
    fn initializer_writes_the_published_defaults() {
        let mut storage = GitStashSaveOptions::<'static>::new();
        let mut options = storage.as_mut();
        git_stash_save_options_init(&mut options, ffi::GIT_STASH_SAVE_OPTIONS_VERSION).unwrap();
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_STASH_SAVE_OPTIONS_VERSION
        );
        assert_eq!(options.as_ref().flags(), 0);
        assert!(options.as_ref().stasher().is_none());
    }
}
