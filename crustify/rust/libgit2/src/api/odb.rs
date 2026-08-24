//! Safe wrappers for libgit2 odb APIs.

use core::ptr::{addr_of, addr_of_mut};

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

impl GitOdbOptionsRef<'_> {
    /// Field: git_odb_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> u32 {
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
    pub fn set_version(&mut self, version: u32) {
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

        assert_cell::<GitOdbOptions>();
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
    }

    #[test]
    fn borrowed_handles_read_and_write_every_option() {
        let mut storage = GitOdbOptions::zeroed();
        let raw = addr_of_mut!(storage).cast::<ffi::git_odb_options>();

        // SAFETY: `raw` points to live, initialized, layout-compatible stack
        // storage and this is its only active access path.
        let mut options = unsafe { GitOdbOptionsMut::from_ptr(raw) }
            .expect("the address of stack storage is non-null");

        assert_eq!(options.as_ref().oid_type(), Ok(None));
        options.set_version(1);
        options.set_oid_type(Some(OidType::Sha256));
        assert_eq!(options.as_ref().version(), 1);
        assert_eq!(options.as_ref().oid_type(), Ok(Some(OidType::Sha256)));

        options.set_oid_type(None);
        assert_eq!(options.as_ref().oid_type(), Ok(None));
    }

    #[test]
    fn oid_type_getter_rejects_unpublished_values() {
        let invalid = ffi::git_oid_t_GIT_OID_SHA256 + 1;
        let mut raw = ffi::git_odb_options {
            version: 1,
            oid_type: invalid,
        };
        // SAFETY: `raw` remains live and initialized for this sole shared
        // handle.
        let options = unsafe { GitOdbOptionsRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(options.oid_type().unwrap_err().value(), invalid);
    }
}
