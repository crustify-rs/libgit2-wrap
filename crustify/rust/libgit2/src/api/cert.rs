//! Safe wrappers for libgit2 cert APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::CSlice;

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

/// Wraps: git_transport_certificate_check_cb
/// Safe callable surface for a transport's final certificate decision.
pub trait GitTransportCertificateCheckCallback {
    /// Returns zero to accept, a negative error to reject, or a positive value
    /// to defer to libgit2's existing validity determination.
    fn call(&mut self, certificate: GitCertRef<'_>, valid: bool, host: &core::ffi::CStr) -> i32;
}

impl<F> GitTransportCertificateCheckCallback for F
where
    F: for<'cert, 'host> FnMut(GitCertRef<'cert>, bool, &'host core::ffi::CStr) -> i32,
{
    fn call(&mut self, certificate: GitCertRef<'_>, valid: bool, host: &core::ffi::CStr) -> i32 {
        self(certificate, valid, host)
    }
}

#[cfg(test)]
mod certificate_callback_tests {
    use super::*;

    #[test]
    fn callback_receives_checked_certificate_data() {
        let mut raw = ffi::git_cert {
            cert_type: ffi::git_cert_t_GIT_CERT_X509,
        };
        // SAFETY: `raw` is initialized and remains live without mutation for
        // the callback invocation.
        let certificate = unsafe { GitCertRef::from_ptr(&raw mut raw) }.unwrap();
        let mut callback = |certificate: GitCertRef<'_>, valid: bool, host: &core::ffi::CStr| {
            assert_eq!(certificate.cert_type(), Ok(GitCertType::X509));
            assert!(valid);
            assert_eq!(host, c"example.com");
            1
        };
        assert_eq!(
            GitTransportCertificateCheckCallback::call(
                &mut callback,
                certificate,
                true,
                c"example.com",
            ),
            1
        );
    }
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
mod unit_tests {
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

ffibox::define_ctype!(
    /// Wraps: git_cert_x509
    /// Layout-compatible X.509 certificate view passed to transport callbacks.
    GitCertX509,
    GitCertX509Ref,
    GitCertX509Mut,
    ffi::git_cert_x509
);

impl<'a> GitCertX509Ref<'a> {
    /// Field: git_cert_x509.parent
    /// Borrows the embedded certificate header.
    #[must_use]
    pub fn parent(&self) -> GitCertRef<'a> {
        // SAFETY: raw-place projection reaches the live initialized header
        // without forming a reference over the C-visible certificate.
        let parent = unsafe { addr_of!((*self.as_ptr()).parent) }.cast_mut();
        // SAFETY: an inline field is non-null and remains live for this
        // certificate handle's complete `'a` borrow.
        unsafe { GitCertRef::from_ptr(parent) }.expect("an embedded certificate is non-null")
    }

    /// Field: git_cert_x509.data
    /// Borrows the DER-encoded certificate bytes.
    #[must_use]
    pub fn data(&self) -> CSlice<'a, u8> {
        let cert = self.as_ptr();
        // SAFETY: both members are initialized fields of this live X.509
        // record and raw-place reads form no references to C-visible storage.
        let (data, len) = unsafe {
            (
                addr_of!((*cert).data).read().cast::<u8>(),
                addr_of!((*cert).len).read(),
            )
        };
        let data = NonNull::new(data).expect("an X.509 certificate has DER data");
        // SAFETY: every X.509 producer supplies `len` initialized DER bytes at
        // this non-null pointer and retains them for the record's lifetime.
        unsafe { CSlice::from_raw_parts(data, len) }
    }

    /// Field: git_cert_x509.len
    /// Returns the length of the DER-encoded certificate.
    #[must_use]
    pub fn data_len(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to the C-visible certificate.
        unsafe { addr_of!((*self.as_ptr()).len).read() }
    }
}

impl GitCertX509Mut<'_> {
    /// Borrows the embedded certificate header exclusively.
    #[must_use]
    pub fn parent_mut(&mut self) -> GitCertMut<'_> {
        // SAFETY: raw-place projection reaches the initialized inline header;
        // the returned exclusive handle is tied to this exclusive reborrow.
        unsafe { GitCertMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).parent)) }
            .expect("an embedded certificate is non-null")
    }
}

#[cfg(test)]
mod x509_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn x509_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<GitCertX509>(), size_of::<ffi::git_cert_x509>());
        assert_eq!(align_of::<GitCertX509>(), align_of::<ffi::git_cert_x509>());
        assert_eq!(
            size_of::<GitCertX509Ref<'_>>(),
            size_of::<*const ffi::git_cert_x509>()
        );
        assert_eq!(
            size_of::<GitCertX509Mut<'_>>(),
            size_of::<*mut ffi::git_cert_x509>()
        );
    }

    #[test]
    fn x509_handle_borrows_header_and_der_bytes() {
        let mut der = [0x30, 0x03, 0x02, 0x01, 0x01];
        let mut raw = ffi::git_cert_x509 {
            parent: ffi::git_cert {
                cert_type: ffi::git_cert_t_GIT_CERT_X509,
            },
            data: der.as_mut_ptr().cast(),
            len: der.len(),
        };

        // SAFETY: `raw` and `der` remain live and unchanged while the shared
        // handle and its derived views are used.
        let cert = unsafe { GitCertX509Ref::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(cert.parent().cert_type(), Ok(GitCertType::X509));
        assert_eq!(cert.data_len(), der.len());
        assert_eq!(cert.data().elems().collect::<Vec<_>>(), der);
    }

    #[test]
    fn exclusive_x509_handle_projects_the_inline_header() {
        let mut der = [0x30];
        let mut raw = ffi::git_cert_x509 {
            parent: ffi::git_cert {
                cert_type: ffi::git_cert_t_GIT_CERT_X509,
            },
            data: der.as_mut_ptr().cast(),
            len: der.len(),
        };

        // SAFETY: `raw` and `der` remain live, and this is the only handle
        // addressing the local record.
        let mut cert = unsafe { GitCertX509Mut::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(cert.parent_mut().as_mut_ptr(), addr_of_mut!(raw.parent));
    }
}

ffibox::define_ctype!(
    /// Wraps: git_cert_hostkey
    /// Layout-compatible SSH host-key information borrowed for a certificate callback.
    ///
    /// The raw key bytes are owned by the SSH backend and remain valid only for
    /// the callback invocation that supplied this value.
    GitCertHostkey,
    GitCertHostkeyRef,
    GitCertHostkeyMut,
    ffi::git_cert_hostkey
);

impl<'a> GitCertHostkeyRef<'a> {
    /// Field: git_cert_hostkey.type
    /// Returns the bit set describing which host-key representations are available.
    #[must_use]
    pub fn available(&self) -> GitCertSsh {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let bits = unsafe { core::ptr::addr_of!((*self.as_ptr()).type_).read() };
        GitCertSsh::from(bits)
    }

    /// Field: git_cert_hostkey.parent
    /// Borrows the inline base certificate.
    #[must_use]
    pub fn parent(&self) -> GitCertRef<'a> {
        // SAFETY: raw-place projection obtains the initialized inline field's
        // address without forming a reference to C-visible memory.
        let parent = unsafe { core::ptr::addr_of!((*self.as_ptr()).parent) }.cast_mut();
        // SAFETY: the inline parent is non-null and remains live for this
        // host-key handle's full `'a` borrow.
        unsafe { GitCertRef::from_ptr(parent) }.expect("an inline field is non-null")
    }

    /// Field: git_cert_hostkey.hostkey_len
    /// Returns the raw host key's byte length.
    ///
    /// The value is meaningful when [`GitCertSsh::RAW`] is available.
    #[must_use]
    pub fn hostkey_len(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).hostkey_len).read() }
    }

    /// Field: git_cert_hostkey.hostkey
    /// Borrows the length-delimited raw host key when it is available.
    #[must_use]
    pub fn hostkey(&self) -> Option<ffibox::CSlice<'a, u8>> {
        if !self.available().contains(GitCertSsh::RAW) {
            return None;
        }
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to the host-key header.
        let bytes = unsafe { core::ptr::addr_of!((*self.as_ptr()).hostkey).read() }
            .cast::<u8>()
            .cast_mut();
        let bytes = core::ptr::NonNull::new(bytes)?;
        // SAFETY: with RAW set, libgit2 supplies `hostkey_len` initialized bytes
        // borrowed from the SSH session for this handle's `'a` lifetime.
        Some(unsafe { ffibox::CSlice::from_raw_parts(bytes, self.hostkey_len()) })
    }

    /// Field: git_cert_hostkey.raw_type
    /// Returns the raw host-key algorithm when the raw key is available.
    pub fn raw_type(&self) -> Result<Option<GitCertSshRawType>, InvalidGitCertSshRawType> {
        if !self.available().contains(GitCertSsh::RAW) {
            return Ok(None);
        }
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let raw_type = unsafe { core::ptr::addr_of!((*self.as_ptr()).raw_type).read() };
        GitCertSshRawType::try_from(raw_type).map(Some)
    }

    /// Field: git_cert_hostkey.hash_sha256
    /// Borrows the SHA-256 fingerprint when it is available.
    #[must_use]
    pub fn hash_sha256(&self) -> Option<ffibox::CSlice<'a, u8>> {
        if !self.available().contains(GitCertSsh::SHA256) {
            return None;
        }
        // SAFETY: raw-place projection obtains the initialized inline array's
        // address without forming a reference to C-visible memory.
        let hash = unsafe { core::ptr::addr_of!((*self.as_ptr()).hash_sha256) }
            .cast::<u8>()
            .cast_mut();
        // SAFETY: an inline field address is non-null; all 32 bytes are
        // initialized and live for this handle's `'a` borrow.
        Some(unsafe { ffibox::CSlice::from_raw_parts(core::ptr::NonNull::new_unchecked(hash), 32) })
    }

    /// Field: git_cert_hostkey.hash_sha1
    /// Borrows the SHA-1 fingerprint when it is available.
    #[must_use]
    pub fn hash_sha1(&self) -> Option<ffibox::CSlice<'a, u8>> {
        if !self.available().contains(GitCertSsh::SHA1) {
            return None;
        }
        // SAFETY: raw-place projection obtains the initialized inline array's
        // address without forming a reference to C-visible memory.
        let hash = unsafe { core::ptr::addr_of!((*self.as_ptr()).hash_sha1) }
            .cast::<u8>()
            .cast_mut();
        // SAFETY: an inline field address is non-null; all 20 bytes are
        // initialized and live for this handle's `'a` borrow.
        Some(unsafe { ffibox::CSlice::from_raw_parts(core::ptr::NonNull::new_unchecked(hash), 20) })
    }

    /// Field: git_cert_hostkey.hash_md5
    /// Borrows the MD5 fingerprint when it is available.
    #[must_use]
    pub fn hash_md5(&self) -> Option<ffibox::CSlice<'a, u8>> {
        if !self.available().contains(GitCertSsh::MD5) {
            return None;
        }
        // SAFETY: raw-place projection obtains the initialized inline array's
        // address without forming a reference to C-visible memory.
        let hash = unsafe { core::ptr::addr_of!((*self.as_ptr()).hash_md5) }
            .cast::<u8>()
            .cast_mut();
        // SAFETY: an inline field address is non-null; all 16 bytes are
        // initialized and live for this handle's `'a` borrow.
        Some(unsafe { ffibox::CSlice::from_raw_parts(core::ptr::NonNull::new_unchecked(hash), 16) })
    }
}

#[cfg(test)]
mod hostkey_tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use super::*;

    #[test]
    fn hostkey_wrapper_preserves_layout_and_handle_shape() {
        assert_eq!(
            size_of::<GitCertHostkey>(),
            size_of::<ffi::git_cert_hostkey>()
        );
        assert_eq!(
            align_of::<GitCertHostkey>(),
            align_of::<ffi::git_cert_hostkey>()
        );
        assert_eq!(
            size_of::<GitCertHostkeyRef<'_>>(),
            size_of::<*mut ffi::git_cert_hostkey>()
        );
        assert_eq!(
            size_of::<GitCertHostkeyMut<'_>>(),
            size_of::<*mut ffi::git_cert_hostkey>()
        );
    }

    #[test]
    fn available_hostkey_views_are_length_delimited_and_flag_gated() {
        let key = [0x41_u8, 0, 0x42, 0x43];
        let mut raw = ffi::git_cert_hostkey {
            parent: ffi::git_cert {
                cert_type: ffi::git_cert_t_GIT_CERT_HOSTKEY_LIBSSH2,
            },
            type_: (GitCertSsh::RAW | GitCertSsh::SHA256).bits(),
            hash_md5: [0x11; 16],
            hash_sha1: [0x22; 20],
            hash_sha256: [0x33; 32],
            raw_type: ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ED25519,
            hostkey: key.as_ptr().cast(),
            hostkey_len: key.len(),
        };

        // SAFETY: `raw` and the borrowed `key` bytes remain live and are not
        // mutated while this shared handle and its views are used.
        let hostkey = unsafe { GitCertHostkeyRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(
            hostkey.parent().cert_type(),
            Ok(GitCertType::HostkeyLibssh2)
        );
        assert!(hostkey.available().contains(GitCertSsh::RAW));
        assert_eq!(hostkey.hostkey_len(), 4);
        let bytes = hostkey.hostkey().unwrap();
        assert_eq!(bytes.len(), 4);
        assert_eq!(bytes.elem(0), Some(0x41));
        assert_eq!(bytes.elem(1), Some(0));
        assert_eq!(hostkey.raw_type(), Ok(Some(GitCertSshRawType::Ed25519)));
        assert_eq!(hostkey.hash_sha256().unwrap().elem(0), Some(0x33));
        assert!(hostkey.hash_sha1().is_none());
        assert!(hostkey.hash_md5().is_none());
    }

    #[test]
    fn absent_representations_do_not_expose_zero_initialized_storage() {
        let mut raw = ffi::git_cert_hostkey {
            parent: ffi::git_cert {
                cert_type: ffi::git_cert_t_GIT_CERT_HOSTKEY_LIBSSH2,
            },
            type_: GitCertSsh::EMPTY.bits(),
            hash_md5: [0; 16],
            hash_sha1: [0; 20],
            hash_sha256: [0; 32],
            raw_type: ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ED25519 + 1,
            hostkey: ptr::null(),
            hostkey_len: 0,
        };

        // SAFETY: `raw` is initialized and exclusively borrowed by this handle.
        let hostkey = unsafe { GitCertHostkeyMut::from_ptr(&raw mut raw) }.unwrap();
        let hostkey = hostkey.as_ref();
        assert!(hostkey.hostkey().is_none());
        assert_eq!(hostkey.raw_type(), Ok(None));
        assert!(hostkey.hash_sha256().is_none());
        assert!(hostkey.hash_sha1().is_none());
        assert!(hostkey.hash_md5().is_none());
    }

    #[test]
    fn invalid_available_raw_type_is_rejected() {
        let mut raw = ffi::git_cert_hostkey {
            parent: ffi::git_cert {
                cert_type: ffi::git_cert_t_GIT_CERT_HOSTKEY_LIBSSH2,
            },
            type_: GitCertSsh::RAW.bits(),
            hash_md5: [0; 16],
            hash_sha1: [0; 20],
            hash_sha256: [0; 32],
            raw_type: ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ED25519 + 1,
            hostkey: ptr::null(),
            hostkey_len: 0,
        };

        // SAFETY: `raw` is initialized and remains live for this shared handle.
        let hostkey = unsafe { GitCertHostkeyRef::from_ptr(&raw mut raw) }.unwrap();
        let error = hostkey.raw_type().unwrap_err();
        assert_eq!(
            error.value(),
            ffi::git_cert_ssh_raw_type_t_GIT_CERT_SSH_RAW_TYPE_KEY_ED25519 + 1
        );
        assert!(hostkey.hostkey().is_none());
    }
}
