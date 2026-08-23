//! Safe wrappers for libgit2 annotated_commit APIs.

use ffibox::{define_ctype, impl_dropped};

use crate::ffi;

define_ctype!(
    /// Wraps: git_annotated_commit
    /// An annotated commit owned by libgit2.
    ///
    /// The public C API keeps this type opaque. Owned pointers use
    /// [`ffibox::CBox<AnnotatedCommit>`], which releases the object with
    /// `git_annotated_commit_free`.
    AnnotatedCommit,
    AnnotatedCommitRef,
    AnnotatedCommitMut,
    ffi::git_annotated_commit
);

// SAFETY: `git_annotated_commit_free` is the public destructor for a complete
// `git_annotated_commit` allocation and accepts null, although `CDropped` only
// supplies a live non-null allocation. `AnnotatedCommit` is transparent over
// the corresponding bindgen C type.
impl_dropped!(
    AnnotatedCommit,
    ffi::git_annotated_commit,
    ffi::git_annotated_commit_free
);

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::CDropped;

    use super::*;

    #[test]
    fn opaque_representation_and_borrowed_handles_match_the_c_seam() {
        assert_eq!(
            size_of::<AnnotatedCommit>(),
            size_of::<ffi::git_annotated_commit>()
        );
        assert_eq!(
            align_of::<AnnotatedCommit>(),
            align_of::<ffi::git_annotated_commit>()
        );
        assert_eq!(
            size_of::<AnnotatedCommitRef<'static>>(),
            size_of::<*const ffi::git_annotated_commit>()
        );
        assert_eq!(
            size_of::<AnnotatedCommitMut<'static>>(),
            size_of::<*mut ffi::git_annotated_commit>()
        );

        let storage = Box::new(MaybeUninit::<ffi::git_annotated_commit>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_annotated_commit>();

        {
            // SAFETY: `raw` addresses live zero-initialized storage for the
            // bindgen opaque type and remains live for this scope.
            let shared = unsafe { AnnotatedCommitRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, `raw` remains live, and this
            // scope has exclusive access to the storage.
            let mut exclusive = unsafe { AnnotatedCommitMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from `Box::into_raw` above, no handle remains,
        // and casting back recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_annotated_commit>>()) });
    }

    #[test]
    fn annotated_commit_registers_its_c_destructor() {
        fn assert_dropped<T: CDropped>() {}
        assert_dropped::<AnnotatedCommit>();
    }
}
