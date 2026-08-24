//! Safe wrappers for libgit2 types APIs.

use core::ffi::CStr;
use core::ops::{BitOr, BitOrAssign};
use core::ptr::NonNull;

use crate::ffi;
use crate::util::alloc::GitStrdupFree;

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
///
/// Libgit2 does not treat `git_time.sign` as a validated discriminator: a
/// parsed signature is zeroed first, so the byte stays `0` when the buffer
/// carried no timezone field, and a malformed timezone marker is copied into
/// it verbatim. Reading it therefore validates rather than transmutes, and an
/// [`InvalidGitTimeSign`] is an ordinary outcome instead of a corruption
/// signal.
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
///
/// The common case is `0`, the value a signature parsed without a timezone
/// field keeps.
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
    /// Field: git_time.offset
    /// Returns the timezone offset from UTC in minutes.
    #[inline]
    #[must_use]
    pub fn offset(&self) -> core::ffi::c_int {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).offset).read() }
    }

    /// Field: git_time.sign
    /// Returns the stored timezone sign after validating the C byte.
    ///
    /// Returns `Err` for the zero byte left by a signature parsed without a
    /// timezone field, and for any other marker libgit2 copied verbatim out of
    /// a malformed buffer.
    #[inline]
    pub fn sign(&self) -> Result<GitTimeSign, InvalidGitTimeSign> {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        let raw = unsafe { core::ptr::addr_of!((*ptr).sign).read() };
        GitTimeSign::from_raw(raw).ok_or(InvalidGitTimeSign(raw))
    }

    /// Field: git_time.time
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

    #[test]
    fn a_signature_parsed_without_a_timezone_reports_no_sign() {
        // `git_signature__parse` zeroes the signature before parsing and only
        // writes `sign` from a timezone field, so this is the state of every
        // signature whose buffer carried none.
        let mut raw = ffi::git_time {
            time: 1_234_567_890,
            offset: 0,
            sign: 0,
        };

        // SAFETY: `raw` is initialized, non-null and live for this handle,
        // which is the only borrow of it.
        let time = unsafe { GitTimeRef::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(time.time(), 1_234_567_890);
        assert_eq!(time.offset(), 0);
        assert_eq!(time.sign().unwrap_err().value(), 0);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_writestream
    /// A polymorphic byte stream that dispatches through callbacks installed
    /// by its concrete implementation.
    GitWriteStream,
    GitWriteStreamRef,
    GitWriteStreamMut,
    ffi::git_writestream
);

/// An exclusively owned write stream whose `free` callback releases its
/// concrete allocation.
pub type GitWriteStreamOwned = ffibox::CBox<GitWriteStream>;

impl GitWriteStream {
    /// Field: git_writestream.close
    /// Finalizes `stream` and then releases it through its `free` callback.
    ///
    /// The stream is consumed because `close` is a one-way transition: a
    /// concrete implementation may tear its internal state down there, so no
    /// further operation stays valid. libgit2's blob stream, for example,
    /// runs `git_filebuf_cleanup`, which zeroes the file buffer including its
    /// `write` function pointer; a later write, close or commit would call
    /// through null. Only `free` remains valid, and it runs here on both the
    /// success and the failure path.
    pub fn close(mut stream: GitWriteStreamOwned) -> Result<(), core::ffi::c_int> {
        let raw = stream.as_mut().as_mut_ptr();
        // SAFETY: the owning handle addresses a live, fully constructed
        // stream, and raw-place projection reads its initialized callback
        // without forming a reference to C-visible memory.
        let close = unsafe { core::ptr::addr_of!((*raw).close).read() }
            .expect("a valid git_writestream has a close callback");
        // SAFETY: `close` is the callback installed for this live stream, and
        // owning it exclusively means no other Rust handle observes the call.
        let error = unsafe { close(raw) };
        // Releases the allocation through the `free` callback, the only
        // operation a closed stream still admits.
        drop(stream);
        if error < 0 { Err(error) } else { Ok(()) }
    }
}

impl GitWriteStreamMut<'_> {
    /// Field: git_writestream.write
    /// Writes `buffer` synchronously to this stream.
    pub fn write(&mut self, buffer: &[u8]) -> Result<(), core::ffi::c_int> {
        let stream = self.as_mut_ptr();
        // SAFETY: this exclusive handle addresses a live, fully constructed
        // stream, and raw-place projection reads its initialized callback
        // without forming a reference to C-visible memory.
        let write = unsafe { core::ptr::addr_of!((*stream).write).read() }
            .expect("a valid git_writestream has a write callback");
        // SAFETY: `write` is installed for this live stream. The slice keeps
        // exactly `buffer.len()` readable bytes alive for the synchronous
        // callback, and the exclusive handle prevents a concurrent Rust call.
        let error = unsafe {
            write(
                stream,
                buffer.as_ptr().cast::<core::ffi::c_char>(),
                buffer.len(),
            )
        };
        if error < 0 { Err(error) } else { Ok(()) }
    }
}

/// Field: git_writestream.free
// SAFETY: adopting `GitWriteStreamOwned` requires an owning pointer whose
// installed callback finalizes and releases that concrete allocation; borrowed
// stack-backed streams use handles instead. `CBox` invokes this callback once
// and never touches the allocation afterward.
unsafe impl ffibox::CDropped for GitWriteStream {
    unsafe fn c_drop(stream: NonNull<Self>) {
        let stream = stream.as_ptr().cast::<ffi::git_writestream>();
        // SAFETY: the `CDropped` contract supplies a live, fully constructed
        // stream, and raw-place projection reads its callback without forming
        // a reference to C-visible memory.
        let free = unsafe { core::ptr::addr_of!((*stream).free).read() }
            .expect("a valid git_writestream has a free callback");
        // SAFETY: `free` is the concrete destructor installed for this owned
        // stream and the `CDropped` contract grants its one final invocation.
        unsafe { free(stream) }
    }
}

#[cfg(test)]
mod writestream_tests {
    use super::*;
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};

    static WRITTEN_LEN: AtomicUsize = AtomicUsize::new(0);
    static FIRST_BYTE: AtomicUsize = AtomicUsize::new(0);
    static CLOSES: AtomicUsize = AtomicUsize::new(0);
    static FREES: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn test_write(
        _stream: *mut ffi::git_writestream,
        buffer: *const core::ffi::c_char,
        len: usize,
    ) -> core::ffi::c_int {
        WRITTEN_LEN.store(len, Ordering::SeqCst);
        if len != 0 {
            // SAFETY: the write-callback contract supplies at least `len`
            // readable bytes, so a nonempty call permits reading byte zero.
            let first = unsafe { buffer.cast::<u8>().read() };
            FIRST_BYTE.store(first.into(), Ordering::SeqCst);
        }
        0
    }

    unsafe extern "C" fn test_close(stream: *mut ffi::git_writestream) -> core::ffi::c_int {
        CLOSES.fetch_add(1, Ordering::SeqCst);
        // Mirrors `blob_writestream_close`, which zeroes its file buffer and
        // with it the `write` callback the stream would dispatch through.
        // SAFETY: the close-callback contract supplies the live stream this
        // callback was installed on, and no other handle observes the call.
        unsafe { core::ptr::addr_of_mut!((*stream).write).write(None) }
        0
    }

    unsafe extern "C" fn failing_close(_stream: *mut ffi::git_writestream) -> core::ffi::c_int {
        CLOSES.fetch_add(1, Ordering::SeqCst);
        -9
    }

    unsafe extern "C" fn test_free(stream: *mut ffi::git_writestream) {
        FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: this test adopts exactly one pointer produced by
        // `Box::into_raw`, and `CBox` calls this destructor exactly once.
        unsafe { drop(Box::from_raw(stream)) }
    }

    #[test]
    fn writestream_wrapper_preserves_layout() {
        assert_eq!(
            size_of::<GitWriteStream>(),
            size_of::<ffi::git_writestream>()
        );
        assert_eq!(
            align_of::<GitWriteStream>(),
            align_of::<ffi::git_writestream>()
        );
        assert_eq!(
            size_of::<GitWriteStreamRef<'_>>(),
            size_of::<*const ffi::git_writestream>()
        );
        assert_eq!(
            size_of::<GitWriteStreamMut<'_>>(),
            size_of::<*mut ffi::git_writestream>()
        );
        assert_eq!(
            size_of::<GitWriteStreamOwned>(),
            size_of::<*mut ffi::git_writestream>()
        );
    }

    #[test]
    fn owned_writestream_calls_operations_and_dynamic_destructor() {
        WRITTEN_LEN.store(0, Ordering::SeqCst);
        FIRST_BYTE.store(0, Ordering::SeqCst);
        CLOSES.store(0, Ordering::SeqCst);
        FREES.store(0, Ordering::SeqCst);

        let raw = Box::into_raw(Box::new(ffi::git_writestream {
            write: Some(test_write),
            close: Some(test_close),
            free: Some(test_free),
        }));
        // SAFETY: `raw` is a unique, fully initialized stream allocation and
        // its installed destructor reclaims the same allocation exactly once.
        let mut stream = unsafe { GitWriteStreamOwned::from_raw(raw) }
            .expect("Box::into_raw never returns null");

        assert_eq!(stream.as_mut().write(b"abc"), Ok(()));
        assert_eq!(WRITTEN_LEN.load(Ordering::SeqCst), 3);
        assert_eq!(FIRST_BYTE.load(Ordering::SeqCst), usize::from(b'a'));

        // Closing consumes the owner, so the torn-down stream this callback
        // leaves behind is unreachable, and `free` runs before returning.
        assert_eq!(GitWriteStream::close(stream), Ok(()));
        assert_eq!(CLOSES.load(Ordering::SeqCst), 1);
        assert_eq!(FREES.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn dropping_an_unclosed_writestream_runs_only_its_destructor() {
        CLOSES.store(0, Ordering::SeqCst);
        FREES.store(0, Ordering::SeqCst);

        let raw = Box::into_raw(Box::new(ffi::git_writestream {
            write: Some(test_write),
            close: Some(test_close),
            free: Some(test_free),
        }));
        // SAFETY: `raw` is a unique, fully initialized stream allocation and
        // its installed destructor reclaims the same allocation exactly once.
        let stream = unsafe { GitWriteStreamOwned::from_raw(raw) }
            .expect("Box::into_raw never returns null");

        drop(stream);
        assert_eq!(CLOSES.load(Ordering::SeqCst), 0);
        assert_eq!(FREES.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_failing_close_still_releases_the_writestream() {
        CLOSES.store(0, Ordering::SeqCst);
        FREES.store(0, Ordering::SeqCst);

        let raw = Box::into_raw(Box::new(ffi::git_writestream {
            write: Some(test_write),
            close: Some(failing_close),
            free: Some(test_free),
        }));
        // SAFETY: `raw` is a unique, fully initialized stream allocation and
        // its installed destructor reclaims the same allocation exactly once.
        let stream = unsafe { GitWriteStreamOwned::from_raw(raw) }
            .expect("Box::into_raw never returns null");

        assert_eq!(GitWriteStream::close(stream), Err(-9));
        assert_eq!(CLOSES.load(Ordering::SeqCst), 1);
        assert_eq!(FREES.load(Ordering::SeqCst), 1);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_signature
    /// A self-contained actor identity and action time allocated by libgit2.
    ///
    /// Every `GitSignature` place carries the `git_signature_free` contract:
    /// its `name` and `email` are distinct allocations made by libgit2's
    /// configured allocator, so both the destructor and the setters below
    /// release them with `git__free`. A signature whose strings came from
    /// somewhere else does not satisfy that contract — `git_signature__pdup`
    /// copies one into a `git_pool`, and such a value may only be viewed
    /// through a borrowed handle, never owned or mutated through one.
    GitSignature,
    GitSignatureRef,
    GitSignatureMut,
    ffi::git_signature
);

/// An owned signature released with `git_signature_free`.
pub type GitSignatureOwned = ffibox::CBox<GitSignature>;

/// An owned signature name or email allocated by libgit2.
pub type GitSignatureString = ffibox::CrustifyStr<GitStrdupFree>;

// SAFETY: `git_signature_free` releases a fully initialized signature header
// and its two uniquely owned strings. `GitSignature` is transparent over the
// corresponding bindgen layout.
ffibox::impl_dropped!(GitSignature, ffi::git_signature, ffi::git_signature_free);

// SAFETY: `git_signature_dup` deep-copies a live signature and both strings
// into a fresh allocation that is independently releasable by
// `git_signature_free`.
unsafe impl ffibox::CCloned for GitSignature {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` contract supplies a live initialized source,
        // and `duplicate` is a valid non-null output slot for the C call.
        let error = unsafe {
            ffi::git_signature_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_signature>(),
            )
        };
        if error < 0 {
            None
        } else {
            NonNull::new(duplicate.cast::<Self>())
        }
    }
}

impl<'a> GitSignatureRef<'a> {
    /// Field: git_signature.name
    /// Borrows the non-null, NUL-terminated actor name.
    ///
    /// The name may be **empty**. `git_signature_new` rejects an empty name,
    /// but `git_signature__parse` — which is what `git_signature_from_buffer`
    /// and every commit, tag and reflog parser use — fills the field with
    /// `extract_trimmed`, which strips leading and trailing control, space and
    /// punctuation bytes and yields `""` when nothing survives. Parsing
    /// `"<ada@example.com> 1234567890 +0000"` produces exactly that.
    #[must_use]
    pub fn name(&self) -> &'a CStr {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the signature header.
        let name = unsafe { core::ptr::addr_of!((*self.as_ptr()).name).read() };
        // SAFETY: every valid signature owns a non-null NUL-terminated name,
        // which remains live for this shared handle's lifetime.
        unsafe { CStr::from_ptr(name) }
    }

    /// Clones the actor name into a new independently owned allocation.
    pub fn try_clone_name(&self) -> Option<GitSignatureString> {
        clone_signature_string(self.name())
    }

    /// Field: git_signature.when
    /// Borrows the signature timestamp stored by value.
    #[must_use]
    pub fn when(&self) -> GitTimeRef<'a> {
        // SAFETY: raw-place projection from this live handle obtains the
        // inline field address without forming a reference to C-visible data.
        let when = unsafe { core::ptr::addr_of!((*self.as_ptr()).when).cast_mut() };
        // SAFETY: `when` projects the initialized inline field of this live
        // shared signature; the returned handle is bounded by the same borrow.
        unsafe { GitTimeRef::from_ptr(when) }.expect("an inline field is non-null")
    }

    /// Field: git_signature.email
    /// Borrows the non-null, NUL-terminated actor email address.
    ///
    /// Like [`Self::name`], the address may be empty: a parsed `"Ada <>"`
    /// leaves the field pointing at `""` rather than at nothing.
    #[must_use]
    pub fn email(&self) -> &'a CStr {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the signature header.
        let email = unsafe { core::ptr::addr_of!((*self.as_ptr()).email).read() };
        // SAFETY: every valid signature owns a non-null NUL-terminated email,
        // which remains live for this shared handle's lifetime.
        unsafe { CStr::from_ptr(email) }
    }

    /// Clones the email address into a new independently owned allocation.
    pub fn try_clone_email(&self) -> Option<GitSignatureString> {
        clone_signature_string(self.email())
    }
}

impl GitSignatureMut<'_> {
    /// Replaces the owned actor name and releases the previous allocation.
    ///
    /// The old name is released with `git__free`, which is what the type's
    /// `git_signature_free` contract promises it can be.
    pub fn set_name(&mut self, name: GitSignatureString) {
        let name = name.into_raw();
        let signature = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits replacing the owned pointer.
        // A valid signature's old name is non-null and uniquely owned.
        let old = unsafe {
            let old = core::ptr::addr_of!((*signature).name).read();
            core::ptr::addr_of_mut!((*signature).name).write(name);
            GitSignatureString::from_raw(old)
        }
        .expect("a valid signature has a non-null name");
        drop(old);
    }

    /// Exclusively borrows the signature timestamp stored by value.
    #[must_use]
    pub fn when_mut(&mut self) -> GitTimeMut<'_> {
        // SAFETY: raw-place projection from this exclusive handle obtains the
        // inline field address without forming a reference to C-visible data.
        let when = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).when) };
        // SAFETY: `when` projects the initialized inline field of this live
        // exclusive signature and the returned handle is tied to the reborrow.
        unsafe { GitTimeMut::from_ptr(when) }.expect("an inline field is non-null")
    }

    /// Replaces the owned email address and releases the previous allocation.
    ///
    /// The old address is released with `git__free`, as for [`Self::set_name`].
    pub fn set_email(&mut self, email: GitSignatureString) {
        let email = email.into_raw();
        let signature = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits replacing the owned pointer.
        // A valid signature's old email is non-null and uniquely owned.
        let old = unsafe {
            let old = core::ptr::addr_of!((*signature).email).read();
            core::ptr::addr_of_mut!((*signature).email).write(email);
            GitSignatureString::from_raw(old)
        }
        .expect("a valid signature has a non-null email");
        drop(old);
    }
}

fn clone_signature_string(value: &CStr) -> Option<GitSignatureString> {
    // SAFETY: `value` supplies a live NUL-terminated string for the duration
    // of this synchronous deep-copy call.
    let duplicate = unsafe { ffi::crustify_git__strdup(value.as_ptr()) };
    // SAFETY: a non-null result is a fresh NUL-terminated allocation made by
    // libgit2's configured allocator and matched by `GitStrdupFree`.
    unsafe { GitSignatureString::from_raw(duplicate) }
}

#[cfg(test)]
mod signature_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCloned, CDropped};

    use super::*;

    #[test]
    fn signature_wrapper_preserves_layout_and_lifecycle() {
        fn assert_lifecycle<T: CDropped + CCloned>() {}
        assert_lifecycle::<GitSignature>();
        assert_eq!(size_of::<GitSignature>(), size_of::<ffi::git_signature>());
        assert_eq!(align_of::<GitSignature>(), align_of::<ffi::git_signature>());
        assert_eq!(
            size_of::<GitSignatureRef<'_>>(),
            size_of::<*const ffi::git_signature>()
        );
        assert_eq!(
            size_of::<GitSignatureMut<'_>>(),
            size_of::<*mut ffi::git_signature>()
        );
        assert_eq!(
            size_of::<Option<GitSignatureOwned>>(),
            size_of::<*mut ffi::git_signature>()
        );
    }

    #[test]
    fn owned_signature_clones_strings_and_projects_time() {
        // SAFETY: libgit2 initialization is process-global and refcounted; the
        // successful call is balanced after every test allocation is dropped.
        let init_count = unsafe { ffi::git_libgit2_init() };
        assert!(init_count > 0);

        // SAFETY: libgit2 is initialized, so these calls use its configured
        // allocator and copy live NUL-terminated literals.
        let (header, name, email) = unsafe {
            (
                ffi::crustify_git__malloc(size_of::<ffi::git_signature>())
                    .cast::<ffi::git_signature>(),
                ffi::crustify_git__strdup(c"Ada".as_ptr()),
                ffi::crustify_git__strdup(c"ada@example.com".as_ptr()),
            )
        };
        assert!(!header.is_null());
        assert!(!name.is_null());
        assert!(!email.is_null());
        // SAFETY: `header` addresses suitably aligned storage large enough for
        // the bindgen type, and the two strings are fresh owned allocations.
        unsafe {
            header.write(ffi::git_signature {
                name,
                email,
                when: ffi::git_time {
                    time: 42,
                    offset: -60,
                    sign: b'-' as core::ffi::c_char,
                },
            });
        }
        // SAFETY: `header` is now one fully initialized, uniquely owned
        // signature allocation matched by `git_signature_free`.
        let mut signature = unsafe { GitSignatureOwned::from_raw(header) }.unwrap();

        assert_eq!(signature.as_ref().name(), c"Ada");
        assert_eq!(signature.as_ref().email(), c"ada@example.com");
        assert_eq!(signature.as_ref().when().time(), 42);
        assert_eq!(signature.as_ref().when().sign(), Ok(GitTimeSign::Negative));

        let cloned = signature.try_clone().expect("signature deep copy");
        assert_eq!(cloned.as_ref().name(), signature.as_ref().name());
        assert_ne!(
            cloned.as_ref().name().as_ptr(),
            signature.as_ref().name().as_ptr()
        );

        let replacement = signature
            .as_ref()
            .try_clone_email()
            .expect("email deep copy");
        signature.as_mut().set_name(replacement);
        signature.as_mut().when_mut().set_time(84);
        assert_eq!(signature.as_ref().name(), c"ada@example.com");
        assert_eq!(signature.as_ref().when().time(), 84);

        drop(cloned);
        drop(signature);
        // SAFETY: balances this test's successful initialization after all
        // libgit2 allocations have been released.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
    }

    #[test]
    fn an_empty_parsed_name_or_email_is_still_a_borrowable_string() {
        // `git_signature__parse` trims with `extract_trimmed`, so parsing
        // "<ada@example.com> 1234567890 +0000" leaves `name` pointing at a
        // non-null empty allocation rather than at nothing. Reproduce that
        // state directly: the getters must borrow it, not treat it as absent.
        // SAFETY: libgit2 initialization is process-global and refcounted; the
        // successful call is balanced after every test allocation is dropped.
        let init_count = unsafe { ffi::git_libgit2_init() };
        assert!(init_count > 0);

        // SAFETY: libgit2 is initialized, so these calls use its configured
        // allocator and copy live NUL-terminated literals.
        let (header, name, email) = unsafe {
            (
                ffi::crustify_git__malloc(size_of::<ffi::git_signature>())
                    .cast::<ffi::git_signature>(),
                ffi::crustify_git__strdup(c"".as_ptr()),
                ffi::crustify_git__strdup(c"".as_ptr()),
            )
        };
        assert!(!header.is_null());
        assert!(!name.is_null());
        assert!(!email.is_null());
        // SAFETY: `header` addresses suitably aligned storage large enough for
        // the bindgen type, and the two strings are fresh owned allocations.
        unsafe {
            header.write(ffi::git_signature {
                name,
                email,
                when: ffi::git_time {
                    time: 1_234_567_890,
                    offset: 0,
                    // The state a buffer without a timezone field leaves.
                    sign: 0,
                },
            });
        }
        // SAFETY: `header` is one fully initialized, uniquely owned signature
        // allocation whose strings are matched by `git_signature_free`.
        let signature = unsafe { GitSignatureOwned::from_raw(header) }.unwrap();

        assert_eq!(signature.as_ref().name(), c"");
        assert_eq!(signature.as_ref().email(), c"");
        assert!(signature.as_ref().name().to_bytes().is_empty());
        assert_eq!(signature.as_ref().try_clone_name().unwrap().as_c_str(), c"");
        assert_eq!(signature.as_ref().when().sign(), Err(InvalidGitTimeSign(0)));

        drop(signature);
        // SAFETY: balances this test's successful initialization after all
        // libgit2 allocations have been released.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
    }
}

/// Wraps: git_submodule_recurse_t
/// Selects whether an operation recurses into submodules.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GitSubmoduleRecurse {
    /// Do not recurse into submodules.
    #[default]
    No = ffi::git_submodule_recurse_t_GIT_SUBMODULE_RECURSE_NO,
    /// Always recurse into submodules.
    Yes = ffi::git_submodule_recurse_t_GIT_SUBMODULE_RECURSE_YES,
    /// Recurse only when the required commit is not already available.
    OnDemand = ffi::git_submodule_recurse_t_GIT_SUBMODULE_RECURSE_ONDEMAND,
}

/// A raw value that is not a published [`GitSubmoduleRecurse`] policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitSubmoduleRecurse(ffi::git_submodule_recurse_t);

impl InvalidGitSubmoduleRecurse {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_submodule_recurse_t {
        self.0
    }
}

impl From<GitSubmoduleRecurse> for ffi::git_submodule_recurse_t {
    fn from(recurse: GitSubmoduleRecurse) -> Self {
        recurse as Self
    }
}

impl TryFrom<ffi::git_submodule_recurse_t> for GitSubmoduleRecurse {
    type Error = InvalidGitSubmoduleRecurse;

    fn try_from(value: ffi::git_submodule_recurse_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_submodule_recurse_t_GIT_SUBMODULE_RECURSE_NO => Ok(Self::No),
            ffi::git_submodule_recurse_t_GIT_SUBMODULE_RECURSE_YES => Ok(Self::Yes),
            ffi::git_submodule_recurse_t_GIT_SUBMODULE_RECURSE_ONDEMAND => Ok(Self::OnDemand),
            value => Err(InvalidGitSubmoduleRecurse(value)),
        }
    }
}

#[cfg(test)]
mod submodule_recurse_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_recurse_policies_round_trip() {
        for policy in [
            GitSubmoduleRecurse::No,
            GitSubmoduleRecurse::Yes,
            GitSubmoduleRecurse::OnDemand,
        ] {
            let raw = ffi::git_submodule_recurse_t::from(policy);
            assert_eq!(GitSubmoduleRecurse::try_from(raw), Ok(policy));
        }

        let invalid = ffi::git_submodule_recurse_t_GIT_SUBMODULE_RECURSE_ONDEMAND + 1;
        assert_eq!(
            GitSubmoduleRecurse::try_from(invalid).unwrap_err().value(),
            invalid
        );
    }

    #[test]
    fn recurse_policy_preserves_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitSubmoduleRecurse>(),
            size_of::<ffi::git_submodule_recurse_t>()
        );
        assert_eq!(
            align_of::<GitSubmoduleRecurse>(),
            align_of::<ffi::git_submodule_recurse_t>()
        );
    }
}
