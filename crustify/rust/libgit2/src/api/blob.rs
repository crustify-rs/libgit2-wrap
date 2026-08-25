//! Safe wrappers for libgit2 blob APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use crate::ffi;

/// Wraps: git_blob_filter_flag_t
/// A checked set of options controlling blob filtering.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitBlobFilterFlags(ffi::git_blob_filter_flag_t);

impl GitBlobFilterFlags {
    /// No filtering options.
    pub const NONE: Self = Self(0);
    /// Skip filtering when the blob is binary.
    pub const CHECK_FOR_BINARY: Self =
        Self(ffi::git_blob_filter_flag_t_GIT_BLOB_FILTER_CHECK_FOR_BINARY);
    /// Do not load attributes from the system-wide attributes file.
    pub const NO_SYSTEM_ATTRIBUTES: Self =
        Self(ffi::git_blob_filter_flag_t_GIT_BLOB_FILTER_NO_SYSTEM_ATTRIBUTES);
    /// Load attributes from the current `HEAD` commit.
    pub const ATTRIBUTES_FROM_HEAD: Self =
        Self(ffi::git_blob_filter_flag_t_GIT_BLOB_FILTER_ATTRIBUTES_FROM_HEAD);
    /// Load attributes from the commit selected by the filter options.
    pub const ATTRIBUTES_FROM_COMMIT: Self =
        Self(ffi::git_blob_filter_flag_t_GIT_BLOB_FILTER_ATTRIBUTES_FROM_COMMIT);
    /// Every flag published by this libgit2 API.
    pub const ALL: Self = Self(
        Self::CHECK_FOR_BINARY.0
            | Self::NO_SYSTEM_ATTRIBUTES.0
            | Self::ATTRIBUTES_FROM_HEAD.0
            | Self::ATTRIBUTES_FROM_COMMIT.0,
    );

    /// Converts raw bits when every set bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_blob_filter_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_blob_filter_flag_t {
        self.0
    }

    /// Returns whether no filtering option is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for GitBlobFilterFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitBlobFilterFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitBlobFilterFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitBlobFilterFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl From<GitBlobFilterFlags> for ffi::git_blob_filter_flag_t {
    fn from(flags: GitBlobFilterFlags) -> Self {
        flags.bits()
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_blob_filter_flags_form_checked_sets() {
        let mut flags = GitBlobFilterFlags::NONE;
        assert!(flags.is_empty());

        flags |= GitBlobFilterFlags::CHECK_FOR_BINARY | GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT;
        assert!(flags.contains(GitBlobFilterFlags::CHECK_FOR_BINARY));
        assert!(flags.contains(GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT));
        assert!(!flags.contains(GitBlobFilterFlags::ATTRIBUTES_FROM_HEAD));

        flags &=
            GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT | GitBlobFilterFlags::NO_SYSTEM_ATTRIBUTES;
        assert_eq!(flags, GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT);
        assert_eq!(ffi::git_blob_filter_flag_t::from(flags), flags.bits());
    }

    #[test]
    fn raw_blob_filter_bits_are_validated() {
        for flags in [
            GitBlobFilterFlags::NONE,
            GitBlobFilterFlags::CHECK_FOR_BINARY,
            GitBlobFilterFlags::NO_SYSTEM_ATTRIBUTES,
            GitBlobFilterFlags::ATTRIBUTES_FROM_HEAD,
            GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT,
            GitBlobFilterFlags::ALL,
        ] {
            assert_eq!(GitBlobFilterFlags::from_bits(flags.bits()), Some(flags));
        }

        assert_eq!(
            GitBlobFilterFlags::from_bits(GitBlobFilterFlags::ALL.bits() << 1),
            None
        );
    }

    #[test]
    fn blob_filter_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitBlobFilterFlags>(),
            size_of::<ffi::git_blob_filter_flag_t>()
        );
        assert_eq!(
            align_of::<GitBlobFilterFlags>(),
            align_of::<ffi::git_blob_filter_flag_t>()
        );
    }
}
