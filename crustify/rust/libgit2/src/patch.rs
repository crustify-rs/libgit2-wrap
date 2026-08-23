//! Safe wrappers for libgit2 patch APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_patch
    /// An opaque, reference-counted patch managed by libgit2.
    ///
    /// Owned patch references use [`GitPatchOwned`]. Dropping an owner
    /// decrements the patch's reference count and releases its concrete patch
    /// representation when the final reference is gone.
    GitPatch,
    GitPatchRef,
    GitPatchMut,
    ffi::git_patch
);

/// An owning reference to a fully formed libgit2 patch.
pub type GitPatchOwned = CBox<GitPatch>;

// SAFETY: `git_patch_free` consumes one owning reference to a fully formed
// `git_patch`. It decrements the embedded reference count and dispatches to the
// concrete patch destructor exactly when the final reference is released.
ffibox::impl_dropped!(GitPatch, ffi::git_patch, ffi::git_patch_free);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn patch_wrapper_preserves_the_opaque_seam_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitPatch>();
        assert_dropped::<GitPatch>();
        assert_eq!(size_of::<GitPatch>(), size_of::<ffi::git_patch>());
        assert_eq!(align_of::<GitPatch>(), align_of::<ffi::git_patch>());
        assert_eq!(
            size_of::<GitPatchRef<'_>>(),
            size_of::<*const ffi::git_patch>()
        );
        assert_eq!(
            size_of::<GitPatchMut<'_>>(),
            size_of::<*mut ffi::git_patch>()
        );
        assert_eq!(size_of::<GitPatchOwned>(), size_of::<*mut ffi::git_patch>());
    }

    #[test]
    fn null_patch_seams_create_no_handle() {
        // SAFETY: all conversions explicitly accept null and return `None`
        // without borrowing or adopting a patch.
        unsafe {
            assert!(GitPatchRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPatchMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPatchOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
