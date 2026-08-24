//! Safe wrappers for libgit2 diff APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

use crate::ffi;
use crate::oid::{InvalidOidType, OidType};

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
