//! Safe wrappers for libgit2 repository APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_repository_init_flag_t
/// A checked set of optional repository-initialization behaviors.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitRepositoryInitFlags(ffi::git_repository_init_flag_t);

impl GitRepositoryInitFlags {
    /// Use libgit2's default initialization behavior.
    pub const EMPTY: Self = Self(0);
    /// Create a bare repository without a working directory.
    pub const BARE: Self = Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_BARE);
    /// Fail if the destination already appears to contain a repository.
    pub const NO_REINIT: Self = Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_NO_REINIT);
    /// Create the trailing repository and working-directory components.
    pub const MKDIR: Self = Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_MKDIR);
    /// Recursively create all missing path components.
    pub const MKPATH: Self = Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_MKPATH);
    /// Load templates from an external template directory when available.
    pub const EXTERNAL_TEMPLATE: Self =
        Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_EXTERNAL_TEMPLATE);
    /// Store relative paths in an alternate working directory's gitlink.
    pub const RELATIVE_GITLINK: Self =
        Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_RELATIVE_GITLINK);
    /// Every initialization flag published by this libgit2 version.
    pub const ALL: Self = Self(
        Self::BARE.0
            | Self::NO_REINIT.0
            | Self::MKDIR.0
            | Self::MKPATH.0
            | Self::EXTERNAL_TEMPLATE.0
            | Self::RELATIVE_GITLINK.0,
    );

    /// Converts raw bits when every bit is published by this libgit2 version.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_repository_init_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_repository_init_flag_t {
        self.0
    }

    /// Returns whether no optional behavior is enabled.
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

impl From<GitRepositoryInitFlags> for ffi::git_repository_init_flag_t {
    fn from(flags: GitRepositoryInitFlags) -> Self {
        flags.bits()
    }
}

impl BitOr for GitRepositoryInitFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitRepositoryInitFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitRepositoryInitFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitRepositoryInitFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitRepositoryInitFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn repository_init_flags_compose_and_round_trip() {
        let flags = GitRepositoryInitFlags::BARE
            | GitRepositoryInitFlags::MKPATH
            | GitRepositoryInitFlags::EXTERNAL_TEMPLATE;
        assert!(flags.contains(GitRepositoryInitFlags::BARE));
        assert!(flags.intersects(GitRepositoryInitFlags::MKPATH));
        assert!(!flags.intersects(GitRepositoryInitFlags::NO_REINIT));
        assert_eq!(GitRepositoryInitFlags::from_bits(flags.bits()), Some(flags));
        assert_eq!(
            ffi::git_repository_init_flag_t::from(flags),
            (1 << 0) | (1 << 4) | (1 << 5)
        );
    }

    #[test]
    fn repository_init_flags_reject_unpublished_bits() {
        assert_eq!(
            GitRepositoryInitFlags::from_bits(GitRepositoryInitFlags::ALL.bits()),
            Some(GitRepositoryInitFlags::ALL)
        );
        assert_eq!(GitRepositoryInitFlags::from_bits(1 << 2), None);
    }

    #[test]
    fn repository_init_flags_match_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitRepositoryInitFlags>(),
            size_of::<ffi::git_repository_init_flag_t>()
        );
        assert_eq!(
            align_of::<GitRepositoryInitFlags>(),
            align_of::<ffi::git_repository_init_flag_t>()
        );
    }
}
