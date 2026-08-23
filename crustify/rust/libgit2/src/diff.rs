//! Safe wrappers for libgit2 diff APIs.

use crate::ffi;

/// The kind of change represented by a diff delta.
///
/// Wraps: git_delta_t
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(C)]
pub enum Delta {
    /// The file is unchanged.
    Unmodified = ffi::git_delta_t_GIT_DELTA_UNMODIFIED as isize,
    /// The file exists only in the new version.
    Added = ffi::git_delta_t_GIT_DELTA_ADDED as isize,
    /// The file exists only in the old version.
    Deleted = ffi::git_delta_t_GIT_DELTA_DELETED as isize,
    /// The file contents changed.
    Modified = ffi::git_delta_t_GIT_DELTA_MODIFIED as isize,
    /// The file was renamed.
    Renamed = ffi::git_delta_t_GIT_DELTA_RENAMED as isize,
    /// The file was copied from another old entry.
    Copied = ffi::git_delta_t_GIT_DELTA_COPIED as isize,
    /// The worktree entry is ignored.
    Ignored = ffi::git_delta_t_GIT_DELTA_IGNORED as isize,
    /// The worktree entry is untracked.
    Untracked = ffi::git_delta_t_GIT_DELTA_UNTRACKED as isize,
    /// The entry's type changed.
    Typechange = ffi::git_delta_t_GIT_DELTA_TYPECHANGE as isize,
    /// The entry could not be read.
    Unreadable = ffi::git_delta_t_GIT_DELTA_UNREADABLE as isize,
    /// The index entry is conflicted.
    Conflicted = ffi::git_delta_t_GIT_DELTA_CONFLICTED as isize,
}

/// A raw delta kind that is not defined by the linked libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDelta(pub ffi::git_delta_t);

impl TryFrom<ffi::git_delta_t> for Delta {
    type Error = InvalidDelta;

    fn try_from(raw: ffi::git_delta_t) -> Result<Self, Self::Error> {
        match raw {
            ffi::git_delta_t_GIT_DELTA_UNMODIFIED => Ok(Self::Unmodified),
            ffi::git_delta_t_GIT_DELTA_ADDED => Ok(Self::Added),
            ffi::git_delta_t_GIT_DELTA_DELETED => Ok(Self::Deleted),
            ffi::git_delta_t_GIT_DELTA_MODIFIED => Ok(Self::Modified),
            ffi::git_delta_t_GIT_DELTA_RENAMED => Ok(Self::Renamed),
            ffi::git_delta_t_GIT_DELTA_COPIED => Ok(Self::Copied),
            ffi::git_delta_t_GIT_DELTA_IGNORED => Ok(Self::Ignored),
            ffi::git_delta_t_GIT_DELTA_UNTRACKED => Ok(Self::Untracked),
            ffi::git_delta_t_GIT_DELTA_TYPECHANGE => Ok(Self::Typechange),
            ffi::git_delta_t_GIT_DELTA_UNREADABLE => Ok(Self::Unreadable),
            ffi::git_delta_t_GIT_DELTA_CONFLICTED => Ok(Self::Conflicted),
            raw => Err(InvalidDelta(raw)),
        }
    }
}

impl From<Delta> for ffi::git_delta_t {
    fn from(value: Delta) -> Self {
        value as Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn delta_values_round_trip_through_the_ffi_representation() {
        let values = [
            Delta::Unmodified,
            Delta::Added,
            Delta::Deleted,
            Delta::Modified,
            Delta::Renamed,
            Delta::Copied,
            Delta::Ignored,
            Delta::Untracked,
            Delta::Typechange,
            Delta::Unreadable,
            Delta::Conflicted,
        ];

        for value in values {
            let raw = ffi::git_delta_t::from(value);
            assert_eq!(Delta::try_from(raw), Ok(value));
        }
    }

    #[test]
    fn unknown_delta_values_are_rejected() {
        let raw = ffi::git_delta_t_GIT_DELTA_CONFLICTED + 1;
        assert_eq!(Delta::try_from(raw), Err(InvalidDelta(raw)));
    }

    #[test]
    fn delta_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<Delta>(), size_of::<ffi::git_delta_t>());
        assert_eq!(align_of::<Delta>(), align_of::<ffi::git_delta_t>());
    }
}
