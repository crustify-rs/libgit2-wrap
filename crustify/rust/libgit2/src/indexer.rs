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

ffibox::define_ctype!(
    /// Wraps: git_indexer_progress
    /// Progress reported while a packfile is downloaded and indexed.
    IndexerProgress,
    IndexerProgressRef,
    IndexerProgressMut,
    ffi::git_indexer_progress
);

impl IndexerProgressRef<'_> {
    /// Wraps: git_indexer_progress.received_bytes
    /// Returns the number of packfile bytes received so far.
    #[inline]
    #[must_use]
    pub fn received_bytes(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).received_bytes).read() }
    }

    /// Wraps: git_indexer_progress.indexed_deltas
    /// Returns the number of received deltas that have been indexed.
    #[inline]
    #[must_use]
    pub fn indexed_deltas(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).indexed_deltas).read() }
    }

    /// Wraps: git_indexer_progress.total_deltas
    /// Returns the number of deltas in the packfile.
    #[inline]
    #[must_use]
    pub fn total_deltas(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).total_deltas).read() }
    }

    /// Wraps: git_indexer_progress.local_objects
    /// Returns the number of local objects injected to repair a thin pack.
    #[inline]
    #[must_use]
    pub fn local_objects(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).local_objects).read() }
    }

    /// Wraps: git_indexer_progress.received_objects
    /// Returns the number of objects downloaded so far.
    #[inline]
    #[must_use]
    pub fn received_objects(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).received_objects).read() }
    }

    /// Wraps: git_indexer_progress.indexed_objects
    /// Returns the number of received objects that have been hashed.
    #[inline]
    #[must_use]
    pub fn indexed_objects(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).indexed_objects).read() }
    }

    /// Wraps: git_indexer_progress.total_objects
    /// Returns the number of objects in the packfile.
    #[inline]
    #[must_use]
    pub fn total_objects(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).total_objects).read() }
    }
}

impl IndexerProgressMut<'_> {
    /// Sets the number of packfile bytes received so far.
    #[inline]
    pub fn set_received_bytes(&mut self, value: usize) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).received_bytes).write(value) }
    }

    /// Sets the number of received deltas that have been indexed.
    #[inline]
    pub fn set_indexed_deltas(&mut self, value: u32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).indexed_deltas).write(value) }
    }

    /// Sets the number of deltas in the packfile.
    #[inline]
    pub fn set_total_deltas(&mut self, value: u32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).total_deltas).write(value) }
    }

    /// Sets the number of local objects injected to repair a thin pack.
    #[inline]
    pub fn set_local_objects(&mut self, value: u32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).local_objects).write(value) }
    }

    /// Sets the number of objects downloaded so far.
    #[inline]
    pub fn set_received_objects(&mut self, value: u32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).received_objects).write(value) }
    }

    /// Sets the number of received objects that have been hashed.
    #[inline]
    pub fn set_indexed_objects(&mut self, value: u32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).indexed_objects).write(value) }
    }

    /// Sets the number of objects in the packfile.
    #[inline]
    pub fn set_total_objects(&mut self, value: u32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).total_objects).write(value) }
    }
}

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

    #[test]
    fn progress_wrapper_preserves_the_c_layout() {
        assert_eq!(
            size_of::<IndexerProgress>(),
            size_of::<ffi::git_indexer_progress>()
        );
        assert_eq!(
            align_of::<IndexerProgress>(),
            align_of::<ffi::git_indexer_progress>()
        );
        assert_eq!(
            size_of::<IndexerProgressRef<'_>>(),
            size_of::<*const ffi::git_indexer_progress>()
        );
        assert_eq!(
            size_of::<IndexerProgressMut<'_>>(),
            size_of::<*mut ffi::git_indexer_progress>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_every_progress_counter() {
        let mut raw = ffi::git_indexer_progress {
            total_objects: 1,
            indexed_objects: 2,
            received_objects: 3,
            local_objects: 4,
            total_deltas: 5,
            indexed_deltas: 6,
            received_bytes: 7,
        };

        // SAFETY: `raw` is initialized, non-null, exclusively borrowed for the
        // handle's lifetime, and remains live until the handle is last used.
        let mut progress = unsafe { IndexerProgressMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(progress.as_ref().total_objects(), 1);
        assert_eq!(progress.as_ref().indexed_objects(), 2);
        assert_eq!(progress.as_ref().received_objects(), 3);
        assert_eq!(progress.as_ref().local_objects(), 4);
        assert_eq!(progress.as_ref().total_deltas(), 5);
        assert_eq!(progress.as_ref().indexed_deltas(), 6);
        assert_eq!(progress.as_ref().received_bytes(), 7);

        progress.set_total_objects(11);
        progress.set_indexed_objects(12);
        progress.set_received_objects(13);
        progress.set_local_objects(14);
        progress.set_total_deltas(15);
        progress.set_indexed_deltas(16);
        progress.set_received_bytes(17);

        assert_eq!(progress.as_ref().total_objects(), 11);
        assert_eq!(progress.as_ref().indexed_objects(), 12);
        assert_eq!(progress.as_ref().received_objects(), 13);
        assert_eq!(progress.as_ref().local_objects(), 14);
        assert_eq!(progress.as_ref().total_deltas(), 15);
        assert_eq!(progress.as_ref().indexed_deltas(), 16);
        assert_eq!(progress.as_ref().received_bytes(), 17);
    }
}
