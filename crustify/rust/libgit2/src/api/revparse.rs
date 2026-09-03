//! Safe wrappers for libgit2 revparse APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_revspec_t
/// A checked set of parse-intent and range-semantics flags.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitRevspecFlags(ffi::git_revspec_t);

impl GitRevspecFlags {
    /// No parse intent has been recorded yet.
    pub const EMPTY: Self = Self(0);
    /// The expression identifies one object.
    pub const SINGLE: Self = Self(ffi::git_revspec_t_GIT_REVSPEC_SINGLE);
    /// The expression identifies a range.
    pub const RANGE: Self = Self(ffi::git_revspec_t_GIT_REVSPEC_RANGE);
    /// The range uses symmetric-difference merge-base semantics.
    pub const MERGE_BASE: Self = Self(ffi::git_revspec_t_GIT_REVSPEC_MERGE_BASE);
    /// Every revspec flag published by this libgit2 version.
    pub const ALL: Self = Self(Self::SINGLE.0 | Self::RANGE.0 | Self::MERGE_BASE.0);

    /// Converts raw bits when every bit is published by this libgit2 version.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_revspec_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_revspec_t {
        self.0
    }

    /// Returns whether no intent is recorded.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitRevspecFlags> for ffi::git_revspec_t {
    fn from(flags: GitRevspecFlags) -> Self {
        flags.bits()
    }
}

/// Raw revspec bits not published by this libgit2 version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitRevspecFlags(ffi::git_revspec_t);

impl InvalidGitRevspecFlags {
    /// Returns the unrecognized bit set.
    #[must_use]
    pub const fn value(self) -> ffi::git_revspec_t {
        self.0
    }
}

impl TryFrom<ffi::git_revspec_t> for GitRevspecFlags {
    type Error = InvalidGitRevspecFlags;

    fn try_from(bits: ffi::git_revspec_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(InvalidGitRevspecFlags(bits))
    }
}

impl BitOr for GitRevspecFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitRevspecFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitRevspecFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitRevspecFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitRevspecFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn revspec_flags_compose_and_validate() {
        let flags = GitRevspecFlags::RANGE | GitRevspecFlags::MERGE_BASE;
        assert!(flags.contains(GitRevspecFlags::RANGE));
        assert!(flags.intersects(GitRevspecFlags::MERGE_BASE));
        assert_eq!(GitRevspecFlags::from_bits(flags.bits()), Some(flags));
        assert_eq!(GitRevspecFlags::from_bits(1 << 3), None);
        assert_eq!((!GitRevspecFlags::SINGLE).bits(), 0b110);
    }

    #[test]
    fn revspec_flags_match_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitRevspecFlags>(),
            size_of::<ffi::git_revspec_t>()
        );
        assert_eq!(
            align_of::<GitRevspecFlags>(),
            align_of::<ffi::git_revspec_t>()
        );
    }
}
