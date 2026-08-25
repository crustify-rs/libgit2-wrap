//! Safe wrappers for libgit2 common APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

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

/// Wraps: git_feature_t
/// A set of compile-time features supported by libgit2.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitFeatureFlags(ffi::git_feature_t);

impl GitFeatureFlags {
    /// No optional or built-in feature bits.
    pub const NONE: Self = Self(0);
    /// Thread-safe operation.
    pub const THREADS: Self = Self(ffi::git_feature_t_GIT_FEATURE_THREADS);
    /// HTTPS remotes.
    pub const HTTPS: Self = Self(ffi::git_feature_t_GIT_FEATURE_HTTPS);
    /// SSH remotes.
    pub const SSH: Self = Self(ffi::git_feature_t_GIT_FEATURE_SSH);
    /// Sub-second index timestamp resolution.
    pub const NSEC: Self = Self(ffi::git_feature_t_GIT_FEATURE_NSEC);
    /// HTTP parsing support.
    pub const HTTP_PARSER: Self = Self(ffi::git_feature_t_GIT_FEATURE_HTTP_PARSER);
    /// Regular-expression support.
    pub const REGEX: Self = Self(ffi::git_feature_t_GIT_FEATURE_REGEX);
    /// Filename internationalization support.
    pub const I18N: Self = Self(ffi::git_feature_t_GIT_FEATURE_I18N);
    /// NTLM authentication.
    pub const AUTH_NTLM: Self = Self(ffi::git_feature_t_GIT_FEATURE_AUTH_NTLM);
    /// Negotiate (SPNEGO) authentication.
    pub const AUTH_NEGOTIATE: Self = Self(ffi::git_feature_t_GIT_FEATURE_AUTH_NEGOTIATE);
    /// Compression support.
    pub const COMPRESSION: Self = Self(ffi::git_feature_t_GIT_FEATURE_COMPRESSION);
    /// SHA-1 object support.
    pub const SHA1: Self = Self(ffi::git_feature_t_GIT_FEATURE_SHA1);
    /// SHA-256 object support.
    pub const SHA256: Self = Self(ffi::git_feature_t_GIT_FEATURE_SHA256);
    /// Plain HTTP remotes.
    pub const HTTP: Self = Self(ffi::git_feature_t_GIT_FEATURE_HTTP);
    /// Every feature bit published by these bindings.
    pub const ALL: Self = Self(
        Self::THREADS.0
            | Self::HTTPS.0
            | Self::SSH.0
            | Self::NSEC.0
            | Self::HTTP_PARSER.0
            | Self::REGEX.0
            | Self::I18N.0
            | Self::AUTH_NTLM.0
            | Self::AUTH_NEGOTIATE.0
            | Self::COMPRESSION.0
            | Self::SHA1.0
            | Self::SHA256.0
            | Self::HTTP.0,
    );

    /// Converts raw bits when every bit is published by these bindings.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_feature_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Retains all raw bits, including features added by a newer linked library.
    #[must_use]
    pub const fn from_bits_retain(bits: ffi::git_feature_t) -> Self {
        Self(bits)
    }

    /// Returns the underlying libgit2 feature bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_feature_t {
        self.0
    }

    /// Returns whether no feature bit is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every feature in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any feature in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitFeatureFlags> for ffi::git_feature_t {
    fn from(flags: GitFeatureFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_feature_t> for GitFeatureFlags {
    type Error = ffi::git_feature_t;

    fn try_from(bits: ffi::git_feature_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitFeatureFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitFeatureFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitFeatureFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitFeatureFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitFeatureFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod feature_flag_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn feature_sets_combine_and_validate() {
        let features = GitFeatureFlags::THREADS | GitFeatureFlags::HTTP;
        assert!(features.contains(GitFeatureFlags::THREADS));
        assert!(features.intersects(GitFeatureFlags::HTTP));
        assert_eq!(GitFeatureFlags::from_bits(features.bits()), Some(features));
        assert!(GitFeatureFlags::NONE.is_empty());

        let unknown = GitFeatureFlags::ALL.bits() << 1;
        assert_eq!(GitFeatureFlags::from_bits(unknown), None);
        assert_eq!(GitFeatureFlags::from_bits_retain(unknown).bits(), unknown);
    }

    #[test]
    fn feature_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitFeatureFlags>(),
            size_of::<ffi::git_feature_t>()
        );
        assert_eq!(
            align_of::<GitFeatureFlags>(),
            align_of::<ffi::git_feature_t>()
        );
    }
}
