//! Safe wrappers for libgit2 push APIs.

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_push
    /// Opaque state for a push operation owned by its [`GitRemote`](crate::remote::GitRemote).
    ///
    /// `git_remote_upload` builds it into `remote->push` with `git_push_new`
    /// and the remote releases it with `git_push_free`, on reconnect and again
    /// in `git_remote_free`. Both operations are internal to
    /// `src/libgit2/push.h`; the public headers only forward-declare the type,
    /// and `git2/sys/transport.h` hands it to a transport's `push` callback as
    /// a borrowed handle. The wrapper therefore carries no ownership: it has
    /// borrowed handles and deliberately no `CDropped` or owning alias.
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
        // No public header defines `git_push`, so bindgen emits an opaque
        // placeholder. The equalities below are only meaningful next to that
        // fact, and a body appearing here means the wrapper needs revisiting.
        assert_eq!(size_of::<ffi::git_push>(), 0);
        assert_eq!(size_of::<GitPush>(), size_of::<ffi::git_push>());
        assert_eq!(align_of::<GitPush>(), align_of::<ffi::git_push>());
        assert_eq!(
            size_of::<GitPushRef<'_>>(),
            size_of::<*const ffi::git_push>()
        );
        assert_eq!(size_of::<GitPushMut<'_>>(), size_of::<*mut ffi::git_push>());
    }

    /// The handles are pointer carriers: nothing here reads through `raw`,
    /// which is what lets an opaque, zero-sized `ffi::git_push` stand in for a
    /// push a caller cannot construct. `Box` of a zero-sized value yields the
    /// aligned, non-null, uniquely-owned address Rust guarantees for one.
    #[test]
    fn borrowed_handles_preserve_the_push_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_push>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_push>();

        {
            // SAFETY: `raw` is the live, aligned, non-null address of the
            // value `storage` owns, and the shared handle, which never
            // dereferences it, stays inside this scope.
            let shared = unsafe { GitPushRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the value remains live, and
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
