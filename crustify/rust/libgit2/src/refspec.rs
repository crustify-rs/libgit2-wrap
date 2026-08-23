//! Safe wrappers for libgit2 refspec APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_refspec
    /// An opaque mapping between local and remote reference names.
    ///
    /// The public C API keeps the layout private. An owned handle represents
    /// a complete refspec allocated by libgit2 and releases it with
    /// `git_refspec_free`.
    GitRefspec,
    GitRefspecRef,
    GitRefspecMut,
    ffi::git_refspec
);

/// An owned libgit2 refspec allocation.
pub type GitRefspecOwned = CBox<GitRefspec>;

// SAFETY: `git_refspec_free` is the public destructor for a complete
// libgit2-allocated `git_refspec`. It disposes all owned strings and frees the
// header exactly once. It accepts null, although `CBox` supplies a live,
// non-null allocation.
ffibox::impl_dropped!(GitRefspec, ffi::git_refspec, ffi::git_refspec_free);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_matches_the_c_seam() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitRefspec>();
        assert_dropped::<GitRefspec>();
        assert_eq!(size_of::<GitRefspec>(), size_of::<ffi::git_refspec>());
        assert_eq!(align_of::<GitRefspec>(), align_of::<ffi::git_refspec>());
        assert_eq!(
            size_of::<GitRefspecRef<'_>>(),
            size_of::<*const ffi::git_refspec>()
        );
        assert_eq!(
            size_of::<GitRefspecMut<'_>>(),
            size_of::<*mut ffi::git_refspec>()
        );
        assert_eq!(
            size_of::<GitRefspecOwned>(),
            size_of::<*mut ffi::git_refspec>()
        );
    }

    #[test]
    fn null_seams_create_no_refspec_handles() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitRefspecRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRefspecMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRefspecOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
