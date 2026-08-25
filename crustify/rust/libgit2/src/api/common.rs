//! Safe wrappers for libgit2 common APIs.

use crate::ffi;

/// Wraps: git_buildinfo_t
/// A checked kind of compile-time build information exposed by libgit2.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitBuildInfo {
    /// The CPU architecture for which libgit2 was built.
    Cpu = ffi::git_buildinfo_t_GIT_BUILDINFO_CPU,
    /// The source commit from which libgit2 was built.
    Commit = ffi::git_buildinfo_t_GIT_BUILDINFO_COMMIT,
}

/// A raw build-information kind not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitBuildInfo(ffi::git_buildinfo_t);

impl InvalidGitBuildInfo {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_buildinfo_t {
        self.0
    }
}

impl From<GitBuildInfo> for ffi::git_buildinfo_t {
    fn from(info: GitBuildInfo) -> Self {
        info as Self
    }
}

impl TryFrom<ffi::git_buildinfo_t> for GitBuildInfo {
    type Error = InvalidGitBuildInfo;

    fn try_from(info: ffi::git_buildinfo_t) -> Result<Self, Self::Error> {
        match info {
            ffi::git_buildinfo_t_GIT_BUILDINFO_CPU => Ok(Self::Cpu),
            ffi::git_buildinfo_t_GIT_BUILDINFO_COMMIT => Ok(Self::Commit),
            value => Err(InvalidGitBuildInfo(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_build_information_kinds_round_trip() {
        for info in [GitBuildInfo::Cpu, GitBuildInfo::Commit] {
            let raw = ffi::git_buildinfo_t::from(info);
            assert_eq!(GitBuildInfo::try_from(raw), Ok(info));
        }
    }

    #[test]
    fn unknown_build_information_kind_is_rejected() {
        let raw = ffi::git_buildinfo_t_GIT_BUILDINFO_COMMIT + 1;
        assert_eq!(GitBuildInfo::try_from(raw).unwrap_err().value(), raw);
    }

    #[test]
    fn build_information_kind_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<GitBuildInfo>(), size_of::<ffi::git_buildinfo_t>());
        assert_eq!(
            align_of::<GitBuildInfo>(),
            align_of::<ffi::git_buildinfo_t>()
        );
    }
}
