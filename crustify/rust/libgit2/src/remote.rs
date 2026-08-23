//! Safe wrappers for libgit2 remote APIs.

use crate::ffi;

/// Wraps: git_fetch_prune_t
/// Controls whether a fetch prunes remote-tracking references.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum GitFetchPrune {
    /// Use the repository or remote configuration.
    #[default]
    Unspecified = ffi::git_fetch_prune_t_GIT_FETCH_PRUNE_UNSPECIFIED,
    /// Force pruning on.
    Prune = ffi::git_fetch_prune_t_GIT_FETCH_PRUNE,
    /// Force pruning off.
    NoPrune = ffi::git_fetch_prune_t_GIT_FETCH_NO_PRUNE,
}

impl From<GitFetchPrune> for ffi::git_fetch_prune_t {
    fn from(value: GitFetchPrune) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_fetch_prune_t> for GitFetchPrune {
    type Error = ffi::git_fetch_prune_t;

    fn try_from(value: ffi::git_fetch_prune_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_fetch_prune_t_GIT_FETCH_PRUNE_UNSPECIFIED => Ok(Self::Unspecified),
            ffi::git_fetch_prune_t_GIT_FETCH_PRUNE => Ok(Self::Prune),
            ffi::git_fetch_prune_t_GIT_FETCH_NO_PRUNE => Ok(Self::NoPrune),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn fetch_prune_has_the_c_layout_and_values() {
        assert_eq!(
            size_of::<GitFetchPrune>(),
            size_of::<ffi::git_fetch_prune_t>()
        );
        assert_eq!(
            align_of::<GitFetchPrune>(),
            align_of::<ffi::git_fetch_prune_t>()
        );
        assert_eq!(ffi::git_fetch_prune_t::from(GitFetchPrune::Unspecified), 0);
        assert_eq!(ffi::git_fetch_prune_t::from(GitFetchPrune::Prune), 1);
        assert_eq!(ffi::git_fetch_prune_t::from(GitFetchPrune::NoPrune), 2);
    }

    #[test]
    fn fetch_prune_validates_raw_values() {
        assert_eq!(GitFetchPrune::try_from(0), Ok(GitFetchPrune::Unspecified));
        assert_eq!(GitFetchPrune::try_from(2), Ok(GitFetchPrune::NoPrune));
        assert_eq!(GitFetchPrune::try_from(3), Err(3));
    }
}
