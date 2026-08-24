//! Safe wrappers for libgit2 checkout APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_checkout_strategy_t
/// Checkout behavior selected for a libgit2 operation.
///
/// This is a layout-compatible bit set. Raw values retain unknown bits so
/// options from a newer libgit2 version can still be carried safely.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitCheckoutStrategy(ffi::git_checkout_strategy_t);

impl GitCheckoutStrategy {
    /// Apply only updates that do not overwrite uncommitted data.
    pub const SAFE: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_SAFE);
    /// Force the work directory to match the index, potentially losing data.
    pub const FORCE: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_FORCE);
    /// Recreate files that are missing from the work directory.
    pub const RECREATE_MISSING: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_RECREATE_MISSING);
    /// Apply safe updates even when conflicts exist.
    pub const ALLOW_CONFLICTS: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_ALLOW_CONFLICTS);
    /// Remove untracked, non-ignored files.
    pub const REMOVE_UNTRACKED: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_REMOVE_UNTRACKED);
    /// Remove ignored files.
    pub const REMOVE_IGNORED: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_REMOVE_IGNORED);
    /// Update existing files without creating or deleting files.
    pub const UPDATE_ONLY: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_UPDATE_ONLY);
    /// Do not update index entries while checking out files.
    pub const DONT_UPDATE_INDEX: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DONT_UPDATE_INDEX);
    /// Do not refresh the index, configuration, or attributes first.
    pub const NO_REFRESH: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_NO_REFRESH);
    /// Skip files with unmerged index entries.
    pub const SKIP_UNMERGED: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_SKIP_UNMERGED);
    /// Resolve unmerged files from the index's "ours" stage.
    pub const USE_OURS: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_USE_OURS);
    /// Resolve unmerged files from the index's "theirs" stage.
    pub const USE_THEIRS: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_USE_THEIRS);
    /// Treat pathspec entries as exact paths instead of patterns.
    pub const DISABLE_PATHSPEC_MATCH: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DISABLE_PATHSPEC_MATCH);
    /// Recursively checkout submodules (not implemented by this libgit2).
    pub const UPDATE_SUBMODULES: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_UPDATE_SUBMODULES);
    /// Recursively checkout changed submodules (not implemented by this libgit2).
    pub const UPDATE_SUBMODULES_IF_CHANGED: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_UPDATE_SUBMODULES_IF_CHANGED);
    /// Leave locked directories empty instead of failing.
    pub const SKIP_LOCKED_DIRECTORIES: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_SKIP_LOCKED_DIRECTORIES);
    /// Do not overwrite ignored files that exist in the checkout target.
    pub const DONT_OVERWRITE_IGNORED: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DONT_OVERWRITE_IGNORED);
    /// Write ordinary merge-style conflict files.
    pub const CONFLICT_STYLE_MERGE: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_CONFLICT_STYLE_MERGE);
    /// Write diff3-style conflict files with common-ancestor data.
    pub const CONFLICT_STYLE_DIFF3: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_CONFLICT_STYLE_DIFF3);
    /// Do not remove existing paths that collide on case-insensitive filesystems.
    pub const DONT_REMOVE_EXISTING: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DONT_REMOVE_EXISTING);
    /// Do not write the index when checkout completes.
    pub const DONT_WRITE_INDEX: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DONT_WRITE_INDEX);
    /// Report the actions checkout would take without changing files or index.
    pub const DRY_RUN: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DRY_RUN);
    /// Write zdiff3-style conflict files with common-ancestor data.
    pub const CONFLICT_STYLE_ZDIFF3: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_CONFLICT_STYLE_ZDIFF3);
    /// Suppress checkout and checkout callbacks completely.
    pub const NONE: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_NONE);
    /// Every strategy bit published by this libgit2 API.
    pub const ALL: Self = Self(
        Self::FORCE.0
            | Self::RECREATE_MISSING.0
            | Self::ALLOW_CONFLICTS.0
            | Self::REMOVE_UNTRACKED.0
            | Self::REMOVE_IGNORED.0
            | Self::UPDATE_ONLY.0
            | Self::DONT_UPDATE_INDEX.0
            | Self::NO_REFRESH.0
            | Self::SKIP_UNMERGED.0
            | Self::USE_OURS.0
            | Self::USE_THEIRS.0
            | Self::DISABLE_PATHSPEC_MATCH.0
            | Self::UPDATE_SUBMODULES.0
            | Self::UPDATE_SUBMODULES_IF_CHANGED.0
            | Self::SKIP_LOCKED_DIRECTORIES.0
            | Self::DONT_OVERWRITE_IGNORED.0
            | Self::CONFLICT_STYLE_MERGE.0
            | Self::CONFLICT_STYLE_DIFF3.0
            | Self::DONT_REMOVE_EXISTING.0
            | Self::DONT_WRITE_INDEX.0
            | Self::DRY_RUN.0
            | Self::CONFLICT_STYLE_ZDIFF3.0
            | Self::NONE.0,
    );

    /// Retain all bits from a raw libgit2 value, including unknown bits.
    #[inline]
    #[must_use]
    pub const fn from_bits_retain(bits: ffi::git_checkout_strategy_t) -> Self {
        Self(bits)
    }

    /// Return the raw libgit2 strategy bits.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> ffi::git_checkout_strategy_t {
        self.0
    }

    /// Return whether every strategy in `other` is selected.
    #[inline]
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return whether at least one strategy in `other` is selected.
    #[inline]
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Return whether this is the default safe strategy with no modifier bits.
    #[inline]
    #[must_use]
    pub const fn is_safe(self) -> bool {
        self.0 == Self::SAFE.0
    }
}

impl From<ffi::git_checkout_strategy_t> for GitCheckoutStrategy {
    fn from(bits: ffi::git_checkout_strategy_t) -> Self {
        Self::from_bits_retain(bits)
    }
}

impl From<GitCheckoutStrategy> for ffi::git_checkout_strategy_t {
    fn from(strategy: GitCheckoutStrategy) -> Self {
        strategy.bits()
    }
}

impl BitOr for GitCheckoutStrategy {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitCheckoutStrategy {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitCheckoutStrategy {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitCheckoutStrategy {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitCheckoutStrategy {
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
    fn checkout_strategies_combine_and_clear() {
        let mut strategy = GitCheckoutStrategy::FORCE | GitCheckoutStrategy::REMOVE_UNTRACKED;
        assert!(strategy.contains(GitCheckoutStrategy::FORCE));
        assert!(strategy.intersects(GitCheckoutStrategy::REMOVE_UNTRACKED));
        assert!(!strategy.is_safe());

        strategy &= !GitCheckoutStrategy::FORCE;
        assert!(!strategy.contains(GitCheckoutStrategy::FORCE));
        assert!(strategy.contains(GitCheckoutStrategy::REMOVE_UNTRACKED));
        assert!(GitCheckoutStrategy::SAFE.is_safe());
    }

    #[test]
    fn checkout_strategies_retain_raw_bits_and_match_the_c_layout() {
        let unknown = GitCheckoutStrategy::from_bits_retain(1 << 29);
        assert_eq!(unknown.bits(), 1 << 29);
        assert_eq!(ffi::git_checkout_strategy_t::from(unknown), 1 << 29);
        assert_eq!(
            size_of::<GitCheckoutStrategy>(),
            size_of::<ffi::git_checkout_strategy_t>()
        );
        assert_eq!(
            align_of::<GitCheckoutStrategy>(),
            align_of::<ffi::git_checkout_strategy_t>()
        );
    }

    #[test]
    fn all_published_checkout_bits_are_accounted_for() {
        for strategy in [
            GitCheckoutStrategy::FORCE,
            GitCheckoutStrategy::RECREATE_MISSING,
            GitCheckoutStrategy::ALLOW_CONFLICTS,
            GitCheckoutStrategy::REMOVE_UNTRACKED,
            GitCheckoutStrategy::REMOVE_IGNORED,
            GitCheckoutStrategy::UPDATE_ONLY,
            GitCheckoutStrategy::DONT_UPDATE_INDEX,
            GitCheckoutStrategy::NO_REFRESH,
            GitCheckoutStrategy::SKIP_UNMERGED,
            GitCheckoutStrategy::USE_OURS,
            GitCheckoutStrategy::USE_THEIRS,
            GitCheckoutStrategy::DISABLE_PATHSPEC_MATCH,
            GitCheckoutStrategy::UPDATE_SUBMODULES,
            GitCheckoutStrategy::UPDATE_SUBMODULES_IF_CHANGED,
            GitCheckoutStrategy::SKIP_LOCKED_DIRECTORIES,
            GitCheckoutStrategy::DONT_OVERWRITE_IGNORED,
            GitCheckoutStrategy::CONFLICT_STYLE_MERGE,
            GitCheckoutStrategy::CONFLICT_STYLE_DIFF3,
            GitCheckoutStrategy::DONT_REMOVE_EXISTING,
            GitCheckoutStrategy::DONT_WRITE_INDEX,
            GitCheckoutStrategy::DRY_RUN,
            GitCheckoutStrategy::CONFLICT_STYLE_ZDIFF3,
            GitCheckoutStrategy::NONE,
        ] {
            assert!(GitCheckoutStrategy::ALL.contains(strategy));
        }
    }
}
