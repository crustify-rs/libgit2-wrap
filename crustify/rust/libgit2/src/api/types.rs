//! Safe wrappers for libgit2 types APIs.

use core::ops::{BitOr, BitOrAssign};

use crate::ffi;

/// Wraps: git_branch_t
/// A checked set of branch kinds used by libgit2 branch APIs.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GitBranchType(ffi::git_branch_t);

impl GitBranchType {
    /// A local branch.
    pub const LOCAL: Self = Self(ffi::git_branch_t_GIT_BRANCH_LOCAL);
    /// A remote-tracking branch.
    pub const REMOTE: Self = Self(ffi::git_branch_t_GIT_BRANCH_REMOTE);
    /// Local and remote-tracking branches.
    pub const ALL: Self = Self(ffi::git_branch_t_GIT_BRANCH_ALL);

    /// Converts a C value if it is a nonempty set of published branch kinds.
    pub const fn from_bits(bits: ffi::git_branch_t) -> Option<Self> {
        if bits != 0 && bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    pub const fn bits(self) -> ffi::git_branch_t {
        self.0
    }

    /// Returns whether every kind in `other` is present.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any kind in `other` is present.
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for GitBranchType {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitBranchType {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl From<GitBranchType> for ffi::git_branch_t {
    fn from(value: GitBranchType) -> Self {
        value.bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn branch_kinds_form_valid_sets() {
        let mut kinds = GitBranchType::LOCAL;
        kinds |= GitBranchType::REMOTE;
        assert_eq!(kinds, GitBranchType::ALL);
        assert!(kinds.contains(GitBranchType::LOCAL));
        assert!(kinds.intersects(GitBranchType::REMOTE));
    }

    #[test]
    fn raw_branch_bits_are_checked() {
        for kind in [
            GitBranchType::LOCAL,
            GitBranchType::REMOTE,
            GitBranchType::ALL,
        ] {
            assert_eq!(GitBranchType::from_bits(kind.bits()), Some(kind));
        }

        assert_eq!(GitBranchType::from_bits(0), None);
        assert_eq!(
            GitBranchType::from_bits(GitBranchType::ALL.bits() << 1),
            None
        );
    }

    #[test]
    fn branch_type_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<GitBranchType>(), size_of::<ffi::git_branch_t>());
        assert_eq!(align_of::<GitBranchType>(), align_of::<ffi::git_branch_t>());
    }
}

/// Wraps: git_filemode_t
/// A file mode accepted by libgit2 index and tree APIs.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GitFileMode(ffi::git_filemode_t);

impl GitFileMode {
    /// An unreadable or absent entry.
    pub const UNREADABLE: Self = Self(ffi::git_filemode_t_GIT_FILEMODE_UNREADABLE);
    /// A tree (directory) entry.
    pub const TREE: Self = Self(ffi::git_filemode_t_GIT_FILEMODE_TREE);
    /// A regular, non-executable blob.
    pub const BLOB: Self = Self(ffi::git_filemode_t_GIT_FILEMODE_BLOB);
    /// An executable blob.
    pub const BLOB_EXECUTABLE: Self = Self(ffi::git_filemode_t_GIT_FILEMODE_BLOB_EXECUTABLE);
    /// A symbolic link.
    pub const LINK: Self = Self(ffi::git_filemode_t_GIT_FILEMODE_LINK);
    /// A gitlink (submodule commit).
    pub const COMMIT: Self = Self(ffi::git_filemode_t_GIT_FILEMODE_COMMIT);

    /// Converts a raw value when it is one of libgit2's published file modes.
    pub const fn from_raw(raw: ffi::git_filemode_t) -> Option<Self> {
        match raw {
            ffi::git_filemode_t_GIT_FILEMODE_UNREADABLE
            | ffi::git_filemode_t_GIT_FILEMODE_TREE
            | ffi::git_filemode_t_GIT_FILEMODE_BLOB
            | ffi::git_filemode_t_GIT_FILEMODE_BLOB_EXECUTABLE
            | ffi::git_filemode_t_GIT_FILEMODE_LINK
            | ffi::git_filemode_t_GIT_FILEMODE_COMMIT => Some(Self(raw)),
            _ => None,
        }
    }

    /// Returns the underlying libgit2 file mode.
    pub const fn as_raw(self) -> ffi::git_filemode_t {
        self.0
    }
}

impl From<GitFileMode> for ffi::git_filemode_t {
    fn from(value: GitFileMode) -> Self {
        value.as_raw()
    }
}

#[cfg(test)]
mod filemode_tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn published_file_modes_round_trip() {
        for mode in [
            GitFileMode::UNREADABLE,
            GitFileMode::TREE,
            GitFileMode::BLOB,
            GitFileMode::BLOB_EXECUTABLE,
            GitFileMode::LINK,
            GitFileMode::COMMIT,
        ] {
            assert_eq!(GitFileMode::from_raw(mode.as_raw()), Some(mode));
            assert_eq!(ffi::git_filemode_t::from(mode), mode.as_raw());
        }

        assert_eq!(GitFileMode::from_raw(1), None);
    }

    #[test]
    fn file_mode_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<GitFileMode>(), size_of::<ffi::git_filemode_t>());
        assert_eq!(align_of::<GitFileMode>(), align_of::<ffi::git_filemode_t>());
    }
}

/// Wraps: git_object_t
/// A checked libgit2 object kind.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GitObjectType(ffi::git_object_t);

impl GitObjectType {
    /// A wildcard accepted by APIs that permit any object kind.
    pub const ANY: Self = Self(ffi::git_object_t_GIT_OBJECT_ANY);
    /// An invalid or unknown object kind.
    pub const INVALID: Self = Self(ffi::git_object_t_GIT_OBJECT_INVALID);
    /// A commit object.
    pub const COMMIT: Self = Self(ffi::git_object_t_GIT_OBJECT_COMMIT);
    /// A tree object.
    pub const TREE: Self = Self(ffi::git_object_t_GIT_OBJECT_TREE);
    /// A blob object.
    pub const BLOB: Self = Self(ffi::git_object_t_GIT_OBJECT_BLOB);
    /// An annotated tag object.
    pub const TAG: Self = Self(ffi::git_object_t_GIT_OBJECT_TAG);

    /// Converts a raw value when it is a published libgit2 object kind.
    pub const fn from_raw(raw: ffi::git_object_t) -> Option<Self> {
        match raw {
            ffi::git_object_t_GIT_OBJECT_ANY
            | ffi::git_object_t_GIT_OBJECT_INVALID
            | ffi::git_object_t_GIT_OBJECT_COMMIT
            | ffi::git_object_t_GIT_OBJECT_TREE
            | ffi::git_object_t_GIT_OBJECT_BLOB
            | ffi::git_object_t_GIT_OBJECT_TAG => Some(Self(raw)),
            _ => None,
        }
    }

    /// Returns the underlying C value.
    pub const fn as_raw(self) -> ffi::git_object_t {
        self.0
    }
}

impl From<GitObjectType> for ffi::git_object_t {
    fn from(value: GitObjectType) -> Self {
        value.as_raw()
    }
}

#[cfg(test)]
mod object_type_tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn published_object_kinds_round_trip() {
        for kind in [
            GitObjectType::ANY,
            GitObjectType::INVALID,
            GitObjectType::COMMIT,
            GitObjectType::TREE,
            GitObjectType::BLOB,
            GitObjectType::TAG,
        ] {
            assert_eq!(GitObjectType::from_raw(kind.as_raw()), Some(kind));
            assert_eq!(ffi::git_object_t::from(kind), kind.as_raw());
        }

        assert_eq!(GitObjectType::from_raw(0), None);
        assert_eq!(GitObjectType::from_raw(5), None);
    }

    #[test]
    fn object_type_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<GitObjectType>(), size_of::<ffi::git_object_t>());
        assert_eq!(align_of::<GitObjectType>(), align_of::<ffi::git_object_t>());
    }
}

/// Wraps: git_reference_t
/// The published kind of a Git reference.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitReferenceType {
    /// An invalid or uninitialized reference kind.
    Invalid = ffi::git_reference_t_GIT_REFERENCE_INVALID,
    /// A direct reference whose target is an object ID.
    Direct = ffi::git_reference_t_GIT_REFERENCE_DIRECT,
    /// A symbolic reference whose target is another reference name.
    Symbolic = ffi::git_reference_t_GIT_REFERENCE_SYMBOLIC,
    /// A mask containing both valid reference kinds.
    All = ffi::git_reference_t_GIT_REFERENCE_ALL,
}

/// A raw reference kind not published by libgit2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitReferenceType(ffi::git_reference_t);

impl InvalidGitReferenceType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_reference_t {
        self.0
    }
}

impl From<GitReferenceType> for ffi::git_reference_t {
    fn from(value: GitReferenceType) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_reference_t> for GitReferenceType {
    type Error = InvalidGitReferenceType;

    fn try_from(value: ffi::git_reference_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_reference_t_GIT_REFERENCE_INVALID => Ok(Self::Invalid),
            ffi::git_reference_t_GIT_REFERENCE_DIRECT => Ok(Self::Direct),
            ffi::git_reference_t_GIT_REFERENCE_SYMBOLIC => Ok(Self::Symbolic),
            ffi::git_reference_t_GIT_REFERENCE_ALL => Ok(Self::All),
            value => Err(InvalidGitReferenceType(value)),
        }
    }
}

#[cfg(test)]
mod reference_type_tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn published_reference_kinds_round_trip() {
        for kind in [
            GitReferenceType::Invalid,
            GitReferenceType::Direct,
            GitReferenceType::Symbolic,
            GitReferenceType::All,
        ] {
            let raw = ffi::git_reference_t::from(kind);
            assert_eq!(GitReferenceType::try_from(raw), Ok(kind));
        }

        let invalid = ffi::git_reference_t_GIT_REFERENCE_ALL + 1;
        let error = GitReferenceType::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn reference_type_matches_the_c_abi_scalar() {
        assert_eq!(
            size_of::<GitReferenceType>(),
            size_of::<ffi::git_reference_t>()
        );
        assert_eq!(
            align_of::<GitReferenceType>(),
            align_of::<ffi::git_reference_t>()
        );
    }
}
