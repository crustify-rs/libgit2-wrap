//! Safe wrappers for libgit2 credential APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

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
