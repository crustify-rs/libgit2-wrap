//! Safe wrappers for libgit2 status APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_status_opt_t
/// A checked set of options controlling status scans.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitStatusOptionFlags(ffi::git_status_opt_t);

impl GitStatusOptionFlags {
    /// No optional status behavior.
    pub const NONE: Self = Self(0);
    /// Include untracked paths.
    pub const INCLUDE_UNTRACKED: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_UNTRACKED);
    /// Include ignored paths.
    pub const INCLUDE_IGNORED: Self = Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_IGNORED);
    /// Include paths without changes.
    pub const INCLUDE_UNMODIFIED: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_UNMODIFIED);
    /// Skip submodules without pending type changes.
    pub const EXCLUDE_SUBMODULES: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_EXCLUDE_SUBMODULES);
    /// Recurse into untracked directories.
    pub const RECURSE_UNTRACKED_DIRS: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RECURSE_UNTRACKED_DIRS);
    /// Treat pathspec entries as literal paths.
    pub const DISABLE_PATHSPEC_MATCH: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_DISABLE_PATHSPEC_MATCH);
    /// Include the contents of ignored directories.
    pub const RECURSE_IGNORED_DIRS: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RECURSE_IGNORED_DIRS);
    /// Detect renames between `HEAD` and the index.
    pub const RENAMES_HEAD_TO_INDEX: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RENAMES_HEAD_TO_INDEX);
    /// Detect renames between the index and working directory.
    pub const RENAMES_INDEX_TO_WORKDIR: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RENAMES_INDEX_TO_WORKDIR);
    /// Sort status entries case-sensitively.
    pub const SORT_CASE_SENSITIVELY: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_SORT_CASE_SENSITIVELY);
    /// Sort status entries case-insensitively.
    pub const SORT_CASE_INSENSITIVELY: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_SORT_CASE_INSENSITIVELY);
    /// Consider rewritten files as rename sources.
    pub const RENAMES_FROM_REWRITES: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RENAMES_FROM_REWRITES);
    /// Do not refresh the index from disk.
    pub const NO_REFRESH: Self = Self(ffi::git_status_opt_t_GIT_STATUS_OPT_NO_REFRESH);
    /// Refresh index stat information for unchanged files.
    pub const UPDATE_INDEX: Self = Self(ffi::git_status_opt_t_GIT_STATUS_OPT_UPDATE_INDEX);
    /// Report unreadable working-directory paths.
    pub const INCLUDE_UNREADABLE: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_UNREADABLE);
    /// Report unreadable paths as untracked instead.
    pub const INCLUDE_UNREADABLE_AS_UNTRACKED: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_UNREADABLE_AS_UNTRACKED);
    /// Every status option published by this version of libgit2.
    pub const ALL: Self = Self(
        Self::INCLUDE_UNTRACKED.0
            | Self::INCLUDE_IGNORED.0
            | Self::INCLUDE_UNMODIFIED.0
            | Self::EXCLUDE_SUBMODULES.0
            | Self::RECURSE_UNTRACKED_DIRS.0
            | Self::DISABLE_PATHSPEC_MATCH.0
            | Self::RECURSE_IGNORED_DIRS.0
            | Self::RENAMES_HEAD_TO_INDEX.0
            | Self::RENAMES_INDEX_TO_WORKDIR.0
            | Self::SORT_CASE_SENSITIVELY.0
            | Self::SORT_CASE_INSENSITIVELY.0
            | Self::RENAMES_FROM_REWRITES.0
            | Self::NO_REFRESH.0
            | Self::UPDATE_INDEX.0
            | Self::INCLUDE_UNREADABLE.0
            | Self::INCLUDE_UNREADABLE_AS_UNTRACKED.0,
    );

    /// Converts raw bits when every bit is a published status option.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_status_opt_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_status_opt_t {
        self.0
    }

    /// Returns whether no option is set.
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

impl From<GitStatusOptionFlags> for ffi::git_status_opt_t {
    fn from(flags: GitStatusOptionFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_status_opt_t> for GitStatusOptionFlags {
    type Error = ffi::git_status_opt_t;

    fn try_from(bits: ffi::git_status_opt_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitStatusOptionFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitStatusOptionFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitStatusOptionFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitStatusOptionFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitStatusOptionFlags {
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
    fn status_options_combine_and_validate() {
        let mut options = GitStatusOptionFlags::INCLUDE_UNTRACKED;
        options |= GitStatusOptionFlags::RECURSE_UNTRACKED_DIRS;
        assert!(options.contains(GitStatusOptionFlags::INCLUDE_UNTRACKED));
        assert!(options.intersects(GitStatusOptionFlags::RECURSE_UNTRACKED_DIRS));
        assert!(!options.intersects(GitStatusOptionFlags::INCLUDE_IGNORED));
        assert_eq!(
            GitStatusOptionFlags::from_bits(options.bits()),
            Some(options)
        );
        assert!(GitStatusOptionFlags::NONE.is_empty());

        let unknown = GitStatusOptionFlags::ALL.bits() + 1;
        assert_eq!(GitStatusOptionFlags::from_bits(unknown), None);
        assert_eq!(GitStatusOptionFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn status_options_complement_stays_within_published_bits() {
        let options =
            GitStatusOptionFlags::INCLUDE_UNTRACKED | GitStatusOptionFlags::INCLUDE_IGNORED;
        assert!(!(!options).intersects(options));
        assert_eq!(options | !options, GitStatusOptionFlags::ALL);
    }

    #[test]
    fn status_options_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitStatusOptionFlags>(),
            size_of::<ffi::git_status_opt_t>()
        );
        assert_eq!(
            align_of::<GitStatusOptionFlags>(),
            align_of::<ffi::git_status_opt_t>()
        );
    }
}
