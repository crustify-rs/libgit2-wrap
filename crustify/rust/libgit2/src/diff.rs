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

/// Wraps: git_diff_binary_t
/// Describes the representation of one side of a binary diff.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiffBinaryKind {
    /// No binary data is present.
    None = ffi::git_diff_binary_t_GIT_DIFF_BINARY_NONE,
    /// The data contains the deflated full contents of the file.
    Literal = ffi::git_diff_binary_t_GIT_DIFF_BINARY_LITERAL,
    /// The data contains a deflated delta from the other side.
    Delta = ffi::git_diff_binary_t_GIT_DIFF_BINARY_DELTA,
}

/// A raw value that is not a valid [`DiffBinaryKind`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDiffBinaryKind(ffi::git_diff_binary_t);

impl InvalidDiffBinaryKind {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_diff_binary_t {
        self.0
    }
}

impl From<DiffBinaryKind> for ffi::git_diff_binary_t {
    fn from(kind: DiffBinaryKind) -> Self {
        kind as Self
    }
}

impl TryFrom<ffi::git_diff_binary_t> for DiffBinaryKind {
    type Error = InvalidDiffBinaryKind;

    fn try_from(kind: ffi::git_diff_binary_t) -> Result<Self, Self::Error> {
        match kind {
            ffi::git_diff_binary_t_GIT_DIFF_BINARY_NONE => Ok(Self::None),
            ffi::git_diff_binary_t_GIT_DIFF_BINARY_LITERAL => Ok(Self::Literal),
            ffi::git_diff_binary_t_GIT_DIFF_BINARY_DELTA => Ok(Self::Delta),
            value => Err(InvalidDiffBinaryKind(value)),
        }
    }
}

/// Wraps: git_diff_format_t
/// Selects the text format generated from a diff.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiffFormat {
    /// A full patch.
    Patch = ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH,
    /// Patch file headers without hunks.
    PatchHeader = ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH_HEADER,
    /// Raw diff output.
    Raw = ffi::git_diff_format_t_GIT_DIFF_FORMAT_RAW,
    /// File names only.
    NameOnly = ffi::git_diff_format_t_GIT_DIFF_FORMAT_NAME_ONLY,
    /// File names and status letters.
    NameStatus = ffi::git_diff_format_t_GIT_DIFF_FORMAT_NAME_STATUS,
    /// Patch-id input format.
    PatchId = ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH_ID,
}

/// A raw value that is not a valid [`DiffFormat`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDiffFormat(ffi::git_diff_format_t);

impl InvalidDiffFormat {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_diff_format_t {
        self.0
    }
}

impl From<DiffFormat> for ffi::git_diff_format_t {
    fn from(format: DiffFormat) -> Self {
        format as Self
    }
}

impl TryFrom<ffi::git_diff_format_t> for DiffFormat {
    type Error = InvalidDiffFormat;

    fn try_from(format: ffi::git_diff_format_t) -> Result<Self, Self::Error> {
        match format {
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH => Ok(Self::Patch),
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH_HEADER => Ok(Self::PatchHeader),
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_RAW => Ok(Self::Raw),
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_NAME_ONLY => Ok(Self::NameOnly),
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_NAME_STATUS => Ok(Self::NameStatus),
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH_ID => Ok(Self::PatchId),
            value => Err(InvalidDiffFormat(value)),
        }
    }
}

#[cfg(test)]
mod binary_format_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn binary_kinds_round_trip_and_reject_unknown_values() {
        for kind in [
            DiffBinaryKind::None,
            DiffBinaryKind::Literal,
            DiffBinaryKind::Delta,
        ] {
            let raw = ffi::git_diff_binary_t::from(kind);
            assert_eq!(DiffBinaryKind::try_from(raw), Ok(kind));
        }

        let invalid = ffi::git_diff_binary_t_GIT_DIFF_BINARY_DELTA + 1;
        assert_eq!(
            DiffBinaryKind::try_from(invalid).unwrap_err().value(),
            invalid
        );
    }

    #[test]
    fn diff_formats_round_trip_and_reject_unknown_values() {
        for format in [
            DiffFormat::Patch,
            DiffFormat::PatchHeader,
            DiffFormat::Raw,
            DiffFormat::NameOnly,
            DiffFormat::NameStatus,
            DiffFormat::PatchId,
        ] {
            let raw = ffi::git_diff_format_t::from(format);
            assert_eq!(DiffFormat::try_from(raw), Ok(format));
        }

        let invalid = ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH_ID + 1;
        assert_eq!(DiffFormat::try_from(invalid).unwrap_err().value(), invalid);
    }

    #[test]
    fn wrappers_match_their_c_enum_layouts() {
        assert_eq!(
            size_of::<DiffBinaryKind>(),
            size_of::<ffi::git_diff_binary_t>()
        );
        assert_eq!(
            align_of::<DiffBinaryKind>(),
            align_of::<ffi::git_diff_binary_t>()
        );
        assert_eq!(size_of::<DiffFormat>(), size_of::<ffi::git_diff_format_t>());
        assert_eq!(
            align_of::<DiffFormat>(),
            align_of::<ffi::git_diff_format_t>()
        );
    }
}
