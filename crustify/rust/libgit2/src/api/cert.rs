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

ffibox::define_ctype!(
    /// Wraps: git_cert
    /// Layout-compatible base certificate passed to transport callbacks.
    GitCert,
    GitCertRef,
    GitCertMut,
    ffi::git_cert
);

impl GitCertRef<'_> {
    /// Wraps: git_cert.cert_type
    /// Returns the certificate kind after validating the value supplied by C.
    #[inline]
    pub fn cert_type(&self) -> Result<GitCertType, InvalidGitCertType> {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        let raw = unsafe { core::ptr::addr_of!((*ptr).cert_type).read() };
        GitCertType::try_from(raw)
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

    #[test]
    fn certificate_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<GitCert>(), size_of::<ffi::git_cert>());
        assert_eq!(align_of::<GitCert>(), align_of::<ffi::git_cert>());
        assert_eq!(size_of::<GitCertRef<'_>>(), size_of::<*mut ffi::git_cert>());
        assert_eq!(size_of::<GitCertMut<'_>>(), size_of::<*mut ffi::git_cert>());
    }

    #[test]
    fn borrowed_certificate_handles_read_the_kind() {
        let mut raw = ffi::git_cert {
            cert_type: ffi::git_cert_t_GIT_CERT_X509,
        };

        // SAFETY: `raw` is initialized, remains live, and is not mutated while
        // the shared handle is used.
        let cert = unsafe { GitCertRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(cert.cert_type(), Ok(GitCertType::X509));
    }

    #[test]
    fn borrowed_certificate_rejects_an_unknown_kind() {
        let invalid = ffi::git_cert_t_GIT_CERT_STRARRAY + 1;
        let mut raw = ffi::git_cert { cert_type: invalid };

        // SAFETY: `raw` is initialized, remains live, and is not mutated while
        // the shared handle is used.
        let cert = unsafe { GitCertRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(cert.cert_type().unwrap_err().value(), invalid);
    }

    #[test]
    fn an_exclusive_certificate_handle_reaches_the_shared_getter() {
        let mut raw = ffi::git_cert {
            cert_type: ffi::git_cert_t_GIT_CERT_HOSTKEY_LIBSSH2,
        };

        // SAFETY: `raw` is initialized, remains live, and this exclusive
        // handle is the only one addressing it.
        let mut cert = unsafe { GitCertMut::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(cert.as_mut_ptr().cast_const(), cert.as_ref().as_ptr());
        assert_eq!(cert.as_ref().cert_type(), Ok(GitCertType::HostkeyLibssh2));
    }
}
