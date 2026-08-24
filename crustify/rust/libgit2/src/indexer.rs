//! Safe wrappers for libgit2 indexer APIs.

use core::ffi::CStr;

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_indexer
    /// An opaque packfile indexer managed by libgit2.
    ///
    /// `git_indexer` is declared but never defined in the public headers, so
    /// no field is observable through the API and this wrapper publishes no
    /// accessors. Owned indexers are [`IndexerOwned`].
    ///
    /// An indexer copies the object database, the progress callback and the
    /// callback payload out of the options it is built from. It never up-refs,
    /// replaces or releases any of the three, and its object database is
    /// handed out mutably to the thin-pack repair path. Whoever constructs an
    /// indexer therefore keeps all three alive and unaliased for as long as
    /// the indexer can be used; `IndexerOwned` carries no lifetime of its own,
    /// so that obligation rests on the unsafe seam the pointer is adopted at.
    Indexer,
    IndexerRef,
    IndexerMut,
    ffi::git_indexer
);

/// Wraps: git_indexer_free
/// An owned libgit2 indexer allocation.
///
/// `git_indexer` has no duplicator and no reference count, so this exclusive
/// owner is the type's only ownership variant and is deliberately not `Clone`.
pub type IndexerOwned = CBox<Indexer>;

// SAFETY: `git_indexer_free` is the sole destructor libgit2 publishes for a
// `git_indexer`, and the only lifecycle operation the type records. It
// disposes the packfile stream, the object and delta vectors, the owned
// packfile, the expected-OID map, both hash contexts and the entry buffer,
// then frees the allocation; the borrowed object database, progress callback
// and payload are correctly left untouched. It reads `idx->pack`
// unconditionally, so it requires a fully constructed indexer — every route
// into `CBox` is unsafe and carries that obligation. Null is accepted, though
// `CBox` always supplies a live non-null pointer.
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
    /// Field: git_indexer_progress.received_bytes
    /// Returns the number of packfile bytes received so far.
    #[inline]
    #[must_use]
    pub fn received_bytes(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).received_bytes).read() }
    }

    /// Field: git_indexer_progress.indexed_deltas
    /// Returns the number of received deltas that have been indexed.
    #[inline]
    #[must_use]
    pub fn indexed_deltas(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).indexed_deltas).read() }
    }

    /// Field: git_indexer_progress.total_deltas
    /// Returns the number of deltas in the packfile.
    #[inline]
    #[must_use]
    pub fn total_deltas(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).total_deltas).read() }
    }

    /// Field: git_indexer_progress.local_objects
    /// Returns the number of local objects injected to repair a thin pack.
    #[inline]
    #[must_use]
    pub fn local_objects(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).local_objects).read() }
    }

    /// Field: git_indexer_progress.received_objects
    /// Returns the number of objects downloaded so far.
    #[inline]
    #[must_use]
    pub fn received_objects(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).received_objects).read() }
    }

    /// Field: git_indexer_progress.indexed_objects
    /// Returns the number of received objects that have been hashed.
    #[inline]
    #[must_use]
    pub fn indexed_objects(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).indexed_objects).read() }
    }

    /// Field: git_indexer_progress.total_objects
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
    use std::ffi::CString;
    use std::path::{Path, PathBuf};

    use ffibox::CDropped;

    use super::*;

    fn assert_drop_contract<T: CDropped>() {}

    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and refcounted;
            // `Drop` below balances this successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard. Every indexer built under it is dropped first.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A private directory an indexer may create its temporary packfile in.
    struct PackDir(PathBuf);

    impl PackDir {
        fn create(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-indexer-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a private temporary directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn entries(&self) -> usize {
            std::fs::read_dir(&self.0)
                .expect("the directory outlives the indexer")
                .count()
        }
    }

    impl Drop for PackDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

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
    fn owned_indexer_hands_out_handles_over_the_adopted_pointer() {
        let _init = Libgit2Init::acquire();
        let dir = PackDir::create("handles");
        let prefix = CString::new(dir.path().to_str().expect("a UTF-8 temporary path"))
            .expect("a path without interior NUL");

        let mut raw = core::ptr::null_mut();
        // SAFETY: libgit2 is initialized, `raw` is a writable out-slot, the
        // prefix is a live NUL-terminated existing directory, and a null
        // options pointer selects the documented defaults.
        let status =
            unsafe { ffi::git_indexer_new(&mut raw, prefix.as_ptr(), core::ptr::null_mut()) };
        assert_eq!(status, 0, "the temporary directory is writable");

        // SAFETY: success transfers exactly one complete indexer allocation,
        // which nothing else observes or frees.
        let mut indexer =
            unsafe { IndexerOwned::from_raw(raw) }.expect("success yields a non-null indexer");

        assert_eq!(indexer.as_ptr(), raw);
        assert_eq!(indexer.as_ref().as_ptr(), raw.cast_const());
        assert_eq!(indexer.as_mut().as_mut_ptr(), raw);
        assert_eq!(indexer.as_mut().as_ref().as_ptr(), raw.cast_const());
    }

    #[test]
    fn dropping_an_owned_indexer_runs_the_registered_c_destructor() {
        let _init = Libgit2Init::acquire();
        let dir = PackDir::create("destructor");
        let prefix = CString::new(dir.path().to_str().expect("a UTF-8 temporary path"))
            .expect("a path without interior NUL");

        let mut raw = core::ptr::null_mut();
        // SAFETY: as above; the call only reads the prefix and writes `raw`.
        let status =
            unsafe { ffi::git_indexer_new(&mut raw, prefix.as_ptr(), core::ptr::null_mut()) };
        assert_eq!(status, 0, "the temporary directory is writable");

        // SAFETY: success transfers exactly one complete indexer allocation.
        let indexer =
            unsafe { IndexerOwned::from_raw(raw) }.expect("success yields a non-null indexer");
        assert_eq!(
            dir.entries(),
            1,
            "a live indexer owns one uncommitted temporary packfile"
        );

        drop(indexer);

        assert_eq!(
            dir.entries(),
            0,
            "git_indexer_free unlinks the uncommitted packfile it owns"
        );
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

/// Wraps: git_indexer_append
/// Appends a counted packfile chunk and updates `stats` in place.
pub fn git_indexer_append(
    indexer: &mut IndexerMut<'_>,
    data: &[u8],
    stats: &mut IndexerProgressMut<'_>,
) -> Result<(), i32> {
    let data_ptr = if data.is_empty() {
        b"".as_ptr()
    } else {
        data.as_ptr()
    };
    // SAFETY: both handles are exclusive and live, and `data` supplies exactly
    // `data.len()` readable bytes that C consumes before returning.
    let status = unsafe {
        ffi::git_indexer_append(
            indexer.as_mut_ptr(),
            data_ptr.cast(),
            data.len(),
            stats.as_mut_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_indexer_commit
/// Resolves pending deltas and commits the final pack and index files.
pub fn git_indexer_commit(
    indexer: &mut IndexerMut<'_>,
    stats: &mut IndexerProgressMut<'_>,
) -> Result<(), i32> {
    // SAFETY: both C objects are exclusively borrowed and remain live for the
    // complete call; libgit2 retains neither pointer after it returns.
    let status = unsafe { ffi::git_indexer_commit(indexer.as_mut_ptr(), stats.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_indexer_name
/// Borrows the finalized packfile's unique NUL-terminated name.
#[must_use]
pub fn git_indexer_name<'a>(indexer: IndexerRef<'a>) -> Option<&'a CStr> {
    // SAFETY: `indexer` is live and C only reads its name pointer.
    let name = unsafe { ffi::git_indexer_name(indexer.as_ptr()) };
    if name.is_null() {
        None
    } else {
        // SAFETY: after finalization, a non-null name is NUL-terminated and
        // owned by the indexer for its remaining lifetime.
        Some(unsafe { CStr::from_ptr(name) })
    }
}

/// Wraps: git_indexer_progress_cb
/// Safe callable surface for transient indexer progress notifications.
pub trait GitIndexerProgressCallback {
    /// Returns zero to continue or a nonzero status to cancel indexing.
    fn call(&mut self, progress: IndexerProgressRef<'_>) -> i32;
}

impl<F> GitIndexerProgressCallback for F
where
    F: for<'a> FnMut(IndexerProgressRef<'a>) -> i32,
{
    fn call(&mut self, progress: IndexerProgressRef<'_>) -> i32 {
        self(progress)
    }
}

#[cfg(test)]
mod callback_tests {
    use super::*;

    #[test]
    fn progress_callback_receives_a_typed_borrow() {
        let mut raw = ffi::git_indexer_progress {
            total_objects: 9,
            indexed_objects: 0,
            received_objects: 0,
            local_objects: 0,
            total_deltas: 0,
            indexed_deltas: 0,
            received_bytes: 0,
        };
        // SAFETY: `raw` is initialized and remains live and immutable while
        // the callback uses this shared handle.
        let progress = unsafe { IndexerProgressRef::from_ptr(&raw mut raw) }.unwrap();
        let mut callback = |value: IndexerProgressRef<'_>| value.total_objects() as i32;
        assert_eq!(GitIndexerProgressCallback::call(&mut callback, progress), 9);
    }
}
