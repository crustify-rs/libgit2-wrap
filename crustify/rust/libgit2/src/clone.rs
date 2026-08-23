//! Safe wrappers for libgit2 clone APIs.

use crate::ffi;

/// Wraps: git_clone_local_t
/// Controls whether cloning from a local source bypasses the Git transport.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum GitCloneLocal {
    /// Auto-detect local paths, but use the Git transport for `file://` URLs.
    #[default]
    Auto = ffi::git_clone_local_t_GIT_CLONE_LOCAL_AUTO,
    /// Always bypass the Git transport for local sources.
    Local = ffi::git_clone_local_t_GIT_CLONE_LOCAL,
    /// Never bypass the Git transport.
    NoLocal = ffi::git_clone_local_t_GIT_CLONE_NO_LOCAL,
    /// Bypass the Git transport without creating hardlinks.
    NoLinks = ffi::git_clone_local_t_GIT_CLONE_LOCAL_NO_LINKS,
}

impl From<GitCloneLocal> for ffi::git_clone_local_t {
    fn from(value: GitCloneLocal) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_clone_local_t> for GitCloneLocal {
    type Error = ffi::git_clone_local_t;

    fn try_from(value: ffi::git_clone_local_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_clone_local_t_GIT_CLONE_LOCAL_AUTO => Ok(Self::Auto),
            ffi::git_clone_local_t_GIT_CLONE_LOCAL => Ok(Self::Local),
            ffi::git_clone_local_t_GIT_CLONE_NO_LOCAL => Ok(Self::NoLocal),
            ffi::git_clone_local_t_GIT_CLONE_LOCAL_NO_LINKS => Ok(Self::NoLinks),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn clone_local_has_the_c_layout_and_values() {
        assert_eq!(
            size_of::<GitCloneLocal>(),
            size_of::<ffi::git_clone_local_t>()
        );
        assert_eq!(
            align_of::<GitCloneLocal>(),
            align_of::<ffi::git_clone_local_t>()
        );
        assert_eq!(ffi::git_clone_local_t::from(GitCloneLocal::Auto), 0);
        assert_eq!(ffi::git_clone_local_t::from(GitCloneLocal::NoLinks), 3);
    }

    #[test]
    fn clone_local_validates_raw_values() {
        assert_eq!(GitCloneLocal::try_from(2), Ok(GitCloneLocal::NoLocal));
        assert_eq!(GitCloneLocal::try_from(4), Err(4));
    }
}
