//! Safe wrappers for libgit2 filter APIs.

use crate::ffi;

/// Wraps: git_filter_mode_t
/// A validated direction for transforming content through a filter list.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitFilterMode {
    /// Export content from the object database to the working tree.
    ToWorktree = ffi::git_filter_mode_t_GIT_FILTER_TO_WORKTREE,
    /// Import content from the working tree into the object database.
    ToObjectDatabase = ffi::git_filter_mode_t_GIT_FILTER_TO_ODB,
}

impl GitFilterMode {
    /// The traditional name for a worktree-directed filter.
    pub const SMUDGE: Self = Self::ToWorktree;
    /// The traditional name for an object-database-directed filter.
    pub const CLEAN: Self = Self::ToObjectDatabase;
}

impl From<GitFilterMode> for ffi::git_filter_mode_t {
    fn from(mode: GitFilterMode) -> Self {
        mode as Self
    }
}

impl TryFrom<ffi::git_filter_mode_t> for GitFilterMode {
    type Error = InvalidGitFilterMode;

    fn try_from(mode: ffi::git_filter_mode_t) -> Result<Self, Self::Error> {
        match mode {
            ffi::git_filter_mode_t_GIT_FILTER_TO_WORKTREE => Ok(Self::ToWorktree),
            ffi::git_filter_mode_t_GIT_FILTER_TO_ODB => Ok(Self::ToObjectDatabase),
            value => Err(InvalidGitFilterMode(value)),
        }
    }
}

/// A raw value that is not a published [`GitFilterMode`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitFilterMode(ffi::git_filter_mode_t);

impl InvalidGitFilterMode {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_filter_mode_t {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn filter_modes_validate_and_preserve_aliases() {
        assert_eq!(GitFilterMode::SMUDGE, GitFilterMode::ToWorktree);
        assert_eq!(GitFilterMode::CLEAN, GitFilterMode::ToObjectDatabase);
        assert_eq!(
            GitFilterMode::try_from(ffi::git_filter_mode_t_GIT_FILTER_TO_WORKTREE),
            Ok(GitFilterMode::ToWorktree)
        );

        let invalid = ffi::git_filter_mode_t_GIT_FILTER_TO_ODB + 1;
        assert_eq!(
            GitFilterMode::try_from(invalid).unwrap_err().value(),
            invalid
        );
    }

    #[test]
    fn filter_mode_matches_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitFilterMode>(),
            size_of::<ffi::git_filter_mode_t>()
        );
        assert_eq!(
            align_of::<GitFilterMode>(),
            align_of::<ffi::git_filter_mode_t>()
        );
    }
}
