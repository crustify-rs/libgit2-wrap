//! Safe wrappers for libgit2 indexer APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_indexer
    /// An opaque packfile indexer managed by libgit2.
    ///
    /// The indexer may retain borrowed object-database and callback state.
    /// Constructors that install either dependency must keep it alive for as
    /// long as the indexer can use it.
    Indexer,
    IndexerRef,
    IndexerMut,
    ffi::git_indexer
);

/// An owned libgit2 indexer allocation.
pub type IndexerOwned = CBox<Indexer>;

// SAFETY: `git_indexer_free` is the public destructor for a complete
// libgit2-allocated `git_indexer`. It releases the allocation and each field
// owned by it, and accepts null although `CBox` supplies a live non-null
// pointer.
ffibox::impl_dropped!(Indexer, ffi::git_indexer, ffi::git_indexer_free);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use ffibox::CDropped;

    use super::*;

    fn assert_drop_contract<T: CDropped>() {}

    #[test]
    fn opaque_representation_matches_the_c_seam() {
        assert_eq!(size_of::<Indexer>(), size_of::<ffi::git_indexer>());
        assert_eq!(align_of::<Indexer>(), align_of::<ffi::git_indexer>());
        assert_eq!(
            size_of::<IndexerRef<'static>>(),
            size_of::<*const ffi::git_indexer>()
        );
        assert_eq!(
            size_of::<IndexerMut<'static>>(),
            size_of::<*mut ffi::git_indexer>()
        );
        assert_eq!(
            size_of::<Option<IndexerOwned>>(),
            size_of::<*mut ffi::git_indexer>()
        );
        assert_drop_contract::<Indexer>();
    }

    #[test]
    fn borrowed_handles_preserve_the_opaque_pointer() {
        let mut storage = Indexer::zeroed();
        let ptr = core::ptr::addr_of_mut!(storage).cast::<ffi::git_indexer>();

        // SAFETY: `ptr` addresses live storage for this scope. The shared
        // handle does not escape, and it is no longer used before the
        // exclusive handle is created.
        let shared = unsafe { IndexerRef::from_ptr(ptr) }.unwrap();
        assert_eq!(shared.as_ptr(), ptr.cast_const());

        // SAFETY: `ptr` remains live and the prior shared handle is not used
        // again, so this handle has exclusive access for its lifetime.
        let mut exclusive = unsafe { IndexerMut::from_ptr(ptr) }.unwrap();
        assert_eq!(exclusive.as_mut_ptr(), ptr);
        assert_eq!(exclusive.as_ref().as_ptr(), ptr.cast_const());
    }
}
