//! Safe wrappers for libgit2 odb APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_odb
    /// An opaque, reference-counted object database managed by libgit2.
    ///
    /// Each owned handle represents one reference count. Dropping it calls
    /// `git_odb_free`; libgit2 does not publish an operation for acquiring a
    /// second count directly, so the owner intentionally does not implement
    /// `Clone`.
    GitOdb,
    GitOdbRef,
    GitOdbMut,
    ffi::git_odb
);

/// An owned reference to a libgit2 object database.
pub type GitOdbOwned = CBox<GitOdb>;

// SAFETY: `git_odb_free` consumes exactly one reference to a fully initialized
// object database and frees the object only when its reference count reaches
// zero and no repository owns it. It accepts null, although `CBox` supplies a
// live non-null object exactly once.
ffibox::impl_dropped!(GitOdb, ffi::git_odb, ffi::git_odb_free);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_odb_preserves_the_c_seam_and_drop_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitOdb>();
        assert_dropped::<GitOdb>();
        assert_eq!(size_of::<GitOdb>(), size_of::<ffi::git_odb>());
        assert_eq!(align_of::<GitOdb>(), align_of::<ffi::git_odb>());
        assert_eq!(size_of::<GitOdbRef<'_>>(), size_of::<*const ffi::git_odb>());
        assert_eq!(size_of::<GitOdbMut<'_>>(), size_of::<*mut ffi::git_odb>());
        assert_eq!(size_of::<GitOdbOwned>(), size_of::<*mut ffi::git_odb>());
    }

    #[test]
    fn null_odb_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitOdbRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitOdbMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitOdbOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
