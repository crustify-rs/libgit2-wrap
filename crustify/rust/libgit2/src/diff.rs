//! Safe wrappers for libgit2 diff APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CBox, CDropped};

use crate::api::buffer::GitBufMut;
use crate::api::deprecated::{DiffFormatEmailOptionsMut, DiffFormatEmailOptionsRef};
use crate::api::diff::{DiffBinaryRef, DiffDeltaRef, DiffLineOrigin, InvalidDiffLineOrigin};
use crate::ffi;

pub(crate) struct DiffCallbacks<'a> {
    pub(crate) file: Option<&'a mut dyn crate::api::diff::GitDiffFileCallback>,
    pub(crate) binary: Option<&'a mut dyn crate::api::diff::GitDiffBinaryCallback>,
    pub(crate) hunk: Option<&'a mut dyn crate::api::diff::GitDiffHunkCallback>,
    pub(crate) line: Option<&'a mut dyn crate::api::diff::GitDiffLineCallback>,
}

pub(crate) unsafe extern "C" fn diff_file_trampoline(
    delta: *const ffi::git_diff_delta,
    progress: f32,
    payload: *mut core::ffi::c_void,
) -> i32 {
    if delta.is_null() || payload.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: wrappers pass a live stack `DiffCallbacks` as payload and C
    // invokes this trampoline only before the synchronous call returns.
    let callbacks = unsafe { &mut *payload.cast::<DiffCallbacks<'_>>() };
    let Some(callback) = callbacks.file.as_deref_mut() else {
        return 0;
    };
    // SAFETY: libgit2 supplies a live initialized delta for this invocation.
    let delta = unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }.expect("checked non-null");
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        callback.call(delta, progress)
    }))
    .unwrap_or(ffi::git_error_code_GIT_ERROR)
}

pub(crate) unsafe extern "C" fn diff_binary_trampoline(
    delta: *const ffi::git_diff_delta,
    binary: *const ffi::git_diff_binary,
    payload: *mut core::ffi::c_void,
) -> i32 {
    if delta.is_null() || binary.is_null() || payload.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: the synchronous wrapper supplies this exact live payload.
    let callbacks = unsafe { &mut *payload.cast::<DiffCallbacks<'_>>() };
    let Some(callback) = callbacks.binary.as_deref_mut() else {
        return 0;
    };
    // SAFETY: both records are live initialized callback inputs.
    let delta = unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }.expect("checked non-null");
    // SAFETY: libgit2 supplies a live initialized binary record for this
    // synchronous invocation.
    let binary = unsafe { DiffBinaryRef::from_ptr(binary.cast_mut()) }.expect("checked non-null");
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        callback.call(
            delta,
            binary.contains_data(),
            binary.old_file(),
            binary.new_file(),
        )
    }))
    .unwrap_or(ffi::git_error_code_GIT_ERROR)
}

pub(crate) unsafe extern "C" fn diff_hunk_trampoline(
    delta: *const ffi::git_diff_delta,
    hunk: *const ffi::git_diff_hunk,
    payload: *mut core::ffi::c_void,
) -> i32 {
    if delta.is_null() || hunk.is_null() || payload.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: the synchronous wrapper supplies this exact live payload.
    let callbacks = unsafe { &mut *payload.cast::<DiffCallbacks<'_>>() };
    let Some(callback) = callbacks.hunk.as_deref_mut() else {
        return 0;
    };
    // SAFETY: libgit2 supplies live initialized callback records.
    let delta = unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }.expect("checked non-null");
    // SAFETY: as above, for the hunk record.
    let hunk = unsafe { DiffHunkRef::from_ptr(hunk.cast_mut()) }.expect("checked non-null");
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback.call(delta, hunk)))
        .unwrap_or(ffi::git_error_code_GIT_ERROR)
}

pub(crate) unsafe extern "C" fn diff_line_trampoline(
    delta: *const ffi::git_diff_delta,
    hunk: *const ffi::git_diff_hunk,
    line: *const ffi::git_diff_line,
    payload: *mut core::ffi::c_void,
) -> i32 {
    if delta.is_null() || line.is_null() || payload.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: the synchronous wrapper supplies this exact live payload.
    let callbacks = unsafe { &mut *payload.cast::<DiffCallbacks<'_>>() };
    let Some(callback) = callbacks.line.as_deref_mut() else {
        return 0;
    };
    // SAFETY: libgit2 supplies live initialized callback records.
    let delta = unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }.expect("checked non-null");
    // SAFETY: a null hunk is documented for formatted header lines; otherwise
    // it is a live initialized transient hunk.
    let hunk = unsafe { DiffHunkRef::from_ptr(hunk.cast_mut()) };
    // SAFETY: the line was checked non-null and is live for this invocation.
    let line = unsafe { DiffLineRef::from_ptr(line.cast_mut()) }.expect("checked non-null");
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        callback.call(delta, hunk, line)
    }))
    .unwrap_or(ffi::git_error_code_GIT_ERROR)
}

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
    /// Field: git_diff_patchid_options.version
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

ffibox::define_ctype!(
    /// Wraps: git_diff_hunk
    /// A span of modified lines and its textual header.
    DiffHunk,
    DiffHunkRef,
    DiffHunkMut,
    ffi::git_diff_hunk
);

impl<'a> DiffHunkRef<'a> {
    /// Field: git_diff_hunk.header
    /// Returns the initialized header bytes, excluding the trailing NUL.
    ///
    /// Returns `None` when a malformed C value has no room for its required
    /// trailing NUL byte.
    #[must_use]
    pub fn header(&self) -> Option<ffibox::CSlice<'a, u8>> {
        let len = self.header_len();
        if len >= ffi::GIT_DIFF_HUNK_HEADER_SIZE as usize {
            return None;
        }

        // SAFETY: this live shared handle covers the complete C value, and
        // raw-place projection does not form a reference to its header.
        let header = unsafe { core::ptr::addr_of!((*self.as_ptr()).header) }
            .cast::<u8>()
            .cast_mut();
        // SAFETY: `header` points into the live hunk carried by this handle,
        // and `len` was checked to stay within the initialized header prefix.
        Some(unsafe {
            ffibox::CSlice::from_raw_parts(core::ptr::NonNull::new_unchecked(header), len)
        })
    }

    /// Field: git_diff_hunk.header_len
    /// Returns the number of header bytes before the trailing NUL.
    #[must_use]
    pub fn header_len(&self) -> usize {
        // SAFETY: this live shared handle covers the complete C value, and
        // raw-place projection reads the scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).header_len).read() }
    }

    /// Field: git_diff_hunk.new_lines
    /// Returns the number of lines in the new file.
    #[must_use]
    pub fn new_lines(&self) -> i32 {
        // SAFETY: as `header_len`, for this initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).new_lines).read() }
    }

    /// Field: git_diff_hunk.new_start
    /// Returns the starting line number in the new file.
    #[must_use]
    pub fn new_start(&self) -> i32 {
        // SAFETY: as `header_len`, for this initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).new_start).read() }
    }

    /// Field: git_diff_hunk.old_lines
    /// Returns the number of lines in the old file.
    #[must_use]
    pub fn old_lines(&self) -> i32 {
        // SAFETY: as `header_len`, for this initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).old_lines).read() }
    }

    /// Field: git_diff_hunk.old_start
    /// Returns the starting line number in the old file.
    #[must_use]
    pub fn old_start(&self) -> i32 {
        // SAFETY: as `header_len`, for this initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).old_start).read() }
    }
}

impl DiffHunkMut<'_> {
    /// Replaces the header and maintains its length and trailing NUL.
    ///
    /// Returns `false` without modifying the hunk when `header` is too long.
    #[must_use]
    pub fn set_header(&mut self, header: &[u8]) -> bool {
        if header.len() >= ffi::GIT_DIFF_HUNK_HEADER_SIZE as usize {
            return false;
        }

        let ptr = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits writes to the hunk. The
        // capacity check leaves one byte for the terminator; the source Rust
        // slice cannot overlap C-visible hunk storage.
        unsafe {
            let destination = core::ptr::addr_of_mut!((*ptr).header).cast::<u8>();
            core::ptr::copy_nonoverlapping(header.as_ptr(), destination, header.len());
            destination.add(header.len()).write(0);
            core::ptr::addr_of_mut!((*ptr).header_len).write(header.len());
        }
        true
    }

    /// Sets the number of lines in the new file.
    pub fn set_new_lines(&mut self, value: i32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).new_lines).write(value) }
    }

    /// Sets the starting line number in the new file.
    pub fn set_new_start(&mut self, value: i32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).new_start).write(value) }
    }

    /// Sets the number of lines in the old file.
    pub fn set_old_lines(&mut self, value: i32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).old_lines).write(value) }
    }

    /// Sets the starting line number in the old file.
    pub fn set_old_start(&mut self, value: i32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).old_start).write(value) }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff_line
    /// A counted byte span and its position in a diff.
    DiffLine,
    DiffLineRef,
    DiffLineMut,
    ffi::git_diff_line
);

impl<'a> DiffLineRef<'a> {
    /// Field: git_diff_line.content_len
    /// Returns the number of bytes in the content span.
    #[must_use]
    pub fn content_len(&self) -> usize {
        // SAFETY: this live shared handle covers the complete C value, and
        // raw-place projection reads the scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).content_len).read() }
    }

    /// Field: git_diff_line.content
    /// Returns the non-NUL-terminated content as a borrowed counted view.
    ///
    /// An empty C value whose pointer is null is represented by `None`.
    #[must_use]
    pub fn content(&self) -> Option<ffibox::CSlice<'a, u8>> {
        // SAFETY: this live shared handle permits reading the pointer field by
        // raw-place projection without forming a reference to C-owned memory.
        let content = unsafe { core::ptr::addr_of!((*self.as_ptr()).content).read() };
        let content = core::ptr::NonNull::new(content.cast_mut().cast::<u8>())?;
        // SAFETY: a valid `git_diff_line` exposes `content_len` initialized
        // bytes at its non-null content pointer. Their backing owner keeps
        // them alive for at least the line handle's lifetime.
        Some(unsafe { ffibox::CSlice::from_raw_parts(content, self.content_len()) })
    }

    /// Field: git_diff_line.origin
    /// Returns the validated line-origin character code.
    pub fn origin(&self) -> Result<DiffLineOrigin, InvalidDiffLineOrigin> {
        // SAFETY: as `content_len`, for this initialized scalar field.
        let origin = unsafe { core::ptr::addr_of!((*self.as_ptr()).origin).read() };
        DiffLineOrigin::from_char(origin)
    }

    /// Field: git_diff_line.new_lineno
    /// Returns the new-file line number, or `-1` for a deleted line.
    #[must_use]
    pub fn new_lineno(&self) -> i32 {
        // SAFETY: as `content_len`, for this initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).new_lineno).read() }
    }

    /// Field: git_diff_line.old_lineno
    /// Returns the old-file line number, or `-1` for an added line.
    #[must_use]
    pub fn old_lineno(&self) -> i32 {
        // SAFETY: as `content_len`, for this initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).old_lineno).read() }
    }

    /// Field: git_diff_line.content_offset
    /// Returns the byte offset of the content in the original file.
    #[must_use]
    pub fn content_offset(&self) -> ffi::git_off_t {
        // SAFETY: as `content_len`, for this initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).content_offset).read() }
    }

    /// Field: git_diff_line.num_lines
    /// Returns the number of newline characters in the content.
    #[must_use]
    pub fn num_lines(&self) -> i32 {
        // SAFETY: as `content_len`, for this initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).num_lines).read() }
    }
}

impl DiffLineMut<'_> {
    /// Stores a borrowed content span and updates its count.
    ///
    /// # Safety
    ///
    /// The span must remain live for every later use of the line. The line
    /// must be in a borrowed-content or empty state, and no enclosing owner may
    /// later try to release the stored pointer as owned allocation.
    pub unsafe fn set_borrowed_content(&mut self, content: Option<&[u8]>) {
        let (content, len) = content.map_or((core::ptr::null(), 0), |content| {
            (content.as_ptr().cast::<core::ffi::c_char>(), content.len())
        });
        let ptr = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits raw-place writes. The caller
        // supplies the stored pointer's lifetime and ownership obligations.
        unsafe {
            core::ptr::addr_of_mut!((*ptr).content).write(content);
            core::ptr::addr_of_mut!((*ptr).content_len).write(len);
        }
    }

    /// Sets a published `git_diff_line_t` character code.
    pub fn set_origin(&mut self, value: DiffLineOrigin) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).origin).write(value.as_char()) }
    }

    /// Sets the new-file line number.
    pub fn set_new_lineno(&mut self, value: i32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).new_lineno).write(value) }
    }

    /// Sets the old-file line number.
    pub fn set_old_lineno(&mut self, value: i32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).old_lineno).write(value) }
    }

    /// Sets the byte offset of the content in the original file.
    pub fn set_content_offset(&mut self, value: ffi::git_off_t) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).content_offset).write(value) }
    }

    /// Sets the number of newline characters in the content.
    pub fn set_num_lines(&mut self, value: i32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).num_lines).write(value) }
    }
}

#[cfg(test)]
mod diff_record_tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn diff_record_wrappers_preserve_the_c_layouts() {
        assert_eq!(size_of::<DiffHunk>(), size_of::<ffi::git_diff_hunk>());
        assert_eq!(align_of::<DiffHunk>(), align_of::<ffi::git_diff_hunk>());
        assert_eq!(size_of::<DiffLine>(), size_of::<ffi::git_diff_line>());
        assert_eq!(align_of::<DiffLine>(), align_of::<ffi::git_diff_line>());
    }

    #[test]
    fn hunk_handles_read_and_write_fields() {
        let mut hunk = DiffHunk::zeroed();
        let raw = core::ptr::addr_of_mut!(hunk).cast::<ffi::git_diff_hunk>();
        // SAFETY: `raw` points to the initialized, layout-compatible stack
        // value above and this is its only active handle.
        let mut hunk = unsafe { DiffHunkMut::from_ptr(raw) }.unwrap();

        assert!(hunk.set_header(b"@@ -1,2 +3,4 @@\n"));
        hunk.set_old_start(1);
        hunk.set_old_lines(2);
        hunk.set_new_start(3);
        hunk.set_new_lines(4);

        let shared = hunk.as_ref();
        assert_eq!(shared.old_start(), 1);
        assert_eq!(shared.old_lines(), 2);
        assert_eq!(shared.new_start(), 3);
        assert_eq!(shared.new_lines(), 4);
        let header = shared.header().unwrap();
        assert_eq!(header.len(), shared.header_len());
        assert_eq!(header.elem(0), Some(b'@'));
        assert_eq!(header.elem(header.len() - 1), Some(b'\n'));

        let oversized = [b'x'; ffi::GIT_DIFF_HUNK_HEADER_SIZE as usize];
        assert!(!hunk.set_header(&oversized));
    }

    #[test]
    fn line_handles_keep_content_counted_and_borrowed() {
        let mut line = DiffLine::zeroed();
        let raw = core::ptr::addr_of_mut!(line).cast::<ffi::git_diff_line>();
        // SAFETY: `raw` points to the initialized, layout-compatible stack
        // value above and this is its only active handle.
        let mut line = unsafe { DiffLineMut::from_ptr(raw) }.unwrap();

        line.set_origin(DiffLineOrigin::Addition);
        line.set_old_lineno(-1);
        line.set_new_lineno(7);
        line.set_num_lines(1);
        line.set_content_offset(42);
        // SAFETY: the byte string has static storage, this standalone line has
        // no enclosing disposer, and its zeroed content pointer owns nothing.
        unsafe { line.set_borrowed_content(Some(b"hello\n")) };

        let shared = line.as_ref();
        assert_eq!(shared.origin(), Ok(DiffLineOrigin::Addition));
        assert_eq!(shared.old_lineno(), -1);
        assert_eq!(shared.new_lineno(), 7);
        assert_eq!(shared.num_lines(), 1);
        assert_eq!(shared.content_offset(), 42);
        assert_eq!(shared.content_len(), 6);
        let content = shared.content().unwrap();
        assert_eq!(content.elem(0), Some(b'h'));
        assert_eq!(content.elem(5), Some(b'\n'));
    }

    /// Collects a borrowed counted view into an owned buffer for comparison.
    fn collect(view: ffibox::CSlice<'_, u8>) -> Vec<u8> {
        (0..view.len())
            .map(|index| view.elem(index).expect("index is in range"))
            .collect()
    }

    #[test]
    fn parsed_patch_hunks_and_lines_read_real_libgit2_storage() {
        let _libgit2 = super::diff_tests::Libgit2Init::acquire();
        let text = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,2 +1,2 @@\n context\n-old\n+new\n";
        let mut diff = crate::diff_parse::git_diff_from_buffer(text).expect("a valid patch");
        let patch = crate::patch::git_patch_from_diff(&mut diff.as_mut(), 0)
            .expect("the single delta expands")
            .expect("a text delta produces a patch");
        let patch = patch.as_ref();

        let (hunk, line_count) =
            crate::patch::git_patch_get_hunk(patch, 0).expect("the single hunk");
        assert_eq!(hunk.old_start(), 1);
        assert_eq!(hunk.old_lines(), 2);
        assert_eq!(hunk.new_start(), 1);
        assert_eq!(hunk.new_lines(), 2);
        let header = hunk.header().expect("a NUL-terminated hunk header");
        assert_eq!(header.len(), hunk.header_len());
        assert_eq!(collect(header), b"@@ -1,2 +1,2 @@\n");
        assert_eq!(line_count, 3);

        let lines: Vec<(u8, i32, i32, Vec<u8>)> = (0..line_count)
            .map(|index| {
                let line = crate::patch::git_patch_get_line_in_hunk(patch, 0, index)
                    .expect("an in-range line");
                // A parsed patch owns its line bytes; the borrowed counted
                // view stays valid for the patch borrow either way.
                let content = line.content().expect("a parsed line carries content");
                assert_eq!(content.len(), line.content_len());
                assert_eq!(line.num_lines(), 1);
                #[allow(clippy::cast_sign_loss)]
                (
                    line.origin().expect("a published line origin").as_char() as u8,
                    line.old_lineno(),
                    line.new_lineno(),
                    collect(content),
                )
            })
            .collect();

        assert_eq!(
            lines,
            vec![
                (b' ', 1, 1, b"context\n".to_vec()),
                (b'-', 2, -1, b"old\n".to_vec()),
                (b'+', -1, 2, b"new\n".to_vec()),
            ]
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff
    /// An opaque diff result managed by libgit2.
    ///
    /// Use [`DiffOwned`] for an owned reference count. Some diff constructors
    /// retain a non-owning repository pointer, so their safe wrappers must keep
    /// that repository alive for every operation that can consult it.
    Diff,
    DiffRef,
    DiffMut,
    ffi::git_diff
);

/// Wraps: git_diff_free
/// An owned reference count to a [`Diff`].
///
/// Dropping it calls `git_diff_free`. The only routine that increments a
/// diff's reference count is `git_diff_addref`, declared in the private
/// `src/libgit2/diff_generate.h` and used internally by patch construction; it
/// is not exported, so this owner is not `Clone`.
pub type DiffOwned = CBox<Diff>;

// SAFETY: `git_diff_free` consumes one reference count to a fully constructed
// `git_diff`; on the final count it invokes the concrete generated- or
// parsed-diff destructor. Although the C function accepts null, `CBox` always
// supplies a live non-null pointer.
unsafe impl CDropped for Diff {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the `CDropped` contract guarantees that `obj` represents one
        // live owned count, and `Diff` is transparent over `ffi::git_diff`.
        unsafe { ffi::git_diff_free(obj.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod diff_tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::CCell;

    use super::*;

    pub(super) struct Libgit2Init;

    impl Libgit2Init {
        pub(super) fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and
            // refcounted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard, after the diff owner has already been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    #[test]
    fn opaque_diff_preserves_the_ffi_layout_and_handle_shape() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<Diff>();
        assert_dropped::<Diff>();
        assert_eq!(size_of::<Diff>(), size_of::<ffi::git_diff>());
        assert_eq!(align_of::<Diff>(), align_of::<ffi::git_diff>());
        assert_eq!(size_of::<DiffRef<'_>>(), size_of::<*const ffi::git_diff>());
        assert_eq!(size_of::<DiffMut<'_>>(), size_of::<*mut ffi::git_diff>());
        assert_eq!(size_of::<DiffOwned>(), size_of::<*mut ffi::git_diff>());
    }

    #[test]
    fn null_diff_seams_create_no_handle_or_owner() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(DiffRef::from_ptr(ptr::null_mut()).is_none());
            assert!(DiffMut::from_ptr(ptr::null_mut()).is_none());
            assert!(DiffOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn parsed_diff_supports_owned_shared_and_exclusive_handles() {
        let _libgit2 = Libgit2Init::acquire();
        let mut raw = ptr::null_mut();

        // SAFETY: `raw` is a valid out-slot and the empty input is readable for
        // zero bytes. On success libgit2 initializes `raw` with one owned diff
        // reference count.
        let status = unsafe { ffi::git_diff_from_buffer(&mut raw, b"".as_ptr().cast(), 0) };
        assert_eq!(status, 0);

        // SAFETY: the successful constructor transferred one non-null owned
        // count through `raw`, which has not been adopted elsewhere.
        let mut diff = unsafe { DiffOwned::from_raw(raw) }.expect("empty parsed diff");
        assert_eq!(diff.as_ref().as_ptr(), raw.cast_const());
        assert_eq!(diff.as_mut().as_mut_ptr(), raw);
        assert_eq!(diff.as_mut().as_ref().as_ptr(), raw.cast_const());
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff_binary_file
    /// One owned, compressed side of a binary diff.
    ///
    /// The header owns its `data` span: every libgit2 producer allocates the
    /// deflated bytes through the configured allocator and releases them with
    /// `git__free`, leaving the field null when the side carries no binary
    /// content. Borrowing a header through `from_ptr` therefore also asserts
    /// that invariant, which [`DiffBinaryFileMut::take_data`] and
    /// [`DiffBinaryFileMut::set_data`] rely on to move the allocation.
    DiffBinaryFile,
    DiffBinaryFileRef,
    DiffBinaryFileMut,
    ffi::git_diff_binary_file
);

/// Deleter for a binary-diff byte allocation detached from its header.
pub struct DiffBinaryDataFree;

// SAFETY: a valid binary file's non-null data pointer uniquely owns an
// allocation made through libgit2's configured allocator. As elsewhere in
// this crate, that allocator must remain compatible until the owner is
// dropped.
unsafe impl ffibox::CLenDropped for DiffBinaryDataFree {
    unsafe fn c_drop_len(ptr: *mut u8, _byte_len: usize) {
        // SAFETY: the `CLenDropped` contract guarantees unique ownership of a
        // compatible libgit2 allocation; `git__free` does not need its length.
        unsafe { ffi::crustify_git__free(ptr.cast()) }
    }
}

/// Compressed bytes detached from a [`DiffBinaryFile`].
pub type DiffBinaryData = ffibox::CVec<u8, DiffBinaryDataFree>;

impl<'a> DiffBinaryFileRef<'a> {
    /// Field: git_diff_binary_file.data
    /// Borrows the compressed binary data.
    ///
    /// A null pointer is represented by `None`, including a side for which no
    /// binary content was generated.
    #[must_use]
    pub fn data(&self) -> Option<ffibox::CSlice<'a, u8>> {
        let file = self.as_ptr();
        // SAFETY: both fields are initialized members of this live shared
        // handle and are read with raw-place projections.
        let (data, len) = unsafe {
            (
                addr_of!((*file).data).read().cast_mut().cast::<u8>(),
                addr_of!((*file).datalen).read(),
            )
        };
        let data = NonNull::new(data)?;
        // SAFETY: a valid binary file exposes `len` initialized bytes at its
        // non-null data pointer. The view is bounded by this handle's borrow.
        Some(unsafe { ffibox::CSlice::from_raw_parts(data, len) })
    }

    /// Field: git_diff_binary_file.type
    /// Returns the binary representation kind, rejecting unknown C values.
    pub fn kind(&self) -> Result<DiffBinaryKind, InvalidDiffBinaryKind> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let kind = unsafe { addr_of!((*self.as_ptr()).type_).read() };
        DiffBinaryKind::try_from(kind)
    }

    /// Field: git_diff_binary_file.datalen
    /// Returns the compressed byte count.
    #[must_use]
    pub fn data_len(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).datalen).read() }
    }

    /// Field: git_diff_binary_file.inflatedlen
    /// Returns the byte count after inflation.
    #[must_use]
    pub fn inflated_len(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).inflatedlen).read() }
    }
}

impl DiffBinaryFileMut<'_> {
    /// Sets the binary representation kind.
    pub fn set_kind(&mut self, kind: DiffBinaryKind) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).type_).write(kind.into()) }
    }

    /// Sets the byte count after inflation.
    pub fn set_inflated_len(&mut self, inflated_len: usize) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).inflatedlen).write(inflated_len) }
    }

    /// Moves out the compressed allocation, leaving a null, empty span.
    #[must_use]
    pub fn take_data(&mut self) -> Option<DiffBinaryData> {
        let file = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits transferring the pointer and
        // its count together, then leaving the valid null/zero representation.
        let (data, len) = unsafe {
            let data = addr_of!((*file).data).read().cast_mut().cast::<u8>();
            let len = addr_of!((*file).datalen).read();
            addr_of_mut!((*file).data).write(core::ptr::null());
            addr_of_mut!((*file).datalen).write(0);
            (data, len)
        };
        // SAFETY: a non-null pointer moved from a valid binary file uniquely
        // owns `len` initialized bytes from libgit2's configured allocator.
        unsafe { DiffBinaryData::from_raw_parts(data, len) }
    }

    /// Replaces the compressed allocation and disposes the previous one.
    pub fn set_data(&mut self, data: Option<DiffBinaryData>) {
        let (data, len) = data.map_or((core::ptr::null_mut(), 0), ffibox::CVec::into_raw_parts);
        let old = self.take_data();
        let file = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits installing the compatible
        // allocation and its exact initialized length after clearing the old
        // span.
        unsafe {
            addr_of_mut!((*file).data).write(data.cast());
            addr_of_mut!((*file).datalen).write(len);
        }
        drop(old);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff_file
    /// One file-like side of a diff delta.
    DiffFile,
    DiffFileRef,
    DiffFileMut,
    ffi::git_diff_file
);

impl<'a> DiffFileRef<'a> {
    /// Field: git_diff_file.size
    /// Returns the entry size in bytes.
    #[must_use]
    pub fn size(&self) -> u64 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).size).read() }
    }

    /// Field: git_diff_file.path
    /// Borrows the optional NUL-terminated repository-relative path.
    #[must_use]
    pub fn path(&self) -> Option<&'a core::ffi::CStr> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to the C object.
        let path = unsafe { addr_of!((*self.as_ptr()).path).read() };
        if path.is_null() {
            return None;
        }
        // SAFETY: a valid diff file's non-null path is NUL-terminated and is
        // kept alive by the enclosing diff or patch for the handle's lifetime.
        Some(unsafe { core::ffi::CStr::from_ptr(path) })
    }

    /// Field: git_diff_file.mode
    /// Returns the published file mode, or `None` for a malformed C value.
    #[must_use]
    pub fn mode(&self) -> Option<crate::api::types::GitFileMode> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let mode = unsafe { addr_of!((*self.as_ptr()).mode).read() };
        crate::api::types::GitFileMode::from_raw(mode.into())
    }

    /// Field: git_diff_file.flags
    /// Returns the checked file state flags.
    pub fn flags(&self) -> Result<crate::api::diff::GitDiffFlags, crate::ffi::git_diff_flag_t> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let bits = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        crate::api::diff::GitDiffFlags::from_bits(bits).ok_or(bits)
    }

    /// Field: git_diff_file.id
    /// Borrows the inline object identifier.
    #[must_use]
    pub fn id(&self) -> crate::oid::OidRef<'a> {
        // SAFETY: raw-place projection reaches the live inline `git_oid`
        // without forming a reference to it.
        let id = unsafe { addr_of!((*self.as_ptr()).id) }.cast_mut();
        // SAFETY: `id` is non-null, initialized, and remains part of this diff
        // file for the shared handle's full `'a` lifetime.
        unsafe { crate::oid::OidRef::from_ptr(id) }.expect("an inline field is non-null")
    }

    /// Field: git_diff_file.id_abbrev
    /// Returns the known hexadecimal object-ID width.
    #[must_use]
    pub fn id_abbrev(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).id_abbrev).read() }
    }
}

impl DiffFileMut<'_> {
    /// Sets the entry size in bytes.
    pub fn set_size(&mut self, size: u64) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).size).write(size) }
    }

    /// Stores a borrowed NUL-terminated repository-relative path.
    ///
    /// # Safety
    ///
    /// A non-null `path` must remain alive and NUL-terminated for every later
    /// use of the diff file, including uses after this handle is released.
    pub unsafe fn set_borrowed_path(&mut self, path: Option<&core::ffi::CStr>) {
        let path = path.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        // SAFETY: this exclusive handle permits the raw-place write, and the
        // caller upholds the stored pointer's lifetime and validity.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).path).write(path) }
    }

    /// Clears the optional borrowed path.
    pub fn clear_path(&mut self) {
        // SAFETY: null stores no borrow and creates no lifetime obligation.
        unsafe { self.set_borrowed_path(None) }
    }

    /// Sets the published file mode.
    pub fn set_mode(&mut self, mode: crate::api::types::GitFileMode) {
        let mode = mode.as_raw();
        debug_assert!(u16::try_from(mode).is_ok());
        // SAFETY: every published libgit2 file mode fits the field's 16 bits,
        // and this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).mode).write(mode as u16) }
    }

    /// Sets the file state flags.
    pub fn set_flags(&mut self, flags: crate::api::diff::GitDiffFlags) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Borrows the inline object identifier exclusively.
    #[must_use]
    pub fn id_mut(&mut self) -> crate::oid::OidMut<'_> {
        // SAFETY: raw-place projection reaches the live inline `git_oid`
        // without forming a reference to it.
        let id = unsafe { addr_of_mut!((*self.as_mut_ptr()).id) };
        // SAFETY: `id` is non-null and initialized, and the returned handle is
        // bounded by this exclusive reborrow.
        unsafe { crate::oid::OidMut::from_ptr(id) }.expect("an inline field is non-null")
    }

    /// Sets the known hexadecimal object-ID width.
    pub fn set_id_abbrev(&mut self, id_abbrev: u16) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).id_abbrev).write(id_abbrev) }
    }
}

#[cfg(test)]
mod diff_file_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn wrappers_preserve_c_layout_and_handle_shape() {
        assert_eq!(
            size_of::<DiffBinaryFile>(),
            size_of::<ffi::git_diff_binary_file>()
        );
        assert_eq!(
            align_of::<DiffBinaryFile>(),
            align_of::<ffi::git_diff_binary_file>()
        );
        assert_eq!(
            size_of::<DiffBinaryFileRef<'_>>(),
            size_of::<*const ffi::git_diff_binary_file>()
        );
        assert_eq!(size_of::<DiffFile>(), size_of::<ffi::git_diff_file>());
        assert_eq!(align_of::<DiffFile>(), align_of::<ffi::git_diff_file>());
        assert_eq!(
            size_of::<DiffFileMut<'_>>(),
            size_of::<*mut ffi::git_diff_file>()
        );
    }

    #[test]
    fn binary_file_handles_read_and_update_fields() {
        // An empty side owns nothing, so this header satisfies the type's
        // data-ownership invariant without holding a libgit2 allocation.
        let mut raw = ffi::git_diff_binary_file {
            type_: ffi::git_diff_binary_t_GIT_DIFF_BINARY_LITERAL,
            data: core::ptr::null(),
            datalen: 0,
            inflatedlen: 42,
        };

        // SAFETY: `raw` is initialized and exclusively borrowed for the
        // handle's lifetime.
        let mut file = unsafe { DiffBinaryFileMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(file.as_ref().kind(), Ok(DiffBinaryKind::Literal));
        assert!(file.as_ref().data().is_none());
        assert_eq!(file.as_ref().data_len(), 0);
        assert_eq!(file.as_ref().inflated_len(), 42);

        file.set_kind(DiffBinaryKind::Delta);
        file.set_inflated_len(84);
        assert_eq!(file.as_ref().kind(), Ok(DiffBinaryKind::Delta));
        assert_eq!(file.as_ref().inflated_len(), 84);

        let unknown = ffi::git_diff_binary_t_GIT_DIFF_BINARY_DELTA + 1;
        raw.type_ = unknown;
        // SAFETY: the previous handle is dead and `raw` is still a live,
        // initialized header with no owned data.
        let file = unsafe { DiffBinaryFileRef::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(
            file.kind().map_err(InvalidDiffBinaryKind::value),
            Err(unknown)
        );
    }

    #[test]
    fn binary_data_moves_through_the_header_without_double_free() {
        // SAFETY: libgit2 initialization is process-global and refcounted;
        // this test balances its successful acquisition below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let source = b"data";
        // SAFETY: libgit2 is initialized, so a non-null result is a fresh
        // allocation from its configured allocator.
        let allocation = unsafe { ffi::crustify_git__malloc(source.len()) }.cast::<u8>();
        assert!(!allocation.is_null());
        // SAFETY: `allocation` has `source.len()` writable bytes and is fresh,
        // so the source and destination do not overlap.
        unsafe { allocation.copy_from_nonoverlapping(source.as_ptr(), source.len()) }
        // SAFETY: the fresh allocation uniquely owns exactly `source.len()`
        // initialized bytes and matches `DiffBinaryDataFree`.
        let owned = unsafe { DiffBinaryData::from_raw_parts(allocation, source.len()) }
            .expect("libgit2 allocated the binary data");

        let mut raw = ffi::git_diff_binary_file {
            type_: ffi::git_diff_binary_t_GIT_DIFF_BINARY_LITERAL,
            data: core::ptr::null(),
            datalen: 0,
            inflatedlen: source.len(),
        };
        // SAFETY: `raw` is initialized and exclusively borrowed for the
        // handle's lifetime.
        let mut file = unsafe { DiffBinaryFileMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        file.set_data(Some(owned));
        assert_eq!(file.as_ref().data_len(), source.len());

        let mut copied = [0; 4];
        assert!(
            file.as_ref()
                .data()
                .expect("non-null data")
                .copy_to_slice(&mut copied)
        );
        assert_eq!(&copied, source);

        let detached = file.take_data().expect("the installed allocation");
        assert_eq!(detached.as_slice(), source);
        assert!(file.as_ref().data().is_none());
        assert_eq!(file.as_ref().data_len(), 0);
        drop(detached);

        // SAFETY: balances this test's successful initialization after its
        // libgit2 allocation has been released.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn diff_file_handles_project_inline_and_borrowed_fields() {
        let mut raw = ffi::git_diff_file {
            id: ffi::git_oid {
                type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
                id: [0; 32],
            },
            path: c"old.txt".as_ptr(),
            size: 7,
            flags: 3,
            mode: crate::api::types::GitFileMode::BLOB.as_raw() as u16,
            id_abbrev: 7,
        };

        // SAFETY: `raw` is initialized and exclusively borrowed for the
        // handle's lifetime; both test paths have static storage.
        let mut file = unsafe { DiffFileMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(file.as_ref().path(), Some(c"old.txt"));
        assert_eq!(file.as_ref().size(), 7);
        assert_eq!(
            file.as_ref().flags(),
            Ok(crate::api::diff::GitDiffFlags::BINARY | crate::api::diff::GitDiffFlags::NOT_BINARY)
        );
        assert_eq!(
            file.as_ref().mode(),
            Some(crate::api::types::GitFileMode::BLOB)
        );
        assert_eq!(file.as_ref().id_abbrev(), 7);

        file.set_size(11);
        file.set_flags(crate::api::diff::GitDiffFlags::VALID_SIZE);
        file.set_mode(crate::api::types::GitFileMode::BLOB_EXECUTABLE);
        file.set_id_abbrev(12);
        // SAFETY: the static path outlives every use of `file` and `raw`.
        unsafe { file.set_borrowed_path(Some(c"new.txt")) }
        file.id_mut().set_oid_type(crate::oid::OidType::Sha256);

        {
            let shared = file.as_ref();
            assert_eq!(shared.path(), Some(c"new.txt"));
            assert_eq!(shared.size(), 11);
            assert_eq!(
                shared.flags(),
                Ok(crate::api::diff::GitDiffFlags::VALID_SIZE)
            );
            assert_eq!(
                shared.mode(),
                Some(crate::api::types::GitFileMode::BLOB_EXECUTABLE)
            );
            assert_eq!(shared.id_abbrev(), 12);
            assert_eq!(shared.id().oid_type(), Ok(crate::oid::OidType::Sha256));
        }
        file.clear_path();
        assert_eq!(file.as_ref().path(), None);
    }
}

/// Wraps: git_diff_is_sorted_icase
/// Returns whether the diff's deltas use case-insensitive ordering.
#[must_use]
pub fn git_diff_is_sorted_icase(diff: DiffRef<'_>) -> bool {
    // SAFETY: `diff` is a live shared handle and C only reads its ordering
    // configuration for this call.
    unsafe { ffi::git_diff_is_sorted_icase(diff.as_ptr()) != 0 }
}

/// Wraps: git_diff_num_deltas
/// Returns the number of deltas in `diff`.
#[must_use]
pub fn git_diff_num_deltas(diff: DiffRef<'_>) -> usize {
    // SAFETY: `diff` is a live shared handle and C performs a scalar read.
    unsafe { ffi::git_diff_num_deltas(diff.as_ptr()) }
}

/// Wraps: git_diff_patchid
/// Computes the stable patch identifier for a diff.
pub fn git_diff_patchid(
    diff: &mut DiffMut<'_>,
    options: Option<DiffPatchIdOptions>,
) -> Result<crate::oid::Oid, i32> {
    let mut out = crate::oid::Oid::zeroed();
    let out_ptr = core::ptr::addr_of_mut!(out).cast::<ffi::git_oid>();
    // Keep a by-value options layout in stable local storage for the call,
    // avoiding any Rust reference over its C-visible bytes.
    let status = match options {
        Some(mut options) => {
            let options = core::ptr::addr_of_mut!(options).cast::<ffi::git_diff_patchid_options>();
            // SAFETY: `out_ptr` and `options` address initialized,
            // layout-compatible storage and `diff` is exclusively borrowed.
            unsafe { ffi::git_diff_patchid(out_ptr, diff.as_mut_ptr(), options) }
        }
        None => {
            // SAFETY: `out_ptr` is writable, `diff` is exclusively borrowed,
            // and null requests the published default options.
            unsafe { ffi::git_diff_patchid(out_ptr, diff.as_mut_ptr(), core::ptr::null_mut()) }
        }
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_diff_patchid_options_init
/// Constructs patch-id options for the current ABI version.
pub fn git_diff_patchid_options_init() -> Result<DiffPatchIdOptions, i32> {
    let mut options = DiffPatchIdOptions::zeroed();
    let raw = core::ptr::addr_of_mut!(options).cast::<ffi::git_diff_patchid_options>();
    // SAFETY: `raw` addresses writable, layout-compatible storage and the
    // requested version is published by the headers used to build this crate.
    let status =
        unsafe { ffi::git_diff_patchid_options_init(raw, ffi::GIT_DIFF_PATCHID_OPTIONS_VERSION) };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_diff_format_email
/// Appends the deprecated e-mail representation of `diff` to `out`.
///
/// The diff is borrowed exclusively. Formatting walks it through
/// `git_diff_get_stats` and `git_patch_from_diff`, both of which up-reference
/// the diff and let patch generation record binary and load state back into
/// its deltas, so this is not a read-only traversal.
///
/// libgit2 requires `options` to carry a non-null summary, ID and author; a
/// value missing any of them fails with an error status rather than
/// formatting.
pub fn git_diff_format_email(
    out: &mut GitBufMut<'_>,
    diff: &mut DiffMut<'_>,
    options: DiffFormatEmailOptionsRef<'_>,
) -> Result<(), i32> {
    // SAFETY: all three typed borrows are live for this synchronous call. The
    // exclusive diff handle permits the refcount and delta-state writes patch
    // generation performs, and C retains no pointer after it returns.
    let status = unsafe {
        ffi::git_diff_format_email(out.as_mut_ptr(), diff.as_mut_ptr(), options.as_ptr())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_diff_format_email_options_init
/// Initializes deprecated e-mail options for the requested ABI version.
pub fn git_diff_format_email_options_init(
    options: &mut DiffFormatEmailOptionsMut<'_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle exposes writable layout-compatible options
    // storage, and C retains no pointer after initialization.
    let status = unsafe { ffi::git_diff_format_email_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_diff_get_delta
/// Borrows a delta by zero-based index.
#[must_use]
pub fn git_diff_get_delta<'a>(diff: DiffRef<'a>, index: usize) -> Option<DiffDeltaRef<'a>> {
    // SAFETY: `diff` is live and shared; the accessor only reads its delta
    // vector, and a non-null result remains owned by that diff.
    let delta = unsafe { ffi::git_diff_get_delta(diff.as_ptr(), index) };
    // SAFETY: null denotes an out-of-range index. Otherwise the returned
    // pointer is internal to `diff` and lives for the carried `'a` borrow.
    unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }
}

#[cfg(test)]
mod scheduled_accessor_tests {
    use super::*;
    use crate::api::deprecated::DiffFormatEmailOptions;

    #[test]
    fn email_options_initializer_writes_documented_defaults() {
        let mut raw = DiffFormatEmailOptions::zeroed();
        // SAFETY: `raw` is live layout-compatible storage exclusively accessed
        // through this handle for the duration of the call.
        let mut options = unsafe {
            DiffFormatEmailOptionsMut::from_ptr(
                core::ptr::addr_of_mut!(raw).cast::<ffi::git_diff_format_email_options>(),
            )
        }
        .unwrap();
        git_diff_format_email_options_init(
            &mut options,
            ffi::GIT_DIFF_FORMAT_EMAIL_OPTIONS_VERSION,
        )
        .unwrap();
        assert_eq!(options.as_ref().patch_no(), 1);
        assert_eq!(options.as_ref().total_patches(), 1);
    }

    #[test]
    fn delta_lookup_carries_the_diff_lifetime() {
        let _: for<'a> fn(DiffRef<'a>, usize) -> Option<DiffDeltaRef<'a>> = git_diff_get_delta;
    }
}

/// Wraps: git_diff_find_init_options
/// Initializes deprecated rename-detection options for `version`.
pub fn git_diff_find_init_options(
    options: &mut crate::api::diff::DiffFindOptionsMut<'_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // options storage and the initializer retains no pointer to it.
    let status = unsafe { ffi::git_diff_find_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_diff_foreach
/// Traverses a diff synchronously through optional typed callbacks.
pub fn git_diff_foreach<'callbacks>(
    diff: &mut DiffMut<'_>,
    file: Option<&'callbacks mut dyn crate::api::diff::GitDiffFileCallback>,
    binary: Option<&'callbacks mut dyn crate::api::diff::GitDiffBinaryCallback>,
    hunk: Option<&'callbacks mut dyn crate::api::diff::GitDiffHunkCallback>,
    line: Option<&'callbacks mut dyn crate::api::diff::GitDiffLineCallback>,
) -> Result<(), i32> {
    let mut callbacks = DiffCallbacks {
        file,
        binary,
        hunk,
        line,
    };
    let file = callbacks.file.as_ref().map(|_| diff_file_trampoline as _);
    let binary = callbacks
        .binary
        .as_ref()
        .map(|_| diff_binary_trampoline as _);
    let hunk = callbacks.hunk.as_ref().map(|_| diff_hunk_trampoline as _);
    let line = callbacks.line.as_ref().map(|_| diff_line_trampoline as _);
    // SAFETY: the diff is exclusive and every callback plus the stack payload
    // remains live for the complete synchronous traversal.
    let status = unsafe {
        ffi::git_diff_foreach(
            diff.as_mut_ptr(),
            file,
            binary,
            hunk,
            line,
            core::ptr::from_mut(&mut callbacks).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_diff_init_options
/// Initializes deprecated diff-options storage for `version`.
pub fn git_diff_init_options(
    options: &mut crate::api::diff::GitDiffOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // options storage and the initializer retains no pointer to it.
    let status = unsafe { ffi::git_diff_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_symbol_tests {
    use super::*;

    #[test]
    fn deprecated_initializers_write_current_defaults() {
        let mut diff = crate::api::diff::GitDiffOptions::new();
        git_diff_init_options(&mut diff.as_mut(), ffi::GIT_DIFF_OPTIONS_VERSION).unwrap();
        assert_eq!(diff.as_ref().version(), ffi::GIT_DIFF_OPTIONS_VERSION);

        let mut find = crate::api::diff::DiffFindOptions::new();
        git_diff_find_init_options(&mut find.as_mut(), ffi::GIT_DIFF_FIND_OPTIONS_VERSION).unwrap();
        assert_eq!(find.as_ref().version(), ffi::GIT_DIFF_FIND_OPTIONS_VERSION);
    }

    #[test]
    fn file_trampoline_delivers_a_typed_transient_delta() {
        // SAFETY: all-zero is valid for this C record's scalar, pointer, and
        // inline object-ID fields; status is replaced before it is observed.
        let mut raw: ffi::git_diff_delta = unsafe { core::mem::zeroed() };
        raw.status = ffi::git_delta_t_GIT_DELTA_ADDED;
        let mut seen = false;
        let mut file = |delta: DiffDeltaRef<'_>, progress: f32| {
            seen = delta.status() == Ok(Delta::Added) && progress == 0.5;
            0
        };
        let mut callbacks = DiffCallbacks {
            file: Some(&mut file),
            binary: None,
            hunk: None,
            line: None,
        };
        // SAFETY: both raw records remain live and the payload has the exact
        // stack type expected by the trampoline for this synchronous call.
        let status =
            unsafe { diff_file_trampoline(&raw, 0.5, core::ptr::from_mut(&mut callbacks).cast()) };
        assert_eq!(status, 0);
        assert!(seen);
    }
}

/// Wraps: git_diff_num_deltas_of_type
/// Counts deltas having `kind`.
#[must_use]
pub fn git_diff_num_deltas_of_type(diff: DiffRef<'_>, kind: Delta) -> usize {
    // SAFETY: `diff` is live and the operation only scans its delta vector.
    unsafe { ffi::git_diff_num_deltas_of_type(diff.as_ptr(), kind.into()) }
}
