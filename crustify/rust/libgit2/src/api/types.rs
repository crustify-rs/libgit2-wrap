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

/// Wraps: git_submodule_ignore_t
/// Controls which changes libgit2 ignores when checking a submodule's status.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitSubmoduleIgnore {
    /// Use the ignore rule from the submodule's configuration.
    Unspecified = ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_UNSPECIFIED,
    /// Treat every tracked or untracked change as dirty.
    None = ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_NONE,
    /// Ignore untracked files but detect tracked-file changes.
    Untracked = ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_UNTRACKED,
    /// Ignore worktree changes and only detect a moved `HEAD`.
    Dirty = ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_DIRTY,
    /// Never inspect the submodule for dirty state.
    All = ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_ALL,
}

/// A raw value that is not a published [`GitSubmoduleIgnore`] rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitSubmoduleIgnore(ffi::git_submodule_ignore_t);

impl InvalidGitSubmoduleIgnore {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_submodule_ignore_t {
        self.0
    }
}

impl From<GitSubmoduleIgnore> for ffi::git_submodule_ignore_t {
    fn from(value: GitSubmoduleIgnore) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_submodule_ignore_t> for GitSubmoduleIgnore {
    type Error = InvalidGitSubmoduleIgnore;

    fn try_from(value: ffi::git_submodule_ignore_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_UNSPECIFIED => Ok(Self::Unspecified),
            ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_NONE => Ok(Self::None),
            ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_UNTRACKED => Ok(Self::Untracked),
            ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_DIRTY => Ok(Self::Dirty),
            ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_ALL => Ok(Self::All),
            value => Err(InvalidGitSubmoduleIgnore(value)),
        }
    }
}

/// Wraps: git_submodule_update_t
/// Selects how libgit2 updates a submodule.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GitSubmoduleUpdate {
    /// No update rule has been selected.
    #[default]
    Default = ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_DEFAULT,
    /// Check out the superproject's recorded commit as a detached `HEAD`.
    Checkout = ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_CHECKOUT,
    /// Rebase the checked-out branch onto the superproject's recorded commit.
    Rebase = ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_REBASE,
    /// Merge the superproject's recorded commit into the checked-out branch.
    Merge = ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_MERGE,
    /// Do not update the submodule.
    None = ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_NONE,
}

/// A raw value that is not a published [`GitSubmoduleUpdate`] strategy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitSubmoduleUpdate(ffi::git_submodule_update_t);

impl InvalidGitSubmoduleUpdate {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_submodule_update_t {
        self.0
    }
}

impl From<GitSubmoduleUpdate> for ffi::git_submodule_update_t {
    fn from(value: GitSubmoduleUpdate) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_submodule_update_t> for GitSubmoduleUpdate {
    type Error = InvalidGitSubmoduleUpdate;

    fn try_from(value: ffi::git_submodule_update_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_DEFAULT => Ok(Self::Default),
            ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_CHECKOUT => Ok(Self::Checkout),
            ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_REBASE => Ok(Self::Rebase),
            ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_MERGE => Ok(Self::Merge),
            ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_NONE => Ok(Self::None),
            value => Err(InvalidGitSubmoduleUpdate(value)),
        }
    }
}

#[cfg(test)]
mod submodule_type_tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn published_ignore_rules_round_trip() {
        for rule in [
            GitSubmoduleIgnore::Unspecified,
            GitSubmoduleIgnore::None,
            GitSubmoduleIgnore::Untracked,
            GitSubmoduleIgnore::Dirty,
            GitSubmoduleIgnore::All,
        ] {
            let raw = ffi::git_submodule_ignore_t::from(rule);
            assert_eq!(GitSubmoduleIgnore::try_from(raw), Ok(rule));
        }

        let error = GitSubmoduleIgnore::try_from(0).unwrap_err();
        assert_eq!(error.value(), 0);
    }

    #[test]
    fn published_update_strategies_round_trip() {
        for strategy in [
            GitSubmoduleUpdate::Default,
            GitSubmoduleUpdate::Checkout,
            GitSubmoduleUpdate::Rebase,
            GitSubmoduleUpdate::Merge,
            GitSubmoduleUpdate::None,
        ] {
            let raw = ffi::git_submodule_update_t::from(strategy);
            assert_eq!(GitSubmoduleUpdate::try_from(raw), Ok(strategy));
        }

        let invalid = ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_NONE + 1;
        let error = GitSubmoduleUpdate::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn submodule_types_match_the_c_abi_scalars() {
        assert_eq!(
            size_of::<GitSubmoduleIgnore>(),
            size_of::<ffi::git_submodule_ignore_t>()
        );
        assert_eq!(
            align_of::<GitSubmoduleIgnore>(),
            align_of::<ffi::git_submodule_ignore_t>()
        );
        assert_eq!(
            size_of::<GitSubmoduleUpdate>(),
            size_of::<ffi::git_submodule_update_t>()
        );
        assert_eq!(
            align_of::<GitSubmoduleUpdate>(),
            align_of::<ffi::git_submodule_update_t>()
        );
    }
}
ffibox::define_ctype!(
    /// Wraps: git_time
    /// A layout-compatible timestamp and timezone offset from a Git signature.
    GitTime,
    GitTimeRef,
    GitTimeMut,
    ffi::git_time
);

/// A sign stored with a [`GitTime`] timezone offset.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitTimeSign {
    /// A nonnegative offset, including ordinary `+0000` UTC.
    Positive,
    /// A negative offset, including the questionable `-0000` representation.
    Negative,
}

impl GitTimeSign {
    const fn from_raw(raw: core::ffi::c_char) -> Option<Self> {
        match raw as u8 {
            b'+' => Some(Self::Positive),
            b'-' => Some(Self::Negative),
            _ => None,
        }
    }

    const fn as_raw(self) -> core::ffi::c_char {
        match self {
            Self::Positive => b'+' as core::ffi::c_char,
            Self::Negative => b'-' as core::ffi::c_char,
        }
    }
}

/// A raw `git_time.sign` byte that is neither `+` nor `-`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitTimeSign(core::ffi::c_char);

impl InvalidGitTimeSign {
    /// Returns the unrecognized C byte.
    #[must_use]
    pub const fn value(self) -> core::ffi::c_char {
        self.0
    }
}

impl GitTimeRef<'_> {
    /// Wraps: git_time.offset
    /// Returns the timezone offset from UTC in minutes.
    #[inline]
    #[must_use]
    pub fn offset(&self) -> core::ffi::c_int {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).offset).read() }
    }

    /// Wraps: git_time.sign
    /// Returns the stored timezone sign after validating the C byte.
    #[inline]
    pub fn sign(&self) -> Result<GitTimeSign, InvalidGitTimeSign> {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        let raw = unsafe { core::ptr::addr_of!((*ptr).sign).read() };
        GitTimeSign::from_raw(raw).ok_or(InvalidGitTimeSign(raw))
    }

    /// Wraps: git_time.time
    /// Returns the number of seconds since the Unix epoch.
    #[inline]
    #[must_use]
    pub fn time(&self) -> ffi::git_time_t {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).time).read() }
    }
}

impl GitTimeMut<'_> {
    /// Sets the timezone offset from UTC in minutes.
    #[inline]
    pub fn set_offset(&mut self, offset: core::ffi::c_int) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).offset).write(offset) }
    }

    /// Sets the stored timezone sign.
    #[inline]
    pub fn set_sign(&mut self, sign: GitTimeSign) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes a valid sign byte without forming a reference.
        unsafe { core::ptr::addr_of_mut!((*ptr).sign).write(sign.as_raw()) }
    }

    /// Sets the number of seconds since the Unix epoch.
    #[inline]
    pub fn set_time(&mut self, time: ffi::git_time_t) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).time).write(time) }
    }
}

#[cfg(test)]
mod time_tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn time_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<GitTime>(), size_of::<ffi::git_time>());
        assert_eq!(align_of::<GitTime>(), align_of::<ffi::git_time>());
        assert_eq!(
            size_of::<GitTimeRef<'_>>(),
            size_of::<*const ffi::git_time>()
        );
        assert_eq!(size_of::<GitTimeMut<'_>>(), size_of::<*mut ffi::git_time>());
    }

    #[test]
    fn borrowed_time_handles_read_write_and_validate_fields() {
        let mut raw = ffi::git_time {
            time: -1,
            offset: -90,
            sign: b'-' as core::ffi::c_char,
        };

        // SAFETY: `raw` is initialized, non-null, exclusively borrowed for the
        // handle's lifetime, and remains live until the handle is last used.
        let mut time = unsafe { GitTimeMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(time.as_ref().time(), -1);
        assert_eq!(time.as_ref().offset(), -90);
        assert_eq!(time.as_ref().sign(), Ok(GitTimeSign::Negative));

        time.set_time(2);
        time.set_offset(30);
        time.set_sign(GitTimeSign::Positive);
        assert_eq!(time.as_ref().time(), 2);
        assert_eq!(time.as_ref().offset(), 30);
        assert_eq!(time.as_ref().sign(), Ok(GitTimeSign::Positive));

        raw.sign = b'?' as core::ffi::c_char;
        // SAFETY: `raw` remains initialized and live and is only shared for
        // this handle's lifetime.
        let time = unsafe { GitTimeRef::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(time.sign().unwrap_err().value(), b'?' as core::ffi::c_char);
    }
}
