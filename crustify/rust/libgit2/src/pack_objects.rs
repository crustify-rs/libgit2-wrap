//! Safe wrappers for libgit2 pack objects APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_packbuilder
    /// Opaque packfile-builder state managed by libgit2.
    ///
    /// The builder retains a borrowed repository pointer and may retain
    /// callback state. Construction and callback-registration wrappers must
    /// therefore keep those dependencies alive while the builder can use them.
    GitPackbuilder,
    GitPackbuilderRef,
    GitPackbuilderMut,
    ffi::git_packbuilder
);

/// An owned libgit2 packfile builder.
pub type GitPackbuilderOwned = CBox<GitPackbuilder>;

// SAFETY: `git_packbuilder_free` is the public destructor for a fully
// initialized, libgit2-allocated `git_packbuilder`. It releases the allocation
// and every field owned by it, and accepts null although `CBox` supplies a live
// non-null allocation.
ffibox::impl_dropped!(
    GitPackbuilder,
    ffi::git_packbuilder,
    ffi::git_packbuilder_free
);

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitPackbuilder>();
        assert_dropped::<GitPackbuilder>();
        assert_eq!(
            size_of::<GitPackbuilder>(),
            size_of::<ffi::git_packbuilder>()
        );
        assert_eq!(
            align_of::<GitPackbuilder>(),
            align_of::<ffi::git_packbuilder>()
        );
        assert_eq!(
            size_of::<GitPackbuilderRef<'_>>(),
            size_of::<*const ffi::git_packbuilder>()
        );
        assert_eq!(
            size_of::<GitPackbuilderMut<'_>>(),
            size_of::<*mut ffi::git_packbuilder>()
        );
        assert_eq!(
            size_of::<Option<GitPackbuilderOwned>>(),
            size_of::<*mut ffi::git_packbuilder>()
        );
    }

    #[test]
    fn borrowed_handles_preserve_the_packbuilder_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_packbuilder>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_packbuilder>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage,
            // and the shared handle remains within this scope.
            let shared = unsafe { GitPackbuilderRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitPackbuilderMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_packbuilder>>()) });
    }
}
