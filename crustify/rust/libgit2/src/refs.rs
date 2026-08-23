//! Safe wrappers for libgit2 refs APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CCloned};

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_reference
    /// An opaque Git reference managed by libgit2.
    ///
    /// Owned handles release references with `git_reference_free`. Cloning
    /// performs the public `git_reference_dup` deep copy, including an
    /// independent reference to the reference database.
    GitReference,
    GitReferenceRef,
    GitReferenceMut,
    ffi::git_reference
);

/// An owned libgit2 reference.
pub type GitReferenceOwned = CBox<GitReference>;

// SAFETY: `git_reference_free` is the public destructor for a complete
// `git_reference` allocation and accepts null, although `CDropped` supplies a
// live non-null allocation. `GitReference` is transparent over the matching
// bindgen type.
ffibox::impl_dropped!(GitReference, ffi::git_reference, ffi::git_reference_free);

// SAFETY: `git_reference_dup` leaves its live source unchanged and, on
// success, writes a fresh fully initialized allocation that is independently
// releasable by `git_reference_free`.
unsafe impl CCloned for GitReference {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live source. `duplicate` is
        // a valid output slot, and `GitReference` is layout-compatible with
        // `ffi::git_reference`.
        let result = unsafe {
            ffi::git_reference_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_reference>(),
            )
        };

        if result == 0 {
            NonNull::new(duplicate.cast::<Self>())
        } else {
            debug_assert!(duplicate.is_null());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::{CCloned, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<GitReference>(), size_of::<ffi::git_reference>());
        assert_eq!(align_of::<GitReference>(), align_of::<ffi::git_reference>());
        assert_eq!(
            size_of::<GitReferenceRef<'_>>(),
            size_of::<*const ffi::git_reference>()
        );
        assert_eq!(
            size_of::<GitReferenceMut<'_>>(),
            size_of::<*mut ffi::git_reference>()
        );
        assert_eq!(
            size_of::<Option<GitReferenceOwned>>(),
            size_of::<*mut ffi::git_reference>()
        );
    }

    #[test]
    fn reference_registers_deep_copy_lifecycle() {
        fn assert_lifecycle<T: CDropped + CCloned>() {}
        assert_lifecycle::<GitReference>();
    }

    #[test]
    fn borrowed_handles_preserve_the_reference_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_reference>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_reference>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage,
            // and the shared handle remains within this scope.
            let shared = unsafe { GitReferenceRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitReferenceMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from `Box::into_raw`, no handle remains, and this
        // cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_reference>>()) });
    }
}
