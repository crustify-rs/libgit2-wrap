//! Safe wrappers for libgit2 commit APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CCloned};

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_commit
    /// An opaque, reference-counted Git commit.
    ///
    /// Owned handles release one reference with `git_commit_free`, while
    /// cloning acquires another reference with `git_commit_dup`. A commit
    /// internally borrows its repository, so operations that consult the
    /// repository require it to remain alive.
    GitCommit,
    GitCommitRef,
    GitCommitMut,
    ffi::git_commit
);

/// An owned reference to a libgit2 commit.
pub type GitCommitOwned = CBox<GitCommit>;

// SAFETY: `git_commit_free` consumes one reference to a fully initialized
// `git_commit`; the underlying object refcount releases the allocation only
// after the final reference, and `GitCommit` is transparent over the bindgen
// type.
ffibox::impl_dropped!(GitCommit, ffi::git_commit, ffi::git_commit_free);

// SAFETY: `git_commit_dup` increments the live commit's underlying object
// refcount and writes the same pointer to its non-null output slot. The new
// reference is independently released by the `CDropped` implementation above.
unsafe impl CCloned for GitCommit {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live commit; `duplicate` is
        // a valid output slot, and the wrapper is layout-compatible with
        // `ffi::git_commit`.
        let result = unsafe {
            ffi::git_commit_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_commit>(),
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
        assert_eq!(size_of::<GitCommit>(), size_of::<ffi::git_commit>());
        assert_eq!(align_of::<GitCommit>(), align_of::<ffi::git_commit>());
        assert_eq!(
            size_of::<GitCommitRef<'_>>(),
            size_of::<*const ffi::git_commit>()
        );
        assert_eq!(
            size_of::<GitCommitMut<'_>>(),
            size_of::<*mut ffi::git_commit>()
        );
        assert_eq!(
            size_of::<Option<GitCommitOwned>>(),
            size_of::<*mut ffi::git_commit>()
        );
    }

    #[test]
    fn commit_registers_refcount_lifecycle() {
        fn assert_refcounted<T: CDropped + CCloned>() {}
        assert_refcounted::<GitCommit>();
    }

    #[test]
    fn borrowed_handles_preserve_the_commit_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_commit>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_commit>();

        {
            // SAFETY: `raw` addresses live storage for the bindgen opaque type
            // and remains allocated for the duration of this shared handle.
            let shared = unsafe { GitCommitRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitCommitMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_commit>>()) });
    }
}
