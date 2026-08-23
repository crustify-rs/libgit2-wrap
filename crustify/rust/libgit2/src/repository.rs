//! Safe wrappers for libgit2 repository APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_repository
    /// An opaque repository handle managed by libgit2.
    ///
    /// The public C API keeps the layout private. Owned handles represent
    /// complete repository allocations and release them with
    /// `git_repository_free`; borrowed access uses [`GitRepositoryRef`] and
    /// [`GitRepositoryMut`] without forming Rust references to C-owned memory.
    GitRepository,
    GitRepositoryRef,
    GitRepositoryMut,
    ffi::git_repository
);

/// An owned libgit2 repository allocation.
pub type GitRepositoryOwned = CBox<GitRepository>;

// SAFETY: `git_repository_free` is the public destructor for a complete
// libgit2-allocated repository. It cleans up all repository-owned internals,
// clears the header, and frees its allocation exactly once. It accepts null,
// although `CBox` supplies a live non-null allocation.
ffibox::impl_dropped!(GitRepository, ffi::git_repository, ffi::git_repository_free);

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

        assert_cell::<GitRepository>();
        assert_dropped::<GitRepository>();
        assert_eq!(size_of::<GitRepository>(), size_of::<ffi::git_repository>());
        assert_eq!(
            align_of::<GitRepository>(),
            align_of::<ffi::git_repository>()
        );
        assert_eq!(
            size_of::<GitRepositoryRef<'_>>(),
            size_of::<*const ffi::git_repository>()
        );
        assert_eq!(
            size_of::<GitRepositoryMut<'_>>(),
            size_of::<*mut ffi::git_repository>()
        );
        assert_eq!(
            size_of::<GitRepositoryOwned>(),
            size_of::<*mut ffi::git_repository>()
        );
    }

    #[test]
    fn null_seams_create_no_repository_handles() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitRepositoryRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRepositoryMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRepositoryOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
