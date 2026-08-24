//! Safe wrappers for libgit2 merge APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_merge_file_flag_t
/// Known behavior flags accepted by file-level merges.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MergeFileFlags(ffi::git_merge_file_flag_t);

impl MergeFileFlags {
    /// Use libgit2's default file-merge behavior.
    pub const DEFAULT: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DEFAULT);
    /// Produce standard two-sided conflict markers.
    pub const STYLE_MERGE: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_MERGE);
    /// Include the common ancestor in conflict markers.
    pub const STYLE_DIFF3: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_DIFF3);
    /// Condense non-alphanumeric regions while comparing.
    pub const SIMPLIFY_ALNUM: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_SIMPLIFY_ALNUM);
    /// Ignore all whitespace changes.
    pub const IGNORE_WHITESPACE: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_IGNORE_WHITESPACE);
    /// Ignore changes in the amount of whitespace.
    pub const IGNORE_WHITESPACE_CHANGE: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_IGNORE_WHITESPACE_CHANGE);
    /// Ignore whitespace changes at the ends of lines.
    pub const IGNORE_WHITESPACE_EOL: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_IGNORE_WHITESPACE_EOL);
    /// Use the patience-diff algorithm.
    pub const DIFF_PATIENCE: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DIFF_PATIENCE);
    /// Spend extra time finding a minimal diff.
    pub const DIFF_MINIMAL: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DIFF_MINIMAL);
    /// Produce zealous diff3 conflict markers.
    pub const STYLE_ZDIFF3: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_ZDIFF3);
    /// Accept output containing conflict markers as a merge result.
    pub const ACCEPT_CONFLICTS: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_ACCEPT_CONFLICTS);
    /// Every file-merge flag published by this libgit2 version.
    pub const ALL: Self = Self(
        Self::STYLE_MERGE.0
            | Self::STYLE_DIFF3.0
            | Self::SIMPLIFY_ALNUM.0
            | Self::IGNORE_WHITESPACE.0
            | Self::IGNORE_WHITESPACE_CHANGE.0
            | Self::IGNORE_WHITESPACE_EOL.0
            | Self::DIFF_PATIENCE.0
            | Self::DIFF_MINIMAL.0
            | Self::STYLE_ZDIFF3.0
            | Self::ACCEPT_CONFLICTS.0,
    );

    /// Converts raw bits when every bit is known to this libgit2 version.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_merge_file_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_merge_file_flag_t {
        self.0
    }

    /// Returns whether no behavior-changing flag is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for MergeFileFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for MergeFileFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for MergeFileFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for MergeFileFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for MergeFileFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

impl From<MergeFileFlags> for ffi::git_merge_file_flag_t {
    fn from(flags: MergeFileFlags) -> Self {
        flags.bits()
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_file_merge_flags_form_checked_sets() {
        let mut flags = MergeFileFlags::STYLE_DIFF3 | MergeFileFlags::DIFF_PATIENCE;
        assert!(flags.contains(MergeFileFlags::STYLE_DIFF3));
        flags |= MergeFileFlags::ACCEPT_CONFLICTS;
        assert!(flags.contains(MergeFileFlags::ACCEPT_CONFLICTS));
        assert_eq!(MergeFileFlags::from_bits(flags.bits()), Some(flags));
        assert_eq!(ffi::git_merge_file_flag_t::from(flags), flags.bits());
        assert!(MergeFileFlags::DEFAULT.is_empty());
    }

    #[test]
    fn unknown_file_merge_flags_are_rejected() {
        let unknown = MergeFileFlags::ALL.bits() << 1;
        assert_eq!(MergeFileFlags::from_bits(unknown), None);
    }

    #[test]
    fn file_merge_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<MergeFileFlags>(),
            size_of::<ffi::git_merge_file_flag_t>()
        );
        assert_eq!(
            align_of::<MergeFileFlags>(),
            align_of::<ffi::git_merge_file_flag_t>()
        );
    }
}
