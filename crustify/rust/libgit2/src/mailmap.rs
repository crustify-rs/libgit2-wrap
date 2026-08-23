//! Safe wrappers for libgit2 mailmap APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_mailmap
    /// Opaque mailmap state managed by libgit2.
    ///
    /// Owned mailmaps are represented by [`GitMailmapOwned`] and released by
    /// `git_mailmap_free`.
    GitMailmap,
    GitMailmapRef,
    GitMailmapMut,
    ffi::git_mailmap
);

/// An exclusively owned libgit2 mailmap.
pub type GitMailmapOwned = CBox<GitMailmap>;

// SAFETY: `git_mailmap_free` is the public destructor for a fully initialized
// `git_mailmap`. It releases the entries and the allocation, and accepts null,
// although `CBox` always supplies a live non-null pointer.
ffibox::impl_dropped!(GitMailmap, ffi::git_mailmap, ffi::git_mailmap_free);

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::CDropped;

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<GitMailmap>(), size_of::<ffi::git_mailmap>());
        assert_eq!(align_of::<GitMailmap>(), align_of::<ffi::git_mailmap>());
        assert_eq!(
            size_of::<GitMailmapRef<'_>>(),
            size_of::<*const ffi::git_mailmap>()
        );
        assert_eq!(
            size_of::<GitMailmapMut<'_>>(),
            size_of::<*mut ffi::git_mailmap>()
        );
        assert_eq!(
            size_of::<Option<GitMailmapOwned>>(),
            size_of::<*mut ffi::git_mailmap>()
        );
    }

    #[test]
    fn mailmap_registers_its_c_destructor() {
        fn assert_dropped<T: CDropped>() {}
        assert_dropped::<GitMailmap>();
    }

    #[test]
    fn borrowed_handles_preserve_the_mailmap_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_mailmap>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_mailmap>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage
            // and the shared handle remains within this scope.
            let shared = unsafe { GitMailmapRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live,
            // and this scope has exclusive access to it.
            let mut exclusive = unsafe { GitMailmapMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_mailmap>>()) });
    }
}
