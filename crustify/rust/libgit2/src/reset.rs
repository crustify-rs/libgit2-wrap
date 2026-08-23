//! Safe wrappers for libgit2 reset APIs.

use crate::ffi;

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
}
