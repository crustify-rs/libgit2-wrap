//! Safe wrappers for libgit2 index APIs.

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
