//! Safe wrappers for libgit2 merge APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_merge_analysis_t
/// A layout-compatible set of merge opportunities reported by libgit2.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitMergeAnalysis(ffi::git_merge_analysis_t);

impl GitMergeAnalysis {
    /// No merge is possible.
    pub const NONE: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_NONE);
    /// The inputs and `HEAD` have diverged and require a normal merge.
    pub const NORMAL: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_NORMAL);
    /// Every input is already reachable from `HEAD`.
    pub const UP_TO_DATE: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_UP_TO_DATE);
    /// The input can be checked out as a fast-forward.
    pub const FASTFORWARD: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_FASTFORWARD);
    /// `HEAD` is unborn and can be set directly to the target.
    pub const UNBORN: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_UNBORN);
    /// Every merge-analysis bit published by this version of libgit2.
    pub const ALL: Self =
        Self(Self::NORMAL.0 | Self::UP_TO_DATE.0 | Self::FASTFORWARD.0 | Self::UNBORN.0);

    /// Converts raw bits when they contain only published opportunities.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_merge_analysis_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_merge_analysis_t {
        self.0
    }

    /// Returns whether no merge opportunity is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every opportunity in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any opportunity in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitMergeAnalysis> for ffi::git_merge_analysis_t {
    fn from(analysis: GitMergeAnalysis) -> Self {
        analysis.bits()
    }
}

impl TryFrom<ffi::git_merge_analysis_t> for GitMergeAnalysis {
    type Error = ffi::git_merge_analysis_t;

    fn try_from(bits: ffi::git_merge_analysis_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitMergeAnalysis {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitMergeAnalysis {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitMergeAnalysis {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitMergeAnalysis {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitMergeAnalysis {
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
    fn analysis_flags_form_valid_combinations() {
        let mut analysis = GitMergeAnalysis::NORMAL;
        analysis |= GitMergeAnalysis::FASTFORWARD;

        assert!(analysis.contains(GitMergeAnalysis::NORMAL));
        assert!(analysis.intersects(GitMergeAnalysis::FASTFORWARD));
        assert!(!analysis.intersects(GitMergeAnalysis::UNBORN));
        assert_eq!(GitMergeAnalysis::from_bits(analysis.bits()), Some(analysis));
        assert!(GitMergeAnalysis::NONE.is_empty());
    }

    #[test]
    fn unknown_analysis_bits_are_rejected() {
        let unknown = GitMergeAnalysis::ALL.bits() << 1;
        assert_eq!(GitMergeAnalysis::from_bits(unknown), None);
        assert_eq!(GitMergeAnalysis::try_from(unknown), Err(unknown));
    }

    #[test]
    fn analysis_flags_match_the_c_abi_scalar() {
        assert_eq!(
            size_of::<GitMergeAnalysis>(),
            size_of::<ffi::git_merge_analysis_t>()
        );
        assert_eq!(
            align_of::<GitMergeAnalysis>(),
            align_of::<ffi::git_merge_analysis_t>()
        );
    }
}
