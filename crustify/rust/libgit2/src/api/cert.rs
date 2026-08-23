//! Safe wrappers for libgit2 cert APIs.

use crate::ffi;

/// Wraps: git_cert_t
/// Kind of certificate data presented to a transport certificate callback.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitCertType {
    /// No certificate information is available.
    None = ffi::git_cert_t_GIT_CERT_NONE,
    /// DER-encoded X.509 certificate data.
    X509 = ffi::git_cert_t_GIT_CERT_X509,
    /// SSH host-key information from libssh2.
    HostkeyLibssh2 = ffi::git_cert_t_GIT_CERT_HOSTKEY_LIBSSH2,
    /// A `git_strarray` of certificate information.
    Strarray = ffi::git_cert_t_GIT_CERT_STRARRAY,
}

/// A raw certificate type not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitCertType(ffi::git_cert_t);

impl InvalidGitCertType {
    /// Return the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_cert_t {
        self.0
    }
}

impl From<GitCertType> for ffi::git_cert_t {
    fn from(value: GitCertType) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_cert_t> for GitCertType {
    type Error = InvalidGitCertType;

    fn try_from(value: ffi::git_cert_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_cert_t_GIT_CERT_NONE => Ok(Self::None),
            ffi::git_cert_t_GIT_CERT_X509 => Ok(Self::X509),
            ffi::git_cert_t_GIT_CERT_HOSTKEY_LIBSSH2 => Ok(Self::HostkeyLibssh2),
            ffi::git_cert_t_GIT_CERT_STRARRAY => Ok(Self::Strarray),
            value => Err(InvalidGitCertType(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn certificate_types_round_trip_through_the_c_type() {
        for cert_type in [
            GitCertType::None,
            GitCertType::X509,
            GitCertType::HostkeyLibssh2,
            GitCertType::Strarray,
        ] {
            let raw = ffi::git_cert_t::from(cert_type);
            assert_eq!(GitCertType::try_from(raw), Ok(cert_type));
        }
    }

    #[test]
    fn invalid_certificate_type_is_rejected_without_constructing_an_enum() {
        let invalid = ffi::git_cert_t_GIT_CERT_STRARRAY + 1;
        let error = GitCertType::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn certificate_type_matches_the_c_abi_scalar() {
        assert_eq!(size_of::<GitCertType>(), size_of::<ffi::git_cert_t>());
        assert_eq!(align_of::<GitCertType>(), align_of::<ffi::git_cert_t>());
    }
}
