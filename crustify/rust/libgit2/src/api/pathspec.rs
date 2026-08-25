//! Safe wrappers for libgit2 pathspec APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_pathspec_flag_t
/// A checked set of options controlling pathspec matching.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitPathspecFlags(ffi::git_pathspec_flag_t);

impl GitPathspecFlags {
    /// Use libgit2's default pathspec matching behavior.
    pub const DEFAULT: Self = Self(ffi::git_pathspec_flag_t_GIT_PATHSPEC_DEFAULT);
    /// Match paths without regard to case.
    pub const IGNORE_CASE: Self = Self(ffi::git_pathspec_flag_t_GIT_PATHSPEC_IGNORE_CASE);
    /// Force case-sensitive matching.
    pub const USE_CASE: Self = Self(ffi::git_pathspec_flag_t_GIT_PATHSPEC_USE_CASE);
    /// Treat patterns as literal strings instead of globs.
    pub const NO_GLOB: Self = Self(ffi::git_pathspec_flag_t_GIT_PATHSPEC_NO_GLOB);
    /// Return `GIT_ENOTFOUND` when no paths match.
    pub const NO_MATCH_ERROR: Self = Self(ffi::git_pathspec_flag_t_GIT_PATHSPEC_NO_MATCH_ERROR);
    /// Retain pathspec patterns that matched no paths.
    pub const FIND_FAILURES: Self = Self(ffi::git_pathspec_flag_t_GIT_PATHSPEC_FIND_FAILURES);
    /// Retain only failed pathspec patterns, not matching filenames.
    pub const FAILURES_ONLY: Self = Self(ffi::git_pathspec_flag_t_GIT_PATHSPEC_FAILURES_ONLY);
    /// Every pathspec option published by this libgit2 version.
    pub const ALL: Self = Self(
        Self::IGNORE_CASE.0
            | Self::USE_CASE.0
            | Self::NO_GLOB.0
            | Self::NO_MATCH_ERROR.0
            | Self::FIND_FAILURES.0
            | Self::FAILURES_ONLY.0,
    );

    /// Converts raw bits when every bit is a published pathspec option.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_pathspec_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_pathspec_flag_t {
        self.0
    }

    /// Returns whether no option is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any option in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitPathspecFlags> for ffi::git_pathspec_flag_t {
    fn from(flags: GitPathspecFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_pathspec_flag_t> for GitPathspecFlags {
    type Error = ffi::git_pathspec_flag_t;

    fn try_from(bits: ffi::git_pathspec_flag_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitPathspecFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitPathspecFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitPathspecFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitPathspecFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitPathspecFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn pathspec_flags_combine_and_validate() {
        let flags = GitPathspecFlags::IGNORE_CASE
            | GitPathspecFlags::NO_GLOB
            | GitPathspecFlags::FIND_FAILURES;
        assert!(flags.contains(GitPathspecFlags::IGNORE_CASE));
        assert!(flags.intersects(GitPathspecFlags::FIND_FAILURES));
        assert!(!flags.intersects(GitPathspecFlags::USE_CASE));
        assert_eq!(GitPathspecFlags::from_bits(flags.bits()), Some(flags));
        assert!(GitPathspecFlags::DEFAULT.is_empty());

        let unknown = GitPathspecFlags::ALL.bits() << 1;
        assert_eq!(GitPathspecFlags::from_bits(unknown), None);
        assert_eq!(GitPathspecFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn pathspec_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitPathspecFlags>(),
            size_of::<ffi::git_pathspec_flag_t>()
        );
        assert_eq!(
            align_of::<GitPathspecFlags>(),
            align_of::<ffi::git_pathspec_flag_t>()
        );
    }

    #[test]
    fn pathspec_flag_complement_stays_within_published_bits() {
        assert_eq!(
            !GitPathspecFlags::NO_GLOB,
            GitPathspecFlags::ALL & !GitPathspecFlags::NO_GLOB
        );
    }
}
