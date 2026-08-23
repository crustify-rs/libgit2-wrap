//! Safe wrappers for libgit2 revwalk APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_revwalk
    /// An opaque revision walker managed by libgit2.
    ///
    /// A walker borrows the repository supplied at construction, so that
    /// repository must remain alive while the walker is used. Owned walker
    /// handles release their allocation with `git_revwalk_free`.
    GitRevwalk,
    GitRevwalkRef,
    GitRevwalkMut,
    ffi::git_revwalk
);

/// An owning handle to a complete revision walker.
pub type GitRevwalkOwned = CBox<GitRevwalk>;

// SAFETY: `git_revwalk_free` is the public destructor for a fully initialized
// walker. It disposes every walker-owned resource and frees the allocation;
// although the C function accepts null, `CBox` supplies a live non-null object
// exactly once.
ffibox::impl_dropped!(GitRevwalk, ffi::git_revwalk, ffi::git_revwalk_free);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_revwalk_preserves_the_c_seam_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitRevwalk>();
        assert_dropped::<GitRevwalk>();
        assert_eq!(size_of::<GitRevwalk>(), size_of::<ffi::git_revwalk>());
        assert_eq!(align_of::<GitRevwalk>(), align_of::<ffi::git_revwalk>());
        assert_eq!(
            size_of::<GitRevwalkRef<'_>>(),
            size_of::<*const ffi::git_revwalk>()
        );
        assert_eq!(
            size_of::<GitRevwalkMut<'_>>(),
            size_of::<*mut ffi::git_revwalk>()
        );
        assert_eq!(
            size_of::<GitRevwalkOwned>(),
            size_of::<*mut ffi::git_revwalk>()
        );
    }

    #[test]
    fn null_seams_create_no_revwalk_handles() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitRevwalkRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRevwalkMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRevwalkOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
