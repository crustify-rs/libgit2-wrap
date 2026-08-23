//! Safe wrappers for libgit2 blob APIs.

use core::ptr::{NonNull, addr_of_mut};

use ffibox::{CBox, CCloned};

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_blob
    /// An opaque, reference-counted blob managed by libgit2.
    ///
    /// Owned handles release one cache reference with `git_blob_free`.
    /// Cloning an owned handle uses `git_blob_dup` to acquire an independent
    /// reference to the same blob.
    GitBlob,
    GitBlobRef,
    GitBlobMut,
    ffi::git_blob
);

/// An owned reference to a libgit2 blob.
pub type GitBlobOwned = CBox<GitBlob>;

// SAFETY: `git_blob_free` consumes one reference to a fully initialized blob
// and releases the allocation only when its cache reference count reaches zero.
ffibox::impl_dropped!(GitBlob, ffi::git_blob, ffi::git_blob_free);

// SAFETY: `git_blob_dup` increments the live source blob's cache reference
// count and writes the same pointer to its required output slot. That new
// reference is independently released by `GitBlob`'s `CDropped` contract.
unsafe impl CCloned for GitBlob {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live blob, `duplicate` is a
        // valid output slot, and `GitBlob` is layout-compatible with
        // `ffi::git_blob`.
        let result = unsafe {
            ffi::git_blob_dup(
                addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_blob>(),
            )
        };
        debug_assert_eq!(result, 0);
        NonNull::new(duplicate.cast::<Self>())
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_blob_preserves_layout_and_refcount_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_refcounted<T: CDropped + CCloned>() {}

        assert_cell::<GitBlob>();
        assert_refcounted::<GitBlob>();
        assert_eq!(size_of::<GitBlob>(), size_of::<ffi::git_blob>());
        assert_eq!(align_of::<GitBlob>(), align_of::<ffi::git_blob>());
        assert_eq!(
            size_of::<GitBlobRef<'_>>(),
            size_of::<*const ffi::git_blob>()
        );
        assert_eq!(size_of::<GitBlobMut<'_>>(), size_of::<*mut ffi::git_blob>());
        assert_eq!(size_of::<GitBlobOwned>(), size_of::<*mut ffi::git_blob>());
    }

    #[test]
    fn null_blob_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitBlobRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitBlobMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitBlobOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
