//! Safe wrappers for libgit2 index APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

use crate::ffi;
use crate::oid::{InvalidOidType, OidType};

ffibox::define_ctype!(
    /// Wraps: git_index_options
    /// Layout-compatible options for opening or creating an index.
    GitIndexOptions,
    GitIndexOptionsRef,
    GitIndexOptionsMut,
    ffi::git_index_options
);

// SAFETY: `git_index_options` contains only scalar configuration fields and
// owns no resources, so disposing an inline value requires no action.
unsafe impl CValued for GitIndexOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitIndexOptions {
    /// Constructs options equivalent to `GIT_INDEX_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        options.as_mut().set_version(ffi::GIT_INDEX_OPTIONS_VERSION);
        options
    }
}

impl GitIndexOptionsRef<'_> {
    /// Field: git_index_options.version
    /// Returns the ABI version stored in this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_index_options.oid_type
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

impl GitIndexOptionsMut<'_> {
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
mod tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn index_options_preserve_layout_and_inline_ownership() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<GitIndexOptions>();
        assert_valued::<GitIndexOptions>();
        assert_eq!(
            size_of::<GitIndexOptions>(),
            size_of::<ffi::git_index_options>()
        );
        assert_eq!(
            align_of::<GitIndexOptions>(),
            align_of::<ffi::git_index_options>()
        );
        assert_eq!(
            size_of::<GitIndexOptionsRef<'_>>(),
            size_of::<*const ffi::git_index_options>()
        );
        assert_eq!(
            size_of::<GitIndexOptionsMut<'_>>(),
            size_of::<*mut ffi::git_index_options>()
        );
        assert_eq!(
            size_of::<CVal<GitIndexOptions>>(),
            size_of::<ffi::git_index_options>()
        );
    }

    #[test]
    fn index_options_defaults_and_mutation_are_checked() {
        let mut options = GitIndexOptions::new();
        assert_eq!(options.as_ref().version(), ffi::GIT_INDEX_OPTIONS_VERSION);
        assert_eq!(options.as_ref().oid_type(), Ok(None));

        options.as_mut().set_oid_type(Some(OidType::Sha256));
        assert_eq!(options.as_ref().oid_type(), Ok(Some(OidType::Sha256)));

        options.as_mut().set_version(7);
        assert_eq!(options.as_ref().version(), 7);
    }

    #[test]
    fn index_options_reject_an_unknown_oid_type() {
        let invalid = ffi::git_oid_t_GIT_OID_SHA256 + 1;
        let mut raw = ffi::git_index_options {
            version: ffi::GIT_INDEX_OPTIONS_VERSION,
            oid_type: invalid,
        };
        // SAFETY: `raw` is initialized and remains live and unmodified while
        // this shared handle is used.
        let options = unsafe { GitIndexOptionsRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(options.oid_type().unwrap_err().value(), invalid);
    }
}

/// Wraps: git_index_add_option_t
/// A checked set of options controlling bulk additions to an index.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitIndexAddOptions(ffi::git_index_add_option_t);

impl GitIndexAddOptions {
    /// Use the default path matching and ignore behavior.
    pub const DEFAULT: Self = Self(ffi::git_index_add_option_t_GIT_INDEX_ADD_DEFAULT);
    /// Add ignored files too.
    pub const FORCE: Self = Self(ffi::git_index_add_option_t_GIT_INDEX_ADD_FORCE);
    /// Treat pathspec entries as literal paths.
    pub const DISABLE_PATHSPEC_MATCH: Self =
        Self(ffi::git_index_add_option_t_GIT_INDEX_ADD_DISABLE_PATHSPEC_MATCH);
    /// Reject an explicitly named ignored path.
    pub const CHECK_PATHSPEC: Self = Self(ffi::git_index_add_option_t_GIT_INDEX_ADD_CHECK_PATHSPEC);
    /// Every bulk-add option published by this libgit2 version.
    pub const ALL: Self =
        Self(Self::FORCE.0 | Self::DISABLE_PATHSPEC_MATCH.0 | Self::CHECK_PATHSPEC.0);

    /// Converts raw bits when every bit is a published option.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_index_add_option_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_index_add_option_t {
        self.0
    }

    /// Returns whether no optional behavior is enabled.
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

impl From<GitIndexAddOptions> for ffi::git_index_add_option_t {
    fn from(options: GitIndexAddOptions) -> Self {
        options.bits()
    }
}

impl TryFrom<ffi::git_index_add_option_t> for GitIndexAddOptions {
    type Error = ffi::git_index_add_option_t;

    fn try_from(bits: ffi::git_index_add_option_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitIndexAddOptions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitIndexAddOptions {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitIndexAddOptions {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitIndexAddOptions {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitIndexAddOptions {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

/// Wraps: git_index_capability_t
/// Validated filesystem capabilities, or a request to derive them from the owner.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitIndexCapabilities(ffi::git_index_capability_t);

impl GitIndexCapabilities {
    /// Derive capabilities from the owning repository's configuration.
    pub const FROM_OWNER: Self = Self(ffi::git_index_capability_t_GIT_INDEX_CAPABILITY_FROM_OWNER);
    /// Specify that no optional filesystem capability is present.
    pub const NONE: Self = Self(0);
    /// Compare paths without case sensitivity.
    pub const IGNORE_CASE: Self =
        Self(ffi::git_index_capability_t_GIT_INDEX_CAPABILITY_IGNORE_CASE);
    /// Ignore executable-bit differences in file modes.
    pub const NO_FILEMODE: Self =
        Self(ffi::git_index_capability_t_GIT_INDEX_CAPABILITY_NO_FILEMODE);
    /// Treat symbolic links as ordinary files.
    pub const NO_SYMLINKS: Self =
        Self(ffi::git_index_capability_t_GIT_INDEX_CAPABILITY_NO_SYMLINKS);
    /// Every explicit capability published by this libgit2 version.
    pub const ALL: Self = Self(Self::IGNORE_CASE.0 | Self::NO_FILEMODE.0 | Self::NO_SYMLINKS.0);

    /// Converts an explicit capability bit set.
    ///
    /// The `FROM_OWNER` sentinel is intentionally accepted only by
    /// [`Self::from_raw`] so it cannot be mistaken for a set containing every
    /// capability.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_index_capability_t) -> Option<Self> {
        if bits >= 0 && bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Converts either an explicit bit set or the `FROM_OWNER` sentinel.
    #[must_use]
    pub const fn from_raw(raw: ffi::git_index_capability_t) -> Option<Self> {
        if raw == Self::FROM_OWNER.0 {
            Some(Self::FROM_OWNER)
        } else {
            Self::from_bits(raw)
        }
    }

    /// Returns the underlying libgit2 value.
    #[must_use]
    pub const fn as_raw(self) -> ffi::git_index_capability_t {
        self.0
    }

    /// Returns the explicit bits, or `None` for [`Self::FROM_OWNER`].
    #[must_use]
    pub const fn bits(self) -> Option<ffi::git_index_capability_t> {
        if self.0 == Self::FROM_OWNER.0 {
            None
        } else {
            Some(self.0)
        }
    }

    /// Returns whether capabilities should be derived from the owner.
    #[must_use]
    pub const fn is_from_owner(self) -> bool {
        self.0 == Self::FROM_OWNER.0
    }

    /// Combines two explicit capability sets.
    ///
    /// Returns `None` rather than silently combining the `FROM_OWNER`
    /// sentinel with explicit bits.
    #[must_use]
    pub const fn union(self, other: Self) -> Option<Self> {
        match (self.bits(), other.bits()) {
            (Some(left), Some(right)) => Some(Self(left | right)),
            _ => None,
        }
    }

    /// Returns whether every explicit capability in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        match (self.bits(), other.bits()) {
            (Some(bits), Some(other)) => bits & other == other,
            _ => false,
        }
    }
}

impl From<GitIndexCapabilities> for ffi::git_index_capability_t {
    fn from(capabilities: GitIndexCapabilities) -> Self {
        capabilities.as_raw()
    }
}

impl TryFrom<ffi::git_index_capability_t> for GitIndexCapabilities {
    type Error = ffi::git_index_capability_t;

    fn try_from(raw: ffi::git_index_capability_t) -> Result<Self, Self::Error> {
        Self::from_raw(raw).ok_or(raw)
    }
}

/// Wraps: git_index_entry_extended_flag_t
/// A checked set of extended state bits stored in an index entry.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitIndexEntryExtendedFlags(ffi::git_index_entry_extended_flag_t);

impl GitIndexEntryExtendedFlags {
    /// No extended state bits are set.
    pub const NONE: Self = Self(0);
    /// The entry records an intent to add content later.
    pub const INTENT_TO_ADD: Self =
        Self(ffi::git_index_entry_extended_flag_t_GIT_INDEX_ENTRY_INTENT_TO_ADD);
    /// Worktree operations should leave this entry alone.
    pub const SKIP_WORKTREE: Self =
        Self(ffi::git_index_entry_extended_flag_t_GIT_INDEX_ENTRY_SKIP_WORKTREE);
    /// The entry's cached stat information is current.
    pub const UPTODATE: Self = Self(ffi::git_index_entry_extended_flag_t_GIT_INDEX_ENTRY_UPTODATE);
    /// The subset of bits persisted in an on-disk index.
    pub const PERSISTED: Self =
        Self(ffi::git_index_entry_extended_flag_t_GIT_INDEX_ENTRY_EXTENDED_FLAGS);
    /// Every extended-entry bit published by this libgit2 version.
    pub const ALL: Self = Self(Self::PERSISTED.0 | Self::UPTODATE.0);

    /// Converts raw bits when every bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_index_entry_extended_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_index_entry_extended_flag_t {
        self.0
    }

    /// Returns whether no extended state is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitIndexEntryExtendedFlags> for ffi::git_index_entry_extended_flag_t {
    fn from(flags: GitIndexEntryExtendedFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_index_entry_extended_flag_t> for GitIndexEntryExtendedFlags {
    type Error = ffi::git_index_entry_extended_flag_t;

    fn try_from(bits: ffi::git_index_entry_extended_flag_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitIndexEntryExtendedFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitIndexEntryExtendedFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitIndexEntryExtendedFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitIndexEntryExtendedFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitIndexEntryExtendedFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

#[cfg(test)]
mod entry_flag_and_stage_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn index_add_options_compose_and_validate() {
        let options = GitIndexAddOptions::FORCE | GitIndexAddOptions::CHECK_PATHSPEC;
        assert!(options.contains(GitIndexAddOptions::FORCE));
        assert!(options.intersects(GitIndexAddOptions::CHECK_PATHSPEC));
        assert_eq!(GitIndexAddOptions::from_bits(options.bits()), Some(options));
        assert_eq!(GitIndexAddOptions::from_bits(1 << 31), None);
        assert_eq!(GitIndexAddOptions::DEFAULT.bits(), 0);
    }

    #[test]
    fn index_capabilities_keep_the_owner_sentinel_distinct() {
        let explicit = GitIndexCapabilities::IGNORE_CASE
            .union(GitIndexCapabilities::NO_SYMLINKS)
            .unwrap();
        assert!(explicit.contains(GitIndexCapabilities::IGNORE_CASE));
        assert_eq!(explicit.bits(), Some(5));
        assert_eq!(GitIndexCapabilities::FROM_OWNER.bits(), None);
        assert!(GitIndexCapabilities::FROM_OWNER.is_from_owner());
        assert_eq!(
            GitIndexCapabilities::from_raw(
                ffi::git_index_capability_t_GIT_INDEX_CAPABILITY_FROM_OWNER
            ),
            Some(GitIndexCapabilities::FROM_OWNER)
        );
        assert_eq!(GitIndexCapabilities::from_bits(-1), None);
        assert_eq!(GitIndexCapabilities::from_raw(8), None);
    }

    #[test]
    fn extended_entry_flags_preserve_the_persisted_subset() {
        let flags = GitIndexEntryExtendedFlags::INTENT_TO_ADD
            | GitIndexEntryExtendedFlags::SKIP_WORKTREE
            | GitIndexEntryExtendedFlags::UPTODATE;
        assert!(flags.contains(GitIndexEntryExtendedFlags::PERSISTED));
        assert!(flags.intersects(GitIndexEntryExtendedFlags::UPTODATE));
        assert_eq!(
            GitIndexEntryExtendedFlags::from_bits(flags.bits()),
            Some(flags)
        );
        assert_eq!(GitIndexEntryExtendedFlags::from_bits(1), None);
    }

    #[test]
    fn index_enum_wrappers_match_the_c_layouts() {
        assert_eq!(
            (
                size_of::<GitIndexAddOptions>(),
                align_of::<GitIndexAddOptions>()
            ),
            (
                size_of::<ffi::git_index_add_option_t>(),
                align_of::<ffi::git_index_add_option_t>()
            )
        );
        assert_eq!(
            (
                size_of::<GitIndexCapabilities>(),
                align_of::<GitIndexCapabilities>()
            ),
            (
                size_of::<ffi::git_index_capability_t>(),
                align_of::<ffi::git_index_capability_t>()
            )
        );
        assert_eq!(
            (
                size_of::<GitIndexEntryExtendedFlags>(),
                align_of::<GitIndexEntryExtendedFlags>()
            ),
            (
                size_of::<ffi::git_index_entry_extended_flag_t>(),
                align_of::<ffi::git_index_entry_extended_flag_t>()
            )
        );
    }
}

/// Wraps: git_index_entry_flag_t
/// A checked set of flags stored in `git_index_entry.flags`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitIndexEntryFlags(ffi::git_index_entry_flag_t);

impl GitIndexEntryFlags {
    /// The entry includes an extended-flags word.
    pub const EXTENDED: Self = Self(ffi::git_index_entry_flag_t_GIT_INDEX_ENTRY_EXTENDED);
    /// The entry is assumed to remain valid in the working directory.
    pub const VALID: Self = Self(ffi::git_index_entry_flag_t_GIT_INDEX_ENTRY_VALID);

    /// Builds a flag set if it contains only published bits.
    pub const fn from_bits(bits: ffi::git_index_entry_flag_t) -> Option<Self> {
        let all = Self::EXTENDED.0 | Self::VALID.0;
        if bits & !all == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_index_entry_flag_t {
        self.0
    }

    /// Returns whether every flag in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl core::ops::BitOr for GitIndexEntryFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for GitIndexEntryFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl From<GitIndexEntryFlags> for ffi::git_index_entry_flag_t {
    fn from(flags: GitIndexEntryFlags) -> Self {
        flags.bits()
    }
}

/// Wraps: git_index_stage_t
/// A checked index stage selector.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitIndexStage {
    /// Match an entry in any stage.
    Any = ffi::git_index_stage_t_GIT_INDEX_STAGE_ANY,
    /// A normal, non-conflicted entry.
    Normal = ffi::git_index_stage_t_GIT_INDEX_STAGE_NORMAL,
    /// The common-ancestor side of a conflict.
    Ancestor = ffi::git_index_stage_t_GIT_INDEX_STAGE_ANCESTOR,
    /// Our side of a conflict.
    Ours = ffi::git_index_stage_t_GIT_INDEX_STAGE_OURS,
    /// Their side of a conflict.
    Theirs = ffi::git_index_stage_t_GIT_INDEX_STAGE_THEIRS,
}

/// A raw index stage not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitIndexStage(ffi::git_index_stage_t);

impl InvalidGitIndexStage {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_index_stage_t {
        self.0
    }
}

impl From<GitIndexStage> for ffi::git_index_stage_t {
    fn from(stage: GitIndexStage) -> Self {
        stage as Self
    }
}

impl TryFrom<ffi::git_index_stage_t> for GitIndexStage {
    type Error = InvalidGitIndexStage;

    fn try_from(stage: ffi::git_index_stage_t) -> Result<Self, Self::Error> {
        match stage {
            ffi::git_index_stage_t_GIT_INDEX_STAGE_ANY => Ok(Self::Any),
            ffi::git_index_stage_t_GIT_INDEX_STAGE_NORMAL => Ok(Self::Normal),
            ffi::git_index_stage_t_GIT_INDEX_STAGE_ANCESTOR => Ok(Self::Ancestor),
            ffi::git_index_stage_t_GIT_INDEX_STAGE_OURS => Ok(Self::Ours),
            ffi::git_index_stage_t_GIT_INDEX_STAGE_THEIRS => Ok(Self::Theirs),
            value => Err(InvalidGitIndexStage(value)),
        }
    }
}

#[cfg(test)]
mod enum_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn entry_flags_form_only_published_sets() {
        let mut flags = GitIndexEntryFlags::default();
        flags |= GitIndexEntryFlags::EXTENDED;
        flags |= GitIndexEntryFlags::VALID;

        assert!(flags.contains(GitIndexEntryFlags::EXTENDED));
        assert!(flags.contains(GitIndexEntryFlags::VALID));
        assert_eq!(GitIndexEntryFlags::from_bits(flags.bits()), Some(flags));
        assert_eq!(GitIndexEntryFlags::from_bits(1), None);
    }

    #[test]
    fn entry_flags_match_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitIndexEntryFlags>(),
            size_of::<ffi::git_index_entry_flag_t>()
        );
        assert_eq!(
            align_of::<GitIndexEntryFlags>(),
            align_of::<ffi::git_index_entry_flag_t>()
        );
    }

    #[test]
    fn published_index_stages_round_trip() {
        for stage in [
            GitIndexStage::Any,
            GitIndexStage::Normal,
            GitIndexStage::Ancestor,
            GitIndexStage::Ours,
            GitIndexStage::Theirs,
        ] {
            let raw = ffi::git_index_stage_t::from(stage);
            assert_eq!(GitIndexStage::try_from(raw), Ok(stage));
        }
    }

    #[test]
    fn index_stage_rejects_unknown_values_and_matches_the_c_layout() {
        let raw = ffi::git_index_stage_t_GIT_INDEX_STAGE_THEIRS + 1;
        assert_eq!(GitIndexStage::try_from(raw).unwrap_err().value(), raw);
        assert_eq!(
            size_of::<GitIndexStage>(),
            size_of::<ffi::git_index_stage_t>()
        );
        assert_eq!(
            align_of::<GitIndexStage>(),
            align_of::<ffi::git_index_stage_t>()
        );
    }
}
