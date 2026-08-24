//! Safe wrappers for libgit2 apply APIs.

use core::ops::{BitOr, BitOrAssign};

use crate::ffi;

/// Wraps: git_apply_flags_t
/// A checked set of flags controlling how libgit2 applies a patch.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitApplyFlags(ffi::git_apply_flags_t);

impl GitApplyFlags {
    /// Apply normally and write the resulting changes.
    pub const NONE: Self = Self(0);
    /// Check whether the patch applies without making changes.
    pub const CHECK: Self = Self(ffi::git_apply_flags_t_GIT_APPLY_CHECK);

    /// Converts raw bits when every flag is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_apply_flags_t) -> Option<Self> {
        if bits & !Self::CHECK.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_apply_flags_t {
        self.0
    }

    /// Returns whether no apply flags are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for GitApplyFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitApplyFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl From<GitApplyFlags> for ffi::git_apply_flags_t {
    fn from(value: GitApplyFlags) -> Self {
        value.bits()
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_apply_flags_form_valid_sets() {
        assert_eq!(GitApplyFlags::from_bits(0), Some(GitApplyFlags::NONE));
        assert_eq!(
            GitApplyFlags::from_bits(ffi::git_apply_flags_t_GIT_APPLY_CHECK),
            Some(GitApplyFlags::CHECK)
        );

        let mut flags = GitApplyFlags::NONE;
        assert!(flags.is_empty());
        flags |= GitApplyFlags::CHECK;
        assert!(flags.contains(GitApplyFlags::CHECK));
        assert_eq!(ffi::git_apply_flags_t::from(flags), flags.bits());
    }

    #[test]
    fn unknown_apply_flag_bits_are_rejected() {
        let unknown = ffi::git_apply_flags_t_GIT_APPLY_CHECK << 1;
        assert_eq!(GitApplyFlags::from_bits(unknown), None);
    }

    #[test]
    fn apply_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitApplyFlags>(),
            size_of::<ffi::git_apply_flags_t>()
        );
        assert_eq!(
            align_of::<GitApplyFlags>(),
            align_of::<ffi::git_apply_flags_t>()
        );
    }
}
