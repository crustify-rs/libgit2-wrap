//! Safe wrappers for libgit2 refs APIs.

use core::ffi::CStr;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_reference_format_t
/// A checked set of options for validating and normalizing reference names.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitReferenceFormatFlags(ffi::git_reference_format_t);

impl GitReferenceFormatFlags {
    /// Apply the normal multi-level reference-name rules.
    pub const NORMAL: Self = Self(ffi::git_reference_format_t_GIT_REFERENCE_FORMAT_NORMAL);
    /// Permit one-level reference names such as `HEAD`.
    pub const ALLOW_ONELEVEL: Self =
        Self(ffi::git_reference_format_t_GIT_REFERENCE_FORMAT_ALLOW_ONELEVEL);
    /// Permit one full-component wildcard in a refspec pattern.
    pub const REFSPEC_PATTERN: Self =
        Self(ffi::git_reference_format_t_GIT_REFERENCE_FORMAT_REFSPEC_PATTERN);
    /// Interpret the name as a shorthand refspec component.
    pub const REFSPEC_SHORTHAND: Self =
        Self(ffi::git_reference_format_t_GIT_REFERENCE_FORMAT_REFSPEC_SHORTHAND);
    /// Every reference-format option published by this libgit2 version.
    pub const ALL: Self =
        Self(Self::ALLOW_ONELEVEL.0 | Self::REFSPEC_PATTERN.0 | Self::REFSPEC_SHORTHAND.0);

    /// Converts raw bits when every bit is a published format option.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_reference_format_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_reference_format_t {
        self.0
    }

    /// Returns whether no optional behavior is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any option in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitReferenceFormatFlags> for ffi::git_reference_format_t {
    fn from(flags: GitReferenceFormatFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_reference_format_t> for GitReferenceFormatFlags {
    type Error = ffi::git_reference_format_t;

    fn try_from(bits: ffi::git_reference_format_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitReferenceFormatFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitReferenceFormatFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitReferenceFormatFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitReferenceFormatFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitReferenceFormatFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

/// Wraps: git_reference_foreach_name_cb
/// Safe callable surface for one transient reference name.
pub trait GitReferenceForeachNameCallback {
    /// Receives a reference name borrowed for this invocation.
    ///
    /// Returning nonzero stops iteration and propagates that value to the
    /// caller.
    fn call(&mut self, name: &CStr) -> i32;
}

impl<F> GitReferenceForeachNameCallback for F
where
    F: FnMut(&CStr) -> i32,
{
    fn call(&mut self, name: &CStr) -> i32 {
        self(name)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn reference_format_flags_combine_and_validate() {
        let flags =
            GitReferenceFormatFlags::ALLOW_ONELEVEL | GitReferenceFormatFlags::REFSPEC_PATTERN;
        assert!(flags.contains(GitReferenceFormatFlags::ALLOW_ONELEVEL));
        assert!(flags.intersects(GitReferenceFormatFlags::REFSPEC_PATTERN));
        assert!(!flags.intersects(GitReferenceFormatFlags::REFSPEC_SHORTHAND));
        assert_eq!(
            GitReferenceFormatFlags::from_bits(flags.bits()),
            Some(flags)
        );
        assert!(GitReferenceFormatFlags::NORMAL.is_empty());

        let unknown = GitReferenceFormatFlags::ALL.bits() << 1;
        assert_eq!(GitReferenceFormatFlags::from_bits(unknown), None);
        assert_eq!(GitReferenceFormatFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn reference_format_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitReferenceFormatFlags>(),
            size_of::<ffi::git_reference_format_t>()
        );
        assert_eq!(
            align_of::<GitReferenceFormatFlags>(),
            align_of::<ffi::git_reference_format_t>()
        );
    }

    #[test]
    fn reference_format_complement_stays_within_published_bits() {
        assert_eq!(
            !GitReferenceFormatFlags::ALLOW_ONELEVEL,
            GitReferenceFormatFlags::REFSPEC_PATTERN | GitReferenceFormatFlags::REFSPEC_SHORTHAND
        );
    }

    #[test]
    fn closure_implements_reference_name_callback() {
        let mut seen = |name: &CStr| i32::from(name == c"refs/heads/main");
        assert_eq!(
            GitReferenceForeachNameCallback::call(&mut seen, c"refs/heads/main"),
            1
        );
    }
}
