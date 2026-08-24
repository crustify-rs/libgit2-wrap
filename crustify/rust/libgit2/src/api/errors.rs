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

#[cfg(test)]
mod tests {
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
