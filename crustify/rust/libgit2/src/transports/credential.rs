//! Safe wrappers for libgit2 credential APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use core::ptr::addr_of;

use ffibox::CBox;

use crate::ffi;

/// Wraps: git_credential_t
/// Bit set describing credential kinds accepted by a transport.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitCredentialType(ffi::git_credential_t);

impl GitCredentialType {
    /// No credential kinds.
    pub const EMPTY: Self = Self(0);
    /// A plaintext username and password.
    pub const USERPASS_PLAINTEXT: Self =
        Self(ffi::git_credential_t_GIT_CREDENTIAL_USERPASS_PLAINTEXT);
    /// An SSH key read from disk.
    pub const SSH_KEY: Self = Self(ffi::git_credential_t_GIT_CREDENTIAL_SSH_KEY);
    /// An SSH key with a custom signing callback.
    pub const SSH_CUSTOM: Self = Self(ffi::git_credential_t_GIT_CREDENTIAL_SSH_CUSTOM);
    /// The platform's default credentials, such as NTLM or Negotiate.
    pub const DEFAULT: Self = Self(ffi::git_credential_t_GIT_CREDENTIAL_DEFAULT);
    /// An interactive SSH credential.
    pub const SSH_INTERACTIVE: Self = Self(ffi::git_credential_t_GIT_CREDENTIAL_SSH_INTERACTIVE);
    /// A username-only credential.
    pub const USERNAME: Self = Self(ffi::git_credential_t_GIT_CREDENTIAL_USERNAME);
    /// An SSH key read from memory.
    pub const SSH_MEMORY: Self = Self(ffi::git_credential_t_GIT_CREDENTIAL_SSH_MEMORY);
    /// Every credential kind published by this version of libgit2.
    pub const ALL: Self = Self(
        Self::USERPASS_PLAINTEXT.0
            | Self::SSH_KEY.0
            | Self::SSH_CUSTOM.0
            | Self::DEFAULT.0
            | Self::SSH_INTERACTIVE.0
            | Self::USERNAME.0
            | Self::SSH_MEMORY.0,
    );

    /// Converts a C bit set if it contains only published credential kinds.
    pub const fn from_bits(bits: ffi::git_credential_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    pub const fn bits(self) -> ffi::git_credential_t {
        self.0
    }

    /// Returns whether no credential kinds are present.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every kind in `other` is present.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any kind in `other` is present.
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Combines two sets of credential kinds.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Retains the credential kinds common to both sets.
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl BitOr for GitCredentialType {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl BitOrAssign for GitCredentialType {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

impl BitAnd for GitCredentialType {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(rhs)
    }
}

impl BitAndAssign for GitCredentialType {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.intersection(rhs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_kinds_form_valid_bit_sets() {
        let mut types = GitCredentialType::USERNAME | GitCredentialType::SSH_KEY;
        assert!(types.contains(GitCredentialType::USERNAME));
        assert!(types.intersects(GitCredentialType::SSH_KEY));
        assert!(!types.intersects(GitCredentialType::DEFAULT));

        types |= GitCredentialType::DEFAULT;
        assert!(types.contains(GitCredentialType::USERNAME | GitCredentialType::DEFAULT));

        types &= GitCredentialType::DEFAULT | GitCredentialType::SSH_KEY;
        assert_eq!(
            types,
            GitCredentialType::DEFAULT | GitCredentialType::SSH_KEY
        );
    }

    #[test]
    fn raw_bits_are_checked() {
        assert_eq!(
            GitCredentialType::from_bits(GitCredentialType::ALL.bits()),
            Some(GitCredentialType::ALL)
        );
        assert_eq!(GitCredentialType::from_bits(1 << 31), None);
        assert!(GitCredentialType::EMPTY.is_empty());
    }
}

ffibox::define_ctype!(
    /// Wraps: git_credential
    /// Polymorphic base header for an authentication credential.
    GitCredential,
    GitCredentialRef,
    GitCredentialMut,
    ffi::git_credential
);

/// An owned, fully constructed credential.
pub type GitCredentialOwned = CBox<GitCredential>;

// SAFETY: every fully formed credential has a non-null concrete finalizer in
// its base header. `git_credential_free` invokes it exactly once and accepts
// null, although `CBox` always supplies a non-null pointer.
ffibox::impl_dropped!(GitCredential, ffi::git_credential, ffi::git_credential_free);

impl GitCredentialRef<'_> {
    /// Wraps: git_credential.free
    /// Returns whether the credential header contains its required finalizer.
    #[must_use]
    pub fn has_deallocator(&self) -> bool {
        // SAFETY: this live handle permits a raw-place field read without
        // forming a reference to the C-visible credential.
        unsafe { addr_of!((*self.as_ptr()).free).read().is_some() }
    }

    /// Wraps: git_credential.credtype
    /// Returns the credential kind when its bit set is published by libgit2.
    #[must_use]
    pub fn credential_type(&self) -> Option<GitCredentialType> {
        // SAFETY: this live handle permits a raw-place scalar read without
        // forming a reference to the C-visible credential.
        let credtype = unsafe { addr_of!((*self.as_ptr()).credtype).read() };
        GitCredentialType::from_bits(credtype)
    }
}

#[cfg(test)]
mod credential_tests {
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn test_credential_free(credential: *mut ffi::git_credential) {
        DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the lifecycle test passes the unique pointer produced by
        // `Box::into_raw`, and this callback is invoked exactly once.
        drop(unsafe { Box::from_raw(credential) });
    }

    #[test]
    fn credential_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<GitCredential>(), size_of::<ffi::git_credential>());
        assert_eq!(
            align_of::<GitCredential>(),
            align_of::<ffi::git_credential>()
        );
        assert_eq!(
            size_of::<GitCredentialRef<'_>>(),
            size_of::<*const ffi::git_credential>()
        );
        assert_eq!(
            size_of::<GitCredentialMut<'_>>(),
            size_of::<*mut ffi::git_credential>()
        );
    }

    #[test]
    fn credential_handle_validates_kind_and_finalizer() {
        let mut raw = ffi::git_credential {
            credtype: GitCredentialType::USERNAME.bits(),
            free: Some(test_credential_free),
        };

        // SAFETY: `raw` remains live for the complete handle use and this is
        // the only handle accessing the local credential header.
        let credential = unsafe { GitCredentialRef::from_ptr(&raw mut raw) }
            .expect("the address of a local credential is non-null");
        assert_eq!(
            credential.credential_type(),
            Some(GitCredentialType::USERNAME)
        );
        assert!(credential.has_deallocator());

        raw.credtype = 1 << 31;
        raw.free = None;
        // SAFETY: the previous handle is no longer used and `raw` remains a
        // live initialized header for these diagnostic getters.
        let credential = unsafe { GitCredentialRef::from_ptr(&raw mut raw) }
            .expect("the address of a local credential is non-null");
        assert_eq!(credential.credential_type(), None);
        assert!(!credential.has_deallocator());
    }

    #[test]
    fn credential_owner_invokes_the_concrete_finalizer_once() {
        DROP_COUNT.store(0, Ordering::SeqCst);
        let raw = Box::into_raw(Box::new(ffi::git_credential {
            credtype: GitCredentialType::DEFAULT.bits(),
            free: Some(test_credential_free),
        }));

        // SAFETY: `raw` is a fresh, fully formed credential allocation whose
        // installed finalizer reclaims this exact `Box` allocation.
        let credential =
            unsafe { GitCredentialOwned::from_raw(raw) }.expect("Box::into_raw never returns null");
        assert!(credential.as_ref().has_deallocator());
        drop(credential);
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);
    }
}
