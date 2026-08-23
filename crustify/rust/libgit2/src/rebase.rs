//! Safe wrappers for libgit2 rebase APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_rebase
    /// An opaque in-progress rebase managed by libgit2.
    ///
    /// Owned rebases use [`GitRebaseOwned`] and are released by
    /// `git_rebase_free`. A rebase retains a borrowed repository pointer, so
    /// safe constructors must keep that repository alive for the full lifetime
    /// of the returned owner.
    GitRebase,
    GitRebaseRef,
    GitRebaseMut,
    ffi::git_rebase
);

/// An exclusively owned, fully constructed libgit2 rebase.
pub type GitRebaseOwned = CBox<GitRebase>;

// SAFETY: `git_rebase_free` is the public destructor for a fully constructed
// `git_rebase`. It releases every owned field and the allocation, and accepts
// null although `CBox` supplies one live non-null allocation exactly once.
ffibox::impl_dropped!(GitRebase, ffi::git_rebase, ffi::git_rebase_free);

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_rebase_preserves_the_c_layout_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitRebase>();
        assert_dropped::<GitRebase>();
        assert_eq!(size_of::<GitRebase>(), size_of::<ffi::git_rebase>());
        assert_eq!(align_of::<GitRebase>(), align_of::<ffi::git_rebase>());
        assert_eq!(
            size_of::<GitRebaseRef<'_>>(),
            size_of::<*const ffi::git_rebase>()
        );
        assert_eq!(
            size_of::<GitRebaseMut<'_>>(),
            size_of::<*mut ffi::git_rebase>()
        );
        assert_eq!(
            size_of::<GitRebaseOwned>(),
            size_of::<*mut ffi::git_rebase>()
        );
    }

    #[test]
    fn borrowed_rebase_handles_preserve_the_ffi_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_rebase>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_rebase>();

        {
            // SAFETY: `raw` addresses live, suitably aligned storage for the
            // opaque FFI type and remains live for this shared handle.
            let shared = unsafe { GitRebaseRef::from_ptr(raw) }
                .expect("the Box-derived pointer is non-null");
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitRebaseMut::from_ptr(raw) }
                .expect("the Box-derived pointer is non-null");
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from `Box::into_raw`, no handle remains, and the
        // cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_rebase>>()) });
    }

    #[test]
    fn null_rebase_seams_create_no_handle() {
        // SAFETY: each conversion accepts null and returns `None` without
        // borrowing or adopting an object.
        unsafe {
            assert!(GitRebaseRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRebaseMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRebaseOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
