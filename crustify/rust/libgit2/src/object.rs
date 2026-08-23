//! Safe wrappers for libgit2 object APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CCloned};

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_object
    /// Opaque base object managed by libgit2.
    ///
    /// An object carries one reference to cached object storage. Owned handles
    /// release that reference with `git_object_free`, while cloning increments
    /// its reference count with `git_object_dup`. Repository-backed objects
    /// also borrow their repository, which must remain open while the object is
    /// used.
    GitObject,
    GitObjectRef,
    GitObjectMut,
    ffi::git_object
);

/// An owned reference to a libgit2 object.
pub type GitObjectOwned = CBox<GitObject>;

// SAFETY: `git_object_free` consumes one reference to a fully initialized
// `git_object`; its matching reference-count decrement releases the allocation
// only when the final reference is gone. `GitObject` is transparent over the
// corresponding bindgen type.
ffibox::impl_dropped!(GitObject, ffi::git_object, ffi::git_object_free);

// SAFETY: `git_object_dup` increments the live source object's reference count,
// writes the same pointer to its non-null output slot, and cannot fail. The new
// reference is independently released by the `CDropped` implementation above.
unsafe impl CCloned for GitObject {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live object. `duplicate` is a
        // valid output slot, and `GitObject` is layout-compatible with
        // `ffi::git_object`.
        let result = unsafe {
            ffi::git_object_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_object>(),
            )
        };
        debug_assert_eq!(result, 0);
        NonNull::new(duplicate.cast::<Self>())
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::{CCloned, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<GitObject>(), size_of::<ffi::git_object>());
        assert_eq!(align_of::<GitObject>(), align_of::<ffi::git_object>());
        assert_eq!(
            size_of::<GitObjectRef<'_>>(),
            size_of::<*const ffi::git_object>()
        );
        assert_eq!(
            size_of::<GitObjectMut<'_>>(),
            size_of::<*mut ffi::git_object>()
        );
        assert_eq!(
            size_of::<Option<GitObjectOwned>>(),
            size_of::<*mut ffi::git_object>()
        );
    }

    #[test]
    fn object_registers_refcount_lifecycle() {
        fn assert_refcounted<T: CDropped + CCloned>() {}
        assert_refcounted::<GitObject>();
    }

    #[test]
    fn borrowed_handles_preserve_the_object_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_object>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_object>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage,
            // and the shared handle remains within this scope.
            let shared = unsafe { GitObjectRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitObjectMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_object>>()) });
    }
}
