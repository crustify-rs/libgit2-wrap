//! Safe wrappers for libgit2 reset APIs.

use crate::ffi;
use crate::object::GitObjectRef;
use crate::repository::GitRepositoryMut;
use crate::strarray::GitStrArrayRef;

/// Wraps: git_reset_t
/// Selects which repository state a reset updates.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResetType {
    /// Move `HEAD` to the target commit without changing the index or worktree.
    Soft = ffi::git_reset_t_GIT_RESET_SOFT,
    /// Perform a soft reset and replace the index with the target tree.
    Mixed = ffi::git_reset_t_GIT_RESET_MIXED,
    /// Perform a mixed reset and replace tracked worktree files from the index.
    Hard = ffi::git_reset_t_GIT_RESET_HARD,
}

/// An integer that is not a published [`ResetType`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidResetType(ffi::git_reset_t);

impl InvalidResetType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_reset_t {
        self.0
    }
}

impl From<ResetType> for ffi::git_reset_t {
    fn from(reset_type: ResetType) -> Self {
        reset_type as Self
    }
}

impl TryFrom<ffi::git_reset_t> for ResetType {
    type Error = InvalidResetType;

    fn try_from(reset_type: ffi::git_reset_t) -> Result<Self, Self::Error> {
        match reset_type {
            ffi::git_reset_t_GIT_RESET_SOFT => Ok(Self::Soft),
            ffi::git_reset_t_GIT_RESET_MIXED => Ok(Self::Mixed),
            ffi::git_reset_t_GIT_RESET_HARD => Ok(Self::Hard),
            value => Err(InvalidResetType(value)),
        }
    }
}

/// Wraps: git_reset_default
/// Resets the selected index paths to `target`, or removes them with `None`.
pub fn git_reset_default(
    repository: &mut GitRepositoryMut<'_>,
    target: Option<GitObjectRef<'_>>,
    pathspecs: GitStrArrayRef<'_>,
) -> Result<(), i32> {
    if pathspecs.count() == 0 {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    let target = target.map_or(core::ptr::null(), |object| object.as_ptr());
    // SAFETY: the repository is exclusively borrowed, `target` is null or a
    // live object, and the nonempty pathspec array is live for the call. The
    // function retains none of these pointers.
    let status =
        unsafe { ffi::git_reset_default(repository.as_mut_ptr(), target, pathspecs.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn reset_types_round_trip_through_the_c_type() {
        for reset_type in [ResetType::Soft, ResetType::Mixed, ResetType::Hard] {
            let raw = ffi::git_reset_t::from(reset_type);
            assert_eq!(ResetType::try_from(raw), Ok(reset_type));
        }
    }

    #[test]
    fn invalid_reset_type_is_rejected_without_constructing_an_enum() {
        let invalid = ffi::git_reset_t_GIT_RESET_HARD + 1;
        let error = ResetType::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn reset_type_matches_the_c_abi_scalar() {
        assert_eq!(size_of::<ResetType>(), size_of::<ffi::git_reset_t>());
        assert_eq!(align_of::<ResetType>(), align_of::<ffi::git_reset_t>());
    }

    #[test]
    fn default_reset_rejects_an_empty_pathspec_before_ffi() {
        let repository = Box::new(core::mem::MaybeUninit::<ffi::git_repository>::zeroed());
        let raw = Box::into_raw(repository).cast::<ffi::git_repository>();
        {
            // SAFETY: `raw` addresses live opaque storage and remains exclusively
            // owned for this handle. The wrapper rejects the empty pathspec before
            // the C implementation can inspect repository state.
            let mut repository = unsafe { GitRepositoryMut::from_ptr(raw) }.unwrap();
            let pathspecs = crate::strarray::GitStrArray::new();
            assert_eq!(
                git_reset_default(&mut repository, None, pathspecs.as_ref()),
                Err(ffi::git_error_code_GIT_EINVALID)
            );
        }
        // SAFETY: no handle remains and this recovers the exact allocation.
        drop(unsafe { Box::from_raw(raw.cast::<core::mem::MaybeUninit<ffi::git_repository>>()) });
    }
}
