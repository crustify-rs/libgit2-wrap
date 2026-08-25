//! Safe wrappers for libgit2 blob APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

use crate::ffi;
use crate::oid::{OidMut, OidRef};

/// Wraps: git_blob_filter_flag_t
/// A checked set of options controlling blob filtering.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitBlobFilterFlags(ffi::git_blob_filter_flag_t);

impl GitBlobFilterFlags {
    /// No filtering options.
    pub const NONE: Self = Self(0);
    /// Skip filtering when the blob is binary.
    pub const CHECK_FOR_BINARY: Self =
        Self(ffi::git_blob_filter_flag_t_GIT_BLOB_FILTER_CHECK_FOR_BINARY);
    /// Do not load attributes from the system-wide attributes file.
    pub const NO_SYSTEM_ATTRIBUTES: Self =
        Self(ffi::git_blob_filter_flag_t_GIT_BLOB_FILTER_NO_SYSTEM_ATTRIBUTES);
    /// Load attributes from the current `HEAD` commit.
    pub const ATTRIBUTES_FROM_HEAD: Self =
        Self(ffi::git_blob_filter_flag_t_GIT_BLOB_FILTER_ATTRIBUTES_FROM_HEAD);
    /// Load attributes from the commit selected by the filter options.
    pub const ATTRIBUTES_FROM_COMMIT: Self =
        Self(ffi::git_blob_filter_flag_t_GIT_BLOB_FILTER_ATTRIBUTES_FROM_COMMIT);
    /// Every flag published by this libgit2 API.
    pub const ALL: Self = Self(
        Self::CHECK_FOR_BINARY.0
            | Self::NO_SYSTEM_ATTRIBUTES.0
            | Self::ATTRIBUTES_FROM_HEAD.0
            | Self::ATTRIBUTES_FROM_COMMIT.0,
    );

    /// Converts raw bits when every set bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_blob_filter_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_blob_filter_flag_t {
        self.0
    }

    /// Returns whether no filtering option is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for GitBlobFilterFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitBlobFilterFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitBlobFilterFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitBlobFilterFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl From<GitBlobFilterFlags> for ffi::git_blob_filter_flag_t {
    fn from(flags: GitBlobFilterFlags) -> Self {
        flags.bits()
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_blob_filter_flags_form_checked_sets() {
        let mut flags = GitBlobFilterFlags::NONE;
        assert!(flags.is_empty());

        flags |= GitBlobFilterFlags::CHECK_FOR_BINARY | GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT;
        assert!(flags.contains(GitBlobFilterFlags::CHECK_FOR_BINARY));
        assert!(flags.contains(GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT));
        assert!(!flags.contains(GitBlobFilterFlags::ATTRIBUTES_FROM_HEAD));

        flags &=
            GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT | GitBlobFilterFlags::NO_SYSTEM_ATTRIBUTES;
        assert_eq!(flags, GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT);
        assert_eq!(ffi::git_blob_filter_flag_t::from(flags), flags.bits());
    }

    #[test]
    fn raw_blob_filter_bits_are_validated() {
        for flags in [
            GitBlobFilterFlags::NONE,
            GitBlobFilterFlags::CHECK_FOR_BINARY,
            GitBlobFilterFlags::NO_SYSTEM_ATTRIBUTES,
            GitBlobFilterFlags::ATTRIBUTES_FROM_HEAD,
            GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT,
            GitBlobFilterFlags::ALL,
        ] {
            assert_eq!(GitBlobFilterFlags::from_bits(flags.bits()), Some(flags));
        }

        assert_eq!(
            GitBlobFilterFlags::from_bits(GitBlobFilterFlags::ALL.bits() << 1),
            None
        );
    }

    #[test]
    fn blob_filter_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitBlobFilterFlags>(),
            size_of::<ffi::git_blob_filter_flag_t>()
        );
        assert_eq!(
            align_of::<GitBlobFilterFlags>(),
            align_of::<ffi::git_blob_filter_flag_t>()
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_blob_filter_options
    /// Layout-compatible options controlling blob filtering.
    GitBlobFilterOptions,
    GitBlobFilterOptionsRef,
    GitBlobFilterOptionsMut,
    ffi::git_blob_filter_options
);

// SAFETY: blob-filter options only borrow an optional commit ID and own no
// resource, so disposing their inline storage requires no action.
unsafe impl CValued for GitBlobFilterOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitBlobFilterOptions {
    /// Constructs options equivalent to `GIT_BLOB_FILTER_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        let mut view = options.as_mut();
        view.set_version(ffi::GIT_BLOB_FILTER_OPTIONS_VERSION as core::ffi::c_int);
        view.set_flags(GitBlobFilterFlags::CHECK_FOR_BINARY);
        options
    }
}

impl<'a> GitBlobFilterOptionsRef<'a> {
    /// Field: git_blob_filter_options.flags
    /// Returns the checked set of configured filter flags.
    pub fn flags(&self) -> Result<GitBlobFilterFlags, u32> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let bits = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitBlobFilterFlags::from_bits(bits).ok_or(bits)
    }

    /// Field: git_blob_filter_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_int {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_blob_filter_options.attr_commit_id
    /// Borrows the inline commit ID used by the current API.
    #[must_use]
    pub fn attr_commit_id(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection reaches the live inline field without
        // forming a reference to the C-visible options object.
        let id = unsafe { addr_of!((*self.as_ptr()).attr_commit_id) }.cast_mut();
        // SAFETY: an inline field is non-null and remains live for the options
        // handle's borrow.
        unsafe { OidRef::from_ptr(id) }.expect("an inline commit ID is non-null")
    }

    /// Field: git_blob_filter_options.commit_id
    /// Borrows the deprecated optional commit-ID pointer.
    #[must_use]
    pub fn commit_id(&self) -> Option<OidRef<'a>> {
        // SAFETY: this live shared handle permits the pointer-field read.
        let id = unsafe { addr_of!((*self.as_ptr()).commit_id).read() };
        // SAFETY: a non-null field is a borrowed ID that must remain live for
        // every use of the enclosing options value.
        unsafe { OidRef::from_ptr(id) }
    }

    /// Field: git_blob_filter_options.reserved
    /// Reports whether the conditional reserved ABI slot is non-null.
    ///
    /// With deprecated APIs enabled, bindgen names this layout-compatible slot
    /// `commit_id`; a hard-deprecation build names the same pointer slot
    /// `reserved` and requires it to remain null.
    #[must_use]
    pub fn has_reserved_value(&self) -> bool {
        // SAFETY: the conditional fields occupy the same pointer-sized ABI
        // slot, and this shared handle permits reading its initialized value.
        !unsafe { addr_of!((*self.as_ptr()).commit_id).read() }.is_null()
    }
}

impl GitBlobFilterOptionsMut<'_> {
    /// Replaces the filter flags.
    pub fn set_flags(&mut self, flags: GitBlobFilterFlags) {
        // SAFETY: this exclusive handle permits the scalar write, and the
        // wrapper contains only published flag bits.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_int) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Borrows the current inline commit ID exclusively.
    #[must_use]
    pub fn attr_commit_id_mut(&mut self) -> OidMut<'_> {
        // SAFETY: raw-place projection reaches the inline field through this
        // exclusive reborrow, and its address is non-null.
        let id = unsafe { addr_of_mut!((*self.as_mut_ptr()).attr_commit_id) };
        // SAFETY: this exclusive options reborrow uniquely covers the inline
        // field for the returned handle's lifetime.
        unsafe { OidMut::from_ptr(id) }.expect("an inline commit ID is non-null")
    }

    /// Stores a deprecated borrowed commit ID.
    ///
    /// # Safety
    ///
    /// A non-null `id` must remain live for every later use of the underlying
    /// options value, including uses after this mutable reborrow ends.
    pub unsafe fn set_borrowed_commit_id(&mut self, id: Option<OidRef<'_>>) {
        let id = id.map_or(core::ptr::null_mut(), |id| id.as_ptr().cast_mut());
        // SAFETY: this exclusive handle permits the pointer write; the caller
        // supplies the stored referent's unexpressible lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).commit_id).write(id) }
    }

    /// Clears the conditional reserved/deprecated pointer slot.
    pub fn clear_reserved_value(&mut self) {
        // SAFETY: the conditional fields share this ABI slot, and null is the
        // required reserved value as well as the deprecated default.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).commit_id).write(core::ptr::null_mut()) }
    }
}

#[cfg(test)]
mod blob_filter_options_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn options_preserve_layout_and_defaults() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<GitBlobFilterOptions>();
        assert_valued::<GitBlobFilterOptions>();
        assert_eq!(
            size_of::<GitBlobFilterOptions>(),
            size_of::<ffi::git_blob_filter_options>()
        );
        assert_eq!(
            align_of::<GitBlobFilterOptions>(),
            align_of::<ffi::git_blob_filter_options>()
        );

        let options = GitBlobFilterOptions::new();
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_BLOB_FILTER_OPTIONS_VERSION as core::ffi::c_int
        );
        assert_eq!(
            options.as_ref().flags(),
            Ok(GitBlobFilterFlags::CHECK_FOR_BINARY)
        );
        assert!(options.as_ref().commit_id().is_none());
        assert!(!options.as_ref().has_reserved_value());
    }

    #[test]
    fn flags_and_inline_commit_id_are_mutable() {
        let mut options = GitBlobFilterOptions::new();
        options
            .as_mut()
            .set_flags(GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT);
        options
            .as_mut()
            .attr_commit_id_mut()
            .set_oid_type(crate::oid::OidType::Sha1);
        assert_eq!(
            options.as_ref().flags(),
            Ok(GitBlobFilterFlags::ATTRIBUTES_FROM_COMMIT)
        );
        assert_eq!(
            options.as_ref().attr_commit_id().oid_type(),
            Ok(crate::oid::OidType::Sha1)
        );
    }
}
