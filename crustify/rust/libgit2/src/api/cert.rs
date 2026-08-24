//! Safe wrappers for libgit2 cert APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

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
    /// Field: git_cert.cert_type
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

/// Wraps: git_cert_ssh_t
/// Available SSH host-key fingerprint representations.
///
/// This is a layout-compatible bit set. Raw values retain unknown bits so a
/// certificate produced by a newer libgit2 version can still be inspected and
/// passed through safely.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitCertSsh(ffi::git_cert_ssh_t);

impl GitCertSsh {
    /// No host-key representation is available.
    pub const EMPTY: Self = Self(0);
    /// An MD5 fingerprint is available.
    pub const MD5: Self = Self(ffi::git_cert_ssh_t_GIT_CERT_SSH_MD5);
    /// A SHA-1 fingerprint is available.
    pub const SHA1: Self = Self(ffi::git_cert_ssh_t_GIT_CERT_SSH_SHA1);
    /// A SHA-256 fingerprint is available.
    pub const SHA256: Self = Self(ffi::git_cert_ssh_t_GIT_CERT_SSH_SHA256);
    /// The raw host key is available.
    pub const RAW: Self = Self(ffi::git_cert_ssh_t_GIT_CERT_SSH_RAW);
    /// Every representation published by this libgit2 API.
    pub const ALL: Self = Self(Self::MD5.0 | Self::SHA1.0 | Self::SHA256.0 | Self::RAW.0);

    /// Retain all bits from a raw libgit2 value, including unknown bits.
    #[inline]
    #[must_use]
    pub const fn from_bits_retain(bits: ffi::git_cert_ssh_t) -> Self {
        Self(bits)
    }

    /// Return the raw libgit2 flag bits.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> ffi::git_cert_ssh_t {
        self.0
    }

    /// Return whether every representation in `other` is available.
    #[inline]
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return whether at least one representation in `other` is available.
    #[inline]
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Return whether no representation bits are set.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl From<ffi::git_cert_ssh_t> for GitCertSsh {
    fn from(bits: ffi::git_cert_ssh_t) -> Self {
        Self::from_bits_retain(bits)
    }
}

impl From<GitCertSsh> for ffi::git_cert_ssh_t {
    fn from(flags: GitCertSsh) -> Self {
        flags.bits()
    }
}

impl BitOr for GitCertSsh {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitCertSsh {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitCertSsh {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitCertSsh {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitCertSsh {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

#[cfg(test)]
mod ssh_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn ssh_representations_form_bit_sets() {
        let mut available = GitCertSsh::MD5 | GitCertSsh::SHA256;
        assert!(available.contains(GitCertSsh::MD5));
        assert!(available.intersects(GitCertSsh::SHA256));
        assert!(!available.intersects(GitCertSsh::RAW));

        available |= GitCertSsh::RAW;
        available &= !GitCertSsh::MD5;
        assert!(!available.contains(GitCertSsh::MD5));
        assert!(available.contains(GitCertSsh::RAW));
    }

    #[test]
    fn ssh_representations_retain_raw_bits_and_match_the_c_layout() {
        let unknown = GitCertSsh::from_bits_retain(1 << 10);
        assert_eq!(unknown.bits(), 1 << 10);
        assert_eq!(ffi::git_cert_ssh_t::from(unknown), 1 << 10);
        assert_eq!(size_of::<GitCertSsh>(), size_of::<ffi::git_cert_ssh_t>());
        assert_eq!(align_of::<GitCertSsh>(), align_of::<ffi::git_cert_ssh_t>());
    }
}

/// Wraps: git_cert_ssh_raw_type_t
/// The algorithm used by a raw SSH host key.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitCertSshRawType {
    /// The host-key algorithm is unknown.
    Unknown = ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_UNKNOWN,
    /// RSA.
    Rsa = ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_RSA,
    /// DSS.
    Dss = ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_DSS,
    /// ECDSA over the NIST P-256 curve.
    Ecdsa256 = ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ECDSA_256,
    /// ECDSA over the NIST P-384 curve.
    Ecdsa384 = ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ECDSA_384,
    /// ECDSA over the NIST P-521 curve.
    Ecdsa521 = ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ECDSA_521,
    /// Ed25519.
    Ed25519 = ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ED25519,
}

/// A raw SSH host-key type not published by libgit2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitCertSshRawType(ffi::git_cert_ssh_raw_type_t);

impl InvalidGitCertSshRawType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_cert_ssh_raw_type_t {
        self.0
    }
}

impl From<GitCertSshRawType> for ffi::git_cert_ssh_raw_type_t {
    fn from(value: GitCertSshRawType) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_cert_ssh_raw_type_t> for GitCertSshRawType {
    type Error = InvalidGitCertSshRawType;

    fn try_from(value: ffi::git_cert_ssh_raw_type_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_UNKNOWN => Ok(Self::Unknown),
            ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_RSA => Ok(Self::Rsa),
            ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_DSS => Ok(Self::Dss),
            ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ECDSA_256 => Ok(Self::Ecdsa256),
            ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ECDSA_384 => Ok(Self::Ecdsa384),
            ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ECDSA_521 => Ok(Self::Ecdsa521),
            ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ED25519 => Ok(Self::Ed25519),
            value => Err(InvalidGitCertSshRawType(value)),
        }
    }
}

#[cfg(test)]
mod ssh_raw_type_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_ssh_raw_types_round_trip() {
        for raw_type in [
            GitCertSshRawType::Unknown,
            GitCertSshRawType::Rsa,
            GitCertSshRawType::Dss,
            GitCertSshRawType::Ecdsa256,
            GitCertSshRawType::Ecdsa384,
            GitCertSshRawType::Ecdsa521,
            GitCertSshRawType::Ed25519,
        ] {
            let raw = ffi::git_cert_ssh_raw_type_t::from(raw_type);
            assert_eq!(GitCertSshRawType::try_from(raw), Ok(raw_type));
        }
    }

    #[test]
    fn unknown_ssh_raw_type_is_rejected() {
        let unknown = ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ED25519 + 1;
        let error = GitCertSshRawType::try_from(unknown).unwrap_err();
        assert_eq!(error.value(), unknown);
    }

    #[test]
    fn ssh_raw_type_preserves_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitCertSshRawType>(),
            size_of::<ffi::git_cert_ssh_raw_type_t>()
        );
        assert_eq!(
            align_of::<GitCertSshRawType>(),
            align_of::<ffi::git_cert_ssh_raw_type_t>()
        );
    }
}
