//! Safe wrappers for libgit2 filter APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

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

/// Wraps: git_filter_flag_t
/// A checked set of options controlling attribute lookup and filter safety.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitFilterFlags(ffi::git_filter_flag_t);

impl GitFilterFlags {
    /// Use the default filter behavior.
    pub const DEFAULT: Self = Self(ffi::git_filter_flag_t_GIT_FILTER_DEFAULT);
    /// Continue despite `safecrlf` violations.
    pub const ALLOW_UNSAFE: Self = Self(ffi::git_filter_flag_t_GIT_FILTER_ALLOW_UNSAFE);
    /// Do not load system-level attributes.
    pub const NO_SYSTEM_ATTRIBUTES: Self =
        Self(ffi::git_filter_flag_t_GIT_FILTER_NO_SYSTEM_ATTRIBUTES);
    /// Also load attributes from the root of `HEAD`.
    pub const ATTRIBUTES_FROM_HEAD: Self =
        Self(ffi::git_filter_flag_t_GIT_FILTER_ATTRIBUTES_FROM_HEAD);
    /// Load attributes from the commit selected by filter options.
    pub const ATTRIBUTES_FROM_COMMIT: Self =
        Self(ffi::git_filter_flag_t_GIT_FILTER_ATTRIBUTES_FROM_COMMIT);
    /// Every filter flag published by these bindings.
    pub const ALL: Self = Self(
        Self::ALLOW_UNSAFE.0
            | Self::NO_SYSTEM_ATTRIBUTES.0
            | Self::ATTRIBUTES_FROM_HEAD.0
            | Self::ATTRIBUTES_FROM_COMMIT.0,
    );

    /// Converts raw bits when every bit is a published filter option.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_filter_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_filter_flag_t {
        self.0
    }

    /// Returns whether no filter option is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any option in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitFilterFlags> for ffi::git_filter_flag_t {
    fn from(flags: GitFilterFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_filter_flag_t> for GitFilterFlags {
    type Error = ffi::git_filter_flag_t;

    fn try_from(bits: ffi::git_filter_flag_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitFilterFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitFilterFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitFilterFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitFilterFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitFilterFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod filter_flag_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn filter_options_combine_and_validate() {
        let flags = GitFilterFlags::ALLOW_UNSAFE | GitFilterFlags::NO_SYSTEM_ATTRIBUTES;
        assert!(flags.contains(GitFilterFlags::ALLOW_UNSAFE));
        assert!(flags.intersects(GitFilterFlags::NO_SYSTEM_ATTRIBUTES));
        assert_eq!(GitFilterFlags::from_bits(flags.bits()), Some(flags));
        assert!(GitFilterFlags::DEFAULT.is_empty());

        let unknown = GitFilterFlags::ALL.bits() << 1;
        assert_eq!(GitFilterFlags::from_bits(unknown), None);
    }

    #[test]
    fn filter_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitFilterFlags>(),
            size_of::<ffi::git_filter_flag_t>()
        );
        assert_eq!(
            align_of::<GitFilterFlags>(),
            align_of::<ffi::git_filter_flag_t>()
        );
    }
}
