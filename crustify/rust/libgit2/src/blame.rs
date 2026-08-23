//! Safe wrappers for libgit2 blame APIs.

use core::ptr::NonNull;

use ffibox::{CDropped, define_ctype};

use crate::ffi;

define_ctype!(
    /// Wraps: git_blame
    /// An opaque blame result owned by libgit2.
    ///
    /// Use [`ffibox::CBox<GitBlame>`] for an owning handle. Dropping that handle
    /// releases the result with `git_blame_free`. Shared and exclusive borrows
    /// are represented by [`GitBlameRef`] and [`GitBlameMut`], without forming
    /// Rust references to memory that libgit2 owns.
    ///
    /// Libgit2 retains a borrowed repository pointer inside each result.
    /// Construction wrappers must therefore keep that repository alive for as
    /// long as the owning blame handle can be used.
    GitBlame,
    GitBlameRef,
    GitBlameMut,
    ffi::git_blame
);

// SAFETY: `git_blame_free` is the public destructor for a fully constructed
// `git_blame`. `CBox::from_raw` requires callers to transfer one uniquely owned
// result allocated by libgit2, and `CBox` invokes this implementation once.
unsafe impl CDropped for GitBlame {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the `CDropped` contract guarantees that `obj` denotes one
        // live, uniquely owned `git_blame` allocation.
        unsafe { ffi::git_blame_free(obj.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CBox, CCell};

    use super::*;

    #[test]
    fn opaque_blame_has_c_layout_and_lifecycle_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitBlame>();
        assert_dropped::<GitBlame>();
        assert_eq!(size_of::<GitBlame>(), size_of::<ffi::git_blame>());
        assert_eq!(align_of::<GitBlame>(), align_of::<ffi::git_blame>());
        assert_eq!(
            size_of::<GitBlameRef<'_>>(),
            size_of::<*mut ffi::git_blame>()
        );
        assert_eq!(
            size_of::<GitBlameMut<'_>>(),
            size_of::<*mut ffi::git_blame>()
        );
    }

    #[test]
    fn null_blame_seams_create_no_handle() {
        // SAFETY: each conversion explicitly accepts a null pointer and
        // returns `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitBlameRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitBlameMut::from_ptr(ptr::null_mut()).is_none());
            assert!(CBox::<GitBlame>::from_raw(ptr::null_mut()).is_none());
        }
    }
}
