//! Safe wrappers for libgit2 odb APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

use crate::ffi;
use crate::oid::{InvalidOidType, OidType};

ffibox::define_ctype!(
    /// Wraps: git_odb_options
    /// Options selecting the object-ID format for an object database.
    GitOdbOptions,
    GitOdbOptionsRef,
    GitOdbOptionsMut,
    ffi::git_odb_options
);

// SAFETY: `git_odb_options` holds only scalar configuration fields, owns no
// resource and has no C disposer, so releasing an inline value is a no-op.
unsafe impl CValued for GitOdbOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitOdbOptions {
    /// Constructs options equivalent to `GIT_ODB_OPTIONS_INIT`.
    ///
    /// The stack initializer sets the version and leaves `oid_type` zero, which
    /// `normalize_options` in `src/libgit2/odb.c` resolves to `GIT_OID_DEFAULT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        options.as_mut().set_version(ffi::GIT_ODB_OPTIONS_VERSION);
        options
    }
}

impl GitOdbOptionsRef<'_> {
    /// Field: git_odb_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle covers the initialized C value;
        // raw-place projection reads the scalar without forming a reference
        // to C-visible storage.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_odb_options.oid_type
    /// Returns the selected object-ID format, or `None` for libgit2's default.
    ///
    /// An unrecognized nonzero value is reported instead of constructing an
    /// invalid Rust enum.
    pub fn oid_type(&self) -> Result<Option<OidType>, InvalidOidType> {
        // SAFETY: this live shared handle covers the initialized C value;
        // raw-place projection reads the scalar without forming a reference
        // to C-visible storage.
        let raw = unsafe { addr_of!((*self.as_ptr()).oid_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            OidType::try_from(raw).map(Some)
        }
    }
}

impl GitOdbOptionsMut<'_> {
    /// Sets the ABI version expected by libgit2.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects an object-ID format, or libgit2's default when `None`.
    pub fn set_oid_type(&mut self, oid_type: Option<OidType>) {
        let raw = oid_type.map_or(0, ffi::git_oid_t::from);
        // SAFETY: this exclusive handle permits a raw-place scalar write, and
        // `raw` is either the documented default sentinel or a published
        // `git_oid_t` value.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).oid_type).write(raw) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use ffibox::CCell;

    use super::*;

    #[test]
    fn odb_options_wrapper_matches_the_c_layout() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<GitOdbOptions>();
        assert_valued::<GitOdbOptions>();
        assert_eq!(
            size_of::<GitOdbOptions>(),
            size_of::<ffi::git_odb_options>()
        );
        assert_eq!(
            align_of::<GitOdbOptions>(),
            align_of::<ffi::git_odb_options>()
        );
        assert_eq!(
            size_of::<GitOdbOptionsRef<'_>>(),
            size_of::<*const ffi::git_odb_options>()
        );
        assert_eq!(
            size_of::<GitOdbOptionsMut<'_>>(),
            size_of::<*mut ffi::git_odb_options>()
        );
        assert_eq!(
            size_of::<CVal<GitOdbOptions>>(),
            size_of::<ffi::git_odb_options>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_every_option() {
        let mut options = GitOdbOptions::new();

        assert_eq!(options.as_ref().version(), ffi::GIT_ODB_OPTIONS_VERSION);
        assert_eq!(options.as_ref().oid_type(), Ok(None));

        options.as_mut().set_version(7);
        options.as_mut().set_oid_type(Some(OidType::Sha256));
        assert_eq!(options.as_ref().version(), 7);
        assert_eq!(options.as_ref().oid_type(), Ok(Some(OidType::Sha256)));

        options.as_mut().set_oid_type(None);
        assert_eq!(options.as_ref().oid_type(), Ok(None));
    }

    #[test]
    fn a_borrowed_c_value_reaches_the_same_accessors() {
        let mut storage = GitOdbOptions::zeroed();
        let raw = addr_of_mut!(storage).cast::<ffi::git_odb_options>();

        // SAFETY: `raw` points to live, initialized, layout-compatible stack
        // storage and this is its only active access path.
        let mut options = unsafe { GitOdbOptionsMut::from_ptr(raw) }
            .expect("the address of stack storage is non-null");

        assert_eq!(options.as_ref().version(), 0);
        options.set_version(ffi::GIT_ODB_OPTIONS_VERSION);
        assert_eq!(options.as_ref().version(), ffi::GIT_ODB_OPTIONS_VERSION);
    }

    #[test]
    fn oid_type_getter_rejects_unpublished_values() {
        let invalid = ffi::git_oid_t_GIT_OID_SHA256 + 1;
        let mut raw = ffi::git_odb_options {
            version: ffi::GIT_ODB_OPTIONS_VERSION,
            oid_type: invalid,
        };
        // SAFETY: `raw` remains live and initialized for this sole shared
        // handle.
        let options = unsafe { GitOdbOptionsRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(options.oid_type().unwrap_err().value(), invalid);
    }
}

/// Wraps: git_odb_lookup_flags_t
/// A checked set of flags controlling object-database lookups.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitOdbLookupFlags(ffi::git_odb_lookup_flags_t);

impl GitOdbLookupFlags {
    /// Refresh the object database after a failed first lookup.
    pub const DEFAULT: Self = Self(0);
    /// Do not refresh after a failed lookup.
    pub const NO_REFRESH: Self = Self(ffi::git_odb_lookup_flags_t_GIT_ODB_LOOKUP_NO_REFRESH);
    /// Every lookup flag published by this libgit2 version.
    pub const ALL: Self = Self(Self::NO_REFRESH.0);

    /// Converts raw bits when every bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_odb_lookup_flags_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_odb_lookup_flags_t {
        self.0
    }

    /// Returns whether the default lookup behavior is selected.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitOdbLookupFlags> for ffi::git_odb_lookup_flags_t {
    fn from(flags: GitOdbLookupFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_odb_lookup_flags_t> for GitOdbLookupFlags {
    type Error = ffi::git_odb_lookup_flags_t;

    fn try_from(bits: ffi::git_odb_lookup_flags_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitOdbLookupFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitOdbLookupFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitOdbLookupFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitOdbLookupFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitOdbLookupFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod lookup_flag_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_lookup_flags_validate_and_convert() {
        assert_eq!(
            GitOdbLookupFlags::from_bits(0),
            Some(GitOdbLookupFlags::DEFAULT)
        );
        assert!(GitOdbLookupFlags::DEFAULT.is_empty());
        assert!(GitOdbLookupFlags::NO_REFRESH.contains(GitOdbLookupFlags::NO_REFRESH));
        assert!(GitOdbLookupFlags::NO_REFRESH.intersects(GitOdbLookupFlags::NO_REFRESH));
        assert_eq!(
            ffi::git_odb_lookup_flags_t::from(GitOdbLookupFlags::NO_REFRESH),
            GitOdbLookupFlags::NO_REFRESH.bits()
        );
    }

    #[test]
    fn unknown_lookup_flags_are_rejected() {
        let unknown = GitOdbLookupFlags::ALL.bits() << 1;
        assert_eq!(GitOdbLookupFlags::from_bits(unknown), None);
        assert_eq!(GitOdbLookupFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn lookup_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitOdbLookupFlags>(),
            size_of::<ffi::git_odb_lookup_flags_t>()
        );
        assert_eq!(
            align_of::<GitOdbLookupFlags>(),
            align_of::<ffi::git_odb_lookup_flags_t>()
        );
    }
}
