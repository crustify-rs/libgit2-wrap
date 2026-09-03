//! Safe wrappers for libgit2 revwalk APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;
use crate::oid::OidRef;

/// Wraps: git_revwalk_hide_cb
/// Safe callable surface for deciding whether to hide a transient commit ID.
pub trait GitRevwalkHideCallback {
    /// Returns nonzero to hide this commit and its ancestors.
    fn call(&mut self, commit_id: OidRef<'_>) -> i32;
}

impl<F> GitRevwalkHideCallback for F
where
    F: FnMut(OidRef<'_>) -> i32,
{
    fn call(&mut self, commit_id: OidRef<'_>) -> i32 {
        self(commit_id)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::oid::Oid;

    #[test]
    fn callback_receives_a_typed_oid_handle() {
        let oid = Oid::zeroed();
        let raw = core::ptr::addr_of!(oid)
            .cast::<crate::ffi::git_oid>()
            .cast_mut();
        // SAFETY: `raw` addresses the live layout-compatible local OID for the
        // duration of this callback invocation.
        let oid = unsafe { OidRef::from_ptr(raw) }.unwrap();
        let mut callback = |candidate: OidRef<'_>| i32::from(candidate.as_ptr() == oid.as_ptr());
        assert_eq!(GitRevwalkHideCallback::call(&mut callback, oid), 1);
    }
}

/// Wraps: git_sort_t
/// A checked set of revision-walk ordering controls.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitSortFlags(ffi::git_sort_t);

impl GitSortFlags {
    /// Use reverse chronological order, libgit2's default.
    pub const NONE: Self = Self(ffi::git_sort_t_GIT_SORT_NONE);
    /// Emit no parent before all of its children.
    pub const TOPOLOGICAL: Self = Self(ffi::git_sort_t_GIT_SORT_TOPOLOGICAL);
    /// Order commits by commit time.
    pub const TIME: Self = Self(ffi::git_sort_t_GIT_SORT_TIME);
    /// Reverse the selected traversal order.
    pub const REVERSE: Self = Self(ffi::git_sort_t_GIT_SORT_REVERSE);
    /// Every sorting flag published by this libgit2 version.
    pub const ALL: Self = Self(Self::TOPOLOGICAL.0 | Self::TIME.0 | Self::REVERSE.0);

    /// Converts raw bits when every bit is published by this libgit2 version.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_sort_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_sort_t {
        self.0
    }

    /// Returns whether default sorting is selected.
    #[must_use]
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitSortFlags> for ffi::git_sort_t {
    fn from(flags: GitSortFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_sort_t> for GitSortFlags {
    type Error = ffi::git_sort_t;

    fn try_from(bits: ffi::git_sort_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitSortFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitSortFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitSortFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitSortFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitSortFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod sort_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn sort_flags_compose_and_validate() {
        let flags = GitSortFlags::TOPOLOGICAL | GitSortFlags::TIME;
        assert!(flags.contains(GitSortFlags::TOPOLOGICAL));
        assert!(flags.intersects(GitSortFlags::TIME));
        assert_eq!(GitSortFlags::from_bits(flags.bits()), Some(flags));
        assert_eq!(GitSortFlags::from_bits(1 << 3), None);
        assert_eq!((!GitSortFlags::REVERSE).bits(), 0b011);
    }

    #[test]
    fn sort_flags_match_the_c_enum_layout() {
        assert_eq!(size_of::<GitSortFlags>(), size_of::<ffi::git_sort_t>());
        assert_eq!(align_of::<GitSortFlags>(), align_of::<ffi::git_sort_t>());
    }
}
