//! Safe wrappers for libgit2 describe APIs.

use crate::ffi;

/// Wraps: git_describe_strategy_t
/// A checked strategy selecting which references `git describe` searches.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitDescribeStrategy(ffi::git_describe_strategy_t);

impl GitDescribeStrategy {
    /// Search annotated tags only.
    pub const DEFAULT: Self = Self(ffi::git_describe_strategy_t_GIT_DESCRIBE_DEFAULT);
    /// Search every tag under `refs/tags/`.
    pub const TAGS: Self = Self(ffi::git_describe_strategy_t_GIT_DESCRIBE_TAGS);
    /// Search every reference under `refs/`.
    pub const ALL: Self = Self(ffi::git_describe_strategy_t_GIT_DESCRIBE_ALL);

    /// Converts a raw value when it is a published strategy.
    #[must_use]
    pub const fn from_raw(raw: ffi::git_describe_strategy_t) -> Option<Self> {
        match raw {
            ffi::git_describe_strategy_t_GIT_DESCRIBE_DEFAULT => Some(Self::DEFAULT),
            ffi::git_describe_strategy_t_GIT_DESCRIBE_TAGS => Some(Self::TAGS),
            ffi::git_describe_strategy_t_GIT_DESCRIBE_ALL => Some(Self::ALL),
            _ => None,
        }
    }

    /// Returns the underlying libgit2 value.
    #[must_use]
    pub const fn as_raw(self) -> ffi::git_describe_strategy_t {
        self.0
    }
}

impl From<GitDescribeStrategy> for ffi::git_describe_strategy_t {
    fn from(strategy: GitDescribeStrategy) -> Self {
        strategy.as_raw()
    }
}

impl TryFrom<ffi::git_describe_strategy_t> for GitDescribeStrategy {
    type Error = ffi::git_describe_strategy_t;

    fn try_from(raw: ffi::git_describe_strategy_t) -> Result<Self, Self::Error> {
        Self::from_raw(raw).ok_or(raw)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_describe_strategies_round_trip() {
        for strategy in [
            GitDescribeStrategy::DEFAULT,
            GitDescribeStrategy::TAGS,
            GitDescribeStrategy::ALL,
        ] {
            assert_eq!(
                GitDescribeStrategy::from_raw(strategy.as_raw()),
                Some(strategy)
            );
            assert_eq!(
                GitDescribeStrategy::try_from(strategy.as_raw()),
                Ok(strategy)
            );
        }
        assert_eq!(GitDescribeStrategy::from_raw(3), None);
    }

    #[test]
    fn describe_strategy_preserves_the_c_layout() {
        assert_eq!(
            size_of::<GitDescribeStrategy>(),
            size_of::<ffi::git_describe_strategy_t>()
        );
        assert_eq!(
            align_of::<GitDescribeStrategy>(),
            align_of::<ffi::git_describe_strategy_t>()
        );
    }
}
