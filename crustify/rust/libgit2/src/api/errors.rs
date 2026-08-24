//! Safe wrappers for libgit2 errors APIs.

use crate::ffi;

/// Wraps: git_error_code
/// A generic success or failure code published by libgit2.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitErrorCode {
    /// The operation completed successfully.
    Ok = ffi::git_error_code_GIT_OK,
    /// A generic error occurred; `git_error_last` may provide details.
    Error = ffi::git_error_code_GIT_ERROR,
    /// The requested object could not be found.
    NotFound = ffi::git_error_code_GIT_ENOTFOUND,
    /// An existing object prevented the operation.
    Exists = ffi::git_error_code_GIT_EEXISTS,
    /// More than one object matched.
    Ambiguous = ffi::git_error_code_GIT_EAMBIGUOUS,
    /// An output buffer was too short.
    BufferTooShort = ffi::git_error_code_GIT_EBUFS,
    /// A callback requested user-controlled termination.
    User = ffi::git_error_code_GIT_EUSER,
    /// The operation is unavailable for a bare repository.
    BareRepository = ffi::git_error_code_GIT_EBAREREPO,
    /// `HEAD` refers to an unborn branch.
    UnbornBranch = ffi::git_error_code_GIT_EUNBORNBRANCH,
    /// A merge is still in progress.
    Unmerged = ffi::git_error_code_GIT_EUNMERGED,
    /// The reference could not be fast-forwarded.
    NonFastForward = ffi::git_error_code_GIT_ENONFASTFORWARD,
    /// A name or reference specification was invalid.
    InvalidSpec = ffi::git_error_code_GIT_EINVALIDSPEC,
    /// Checkout conflicts prevented the operation.
    Conflict = ffi::git_error_code_GIT_ECONFLICT,
    /// A lock file prevented the operation.
    Locked = ffi::git_error_code_GIT_ELOCKED,
    /// A reference value did not match the expected value.
    Modified = ffi::git_error_code_GIT_EMODIFIED,
    /// Authentication failed.
    Authentication = ffi::git_error_code_GIT_EAUTH,
    /// A server certificate was invalid.
    Certificate = ffi::git_error_code_GIT_ECERTIFICATE,
    /// A patch or merge was already applied.
    Applied = ffi::git_error_code_GIT_EAPPLIED,
    /// The requested peel operation was not possible.
    Peel = ffi::git_error_code_GIT_EPEEL,
    /// Input ended unexpectedly.
    EndOfFile = ffi::git_error_code_GIT_EEOF,
    /// The operation or its input was invalid.
    Invalid = ffi::git_error_code_GIT_EINVALID,
    /// Uncommitted index changes prevented the operation.
    Uncommitted = ffi::git_error_code_GIT_EUNCOMMITTED,
    /// The operation is not valid for a directory.
    Directory = ffi::git_error_code_GIT_EDIRECTORY,
    /// A merge conflict exists and the operation cannot continue.
    MergeConflict = ffi::git_error_code_GIT_EMERGECONFLICT,
    /// A user-configured callback declined to handle the operation.
    Passthrough = ffi::git_error_code_GIT_PASSTHROUGH,
    /// An iterator reached its end.
    IterationOver = ffi::git_error_code_GIT_ITEROVER,
    /// The internal operation should be retried.
    Retry = ffi::git_error_code_GIT_RETRY,
    /// A computed hash did not match its expected value.
    Mismatch = ffi::git_error_code_GIT_EMISMATCH,
    /// Unsaved index changes would be overwritten.
    IndexDirty = ffi::git_error_code_GIT_EINDEXDIRTY,
    /// Applying a patch failed.
    ApplyFailed = ffi::git_error_code_GIT_EAPPLYFAIL,
    /// The current user does not own the object.
    Owner = ffi::git_error_code_GIT_EOWNER,
    /// The operation timed out.
    Timeout = ffi::git_error_code_GIT_TIMEOUT,
    /// The operation produced no changes.
    Unchanged = ffi::git_error_code_GIT_EUNCHANGED,
    /// The requested option is unsupported.
    NotSupported = ffi::git_error_code_GIT_ENOTSUPPORTED,
    /// The subject is read-only.
    ReadOnly = ffi::git_error_code_GIT_EREADONLY,
}

/// A raw return code not published in `git_error_code`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitErrorCode(ffi::git_error_code);

impl InvalidGitErrorCode {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_error_code {
        self.0
    }
}

impl From<GitErrorCode> for ffi::git_error_code {
    fn from(code: GitErrorCode) -> Self {
        code as Self
    }
}

impl TryFrom<ffi::git_error_code> for GitErrorCode {
    type Error = InvalidGitErrorCode;

    fn try_from(code: ffi::git_error_code) -> Result<Self, InvalidGitErrorCode> {
        match code {
            ffi::git_error_code_GIT_OK => Ok(Self::Ok),
            ffi::git_error_code_GIT_ERROR => Ok(Self::Error),
            ffi::git_error_code_GIT_ENOTFOUND => Ok(Self::NotFound),
            ffi::git_error_code_GIT_EEXISTS => Ok(Self::Exists),
            ffi::git_error_code_GIT_EAMBIGUOUS => Ok(Self::Ambiguous),
            ffi::git_error_code_GIT_EBUFS => Ok(Self::BufferTooShort),
            ffi::git_error_code_GIT_EUSER => Ok(Self::User),
            ffi::git_error_code_GIT_EBAREREPO => Ok(Self::BareRepository),
            ffi::git_error_code_GIT_EUNBORNBRANCH => Ok(Self::UnbornBranch),
            ffi::git_error_code_GIT_EUNMERGED => Ok(Self::Unmerged),
            ffi::git_error_code_GIT_ENONFASTFORWARD => Ok(Self::NonFastForward),
            ffi::git_error_code_GIT_EINVALIDSPEC => Ok(Self::InvalidSpec),
            ffi::git_error_code_GIT_ECONFLICT => Ok(Self::Conflict),
            ffi::git_error_code_GIT_ELOCKED => Ok(Self::Locked),
            ffi::git_error_code_GIT_EMODIFIED => Ok(Self::Modified),
            ffi::git_error_code_GIT_EAUTH => Ok(Self::Authentication),
            ffi::git_error_code_GIT_ECERTIFICATE => Ok(Self::Certificate),
            ffi::git_error_code_GIT_EAPPLIED => Ok(Self::Applied),
            ffi::git_error_code_GIT_EPEEL => Ok(Self::Peel),
            ffi::git_error_code_GIT_EEOF => Ok(Self::EndOfFile),
            ffi::git_error_code_GIT_EINVALID => Ok(Self::Invalid),
            ffi::git_error_code_GIT_EUNCOMMITTED => Ok(Self::Uncommitted),
            ffi::git_error_code_GIT_EDIRECTORY => Ok(Self::Directory),
            ffi::git_error_code_GIT_EMERGECONFLICT => Ok(Self::MergeConflict),
            ffi::git_error_code_GIT_PASSTHROUGH => Ok(Self::Passthrough),
            ffi::git_error_code_GIT_ITEROVER => Ok(Self::IterationOver),
            ffi::git_error_code_GIT_RETRY => Ok(Self::Retry),
            ffi::git_error_code_GIT_EMISMATCH => Ok(Self::Mismatch),
            ffi::git_error_code_GIT_EINDEXDIRTY => Ok(Self::IndexDirty),
            ffi::git_error_code_GIT_EAPPLYFAIL => Ok(Self::ApplyFailed),
            ffi::git_error_code_GIT_EOWNER => Ok(Self::Owner),
            ffi::git_error_code_GIT_TIMEOUT => Ok(Self::Timeout),
            ffi::git_error_code_GIT_EUNCHANGED => Ok(Self::Unchanged),
            ffi::git_error_code_GIT_ENOTSUPPORTED => Ok(Self::NotSupported),
            ffi::git_error_code_GIT_EREADONLY => Ok(Self::ReadOnly),
            value => Err(InvalidGitErrorCode(value)),
        }
    }
}

/// Wraps: git_error_t
/// A checked category attached to libgit2's thread-local error record.
#[non_exhaustive]
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitErrorClass {
    /// No error category.
    None = ffi::git_error_t_GIT_ERROR_NONE,
    /// Memory allocation failed.
    NoMemory = ffi::git_error_t_GIT_ERROR_NOMEMORY,
    /// Operating-system error.
    Os = ffi::git_error_t_GIT_ERROR_OS,
    /// Invalid input or state.
    Invalid = ffi::git_error_t_GIT_ERROR_INVALID,
    /// Reference error.
    Reference = ffi::git_error_t_GIT_ERROR_REFERENCE,
    /// Compression error.
    Zlib = ffi::git_error_t_GIT_ERROR_ZLIB,
    /// Repository error.
    Repository = ffi::git_error_t_GIT_ERROR_REPOSITORY,
    /// Configuration error.
    Config = ffi::git_error_t_GIT_ERROR_CONFIG,
    /// Regular-expression error.
    Regex = ffi::git_error_t_GIT_ERROR_REGEX,
    /// Object-database error.
    Odb = ffi::git_error_t_GIT_ERROR_ODB,
    /// Index error.
    Index = ffi::git_error_t_GIT_ERROR_INDEX,
    /// Git object error.
    Object = ffi::git_error_t_GIT_ERROR_OBJECT,
    /// Network error.
    Net = ffi::git_error_t_GIT_ERROR_NET,
    /// Tag error.
    Tag = ffi::git_error_t_GIT_ERROR_TAG,
    /// Tree error.
    Tree = ffi::git_error_t_GIT_ERROR_TREE,
    /// Indexer error.
    Indexer = ffi::git_error_t_GIT_ERROR_INDEXER,
    /// TLS error.
    Ssl = ffi::git_error_t_GIT_ERROR_SSL,
    /// Submodule error.
    Submodule = ffi::git_error_t_GIT_ERROR_SUBMODULE,
    /// Threading error.
    Thread = ffi::git_error_t_GIT_ERROR_THREAD,
    /// Stash error.
    Stash = ffi::git_error_t_GIT_ERROR_STASH,
    /// Checkout error.
    Checkout = ffi::git_error_t_GIT_ERROR_CHECKOUT,
    /// FETCH_HEAD error.
    FetchHead = ffi::git_error_t_GIT_ERROR_FETCHHEAD,
    /// Merge error.
    Merge = ffi::git_error_t_GIT_ERROR_MERGE,
    /// SSH error.
    Ssh = ffi::git_error_t_GIT_ERROR_SSH,
    /// Filter error.
    Filter = ffi::git_error_t_GIT_ERROR_FILTER,
    /// Revert error.
    Revert = ffi::git_error_t_GIT_ERROR_REVERT,
    /// Callback error.
    Callback = ffi::git_error_t_GIT_ERROR_CALLBACK,
    /// Cherry-pick error.
    CherryPick = ffi::git_error_t_GIT_ERROR_CHERRYPICK,
    /// Describe error.
    Describe = ffi::git_error_t_GIT_ERROR_DESCRIBE,
    /// Rebase error.
    Rebase = ffi::git_error_t_GIT_ERROR_REBASE,
    /// Filesystem error.
    Filesystem = ffi::git_error_t_GIT_ERROR_FILESYSTEM,
    /// Patch error.
    Patch = ffi::git_error_t_GIT_ERROR_PATCH,
    /// Worktree error.
    Worktree = ffi::git_error_t_GIT_ERROR_WORKTREE,
    /// Object-ID hashing error.
    Sha = ffi::git_error_t_GIT_ERROR_SHA,
    /// HTTP error.
    Http = ffi::git_error_t_GIT_ERROR_HTTP,
    /// Internal libgit2 error.
    Internal = ffi::git_error_t_GIT_ERROR_INTERNAL,
    /// Grafts error.
    Grafts = ffi::git_error_t_GIT_ERROR_GRAFTS,
}

impl GitErrorClass {
    /// Converts a raw C enum value when it is published by this libgit2 API.
    #[must_use]
    pub const fn from_raw(raw: ffi::git_error_t) -> Option<Self> {
        match raw {
            ffi::git_error_t_GIT_ERROR_NONE => Some(Self::None),
            ffi::git_error_t_GIT_ERROR_NOMEMORY => Some(Self::NoMemory),
            ffi::git_error_t_GIT_ERROR_OS => Some(Self::Os),
            ffi::git_error_t_GIT_ERROR_INVALID => Some(Self::Invalid),
            ffi::git_error_t_GIT_ERROR_REFERENCE => Some(Self::Reference),
            ffi::git_error_t_GIT_ERROR_ZLIB => Some(Self::Zlib),
            ffi::git_error_t_GIT_ERROR_REPOSITORY => Some(Self::Repository),
            ffi::git_error_t_GIT_ERROR_CONFIG => Some(Self::Config),
            ffi::git_error_t_GIT_ERROR_REGEX => Some(Self::Regex),
            ffi::git_error_t_GIT_ERROR_ODB => Some(Self::Odb),
            ffi::git_error_t_GIT_ERROR_INDEX => Some(Self::Index),
            ffi::git_error_t_GIT_ERROR_OBJECT => Some(Self::Object),
            ffi::git_error_t_GIT_ERROR_NET => Some(Self::Net),
            ffi::git_error_t_GIT_ERROR_TAG => Some(Self::Tag),
            ffi::git_error_t_GIT_ERROR_TREE => Some(Self::Tree),
            ffi::git_error_t_GIT_ERROR_INDEXER => Some(Self::Indexer),
            ffi::git_error_t_GIT_ERROR_SSL => Some(Self::Ssl),
            ffi::git_error_t_GIT_ERROR_SUBMODULE => Some(Self::Submodule),
            ffi::git_error_t_GIT_ERROR_THREAD => Some(Self::Thread),
            ffi::git_error_t_GIT_ERROR_STASH => Some(Self::Stash),
            ffi::git_error_t_GIT_ERROR_CHECKOUT => Some(Self::Checkout),
            ffi::git_error_t_GIT_ERROR_FETCHHEAD => Some(Self::FetchHead),
            ffi::git_error_t_GIT_ERROR_MERGE => Some(Self::Merge),
            ffi::git_error_t_GIT_ERROR_SSH => Some(Self::Ssh),
            ffi::git_error_t_GIT_ERROR_FILTER => Some(Self::Filter),
            ffi::git_error_t_GIT_ERROR_REVERT => Some(Self::Revert),
            ffi::git_error_t_GIT_ERROR_CALLBACK => Some(Self::Callback),
            ffi::git_error_t_GIT_ERROR_CHERRYPICK => Some(Self::CherryPick),
            ffi::git_error_t_GIT_ERROR_DESCRIBE => Some(Self::Describe),
            ffi::git_error_t_GIT_ERROR_REBASE => Some(Self::Rebase),
            ffi::git_error_t_GIT_ERROR_FILESYSTEM => Some(Self::Filesystem),
            ffi::git_error_t_GIT_ERROR_PATCH => Some(Self::Patch),
            ffi::git_error_t_GIT_ERROR_WORKTREE => Some(Self::Worktree),
            ffi::git_error_t_GIT_ERROR_SHA => Some(Self::Sha),
            ffi::git_error_t_GIT_ERROR_HTTP => Some(Self::Http),
            ffi::git_error_t_GIT_ERROR_INTERNAL => Some(Self::Internal),
            ffi::git_error_t_GIT_ERROR_GRAFTS => Some(Self::Grafts),
            _ => None,
        }
    }

    /// Returns the underlying C enum value.
    #[must_use]
    pub const fn as_raw(self) -> ffi::git_error_t {
        self as ffi::git_error_t
    }

    /// Returns the integer form used by `git_error` records and setters.
    #[must_use]
    pub const fn as_c_int(self) -> core::ffi::c_int {
        self.as_raw() as core::ffi::c_int
    }
}

/// An integer error class not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitErrorClass(core::ffi::c_int);

impl InvalidGitErrorClass {
    /// Returns the unrecognized integer.
    #[must_use]
    pub const fn value(self) -> core::ffi::c_int {
        self.0
    }
}

impl From<GitErrorClass> for ffi::git_error_t {
    fn from(class: GitErrorClass) -> Self {
        class.as_raw()
    }
}

impl TryFrom<core::ffi::c_int> for GitErrorClass {
    type Error = InvalidGitErrorClass;

    fn try_from(raw: core::ffi::c_int) -> Result<Self, Self::Error> {
        if raw < 0 {
            return Err(InvalidGitErrorClass(raw));
        }
        Self::from_raw(raw as ffi::git_error_t).ok_or(InvalidGitErrorClass(raw))
    }
}

#[cfg(test)]
mod code_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    const PUBLISHED_CODES: [GitErrorCode; 35] = [
        GitErrorCode::Ok,
        GitErrorCode::Error,
        GitErrorCode::NotFound,
        GitErrorCode::Exists,
        GitErrorCode::Ambiguous,
        GitErrorCode::BufferTooShort,
        GitErrorCode::User,
        GitErrorCode::BareRepository,
        GitErrorCode::UnbornBranch,
        GitErrorCode::Unmerged,
        GitErrorCode::NonFastForward,
        GitErrorCode::InvalidSpec,
        GitErrorCode::Conflict,
        GitErrorCode::Locked,
        GitErrorCode::Modified,
        GitErrorCode::Authentication,
        GitErrorCode::Certificate,
        GitErrorCode::Applied,
        GitErrorCode::Peel,
        GitErrorCode::EndOfFile,
        GitErrorCode::Invalid,
        GitErrorCode::Uncommitted,
        GitErrorCode::Directory,
        GitErrorCode::MergeConflict,
        GitErrorCode::Passthrough,
        GitErrorCode::IterationOver,
        GitErrorCode::Retry,
        GitErrorCode::Mismatch,
        GitErrorCode::IndexDirty,
        GitErrorCode::ApplyFailed,
        GitErrorCode::Owner,
        GitErrorCode::Timeout,
        GitErrorCode::Unchanged,
        GitErrorCode::NotSupported,
        GitErrorCode::ReadOnly,
    ];

    #[test]
    fn every_published_error_code_round_trips() {
        for code in PUBLISHED_CODES {
            let raw = ffi::git_error_code::from(code);
            assert_eq!(GitErrorCode::try_from(raw), Ok(code));
        }
    }

    #[test]
    fn unpublished_error_codes_are_rejected() {
        for raw in [-2, -25, 1] {
            assert_eq!(GitErrorCode::try_from(raw).unwrap_err().value(), raw);
        }
    }

    #[test]
    fn error_code_matches_the_c_abi_scalar() {
        assert_eq!(size_of::<GitErrorCode>(), size_of::<ffi::git_error_code>());
        assert_eq!(
            align_of::<GitErrorCode>(),
            align_of::<ffi::git_error_code>()
        );
    }
}

#[cfg(test)]
mod class_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn every_published_error_class_round_trips() {
        for raw in ffi::git_error_t_GIT_ERROR_NONE..=ffi::git_error_t_GIT_ERROR_GRAFTS {
            let class = GitErrorClass::from_raw(raw).expect("published classes are dense");
            assert_eq!(class.as_raw(), raw);
            assert_eq!(GitErrorClass::try_from(raw as core::ffi::c_int), Ok(class));
        }
    }

    #[test]
    fn unknown_error_classes_are_rejected() {
        let unknown = ffi::git_error_t_GIT_ERROR_GRAFTS as core::ffi::c_int + 1;
        assert_eq!(GitErrorClass::from_raw(unknown as ffi::git_error_t), None);
        assert_eq!(
            GitErrorClass::try_from(unknown).unwrap_err().value(),
            unknown
        );
        assert_eq!(GitErrorClass::try_from(-1).unwrap_err().value(), -1);
    }

    #[test]
    fn error_class_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<GitErrorClass>(), size_of::<ffi::git_error_t>());
        assert_eq!(align_of::<GitErrorClass>(), align_of::<ffi::git_error_t>());
    }
}
