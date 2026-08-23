//! Safe wrappers for libgit2 push APIs.

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_push
    /// Opaque state for a push operation owned by its [`git_remote`](crate::ffi::git_remote).
    ///
    /// Libgit2 exposes `git_push` to transport callbacks as a borrowed handle;
    /// it does not publish construction or destruction operations for callers.
    GitPush,
    GitPushRef,
    GitPushMut,
    ffi::git_push
);

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::CCell;

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        fn assert_cell<T: CCell>() {}

        assert_cell::<GitPush>();
        assert_eq!(size_of::<GitPush>(), size_of::<ffi::git_push>());
        assert_eq!(align_of::<GitPush>(), align_of::<ffi::git_push>());
        assert_eq!(
            size_of::<GitPushRef<'_>>(),
            size_of::<*const ffi::git_push>()
        );
        assert_eq!(size_of::<GitPushMut<'_>>(), size_of::<*mut ffi::git_push>());
    }

    #[test]
    fn borrowed_handles_preserve_the_push_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_push>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_push>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage,
            // and the shared handle remains within this scope.
            let shared = unsafe { GitPushRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitPushMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_push>>()) });
    }
}
