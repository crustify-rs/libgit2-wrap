//! Safe wrappers for libgit2 worktree APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_worktree_prune_t
/// A checked set of overrides for pruning a linked worktree.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitWorktreePruneFlags(ffi::git_worktree_prune_t);

impl GitWorktreePruneFlags {
    /// Apply the normal worktree pruning checks.
    pub const NONE: Self = Self(0);
    /// Prune even when the worktree is otherwise valid.
    pub const VALID: Self = Self(ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_VALID);
    /// Prune even when the worktree is locked.
    pub const LOCKED: Self = Self(ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_LOCKED);
    /// Prune a checked-out worktree.
    pub const WORKING_TREE: Self = Self(ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_WORKING_TREE);
    /// Every prune override published by this version of libgit2.
    pub const ALL: Self = Self(Self::VALID.0 | Self::LOCKED.0 | Self::WORKING_TREE.0);

    /// Converts raw bits when every bit is a published prune override.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_worktree_prune_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_worktree_prune_t {
        self.0
    }

    /// Returns whether no override is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every override in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any override in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitWorktreePruneFlags> for ffi::git_worktree_prune_t {
    fn from(flags: GitWorktreePruneFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_worktree_prune_t> for GitWorktreePruneFlags {
    type Error = ffi::git_worktree_prune_t;

    fn try_from(bits: ffi::git_worktree_prune_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitWorktreePruneFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitWorktreePruneFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitWorktreePruneFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitWorktreePruneFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitWorktreePruneFlags {
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
    fn prune_overrides_combine_and_validate() {
        let overrides = GitWorktreePruneFlags::VALID | GitWorktreePruneFlags::LOCKED;
        assert!(overrides.contains(GitWorktreePruneFlags::VALID));
        assert!(overrides.intersects(GitWorktreePruneFlags::LOCKED));
        assert!(!overrides.intersects(GitWorktreePruneFlags::WORKING_TREE));
        assert_eq!(
            GitWorktreePruneFlags::from_bits(overrides.bits()),
            Some(overrides)
        );
        assert!(GitWorktreePruneFlags::NONE.is_empty());

        let unknown = GitWorktreePruneFlags::ALL.bits() << 1;
        assert_eq!(GitWorktreePruneFlags::from_bits(unknown), None);
        assert_eq!(GitWorktreePruneFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn prune_override_complement_stays_within_published_bits() {
        assert_eq!(
            !GitWorktreePruneFlags::VALID,
            GitWorktreePruneFlags::LOCKED | GitWorktreePruneFlags::WORKING_TREE
        );
    }

    #[test]
    fn prune_overrides_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitWorktreePruneFlags>(),
            size_of::<ffi::git_worktree_prune_t>()
        );
        assert_eq!(
            align_of::<GitWorktreePruneFlags>(),
            align_of::<ffi::git_worktree_prune_t>()
        );
    }
}
