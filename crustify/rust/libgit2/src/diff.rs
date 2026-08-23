//! Safe wrappers for libgit2 diff APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use core::ptr::{addr_of, addr_of_mut};

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

/// Wraps: git_diff_stats_format_t
/// A checked set of formats to include when rendering diff statistics.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct DiffStatsFormat(ffi::git_diff_stats_format_t);

impl DiffStatsFormat {
    /// Do not render any statistics.
    pub const NONE: Self = Self(ffi::git_diff_stats_format_t_GIT_DIFF_STATS_NONE);
    /// Render full per-file statistics, like `--stat`.
    pub const FULL: Self = Self(ffi::git_diff_stats_format_t_GIT_DIFF_STATS_FULL);
    /// Render abbreviated totals, like `--shortstat`.
    pub const SHORT: Self = Self(ffi::git_diff_stats_format_t_GIT_DIFF_STATS_SHORT);
    /// Render numeric per-file statistics, like `--numstat`.
    pub const NUMBER: Self = Self(ffi::git_diff_stats_format_t_GIT_DIFF_STATS_NUMBER);
    /// Include creations, renames, and mode changes.
    pub const INCLUDE_SUMMARY: Self =
        Self(ffi::git_diff_stats_format_t_GIT_DIFF_STATS_INCLUDE_SUMMARY);
    /// Every format published by this version of libgit2.
    pub const ALL: Self =
        Self(Self::FULL.0 | Self::SHORT.0 | Self::NUMBER.0 | Self::INCLUDE_SUMMARY.0);

    /// Converts raw bits when they contain only published formats.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_diff_stats_format_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_diff_stats_format_t {
        self.0
    }

    /// Returns whether no format is selected.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every format in `other` is selected.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any format in `other` is selected.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<DiffStatsFormat> for ffi::git_diff_stats_format_t {
    fn from(format: DiffStatsFormat) -> Self {
        format.bits()
    }
}

impl BitOr for DiffStatsFormat {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for DiffStatsFormat {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for DiffStatsFormat {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for DiffStatsFormat {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

#[cfg(test)]
mod diff_stats_format_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn diff_stats_formats_form_checked_sets() {
        let mut formats = DiffStatsFormat::FULL | DiffStatsFormat::INCLUDE_SUMMARY;
        assert!(formats.contains(DiffStatsFormat::FULL));
        assert!(formats.intersects(DiffStatsFormat::INCLUDE_SUMMARY));
        assert!(!formats.intersects(DiffStatsFormat::NUMBER));

        formats |= DiffStatsFormat::NUMBER;
        formats &= DiffStatsFormat::NUMBER | DiffStatsFormat::INCLUDE_SUMMARY;
        assert_eq!(
            formats,
            DiffStatsFormat::NUMBER | DiffStatsFormat::INCLUDE_SUMMARY
        );
    }

    #[test]
    fn raw_diff_stats_bits_are_validated() {
        assert!(DiffStatsFormat::NONE.is_empty());
        assert_eq!(
            DiffStatsFormat::from_bits(DiffStatsFormat::ALL.bits()),
            Some(DiffStatsFormat::ALL)
        );
        assert_eq!(DiffStatsFormat::from_bits(1 << 31), None);
    }

    #[test]
    fn diff_stats_format_preserves_the_c_enum_layout() {
        assert_eq!(
            size_of::<DiffStatsFormat>(),
            size_of::<ffi::git_diff_stats_format_t>()
        );
        assert_eq!(
            align_of::<DiffStatsFormat>(),
            align_of::<ffi::git_diff_stats_format_t>()
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff_patchid_options
    /// Options controlling patch-id calculation.
    DiffPatchIdOptions,
    DiffPatchIdOptionsRef,
    DiffPatchIdOptionsMut,
    ffi::git_diff_patchid_options
);

impl DiffPatchIdOptionsRef<'_> {
    /// Wraps: git_diff_patchid_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and `addr_of!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }
}

impl DiffPatchIdOptionsMut<'_> {
    /// Sets the ABI version of this options value.
    pub fn set_version(&mut self, version: u32) {
        // SAFETY: `self` carries an exclusive borrow of the complete C object,
        // and `addr_of_mut!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }
}

#[cfg(test)]
mod patch_id_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn patch_id_options_preserve_the_c_layout() {
        assert_eq!(
            size_of::<DiffPatchIdOptions>(),
            size_of::<ffi::git_diff_patchid_options>()
        );
        assert_eq!(
            align_of::<DiffPatchIdOptions>(),
            align_of::<ffi::git_diff_patchid_options>()
        );
        assert_eq!(
            size_of::<DiffPatchIdOptionsRef<'_>>(),
            size_of::<*const ffi::git_diff_patchid_options>()
        );
        assert_eq!(
            size_of::<DiffPatchIdOptionsMut<'_>>(),
            size_of::<*mut ffi::git_diff_patchid_options>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_the_version() {
        let mut options = DiffPatchIdOptions::zeroed();
        let raw = addr_of_mut!(options).cast::<ffi::git_diff_patchid_options>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above, and this is its only active handle.
        let mut options = unsafe { DiffPatchIdOptionsMut::from_ptr(raw) }
            .expect("the address of a stack value is non-null");

        options.set_version(1);
        assert_eq!(options.as_ref().version(), 1);
    }
}
