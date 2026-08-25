//! Safe wrappers for libgit2 diff APIs.

use core::marker::PhantomData;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::types::{GitSubmoduleIgnore, InvalidGitSubmoduleIgnore};
use crate::ffi;
use crate::oid::{InvalidOidType, OidType};
use crate::strarray::GitStrArrayRef;

/// Wraps: git_diff_line_t
/// A validated origin code for a line or header emitted by libgit2.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiffLineOrigin {
    /// An unchanged context line.
    Context = ffi::git_diff_line_t_GIT_DIFF_LINE_CONTEXT,
    /// A line present only in the new version.
    Addition = ffi::git_diff_line_t_GIT_DIFF_LINE_ADDITION,
    /// A line present only in the old version.
    Deletion = ffi::git_diff_line_t_GIT_DIFF_LINE_DELETION,
    /// Both files lack a trailing line feed.
    ContextEofNoLineFeed = ffi::git_diff_line_t_GIT_DIFF_LINE_CONTEXT_EOFNL,
    /// The old file lacks a trailing line feed.
    AdditionEofNoLineFeed = ffi::git_diff_line_t_GIT_DIFF_LINE_ADD_EOFNL,
    /// The new file lacks a trailing line feed.
    DeletionEofNoLineFeed = ffi::git_diff_line_t_GIT_DIFF_LINE_DEL_EOFNL,
    /// A file-header line produced while formatting a diff.
    FileHeader = ffi::git_diff_line_t_GIT_DIFF_LINE_FILE_HDR,
    /// A hunk-header line produced while formatting a diff.
    HunkHeader = ffi::git_diff_line_t_GIT_DIFF_LINE_HUNK_HDR,
    /// A binary-difference marker produced while formatting a diff.
    Binary = ffi::git_diff_line_t_GIT_DIFF_LINE_BINARY,
}

impl DiffLineOrigin {
    /// Converts a raw C enum value when it is a published line origin.
    pub const fn from_raw(raw: ffi::git_diff_line_t) -> Result<Self, InvalidDiffLineOrigin> {
        match raw {
            ffi::git_diff_line_t_GIT_DIFF_LINE_CONTEXT => Ok(Self::Context),
            ffi::git_diff_line_t_GIT_DIFF_LINE_ADDITION => Ok(Self::Addition),
            ffi::git_diff_line_t_GIT_DIFF_LINE_DELETION => Ok(Self::Deletion),
            ffi::git_diff_line_t_GIT_DIFF_LINE_CONTEXT_EOFNL => Ok(Self::ContextEofNoLineFeed),
            ffi::git_diff_line_t_GIT_DIFF_LINE_ADD_EOFNL => Ok(Self::AdditionEofNoLineFeed),
            ffi::git_diff_line_t_GIT_DIFF_LINE_DEL_EOFNL => Ok(Self::DeletionEofNoLineFeed),
            ffi::git_diff_line_t_GIT_DIFF_LINE_FILE_HDR => Ok(Self::FileHeader),
            ffi::git_diff_line_t_GIT_DIFF_LINE_HUNK_HDR => Ok(Self::HunkHeader),
            ffi::git_diff_line_t_GIT_DIFF_LINE_BINARY => Ok(Self::Binary),
            value => Err(InvalidDiffLineOrigin(value)),
        }
    }

    /// Converts the `char` representation stored in [`ffi::git_diff_line`].
    pub const fn from_char(origin: core::ffi::c_char) -> Result<Self, InvalidDiffLineOrigin> {
        Self::from_raw(origin as ffi::git_diff_line_t)
    }

    /// Returns the raw C enum value.
    #[must_use]
    pub const fn as_raw(self) -> ffi::git_diff_line_t {
        self as ffi::git_diff_line_t
    }

    /// Returns the character code stored in [`ffi::git_diff_line::origin`].
    #[must_use]
    pub const fn as_char(self) -> core::ffi::c_char {
        self as core::ffi::c_char
    }
}

impl From<DiffLineOrigin> for ffi::git_diff_line_t {
    fn from(origin: DiffLineOrigin) -> Self {
        origin.as_raw()
    }
}

impl TryFrom<ffi::git_diff_line_t> for DiffLineOrigin {
    type Error = InvalidDiffLineOrigin;

    fn try_from(raw: ffi::git_diff_line_t) -> Result<Self, Self::Error> {
        Self::from_raw(raw)
    }
}

/// A raw value that is not a published [`DiffLineOrigin`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDiffLineOrigin(ffi::git_diff_line_t);

impl InvalidDiffLineOrigin {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_diff_line_t {
        self.0
    }
}

/// Wraps: git_diff_option_t
/// A checked set of options controlling diff generation and formatting.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct DiffOptions(ffi::git_diff_option_t);

impl DiffOptions {
    /// Normal diff behavior with no options enabled.
    pub const NORMAL: Self = Self(ffi::git_diff_option_t_GIT_DIFF_NORMAL);
    /// Reverse the old and new sides.
    pub const REVERSE: Self = Self(ffi::git_diff_option_t_GIT_DIFF_REVERSE);
    /// Include ignored files.
    pub const INCLUDE_IGNORED: Self = Self(ffi::git_diff_option_t_GIT_DIFF_INCLUDE_IGNORED);
    /// Recurse into ignored directories.
    pub const RECURSE_IGNORED_DIRS: Self =
        Self(ffi::git_diff_option_t_GIT_DIFF_RECURSE_IGNORED_DIRS);
    /// Include untracked files.
    pub const INCLUDE_UNTRACKED: Self = Self(ffi::git_diff_option_t_GIT_DIFF_INCLUDE_UNTRACKED);
    /// Recurse into untracked directories.
    pub const RECURSE_UNTRACKED_DIRS: Self =
        Self(ffi::git_diff_option_t_GIT_DIFF_RECURSE_UNTRACKED_DIRS);
    /// Include unmodified files.
    pub const INCLUDE_UNMODIFIED: Self = Self(ffi::git_diff_option_t_GIT_DIFF_INCLUDE_UNMODIFIED);
    /// Emit type-change deltas instead of add/delete pairs.
    pub const INCLUDE_TYPECHANGE: Self = Self(ffi::git_diff_option_t_GIT_DIFF_INCLUDE_TYPECHANGE);
    /// Preserve blob-to-tree changes as type-change deltas.
    pub const INCLUDE_TYPECHANGE_TREES: Self =
        Self(ffi::git_diff_option_t_GIT_DIFF_INCLUDE_TYPECHANGE_TREES);
    /// Ignore file-mode changes.
    pub const IGNORE_FILEMODE: Self = Self(ffi::git_diff_option_t_GIT_DIFF_IGNORE_FILEMODE);
    /// Treat all submodules as unmodified.
    pub const IGNORE_SUBMODULES: Self = Self(ffi::git_diff_option_t_GIT_DIFF_IGNORE_SUBMODULES);
    /// Compare file names without case sensitivity.
    pub const IGNORE_CASE: Self = Self(ffi::git_diff_option_t_GIT_DIFF_IGNORE_CASE);
    /// Report case-only changes as an add/delete pair.
    pub const INCLUDE_CASECHANGE: Self = Self(ffi::git_diff_option_t_GIT_DIFF_INCLUDE_CASECHANGE);
    /// Treat pathspec entries as literal paths.
    pub const DISABLE_PATHSPEC_MATCH: Self =
        Self(ffi::git_diff_option_t_GIT_DIFF_DISABLE_PATHSPEC_MATCH);
    /// Skip binary-content detection.
    pub const SKIP_BINARY_CHECK: Self = Self(ffi::git_diff_option_t_GIT_DIFF_SKIP_BINARY_CHECK);
    /// Avoid scanning ignored entries inside untracked directories.
    pub const ENABLE_FAST_UNTRACKED_DIRS: Self =
        Self(ffi::git_diff_option_t_GIT_DIFF_ENABLE_FAST_UNTRACKED_DIRS);
    /// Update index stat information when content is unchanged.
    pub const UPDATE_INDEX: Self = Self(ffi::git_diff_option_t_GIT_DIFF_UPDATE_INDEX);
    /// Include unreadable files.
    pub const INCLUDE_UNREADABLE: Self = Self(ffi::git_diff_option_t_GIT_DIFF_INCLUDE_UNREADABLE);
    /// Treat unreadable files as untracked.
    pub const INCLUDE_UNREADABLE_AS_UNTRACKED: Self =
        Self(ffi::git_diff_option_t_GIT_DIFF_INCLUDE_UNREADABLE_AS_UNTRACKED);
    /// Apply the indentation-aware hunk heuristic.
    pub const INDENT_HEURISTIC: Self = Self(ffi::git_diff_option_t_GIT_DIFF_INDENT_HEURISTIC);
    /// Ignore blank lines.
    pub const IGNORE_BLANK_LINES: Self = Self(ffi::git_diff_option_t_GIT_DIFF_IGNORE_BLANK_LINES);
    /// Force all files to be treated as text.
    pub const FORCE_TEXT: Self = Self(ffi::git_diff_option_t_GIT_DIFF_FORCE_TEXT);
    /// Force all files to be treated as binary.
    pub const FORCE_BINARY: Self = Self(ffi::git_diff_option_t_GIT_DIFF_FORCE_BINARY);
    /// Ignore all whitespace differences.
    pub const IGNORE_WHITESPACE: Self = Self(ffi::git_diff_option_t_GIT_DIFF_IGNORE_WHITESPACE);
    /// Ignore changes in the amount of whitespace.
    pub const IGNORE_WHITESPACE_CHANGE: Self =
        Self(ffi::git_diff_option_t_GIT_DIFF_IGNORE_WHITESPACE_CHANGE);
    /// Ignore whitespace at the end of a line.
    pub const IGNORE_WHITESPACE_EOL: Self =
        Self(ffi::git_diff_option_t_GIT_DIFF_IGNORE_WHITESPACE_EOL);
    /// Include the contents of untracked files.
    pub const SHOW_UNTRACKED_CONTENT: Self =
        Self(ffi::git_diff_option_t_GIT_DIFF_SHOW_UNTRACKED_CONTENT);
    /// Show unmodified files in applicable output formats.
    pub const SHOW_UNMODIFIED: Self = Self(ffi::git_diff_option_t_GIT_DIFF_SHOW_UNMODIFIED);
    /// Use the patience diff algorithm.
    pub const PATIENCE: Self = Self(ffi::git_diff_option_t_GIT_DIFF_PATIENCE);
    /// Spend extra time finding a minimal diff.
    pub const MINIMAL: Self = Self(ffi::git_diff_option_t_GIT_DIFF_MINIMAL);
    /// Include binary delta data suitable for `git apply`.
    pub const SHOW_BINARY: Self = Self(ffi::git_diff_option_t_GIT_DIFF_SHOW_BINARY);
    /// Every option published by this version of libgit2.
    pub const ALL: Self = Self(
        Self::REVERSE.0
            | Self::INCLUDE_IGNORED.0
            | Self::RECURSE_IGNORED_DIRS.0
            | Self::INCLUDE_UNTRACKED.0
            | Self::RECURSE_UNTRACKED_DIRS.0
            | Self::INCLUDE_UNMODIFIED.0
            | Self::INCLUDE_TYPECHANGE.0
            | Self::INCLUDE_TYPECHANGE_TREES.0
            | Self::IGNORE_FILEMODE.0
            | Self::IGNORE_SUBMODULES.0
            | Self::IGNORE_CASE.0
            | Self::INCLUDE_CASECHANGE.0
            | Self::DISABLE_PATHSPEC_MATCH.0
            | Self::SKIP_BINARY_CHECK.0
            | Self::ENABLE_FAST_UNTRACKED_DIRS.0
            | Self::UPDATE_INDEX.0
            | Self::INCLUDE_UNREADABLE.0
            | Self::INCLUDE_UNREADABLE_AS_UNTRACKED.0
            | Self::INDENT_HEURISTIC.0
            | Self::IGNORE_BLANK_LINES.0
            | Self::FORCE_TEXT.0
            | Self::FORCE_BINARY.0
            | Self::IGNORE_WHITESPACE.0
            | Self::IGNORE_WHITESPACE_CHANGE.0
            | Self::IGNORE_WHITESPACE_EOL.0
            | Self::SHOW_UNTRACKED_CONTENT.0
            | Self::SHOW_UNMODIFIED.0
            | Self::PATIENCE.0
            | Self::MINIMAL.0
            | Self::SHOW_BINARY.0,
    );

    /// Converts raw bits when every enabled option is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_diff_option_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_diff_option_t {
        self.0
    }

    /// Returns whether no option is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any option in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<DiffOptions> for ffi::git_diff_option_t {
    fn from(options: DiffOptions) -> Self {
        options.bits()
    }
}

impl TryFrom<ffi::git_diff_option_t> for DiffOptions {
    type Error = ffi::git_diff_option_t;

    fn try_from(bits: ffi::git_diff_option_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for DiffOptions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for DiffOptions {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for DiffOptions {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for DiffOptions {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for DiffOptions {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_line_origins_round_trip_through_both_c_representations() {
        let origins = [
            DiffLineOrigin::Context,
            DiffLineOrigin::Addition,
            DiffLineOrigin::Deletion,
            DiffLineOrigin::ContextEofNoLineFeed,
            DiffLineOrigin::AdditionEofNoLineFeed,
            DiffLineOrigin::DeletionEofNoLineFeed,
            DiffLineOrigin::FileHeader,
            DiffLineOrigin::HunkHeader,
            DiffLineOrigin::Binary,
        ];

        for origin in origins {
            assert_eq!(DiffLineOrigin::try_from(origin.as_raw()), Ok(origin));
            assert_eq!(DiffLineOrigin::from_char(origin.as_char()), Ok(origin));
        }

        let invalid = b'?' as ffi::git_diff_line_t;
        assert_eq!(
            DiffLineOrigin::try_from(invalid).unwrap_err().value(),
            invalid
        );
    }

    #[test]
    fn diff_options_form_checked_sets() {
        let mut options = DiffOptions::REVERSE | DiffOptions::INCLUDE_UNTRACKED;
        assert!(options.contains(DiffOptions::REVERSE));
        assert!(options.intersects(DiffOptions::INCLUDE_UNTRACKED));
        assert!(!options.intersects(DiffOptions::SHOW_BINARY));

        options |= DiffOptions::SHOW_BINARY;
        options &= DiffOptions::REVERSE | DiffOptions::SHOW_BINARY;
        assert_eq!(options, DiffOptions::REVERSE | DiffOptions::SHOW_BINARY);
        assert_eq!(DiffOptions::from_bits(options.bits()), Some(options));
        assert!(DiffOptions::NORMAL.is_empty());
        assert_eq!(DiffOptions::default(), DiffOptions::NORMAL);
    }

    #[test]
    fn diff_options_reject_reserved_bits() {
        assert_eq!(DiffOptions::from_bits(1 << 27), None);
        assert_eq!(DiffOptions::from_bits(1 << 31), None);
        assert_eq!(
            DiffOptions::from_bits(DiffOptions::ALL.bits()),
            Some(DiffOptions::ALL)
        );
    }

    #[test]
    fn wrappers_match_their_c_enum_layouts() {
        assert_eq!(
            size_of::<DiffLineOrigin>(),
            size_of::<ffi::git_diff_line_t>()
        );
        assert_eq!(
            align_of::<DiffLineOrigin>(),
            align_of::<ffi::git_diff_line_t>()
        );
        assert_eq!(
            size_of::<DiffOptions>(),
            size_of::<ffi::git_diff_option_t>()
        );
        assert_eq!(
            align_of::<DiffOptions>(),
            align_of::<ffi::git_diff_option_t>()
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff_parse_options
    /// Layout-compatible options controlling patch-file parsing.
    DiffParseOptions,
    DiffParseOptionsRef,
    DiffParseOptionsMut,
    ffi::git_diff_parse_options
);

// SAFETY: `git_diff_parse_options` contains only scalar configuration fields
// and owns no resources, so disposing an inline value requires no action.
unsafe impl CValued for DiffParseOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl DiffParseOptions {
    /// Constructs options with the current ABI version and libgit2's default
    /// object-ID algorithm.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        options
            .as_mut()
            .set_version(ffi::GIT_DIFF_PARSE_OPTIONS_VERSION);
        options
    }
}

impl DiffParseOptionsRef<'_> {
    /// Field: git_diff_parse_options.version
    /// Returns the ABI version stored in this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_diff_parse_options.oid_type
    /// Returns the selected object-ID algorithm, or `None` for libgit2's
    /// default algorithm.
    pub fn oid_type(&self) -> Result<Option<OidType>, InvalidOidType> {
        // SAFETY: as `version`, for this initialized scalar field.
        let raw = unsafe { addr_of!((*self.as_ptr()).oid_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            OidType::try_from(raw).map(Some)
        }
    }
}

impl DiffParseOptionsMut<'_> {
    /// Sets the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits a raw-place scalar write
        // without forming a reference to C-visible memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects an object-ID algorithm, or libgit2's default with `None`.
    pub fn set_oid_type(&mut self, oid_type: Option<OidType>) {
        let raw = oid_type.map_or(0, ffi::git_oid_t::from);
        // SAFETY: this exclusive handle permits the scalar write, and `raw`
        // is zero or a published C enum value.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).oid_type).write(raw) }
    }
}

#[cfg(test)]
mod parse_options_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn parse_options_preserve_layout_and_inline_ownership() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<DiffParseOptions>();
        assert_valued::<DiffParseOptions>();
        assert_eq!(
            size_of::<DiffParseOptions>(),
            size_of::<ffi::git_diff_parse_options>()
        );
        assert_eq!(
            align_of::<DiffParseOptions>(),
            align_of::<ffi::git_diff_parse_options>()
        );
        assert_eq!(
            size_of::<DiffParseOptionsRef<'_>>(),
            size_of::<*const ffi::git_diff_parse_options>()
        );
        assert_eq!(
            size_of::<DiffParseOptionsMut<'_>>(),
            size_of::<*mut ffi::git_diff_parse_options>()
        );
        assert_eq!(
            size_of::<CVal<DiffParseOptions>>(),
            size_of::<ffi::git_diff_parse_options>()
        );
    }

    #[test]
    fn parse_options_defaults_and_mutation_are_checked() {
        let mut options = DiffParseOptions::new();
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_DIFF_PARSE_OPTIONS_VERSION
        );
        assert_eq!(options.as_ref().oid_type(), Ok(None));

        options.as_mut().set_oid_type(Some(OidType::Sha256));
        assert_eq!(options.as_ref().oid_type(), Ok(Some(OidType::Sha256)));

        options.as_mut().set_version(7);
        assert_eq!(options.as_ref().version(), 7);
    }

    #[test]
    fn parse_options_reject_an_unknown_oid_type() {
        let invalid = ffi::git_oid_t_GIT_OID_SHA256 + 1;
        let mut raw = ffi::git_diff_parse_options {
            version: ffi::GIT_DIFF_PARSE_OPTIONS_VERSION,
            oid_type: invalid,
        };
        // SAFETY: `raw` is initialized and remains live and unmodified while
        // this shared handle is used.
        let options = unsafe { DiffParseOptionsRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(options.oid_type().unwrap_err().value(), invalid);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff_similarity_metric
    /// A layout-compatible borrowed table of similarity callbacks.
    DiffSimilarityMetric,
    DiffSimilarityMetricRef,
    DiffSimilarityMetricMut,
    ffi::git_diff_similarity_metric
);

// SAFETY: the table owns neither its callback code nor its opaque payload, so
// disposing an inline table requires no action.
unsafe impl CValued for DiffSimilarityMetric {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

/// The C ABI for a callback that creates a signature from a file path.
pub type DiffFileSignatureCallback = unsafe extern "C" fn(
    *mut *mut core::ffi::c_void,
    *const ffi::git_diff_file,
    *const core::ffi::c_char,
    *mut core::ffi::c_void,
) -> core::ffi::c_int;

/// The C ABI for a callback that creates a signature from buffered content.
pub type DiffBufferSignatureCallback = unsafe extern "C" fn(
    *mut *mut core::ffi::c_void,
    *const ffi::git_diff_file,
    *const core::ffi::c_char,
    usize,
    *mut core::ffi::c_void,
) -> core::ffi::c_int;

/// The C ABI for a callback that releases an opaque signature.
pub type DiffFreeSignatureCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, *mut core::ffi::c_void);

/// The C ABI for a callback that compares two opaque signatures.
pub type DiffSimilarityCallback = unsafe extern "C" fn(
    *mut core::ffi::c_int,
    *mut core::ffi::c_void,
    *mut core::ffi::c_void,
    *mut core::ffi::c_void,
) -> core::ffi::c_int;

/// A failure to invoke a similarity-metric callback safely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffSimilarityError {
    /// The file-signature callback is absent.
    MissingFileSignature,
    /// The buffer-signature callback is absent.
    MissingBufferSignature,
    /// The signature destructor is absent.
    MissingFreeSignature,
    /// The comparison callback is absent.
    MissingSimilarity,
    /// A callback reported a libgit2 error code.
    Callback(core::ffi::c_int),
    /// A signature came from a different metric table.
    ForeignSignature,
}

/// An opaque callback-produced signature tied to its metric table.
pub struct DiffSimilaritySignature<'a> {
    ptr: NonNull<core::ffi::c_void>,
    metric: NonNull<ffi::git_diff_similarity_metric>,
    payload: Option<NonNull<core::ffi::c_void>>,
    free: DiffFreeSignatureCallback,
    _lifetime: core::marker::PhantomData<DiffSimilarityMetricRef<'a>>,
}

impl Drop for DiffSimilaritySignature<'_> {
    fn drop(&mut self) {
        // SAFETY: this token was produced by one of the table's signature
        // callbacks and carries that same table's required destructor and
        // payload. It is consumed exactly once here.
        unsafe {
            (self.free)(
                self.ptr.as_ptr(),
                self.payload.map_or(core::ptr::null_mut(), NonNull::as_ptr),
            )
        }
    }
}

impl<'a> DiffSimilarityMetricRef<'a> {
    /// Field: git_diff_similarity_metric.payload
    /// Returns whether the metric carries an application payload.
    #[must_use]
    pub fn has_payload(&self) -> bool {
        // SAFETY: this live shared handle permits a raw-place pointer read.
        !unsafe { addr_of!((*self.as_ptr()).payload).read() }.is_null()
    }

    /// Field: git_diff_similarity_metric.file_signature
    /// Creates an owned opaque signature for a file path.
    pub fn file_signature(
        &self,
        file: crate::diff::DiffFileRef<'_>,
        fullpath: &core::ffi::CStr,
    ) -> Result<Option<DiffSimilaritySignature<'a>>, DiffSimilarityError> {
        // SAFETY: this live shared handle permits raw-place reads of the
        // initialized callback table.
        let (callback, free, payload) = unsafe {
            (
                addr_of!((*self.as_ptr()).file_signature).read(),
                addr_of!((*self.as_ptr()).free_signature).read(),
                addr_of!((*self.as_ptr()).payload).read(),
            )
        };
        let callback = callback.ok_or(DiffSimilarityError::MissingFileSignature)?;
        let free = free.ok_or(DiffSimilarityError::MissingFreeSignature)?;
        let mut signature = core::ptr::null_mut();
        // SAFETY: all borrowed inputs remain live for this synchronous call,
        // `signature` is a valid output slot, and the table's validity
        // contract couples the callback to its payload.
        let error = unsafe { callback(&mut signature, file.as_ptr(), fullpath.as_ptr(), payload) };
        self.finish_signature(error, NonNull::new(signature), free, NonNull::new(payload))
    }

    /// Field: git_diff_similarity_metric.buffer_signature
    /// Creates an owned opaque signature for buffered file content.
    pub fn buffer_signature(
        &self,
        file: crate::diff::DiffFileRef<'_>,
        buffer: &[u8],
    ) -> Result<Option<DiffSimilaritySignature<'a>>, DiffSimilarityError> {
        // SAFETY: this live shared handle permits raw-place reads of the
        // initialized callback table.
        let (callback, free, payload) = unsafe {
            (
                addr_of!((*self.as_ptr()).buffer_signature).read(),
                addr_of!((*self.as_ptr()).free_signature).read(),
                addr_of!((*self.as_ptr()).payload).read(),
            )
        };
        let callback = callback.ok_or(DiffSimilarityError::MissingBufferSignature)?;
        let free = free.ok_or(DiffSimilarityError::MissingFreeSignature)?;
        let mut signature = core::ptr::null_mut();
        // SAFETY: the file and buffer remain live for this synchronous call,
        // `signature` is a valid output slot, and the table's validity
        // contract couples the callback to its payload.
        let error = unsafe {
            callback(
                &mut signature,
                file.as_ptr(),
                buffer.as_ptr().cast(),
                buffer.len(),
                payload,
            )
        };
        self.finish_signature(error, NonNull::new(signature), free, NonNull::new(payload))
    }

    /// Field: git_diff_similarity_metric.free_signature
    /// Returns whether callback-produced signatures have a destructor.
    #[must_use]
    pub fn has_free_signature(&self) -> bool {
        // SAFETY: this live shared handle permits a raw-place callback read.
        unsafe { addr_of!((*self.as_ptr()).free_signature).read() }.is_some()
    }

    /// Field: git_diff_similarity_metric.similarity
    /// Computes the score for two signatures produced by this table.
    pub fn similarity(
        &self,
        left: &DiffSimilaritySignature<'a>,
        right: &DiffSimilaritySignature<'a>,
    ) -> Result<core::ffi::c_int, DiffSimilarityError> {
        if left.metric.as_ptr().cast_const() != self.as_ptr()
            || right.metric.as_ptr().cast_const() != self.as_ptr()
        {
            return Err(DiffSimilarityError::ForeignSignature);
        }
        // SAFETY: this live shared handle permits raw-place reads of the
        // initialized callback table.
        let (callback, payload) = unsafe {
            (
                addr_of!((*self.as_ptr()).similarity).read(),
                addr_of!((*self.as_ptr()).payload).read(),
            )
        };
        let callback = callback.ok_or(DiffSimilarityError::MissingSimilarity)?;
        let mut score = 0;
        // SAFETY: both opaque tokens were produced by this exact metric and
        // remain live for the synchronous comparison; `score` is writable.
        let error = unsafe { callback(&mut score, left.ptr.as_ptr(), right.ptr.as_ptr(), payload) };
        if error < 0 {
            Err(DiffSimilarityError::Callback(error))
        } else {
            Ok(score)
        }
    }

    fn finish_signature(
        &self,
        error: core::ffi::c_int,
        signature: Option<NonNull<core::ffi::c_void>>,
        free: DiffFreeSignatureCallback,
        payload: Option<NonNull<core::ffi::c_void>>,
    ) -> Result<Option<DiffSimilaritySignature<'a>>, DiffSimilarityError> {
        if error < 0 {
            // A failing signature callback transfers no token: `similarity_sig`
            // propagates the status and leaves whatever the callback wrote in
            // the slot alone, so releasing it here could free a signature the
            // callback still owns.
            return Err(DiffSimilarityError::Callback(error));
        }
        Ok(signature.map(|ptr| DiffSimilaritySignature {
            ptr,
            metric: NonNull::new(self.as_ptr().cast_mut())
                .expect("a live borrowed handle is non-null"),
            payload,
            free,
            _lifetime: core::marker::PhantomData,
        }))
    }
}

impl DiffSimilarityMetricMut<'_> {
    /// Stores an opaque borrowed callback payload.
    ///
    /// # Safety
    ///
    /// A non-null payload must remain valid for every callback invocation
    /// through this table, including invocations after this handle is gone.
    pub unsafe fn set_borrowed_payload(&mut self, payload: Option<NonNull<core::ffi::c_void>>) {
        let payload = payload.map_or(core::ptr::null_mut(), NonNull::as_ptr);
        // SAFETY: this exclusive handle permits the write and the caller
        // upholds the erased borrow's lifetime and callback-specific validity.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).payload).write(payload) }
    }

    /// Installs the file-signature callback.
    ///
    /// # Safety
    ///
    /// The callback must obey its C ABI contract and be compatible with the
    /// installed payload and signature destructor.
    pub unsafe fn set_file_signature(&mut self, callback: DiffFileSignatureCallback) {
        // SAFETY: this exclusive handle permits the callback-slot write; the
        // caller supplies the semantic callback contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).file_signature).write(Some(callback)) }
    }

    /// Installs the buffer-signature callback.
    ///
    /// # Safety
    ///
    /// The callback must obey its C ABI contract and be compatible with the
    /// installed payload and signature destructor.
    pub unsafe fn set_buffer_signature(&mut self, callback: DiffBufferSignatureCallback) {
        // SAFETY: this exclusive handle permits the callback-slot write; the
        // caller supplies the semantic callback contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).buffer_signature).write(Some(callback)) }
    }

    /// Installs the signature destructor.
    ///
    /// # Safety
    ///
    /// The callback must release every non-null token produced by both
    /// installed signature callbacks, using the installed payload.
    pub unsafe fn set_free_signature(&mut self, callback: DiffFreeSignatureCallback) {
        // SAFETY: this exclusive handle permits the callback-slot write; the
        // caller supplies the semantic callback contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).free_signature).write(Some(callback)) }
    }

    /// Installs the signature-comparison callback.
    ///
    /// # Safety
    ///
    /// The callback must accept tokens made by both signature callbacks and
    /// obey its C ABI contract with the installed payload.
    pub unsafe fn set_similarity(&mut self, callback: DiffSimilarityCallback) {
        // SAFETY: this exclusive handle permits the callback-slot write; the
        // caller supplies the semantic callback contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).similarity).write(Some(callback)) }
    }
}

#[cfg(test)]
mod similarity_metric_tests {
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use ffibox::{CCell, CValued};

    use super::*;

    #[derive(Default)]
    struct Payload {
        calls: usize,
        frees: usize,
    }

    static NULL_PAYLOAD_FREES: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn file_signature(
        out: *mut *mut core::ffi::c_void,
        _file: *const ffi::git_diff_file,
        fullpath: *const core::ffi::c_char,
        payload: *mut core::ffi::c_void,
    ) -> core::ffi::c_int {
        // SAFETY: the test installs this callback with valid pointers and a
        // NUL-terminated path for the duration of each invocation.
        unsafe {
            assert_eq!(core::ffi::CStr::from_ptr(fullpath), c"file");
            (*payload.cast::<Payload>()).calls += 1;
            *out = payload;
        }
        0
    }

    unsafe extern "C" fn buffer_signature(
        out: *mut *mut core::ffi::c_void,
        _file: *const ffi::git_diff_file,
        buffer: *const core::ffi::c_char,
        len: usize,
        payload: *mut core::ffi::c_void,
    ) -> core::ffi::c_int {
        // SAFETY: the test passes three initialized bytes and valid output and
        // payload pointers for this synchronous callback.
        unsafe {
            assert_eq!(
                core::slice::from_raw_parts(buffer.cast::<u8>(), len),
                b"buf"
            );
            (*payload.cast::<Payload>()).calls += 1;
            *out = payload;
        }
        0
    }

    unsafe extern "C" fn free_signature(
        signature: *mut core::ffi::c_void,
        payload: *mut core::ffi::c_void,
    ) {
        assert_eq!(signature, payload);
        // SAFETY: the test payload remains live and exclusively accessed by
        // these sequential callbacks.
        unsafe { (*payload.cast::<Payload>()).frees += 1 }
    }

    unsafe extern "C" fn free_signature_without_payload(
        signature: *mut core::ffi::c_void,
        payload: *mut core::ffi::c_void,
    ) {
        assert!(!signature.is_null());
        assert!(payload.is_null());
        NULL_PAYLOAD_FREES.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn similarity(
        score: *mut core::ffi::c_int,
        left: *mut core::ffi::c_void,
        right: *mut core::ffi::c_void,
        payload: *mut core::ffi::c_void,
    ) -> core::ffi::c_int {
        assert_eq!(left, payload);
        assert_eq!(right, payload);
        // SAFETY: the wrapper supplies a valid writable score slot.
        unsafe { *score = 91 }
        0
    }

    static PUBLISHING_FAILURE_FREES: AtomicUsize = AtomicUsize::new(0);

    /// A failing signature callback that still writes its output slot, to
    /// prove the wrapper does not release a token C never transferred.
    unsafe extern "C" fn failing_file_signature(
        out: *mut *mut core::ffi::c_void,
        _file: *const ffi::git_diff_file,
        _fullpath: *const core::ffi::c_char,
        payload: *mut core::ffi::c_void,
    ) -> core::ffi::c_int {
        // SAFETY: the wrapper supplies a writable output slot, and this
        // callback keeps ownership of the token it publishes there.
        unsafe { *out = payload }
        -1
    }

    unsafe extern "C" fn counting_free_signature(
        _signature: *mut core::ffi::c_void,
        _payload: *mut core::ffi::c_void,
    ) {
        PUBLISHING_FAILURE_FREES.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn metric_preserves_layout_and_inline_ownership() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<DiffSimilarityMetric>();
        assert_valued::<DiffSimilarityMetric>();
        assert_eq!(
            size_of::<DiffSimilarityMetric>(),
            size_of::<ffi::git_diff_similarity_metric>()
        );
        assert_eq!(
            align_of::<DiffSimilarityMetric>(),
            align_of::<ffi::git_diff_similarity_metric>()
        );
        assert_eq!(
            size_of::<DiffSimilarityMetricRef<'_>>(),
            size_of::<*const ffi::git_diff_similarity_metric>()
        );
        assert_eq!(
            size_of::<DiffSimilarityMetricMut<'_>>(),
            size_of::<*mut ffi::git_diff_similarity_metric>()
        );
    }

    #[test]
    fn callbacks_create_compare_and_release_opaque_signatures() {
        let mut payload = Payload::default();
        let mut metric = CVal::new(DiffSimilarityMetric::zeroed());
        {
            let mut metric_mut = metric.as_mut();
            // SAFETY: the payload outlives `metric`, and these four callbacks
            // implement one compatible signature protocol.
            unsafe {
                metric_mut.set_borrowed_payload(NonNull::new(
                    addr_of_mut!(payload).cast::<core::ffi::c_void>(),
                ));
                metric_mut.set_file_signature(file_signature);
                metric_mut.set_buffer_signature(buffer_signature);
                metric_mut.set_free_signature(free_signature);
                metric_mut.set_similarity(similarity);
            }
        }

        // SAFETY: bindgen's diff-file layout consists only of integer,
        // pointer and inline object-ID fields for which all-zero is valid
        // initialized storage; the callbacks do not inspect the file.
        let mut raw_file: ffi::git_diff_file = unsafe { core::mem::zeroed() };
        // SAFETY: `raw_file` remains live and is not mutated while the shared
        // handle is used.
        let file = unsafe { crate::diff::DiffFileRef::from_ptr(addr_of_mut!(raw_file)) }.unwrap();
        let metric_ref = metric.as_ref();
        assert!(metric_ref.has_payload());
        assert!(metric_ref.has_free_signature());

        let from_file = metric_ref.file_signature(file, c"file").unwrap().unwrap();
        let from_buffer = metric_ref.buffer_signature(file, b"buf").unwrap().unwrap();
        assert_eq!(metric_ref.similarity(&from_file, &from_buffer), Ok(91));
        assert_eq!(payload.calls, 2);
        drop((from_file, from_buffer));
        assert_eq!(payload.frees, 2);
    }

    #[test]
    fn a_failing_signature_callback_keeps_its_published_token() {
        PUBLISHING_FAILURE_FREES.store(0, Ordering::SeqCst);
        let mut token = 0u8;
        let mut metric = CVal::new(DiffSimilarityMetric::zeroed());
        {
            let mut metric_mut = metric.as_mut();
            // SAFETY: `token` outlives `metric`, and the destructor below is
            // compatible with the tokens this signature callback publishes.
            unsafe {
                metric_mut.set_borrowed_payload(NonNull::new(
                    addr_of_mut!(token).cast::<core::ffi::c_void>(),
                ));
                metric_mut.set_file_signature(failing_file_signature);
                metric_mut.set_free_signature(counting_free_signature);
            }
        }

        // SAFETY: as in the protocol test above, an all-zero diff file is
        // valid initialized storage that the callback does not inspect.
        let mut raw_file: ffi::git_diff_file = unsafe { core::mem::zeroed() };
        // SAFETY: `raw_file` remains live and unmodified for this borrow.
        let file = unsafe { crate::diff::DiffFileRef::from_ptr(addr_of_mut!(raw_file)) }.unwrap();

        assert_eq!(
            metric.as_ref().file_signature(file, c"file").err(),
            Some(DiffSimilarityError::Callback(-1))
        );
        assert_eq!(PUBLISHING_FAILURE_FREES.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn signature_owner_preserves_a_null_callback_payload() {
        NULL_PAYLOAD_FREES.store(0, Ordering::SeqCst);
        let metric = CVal::new(DiffSimilarityMetric::zeroed());
        let signature = metric
            .as_ref()
            .finish_signature(
                0,
                Some(NonNull::dangling()),
                free_signature_without_payload,
                None,
            )
            .unwrap()
            .unwrap();

        drop(signature);
        assert_eq!(NULL_PAYLOAD_FREES.load(Ordering::SeqCst), 1);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff_find_options
    /// Layout-compatible options controlling rename and copy detection.
    DiffFindOptions,
    DiffFindOptionsRef,
    DiffFindOptionsMut,
    ffi::git_diff_find_options
);

// SAFETY: this options record owns no resources. A custom metric is borrowed
// and libgit2 never releases it when disposing the by-value options storage.
unsafe impl CValued for DiffFindOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl DiffFindOptions {
    /// Constructs options with the current ABI version and libgit2 defaults.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        options
            .as_mut()
            .set_version(ffi::GIT_DIFF_FIND_OPTIONS_VERSION);
        options
    }
}

impl<'a> DiffFindOptionsRef<'a> {
    /// Field: git_diff_find_options.version
    /// Returns the ABI version stored in this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_diff_find_options.flags
    /// Returns the raw combination of published `git_diff_find_t` bits.
    #[must_use]
    pub fn flags(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Field: git_diff_find_options.rename_threshold
    /// Returns the rename similarity threshold.
    #[must_use]
    pub fn rename_threshold(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).rename_threshold).read() }
    }

    /// Field: git_diff_find_options.rename_from_rewrite_threshold
    /// Returns the similarity threshold for rename sources from rewrites.
    #[must_use]
    pub fn rename_from_rewrite_threshold(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).rename_from_rewrite_threshold).read() }
    }

    /// Field: git_diff_find_options.copy_threshold
    /// Returns the copy similarity threshold.
    #[must_use]
    pub fn copy_threshold(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).copy_threshold).read() }
    }

    /// Field: git_diff_find_options.break_rewrite_threshold
    /// Returns the threshold below which rewrites are split.
    #[must_use]
    pub fn break_rewrite_threshold(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).break_rewrite_threshold).read() }
    }

    /// Field: git_diff_find_options.rename_limit
    /// Returns the maximum number of rename matches to consider.
    #[must_use]
    pub fn rename_limit(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).rename_limit).read() }
    }

    /// Field: git_diff_find_options.metric
    /// Borrows the custom similarity metric, when one is installed.
    #[must_use]
    pub fn metric(&self) -> Option<DiffSimilarityMetricRef<'a>> {
        // SAFETY: this live shared handle permits a raw-place pointer read.
        let metric = unsafe { addr_of!((*self.as_ptr()).metric).read() };
        // SAFETY: a non-null metric in a valid options record remains live for
        // the record's borrow by the field's construction contract.
        unsafe { DiffSimilarityMetricRef::from_ptr(metric) }
    }
}

impl DiffFindOptionsMut<'_> {
    /// Sets the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Sets the raw combination of published `git_diff_find_t` bits.
    pub fn set_flags(&mut self, flags: u32) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Sets the rename similarity threshold.
    pub fn set_rename_threshold(&mut self, threshold: u16) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).rename_threshold).write(threshold) }
    }

    /// Sets the similarity threshold for rename sources from rewrites.
    pub fn set_rename_from_rewrite_threshold(&mut self, threshold: u16) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).rename_from_rewrite_threshold).write(threshold) }
    }

    /// Sets the copy similarity threshold.
    pub fn set_copy_threshold(&mut self, threshold: u16) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).copy_threshold).write(threshold) }
    }

    /// Sets the threshold below which rewrites are split.
    pub fn set_break_rewrite_threshold(&mut self, threshold: u16) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).break_rewrite_threshold).write(threshold) }
    }

    /// Sets the maximum number of rename matches to consider.
    pub fn set_rename_limit(&mut self, limit: usize) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).rename_limit).write(limit) }
    }

    /// Installs a borrowed custom similarity metric.
    ///
    /// Unlike the option records that carry a `'data` parameter, this type is
    /// generated from the plain C layout and cannot express the stored
    /// borrow, so the obligation is the caller's.
    ///
    /// # Safety
    ///
    /// The metric and every payload its callbacks use must remain valid for
    /// as long as this options record can still be observed: both while
    /// libgit2 is passed the record and while safe code can reach the stored
    /// table again through [`DiffFindOptionsRef::metric`], whose returned
    /// handle borrows only the options value. Clearing the slot with
    /// [`clear_metric`](Self::clear_metric) ends the obligation.
    pub unsafe fn set_metric(&mut self, metric: DiffSimilarityMetricRef<'_>) {
        // SAFETY: this live exclusive handle permits the pointer write and the
        // caller upholds the stored borrow's lifetime and callback invariants.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).metric).write(metric.as_ptr().cast_mut()) }
    }

    /// Selects libgit2's default similarity metric.
    pub fn clear_metric(&mut self) {
        // SAFETY: this live exclusive handle permits a raw-place pointer write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).metric).write(core::ptr::null_mut()) }
    }
}

#[cfg(test)]
mod find_options_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn find_options_preserve_layout_and_inline_ownership() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<DiffFindOptions>();
        assert_valued::<DiffFindOptions>();
        assert_eq!(
            size_of::<DiffFindOptions>(),
            size_of::<ffi::git_diff_find_options>()
        );
        assert_eq!(
            align_of::<DiffFindOptions>(),
            align_of::<ffi::git_diff_find_options>()
        );
        assert_eq!(
            size_of::<DiffFindOptionsRef<'_>>(),
            size_of::<*const ffi::git_diff_find_options>()
        );
        assert_eq!(
            size_of::<CVal<DiffFindOptions>>(),
            size_of::<ffi::git_diff_find_options>()
        );
    }

    #[test]
    fn find_options_default_and_scalar_fields_round_trip() {
        let mut options = DiffFindOptions::new();
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_DIFF_FIND_OPTIONS_VERSION
        );
        assert_eq!(options.as_ref().flags(), 0);
        assert!(options.as_ref().metric().is_none());

        {
            let mut options = options.as_mut();
            options.set_flags(0x42);
            options.set_rename_threshold(51);
            options.set_rename_from_rewrite_threshold(52);
            options.set_copy_threshold(53);
            options.set_break_rewrite_threshold(54);
            options.set_rename_limit(999);
        }

        let options = options.as_ref();
        assert_eq!(options.flags(), 0x42);
        assert_eq!(options.rename_threshold(), 51);
        assert_eq!(options.rename_from_rewrite_threshold(), 52);
        assert_eq!(options.copy_threshold(), 53);
        assert_eq!(options.break_rewrite_threshold(), 54);
        assert_eq!(options.rename_limit(), 999);
    }

    #[test]
    fn custom_metric_borrow_round_trips_without_ownership_transfer() {
        let metric = CVal::new(DiffSimilarityMetric::zeroed());
        let mut options = DiffFindOptions::new();
        // SAFETY: `metric` remains live until after `options` is no longer
        // accessed, and the test does not invoke its empty callback slots.
        unsafe { options.as_mut().set_metric(metric.as_ref()) };
        assert_eq!(
            options.as_ref().metric().unwrap().as_ptr(),
            metric.as_ref().as_ptr()
        );
        options.as_mut().clear_metric();
        assert!(options.as_ref().metric().is_none());
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff_delta
    /// A borrowed description of one changed entry and its two sides.
    DiffDelta,
    DiffDeltaRef,
    DiffDeltaMut,
    ffi::git_diff_delta
);

impl<'a> DiffDeltaRef<'a> {
    /// Field: git_diff_delta.flags
    /// Returns the raw combination of published `git_diff_flag_t` bits.
    #[must_use]
    pub fn flags(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Field: git_diff_delta.status
    /// Returns the kind of change, rejecting an unknown C value.
    pub fn status(&self) -> Result<crate::diff::Delta, crate::diff::InvalidDelta> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let status = unsafe { addr_of!((*self.as_ptr()).status).read() };
        crate::diff::Delta::try_from(status)
    }

    /// Field: git_diff_delta.similarity
    /// Returns the similarity score recorded by rename or copy detection.
    #[must_use]
    pub fn similarity(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).similarity).read() }
    }

    /// Field: git_diff_delta.new_file
    /// Borrows the inline file description for the new side.
    #[must_use]
    pub fn new_file(&self) -> crate::diff::DiffFileRef<'a> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // without forming a reference to C-visible memory.
        let file = unsafe { addr_of!((*self.as_ptr()).new_file) }.cast_mut();
        // SAFETY: the inline field is non-null and remains live for the
        // enclosing delta's shared borrow.
        unsafe { crate::diff::DiffFileRef::from_ptr(file) }.expect("an inline field is non-null")
    }

    /// Field: git_diff_delta.old_file
    /// Borrows the inline file description for the old side.
    #[must_use]
    pub fn old_file(&self) -> crate::diff::DiffFileRef<'a> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // without forming a reference to C-visible memory.
        let file = unsafe { addr_of!((*self.as_ptr()).old_file) }.cast_mut();
        // SAFETY: the inline field is non-null and remains live for the
        // enclosing delta's shared borrow.
        unsafe { crate::diff::DiffFileRef::from_ptr(file) }.expect("an inline field is non-null")
    }

    /// Field: git_diff_delta.nfiles
    /// Returns the number of file sides represented by this delta.
    #[must_use]
    pub fn file_count(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).nfiles).read() }
    }
}

impl DiffDeltaMut<'_> {
    /// Sets the raw combination of published `git_diff_flag_t` bits.
    pub fn set_flags(&mut self, flags: u32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Sets the kind of change.
    pub fn set_status(&mut self, status: crate::diff::Delta) {
        // SAFETY: this exclusive handle permits the write, and `Delta`
        // converts only to a published C enum value.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).status).write(status.into()) }
    }

    /// Sets the similarity score.
    pub fn set_similarity(&mut self, similarity: u16) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).similarity).write(similarity) }
    }

    /// Borrows the inline file description for the new side exclusively.
    #[must_use]
    pub fn new_file_mut(&mut self) -> crate::diff::DiffFileMut<'_> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let file = unsafe { addr_of_mut!((*self.as_mut_ptr()).new_file) };
        // SAFETY: the field is non-null and exclusively borrowed for the
        // returned handle's lifetime.
        unsafe { crate::diff::DiffFileMut::from_ptr(file) }.expect("an inline field is non-null")
    }

    /// Borrows the inline file description for the old side exclusively.
    #[must_use]
    pub fn old_file_mut(&mut self) -> crate::diff::DiffFileMut<'_> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let file = unsafe { addr_of_mut!((*self.as_mut_ptr()).old_file) };
        // SAFETY: the field is non-null and exclusively borrowed for the
        // returned handle's lifetime.
        unsafe { crate::diff::DiffFileMut::from_ptr(file) }.expect("an inline field is non-null")
    }

    /// Sets the number of file sides represented by this delta.
    pub fn set_file_count(&mut self, file_count: u16) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).nfiles).write(file_count) }
    }
}

/// A callback receiver installed in [`GitDiffOptions`].
///
/// Both callbacks share one receiver because the C options use one payload
/// slot. Return values retain the C API's convention: zero continues, a
/// positive notification result skips its delta, and a negative value aborts.
pub trait GitDiffOptionsCallbacks {
    /// Receives the typed delta that libgit2 is considering adding.
    fn notify(
        &mut self,
        diff: crate::diff::DiffRef<'_>,
        delta: DiffDeltaRef<'_>,
        matched_pathspec: Option<&core::ffi::CStr>,
    ) -> core::ffi::c_int;

    /// Receives progress before one file comparison.
    fn progress(
        &mut self,
        diff: crate::diff::DiffRef<'_>,
        old_path: Option<&core::ffi::CStr>,
        new_path: Option<&core::ffi::CStr>,
    ) -> core::ffi::c_int;
}

/// Wraps: git_diff_options
/// Layout-compatible diff options borrowing strings, pathspec storage, and
/// callback state for `'data`.
///
/// `'data` is invariant. The setters store a `&'data` referent into the C
/// struct while the getters hand one back out, so a covariant `'data` would
/// let safe code shrink the parameter on the exclusive handle, install a
/// shorter-lived prefix, pathspec or callback receiver, and then read it
/// back through a handle still typed at the longer lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::diff::GitDiffOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitDiffOptionsMut<'object, 'static>,
/// ) -> GitDiffOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitDiffOptions<'data> {
    inner: CType<ffi::git_diff_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitDiffOptions`].
#[repr(transparent)]
pub struct GitDiffOptionsRef<'object, 'data>(CPtr<'object, GitDiffOptions<'data>>);

impl Clone for GitDiffOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitDiffOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitDiffOptions`].
#[repr(transparent)]
pub struct GitDiffOptionsMut<'object, 'data>(GitDiffOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitDiffOptions<'data> {
    type C = ffi::git_diff_options;
    type Ref<'object>
        = GitDiffOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitDiffOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(p: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `p` is a live shared object.
        GitDiffOptionsRef(unsafe { CPtr::new(p) })
    }

    unsafe fn mut_from_raw<'object>(p: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitDiffOptionsMut(GitDiffOptionsRef(unsafe { CPtr::new(p) }))
    }
}

// SAFETY: this options header only borrows its pointer fields and owns no
// resource, so disposing its inline storage requires no action.
unsafe impl CValued for GitDiffOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitDiffOptions<'data> {
    /// Constructs options equivalent to `GIT_DIFF_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every field of the C options struct admits the all-zero bit
        // pattern; published nonzero defaults are installed below.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_DIFF_OPTIONS_VERSION);
            view.set_ignore_submodules(GitSubmoduleIgnore::Unspecified);
            view.set_context_lines(3);
        }
        options
    }
}

impl<'object, 'data> GitDiffOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify an initialized options value that lives for
    /// `'object`. Every borrowed field must remain valid for `'data`, which
    /// must outlive `'object`, and callback state must remain exclusive.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_diff_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitDiffOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_diff_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_diff_options.flags
    /// Returns the checked set of configured diff flags.
    pub fn flags(&self) -> Result<DiffOptions, ffi::git_diff_option_t> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let flags = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        DiffOptions::try_from(flags)
    }

    /// Field: git_diff_options.payload
    /// Reports whether callback state is installed.
    #[must_use]
    pub fn has_callback_payload(&self) -> bool {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        !unsafe { addr_of!((*self.as_ptr()).payload).read() }.is_null()
    }

    /// Field: git_diff_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_diff_options.oid_type
    /// Returns the selected object-ID algorithm, or `None` for repository or
    /// library defaults.
    pub fn oid_type(&self) -> Result<Option<OidType>, InvalidOidType> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let oid_type = unsafe { addr_of!((*self.as_ptr()).oid_type).read() };
        if oid_type == 0 {
            Ok(None)
        } else {
            OidType::try_from(oid_type).map(Some)
        }
    }

    /// Field: git_diff_options.max_size
    /// Returns the automatic binary-detection threshold in bytes.
    #[must_use]
    pub fn max_size(&self) -> ffi::git_off_t {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).max_size).read() }
    }

    /// Field: git_diff_options.progress_cb
    /// Reports whether a progress callback is installed.
    #[must_use]
    pub fn has_progress_callback(&self) -> bool {
        // SAFETY: this live shared handle permits the callback-slot read.
        unsafe { addr_of!((*self.as_ptr()).progress_cb).read() }.is_some()
    }

    /// Field: git_diff_options.new_prefix
    /// Borrows the optional prefix for new paths.
    #[must_use]
    pub fn new_prefix(&self) -> Option<&'object core::ffi::CStr> {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        let prefix = unsafe { addr_of!((*self.as_ptr()).new_prefix).read() };
        if prefix.is_null() {
            None
        } else {
            // SAFETY: the wrapper contract keeps the borrowed NUL string live
            // for at least this object borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(prefix) })
        }
    }

    /// Field: git_diff_options.old_prefix
    /// Borrows the optional prefix for old paths.
    #[must_use]
    pub fn old_prefix(&self) -> Option<&'object core::ffi::CStr> {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        let prefix = unsafe { addr_of!((*self.as_ptr()).old_prefix).read() };
        if prefix.is_null() {
            None
        } else {
            // SAFETY: the wrapper contract keeps the borrowed NUL string live
            // for at least this object borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(prefix) })
        }
    }

    /// Field: git_diff_options.id_abbrev
    /// Returns the requested object-ID abbreviation length.
    #[must_use]
    pub fn id_abbrev(&self) -> u16 {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).id_abbrev).read() }
    }

    /// Field: git_diff_options.interhunk_lines
    /// Returns the maximum unchanged lines merged between adjacent hunks.
    #[must_use]
    pub fn interhunk_lines(&self) -> u32 {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).interhunk_lines).read() }
    }

    /// Field: git_diff_options.context_lines
    /// Returns the unchanged context lines surrounding each hunk.
    #[must_use]
    pub fn context_lines(&self) -> u32 {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).context_lines).read() }
    }

    /// Field: git_diff_options.notify_cb
    /// Reports whether a delta-notification callback is installed.
    #[must_use]
    pub fn has_notify_callback(&self) -> bool {
        // SAFETY: this live shared handle permits the callback-slot read.
        unsafe { addr_of!((*self.as_ptr()).notify_cb).read() }.is_some()
    }

    /// Field: git_diff_options.pathspec
    /// Borrows the inline pathspec-array header.
    #[must_use]
    pub fn pathspec(&self) -> GitStrArrayRef<'object> {
        // SAFETY: raw-place projection locates the initialized inline header
        // without forming a reference to C-visible storage.
        let pathspec = unsafe { addr_of!((*self.as_ptr()).pathspec).cast_mut() };
        // SAFETY: the projected header lives for this options borrow.
        unsafe { GitStrArrayRef::from_ptr(pathspec) }.expect("an inline field is non-null")
    }

    /// Field: git_diff_options.ignore_submodules
    /// Returns the checked submodule-ignore override.
    pub fn ignore_submodules(&self) -> Result<GitSubmoduleIgnore, InvalidGitSubmoduleIgnore> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let value = unsafe { addr_of!((*self.as_ptr()).ignore_submodules).read() };
        GitSubmoduleIgnore::try_from(value)
    }
}

impl<'object, 'data> GitDiffOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path may use
    /// the options value for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_diff_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitDiffOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_diff_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitDiffOptionsRef<'_, 'data> {
        GitDiffOptionsRef(self.0.0)
    }

    /// Replaces the configured diff flags.
    pub fn set_flags(&mut self, flags: DiffOptions) {
        // SAFETY: this exclusive handle permits the scalar write, and the
        // wrapper contains only published flag bits.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Sets the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects an object-ID algorithm, or repository/library defaults.
    pub fn set_oid_type(&mut self, oid_type: Option<OidType>) {
        let oid_type = oid_type.map_or(0, ffi::git_oid_t::from);
        // SAFETY: this exclusive handle permits the scalar write, and the
        // value is zero or a published object-ID enum value.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).oid_type).write(oid_type) }
    }

    /// Sets the automatic binary-detection threshold in bytes.
    pub fn set_max_size(&mut self, max_size: ffi::git_off_t) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).max_size).write(max_size) }
    }

    /// Stores optional borrowed prefixes for old and new paths.
    pub fn set_prefixes(
        &mut self,
        old_prefix: Option<&'data core::ffi::CStr>,
        new_prefix: Option<&'data core::ffi::CStr>,
    ) {
        let old_prefix = old_prefix.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        let new_prefix = new_prefix.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both pointer writes, and the
        // wrapper lifetime keeps each non-null NUL string live.
        unsafe {
            addr_of_mut!((*options).old_prefix).write(old_prefix);
            addr_of_mut!((*options).new_prefix).write(new_prefix);
        }
    }

    /// Sets the object-ID abbreviation length.
    pub fn set_id_abbrev(&mut self, id_abbrev: u16) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).id_abbrev).write(id_abbrev) }
    }

    /// Sets the maximum unchanged lines merged between adjacent hunks.
    pub fn set_interhunk_lines(&mut self, lines: u32) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).interhunk_lines).write(lines) }
    }

    /// Sets the unchanged context lines surrounding each hunk.
    pub fn set_context_lines(&mut self, lines: u32) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).context_lines).write(lines) }
    }

    /// Borrows a pathspec-array header and copies its non-owning C view.
    pub fn set_pathspec(&mut self, pathspec: GitStrArrayRef<'data>) {
        // SAFETY: `pathspec` identifies a live initialized header. Copying the
        // C header transfers no ownership.
        let pathspec = unsafe { pathspec.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline header,
        // and the wrapper lifetime retains its borrowed storage.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).pathspec).write(pathspec) }
    }

    /// Sets the submodule-ignore override.
    pub fn set_ignore_submodules(&mut self, value: GitSubmoduleIgnore) {
        // SAFETY: this exclusive handle permits the scalar write, and `value`
        // is one of the published enum values.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).ignore_submodules).write(value.into()) }
    }

    /// Installs typed notification and progress callbacks with shared state.
    ///
    /// # Safety
    ///
    /// Neither callback method may unwind across the C boundary. The options
    /// value must not be used by a C operation that invokes these callbacks
    /// concurrently or after the `'data` borrow has ended.
    pub unsafe fn set_callbacks<C: GitDiffOptionsCallbacks>(&mut self, callbacks: &'data mut C) {
        unsafe extern "C" fn notify<C: GitDiffOptionsCallbacks>(
            diff: *const ffi::git_diff,
            delta: *const ffi::git_diff_delta,
            matched_pathspec: *const core::ffi::c_char,
            payload: *mut core::ffi::c_void,
        ) -> core::ffi::c_int {
            // SAFETY: installation stores a live exclusive `C` in `payload`.
            let Some(callbacks) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return -1;
            };
            // SAFETY: libgit2 supplies a live diff for this synchronous call.
            let Some(diff) = (unsafe { crate::diff::DiffRef::from_ptr(diff.cast_mut()) }) else {
                return -1;
            };
            // SAFETY: libgit2 supplies a live initialized delta for this
            // synchronous call.
            let Some(delta) = (unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }) else {
                return -1;
            };
            let matched_pathspec = if matched_pathspec.is_null() {
                None
            } else {
                // SAFETY: a non-null matched path from libgit2 is a live NUL
                // string for this synchronous callback.
                Some(unsafe { core::ffi::CStr::from_ptr(matched_pathspec) })
            };
            callbacks.notify(diff, delta, matched_pathspec)
        }

        unsafe extern "C" fn progress<C: GitDiffOptionsCallbacks>(
            diff: *const ffi::git_diff,
            old_path: *const core::ffi::c_char,
            new_path: *const core::ffi::c_char,
            payload: *mut core::ffi::c_void,
        ) -> core::ffi::c_int {
            // SAFETY: installation stores a live exclusive `C` in `payload`.
            let Some(callbacks) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return -1;
            };
            // SAFETY: libgit2 supplies a live diff for this synchronous call.
            let Some(diff) = (unsafe { crate::diff::DiffRef::from_ptr(diff.cast_mut()) }) else {
                return -1;
            };
            let path = |path: *const core::ffi::c_char| {
                if path.is_null() {
                    None
                } else {
                    // SAFETY: non-null paths supplied by libgit2 are live NUL
                    // strings for this synchronous callback.
                    Some(unsafe { core::ffi::CStr::from_ptr(path) })
                }
            };
            callbacks.progress(diff, path(old_path), path(new_path))
        }

        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits all three writes. The caller
        // upholds the callback boundary contract, while `'data` retains the
        // receiver's exclusive borrow.
        unsafe {
            addr_of_mut!((*options).notify_cb).write(Some(notify::<C>));
            addr_of_mut!((*options).progress_cb).write(Some(progress::<C>));
            addr_of_mut!((*options).payload)
                .write(core::ptr::from_mut(callbacks).cast::<core::ffi::c_void>());
        }
    }

    /// Clears both callbacks and their shared payload.
    pub fn clear_callbacks(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits all three writes; the null
        // representation carries no callback lifetime obligation.
        unsafe {
            addr_of_mut!((*options).notify_cb).write(None);
            addr_of_mut!((*options).progress_cb).write(None);
            addr_of_mut!((*options).payload).write(core::ptr::null_mut());
        }
    }
}

#[cfg(test)]
mod diff_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn options_preserve_layout_and_defaults() {
        assert_eq!(
            size_of::<GitDiffOptions<'static>>(),
            size_of::<ffi::git_diff_options>()
        );
        assert_eq!(
            align_of::<GitDiffOptions<'static>>(),
            align_of::<ffi::git_diff_options>()
        );
        assert_eq!(
            size_of::<GitDiffOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_diff_options>()
        );

        let options = GitDiffOptions::new();
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_DIFF_OPTIONS_VERSION);
        assert_eq!(view.flags(), Ok(DiffOptions::NORMAL));
        assert_eq!(
            view.ignore_submodules(),
            Ok(GitSubmoduleIgnore::Unspecified)
        );
        assert_eq!(view.context_lines(), 3);
        assert_eq!(view.interhunk_lines(), 0);
        assert_eq!(view.oid_type(), Ok(None));
        assert!(view.pathspec().strings().is_none());
        assert!(!view.has_notify_callback());
        assert!(!view.has_progress_callback());
        assert!(!view.has_callback_payload());
    }

    #[test]
    fn options_keep_a_scoped_data_borrow_across_shorter_object_borrows() {
        // The `compile_fail` doctest on `GitDiffOptions` covers the direction
        // that must be rejected. This covers the direction that must keep
        // working: `'data` is a local scope rather than `'static`, and the
        // value is reborrowed for several shorter `'object` lifetimes.
        let prefix = std::ffi::CString::new("scoped/").unwrap();
        let mut options = GitDiffOptions::new();
        options.as_mut().set_prefixes(Some(prefix.as_c_str()), None);
        assert_eq!(options.as_ref().old_prefix(), Some(prefix.as_c_str()));
        options.as_mut().set_context_lines(9);
        assert_eq!(options.as_ref().old_prefix(), Some(prefix.as_c_str()));
    }

    #[derive(Default)]
    struct Callbacks {
        notified: bool,
        progressed: bool,
        status: Option<crate::diff::Delta>,
    }

    impl GitDiffOptionsCallbacks for Callbacks {
        fn notify(
            &mut self,
            _diff: crate::diff::DiffRef<'_>,
            delta: DiffDeltaRef<'_>,
            _matched_pathspec: Option<&core::ffi::CStr>,
        ) -> core::ffi::c_int {
            self.notified = true;
            self.status = delta.status().ok();
            0
        }

        fn progress(
            &mut self,
            _diff: crate::diff::DiffRef<'_>,
            old_path: Option<&core::ffi::CStr>,
            new_path: Option<&core::ffi::CStr>,
        ) -> core::ffi::c_int {
            self.progressed = old_path == Some(c"old") && new_path == Some(c"new");
            0
        }
    }

    #[test]
    fn borrowed_fields_and_typed_callbacks_round_trip() {
        let mut entries = [c"src/*.c".as_ptr().cast_mut()];
        let mut pathspec = ffi::git_strarray {
            strings: entries.as_mut_ptr(),
            count: entries.len(),
        };
        // SAFETY: the stack header and pointer run stay live for the options;
        // the only string has static storage.
        let pathspec = unsafe { GitStrArrayRef::from_ptr(addr_of_mut!(pathspec)) }.unwrap();
        let mut callbacks = Callbacks::default();
        let mut options = GitDiffOptions::new();
        {
            let mut view = options.as_mut();
            view.set_flags(DiffOptions::INCLUDE_UNTRACKED | DiffOptions::PATIENCE);
            view.set_oid_type(Some(OidType::Sha256));
            view.set_max_size(-1);
            view.set_prefixes(Some(c"a/"), Some(c"b/"));
            view.set_id_abbrev(12);
            view.set_interhunk_lines(4);
            view.set_context_lines(8);
            view.set_pathspec(pathspec);
            // SAFETY: these test callbacks do not panic and are invoked only
            // synchronously below while their receiver is exclusively held.
            unsafe { view.set_callbacks(&mut callbacks) };
        }

        let view = options.as_ref();
        assert_eq!(
            view.flags(),
            Ok(DiffOptions::INCLUDE_UNTRACKED | DiffOptions::PATIENCE)
        );
        assert_eq!(view.oid_type(), Ok(Some(OidType::Sha256)));
        assert_eq!(view.max_size(), -1);
        assert_eq!(view.old_prefix(), Some(c"a/"));
        assert_eq!(view.new_prefix(), Some(c"b/"));
        assert_eq!(view.id_abbrev(), 12);
        assert_eq!(view.interhunk_lines(), 4);
        assert_eq!(view.context_lines(), 8);
        assert_eq!(view.pathspec().strings().unwrap().get(0), Some(c"src/*.c"));

        // Use layout-compatible wrapped storage for the callback's opaque diff.
        let mut diff = crate::diff::Diff::zeroed();
        // SAFETY: all-zero is valid for the delta's integer, pointer and inline
        // object-ID fields, and status is replaced with a published value.
        let mut delta: ffi::git_diff_delta = unsafe { core::mem::zeroed() };
        delta.status = ffi::git_delta_t_GIT_DELTA_ADDED;
        // SAFETY: the callback slots and payload were installed as one typed
        // protocol; all transient inputs remain live for both calls.
        unsafe {
            addr_of!((*view.as_ptr()).notify_cb).read().unwrap()(
                addr_of_mut!(diff).cast(),
                addr_of!(delta),
                c"src/*.c".as_ptr(),
                addr_of!((*view.as_ptr()).payload).read(),
            );
            addr_of!((*view.as_ptr()).progress_cb).read().unwrap()(
                addr_of_mut!(diff).cast(),
                c"old".as_ptr(),
                c"new".as_ptr(),
                addr_of!((*view.as_ptr()).payload).read(),
            );
        }
        drop(options);
        assert!(callbacks.notified);
        assert!(callbacks.progressed);
        assert_eq!(callbacks.status, Some(crate::diff::Delta::Added));
    }
}

/// Wraps: git_diff_binary_cb
/// Safe callable surface for one transient binary diff record.
pub trait GitDiffBinaryCallback {
    /// Returns zero to continue or an error code to stop traversal.
    fn call(
        &mut self,
        delta: DiffDeltaRef<'_>,
        contains_data: bool,
        old_file: crate::diff::DiffBinaryFileRef<'_>,
        new_file: crate::diff::DiffBinaryFileRef<'_>,
    ) -> i32;
}

impl<F> GitDiffBinaryCallback for F
where
    F: for<'a> FnMut(
        DiffDeltaRef<'a>,
        bool,
        crate::diff::DiffBinaryFileRef<'a>,
        crate::diff::DiffBinaryFileRef<'a>,
    ) -> i32,
{
    fn call(
        &mut self,
        delta: DiffDeltaRef<'_>,
        contains_data: bool,
        old_file: crate::diff::DiffBinaryFileRef<'_>,
        new_file: crate::diff::DiffBinaryFileRef<'_>,
    ) -> i32 {
        self(delta, contains_data, old_file, new_file)
    }
}

/// Wraps: git_diff_file_cb
/// Safe callable surface for per-file diff progress.
pub trait GitDiffFileCallback {
    /// Returns zero to continue or an error code to stop traversal.
    fn call(&mut self, delta: DiffDeltaRef<'_>, progress: f32) -> i32;
}

impl<F> GitDiffFileCallback for F
where
    F: for<'a> FnMut(DiffDeltaRef<'a>, f32) -> i32,
{
    fn call(&mut self, delta: DiffDeltaRef<'_>, progress: f32) -> i32 {
        self(delta, progress)
    }
}

/// Wraps: git_diff_hunk_cb
/// Safe callable surface for one transient text-diff hunk.
pub trait GitDiffHunkCallback {
    /// Returns zero to continue or an error code to stop traversal.
    fn call(&mut self, delta: DiffDeltaRef<'_>, hunk: crate::diff::DiffHunkRef<'_>) -> i32;
}

impl<F> GitDiffHunkCallback for F
where
    F: for<'a> FnMut(DiffDeltaRef<'a>, crate::diff::DiffHunkRef<'a>) -> i32,
{
    fn call(&mut self, delta: DiffDeltaRef<'_>, hunk: crate::diff::DiffHunkRef<'_>) -> i32 {
        self(delta, hunk)
    }
}

/// Wraps: git_diff_line_cb
/// Safe callable surface for a transient diff line or formatted header.
pub trait GitDiffLineCallback {
    /// `hunk` is absent for file headers, hunk headers and binary markers
    /// emitted by the formatting APIs.
    fn call(
        &mut self,
        delta: DiffDeltaRef<'_>,
        hunk: Option<crate::diff::DiffHunkRef<'_>>,
        line: crate::diff::DiffLineRef<'_>,
    ) -> i32;
}

impl<F> GitDiffLineCallback for F
where
    F: for<'a> FnMut(
        DiffDeltaRef<'a>,
        Option<crate::diff::DiffHunkRef<'a>>,
        crate::diff::DiffLineRef<'a>,
    ) -> i32,
{
    fn call(
        &mut self,
        delta: DiffDeltaRef<'_>,
        hunk: Option<crate::diff::DiffHunkRef<'_>>,
        line: crate::diff::DiffLineRef<'_>,
    ) -> i32 {
        self(delta, hunk, line)
    }
}

#[cfg(test)]
mod scheduled_callback_tests {
    use super::*;

    #[test]
    fn callback_traits_accept_typed_callable_shapes() {
        fn binary<C: GitDiffBinaryCallback>(_: &mut C) {}
        fn file<C: GitDiffFileCallback>(_: &mut C) {}
        fn hunk<C: GitDiffHunkCallback>(_: &mut C) {}
        fn line<C: GitDiffLineCallback>(_: &mut C) {}

        binary(&mut |_: DiffDeltaRef<'_>,
                     _: bool,
                     _: crate::diff::DiffBinaryFileRef<'_>,
                     _: crate::diff::DiffBinaryFileRef<'_>| 0);
        file(&mut |_: DiffDeltaRef<'_>, _: f32| 0);
        hunk(&mut |_: DiffDeltaRef<'_>, _: crate::diff::DiffHunkRef<'_>| 0);
        line(&mut |_: DiffDeltaRef<'_>,
                   _: Option<crate::diff::DiffHunkRef<'_>>,
                   _: crate::diff::DiffLineRef<'_>| 0);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_diff_binary
    /// A borrowed description of both sides of a binary diff.
    DiffBinary,
    DiffBinaryRef,
    DiffBinaryMut,
    ffi::git_diff_binary
);

impl<'a> DiffBinaryRef<'a> {
    /// Field: git_diff_binary.new_file
    /// Borrows the inline description of the new side.
    #[must_use]
    pub fn new_file(&self) -> crate::diff::DiffBinaryFileRef<'a> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // without forming a reference to C-visible memory.
        let file = unsafe { addr_of!((*self.as_ptr()).new_file) }.cast_mut();
        // SAFETY: the inline field is non-null and remains live for the
        // enclosing binary record's shared borrow.
        unsafe { crate::diff::DiffBinaryFileRef::from_ptr(file) }
            .expect("an inline field is non-null")
    }

    /// Field: git_diff_binary.old_file
    /// Borrows the inline description of the old side.
    #[must_use]
    pub fn old_file(&self) -> crate::diff::DiffBinaryFileRef<'a> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // without forming a reference to C-visible memory.
        let file = unsafe { addr_of!((*self.as_ptr()).old_file) }.cast_mut();
        // SAFETY: the inline field is non-null and remains live for the
        // enclosing binary record's shared borrow.
        unsafe { crate::diff::DiffBinaryFileRef::from_ptr(file) }
            .expect("an inline field is non-null")
    }

    /// Field: git_diff_binary.contains_data
    /// Returns whether the record carries compressed binary content.
    #[must_use]
    pub fn contains_data(&self) -> bool {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).contains_data).read() != 0 }
    }
}

impl DiffBinaryMut<'_> {
    /// Borrows the inline description of the new side exclusively.
    #[must_use]
    pub fn new_file_mut(&mut self) -> crate::diff::DiffBinaryFileMut<'_> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let file = unsafe { addr_of_mut!((*self.as_mut_ptr()).new_file) };
        // SAFETY: the field is non-null and exclusively borrowed for the
        // returned handle's lifetime.
        unsafe { crate::diff::DiffBinaryFileMut::from_ptr(file) }
            .expect("an inline field is non-null")
    }

    /// Borrows the inline description of the old side exclusively.
    #[must_use]
    pub fn old_file_mut(&mut self) -> crate::diff::DiffBinaryFileMut<'_> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let file = unsafe { addr_of_mut!((*self.as_mut_ptr()).old_file) };
        // SAFETY: the field is non-null and exclusively borrowed for the
        // returned handle's lifetime.
        unsafe { crate::diff::DiffBinaryFileMut::from_ptr(file) }
            .expect("an inline field is non-null")
    }

    /// Sets whether the record carries compressed binary content.
    pub fn set_contains_data(&mut self, contains_data: bool) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe {
            addr_of_mut!((*self.as_mut_ptr()).contains_data)
                .write(core::ffi::c_uint::from(contains_data));
        }
    }
}

#[cfg(test)]
mod diff_binary_tests {
    use core::mem::{align_of, size_of};

    use ffibox::CCell;

    use super::*;

    #[test]
    fn binary_record_preserves_layout_and_handle_shape() {
        fn assert_cell<T: CCell>() {}

        assert_cell::<DiffBinary>();
        assert_eq!(size_of::<DiffBinary>(), size_of::<ffi::git_diff_binary>());
        assert_eq!(align_of::<DiffBinary>(), align_of::<ffi::git_diff_binary>());
        assert_eq!(
            size_of::<DiffBinaryRef<'_>>(),
            size_of::<*const ffi::git_diff_binary>()
        );
        assert_eq!(
            size_of::<DiffBinaryMut<'_>>(),
            size_of::<*mut ffi::git_diff_binary>()
        );
    }

    #[test]
    fn binary_record_projects_and_mutates_inline_fields() {
        let empty_file = || ffi::git_diff_binary_file {
            type_: ffi::git_diff_binary_t_GIT_DIFF_BINARY_NONE,
            data: core::ptr::null(),
            datalen: 0,
            inflatedlen: 0,
        };
        let mut raw = ffi::git_diff_binary {
            contains_data: 0,
            old_file: empty_file(),
            new_file: empty_file(),
        };
        // SAFETY: `raw` is initialized and exclusively live for the handle.
        let mut binary = unsafe { DiffBinaryMut::from_ptr(&raw mut raw) }.unwrap();

        assert!(!binary.as_ref().contains_data());
        assert_eq!(
            binary.as_ref().old_file().kind(),
            Ok(crate::diff::DiffBinaryKind::None)
        );
        binary.set_contains_data(true);
        binary
            .new_file_mut()
            .set_kind(crate::diff::DiffBinaryKind::Literal);
        binary
            .old_file_mut()
            .set_kind(crate::diff::DiffBinaryKind::Delta);

        let binary = binary.as_ref();
        assert!(binary.contains_data());
        assert_eq!(
            binary.new_file().kind(),
            Ok(crate::diff::DiffBinaryKind::Literal)
        );
        assert_eq!(
            binary.old_file().kind(),
            Ok(crate::diff::DiffBinaryKind::Delta)
        );
    }
}
