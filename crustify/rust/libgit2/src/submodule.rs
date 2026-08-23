//! Safe wrappers for libgit2 submodule APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CCloned};

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_submodule
    /// An opaque, reference-counted description of a repository submodule.
    ///
    /// Owned handles release one reference with `git_submodule_free` and
    /// cloning acquires another reference with `git_submodule_dup`. The
    /// submodule internally borrows its parent repository, so operations that
    /// use that repository require it to remain alive.
    GitSubmodule,
    GitSubmoduleRef,
    GitSubmoduleMut,
    ffi::git_submodule
);

/// An owned reference to a libgit2 submodule.
pub type GitSubmoduleOwned = CBox<GitSubmodule>;

// SAFETY: `git_submodule_free` consumes one reference to a fully initialized
// `git_submodule`. Its refcount decrement releases the allocation only after
// the final reference, and `GitSubmodule` is transparent over the bindgen type.
ffibox::impl_dropped!(GitSubmodule, ffi::git_submodule, ffi::git_submodule_free);

// SAFETY: `git_submodule_dup` increments the live source's refcount and writes
// the same pointer to the non-null output slot. That new reference is released
// independently by the `CDropped` implementation above.
unsafe impl CCloned for GitSubmodule {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live submodule; `duplicate`
        // is a valid output slot, and the wrapper is layout-compatible with
        // `ffi::git_submodule`.
        let result = unsafe {
            ffi::git_submodule_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_submodule>(),
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
        assert_eq!(size_of::<GitSubmodule>(), size_of::<ffi::git_submodule>());
        assert_eq!(align_of::<GitSubmodule>(), align_of::<ffi::git_submodule>());
        assert_eq!(
            size_of::<GitSubmoduleRef<'_>>(),
            size_of::<*const ffi::git_submodule>()
        );
        assert_eq!(
            size_of::<GitSubmoduleMut<'_>>(),
            size_of::<*mut ffi::git_submodule>()
        );
        assert_eq!(
            size_of::<Option<GitSubmoduleOwned>>(),
            size_of::<*mut ffi::git_submodule>()
        );
    }

    #[test]
    fn submodule_registers_refcount_lifecycle() {
        fn assert_refcounted<T: CDropped + CCloned>() {}
        assert_refcounted::<GitSubmodule>();
    }

    #[test]
    fn borrowed_handles_preserve_the_submodule_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_submodule>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_submodule>();

        {
            // SAFETY: `raw` addresses live storage for the bindgen opaque type
            // and remains allocated for the duration of this handle.
            let shared = unsafe { GitSubmoduleRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitSubmoduleMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: no borrowed handle remains and this cast recovers the exact
        // allocation returned by `Box::into_raw` above.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_submodule>>()) });
    }
}
