//! Safe wrappers for libgit2 email APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use crate::ffi;

/// Wraps: git_email_create_flags_t
/// A checked set of formatting options for generated patch emails.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitEmailCreateFlags(ffi::git_email_create_flags_t);

impl GitEmailCreateFlags {
    /// The default patch-email formatting behavior.
    pub const DEFAULT: Self = Self(ffi::git_email_create_flags_t_GIT_EMAIL_CREATE_DEFAULT);
    /// Omit patch numbers from the subject prefix.
    pub const OMIT_NUMBERS: Self =
        Self(ffi::git_email_create_flags_t_GIT_EMAIL_CREATE_OMIT_NUMBERS);
    /// Include patch numbers even for a single-commit series.
    pub const ALWAYS_NUMBER: Self =
        Self(ffi::git_email_create_flags_t_GIT_EMAIL_CREATE_ALWAYS_NUMBER);
    /// Disable rename and similarity detection.
    pub const NO_RENAMES: Self = Self(ffi::git_email_create_flags_t_GIT_EMAIL_CREATE_NO_RENAMES);
    /// Every option currently published by libgit2.
    pub const ALL: Self = Self(Self::OMIT_NUMBERS.0 | Self::ALWAYS_NUMBER.0 | Self::NO_RENAMES.0);

    /// Converts a C bit set when every set bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_email_create_flags_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_email_create_flags_t {
        self.0
    }

    /// Returns whether no formatting option is enabled.
    #[must_use]
    pub const fn is_default(self) -> bool {
        self.0 == Self::DEFAULT.0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for GitEmailCreateFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitEmailCreateFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitEmailCreateFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitEmailCreateFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl From<GitEmailCreateFlags> for ffi::git_email_create_flags_t {
    fn from(flags: GitEmailCreateFlags) -> Self {
        flags.bits()
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_email_flags_form_checked_bit_sets() {
        let mut flags = GitEmailCreateFlags::DEFAULT;
        assert!(flags.is_default());

        flags |= GitEmailCreateFlags::OMIT_NUMBERS | GitEmailCreateFlags::NO_RENAMES;
        assert!(flags.contains(GitEmailCreateFlags::OMIT_NUMBERS));
        assert!(flags.contains(GitEmailCreateFlags::NO_RENAMES));
        assert!(!flags.contains(GitEmailCreateFlags::ALWAYS_NUMBER));

        flags &= GitEmailCreateFlags::NO_RENAMES | GitEmailCreateFlags::ALWAYS_NUMBER;
        assert_eq!(flags, GitEmailCreateFlags::NO_RENAMES);
        assert_eq!(ffi::git_email_create_flags_t::from(flags), flags.bits());
    }

    #[test]
    fn raw_email_flag_bits_are_validated() {
        for flags in [
            GitEmailCreateFlags::DEFAULT,
            GitEmailCreateFlags::OMIT_NUMBERS,
            GitEmailCreateFlags::ALWAYS_NUMBER,
            GitEmailCreateFlags::NO_RENAMES,
            GitEmailCreateFlags::ALL,
        ] {
            assert_eq!(GitEmailCreateFlags::from_bits(flags.bits()), Some(flags));
        }

        assert_eq!(
            GitEmailCreateFlags::from_bits(GitEmailCreateFlags::ALL.bits() << 1),
            None
        );
    }

    #[test]
    fn email_flags_match_the_c_abi_scalar() {
        assert_eq!(
            size_of::<GitEmailCreateFlags>(),
            size_of::<ffi::git_email_create_flags_t>()
        );
        assert_eq!(
            align_of::<GitEmailCreateFlags>(),
            align_of::<ffi::git_email_create_flags_t>()
        );
    }
}
