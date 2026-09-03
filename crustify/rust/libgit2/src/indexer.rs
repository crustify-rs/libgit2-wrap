//! Safe wrappers for libgit2 indexer APIs.

use core::ffi::CStr;

use ffibox::CBox;

use crate::api::indexer::{GitIndexerOptionsMut, GitIndexerOptionsRef};
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

/// Wraps: git_indexer_new
/// Creates an indexer that writes its temporary pack in `prefix`.
///
/// Options configured with the unsafe borrowed-ODB or callback setters retain
/// those external obligations until the returned indexer is dropped.
pub fn git_indexer_new(
    prefix: &CStr,
    options: Option<GitIndexerOptionsRef<'_>>,
) -> Result<IndexerOwned, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut());
    // SAFETY: `out` is writable and `prefix` is a live C string. The optional
    // options value is initialized; any retained pointee obligations originate
    // only from its explicitly unsafe setters and therefore remain in force.
    let status = unsafe { ffi::git_indexer_new(&mut out, prefix.as_ptr(), options) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized indexer allocation.
    unsafe { IndexerOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_indexer_options_init
/// Initializes indexer options for the requested ABI version.
pub fn git_indexer_options_init(
    options: &mut GitIndexerOptionsMut<'_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle exposes writable layout-compatible options
    // storage, and initialization retains no pointer to the header.
    let status = unsafe { ffi::git_indexer_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod constructor_tests {
    use super::*;
    use crate::api::indexer::GitIndexerOptions;

    #[test]
    fn options_initializer_sets_the_current_version() {
        let mut raw = GitIndexerOptions::zeroed();
        // SAFETY: this layout-compatible stack value remains live and is
        // exclusively accessed through the handle.
        let mut options = unsafe {
            GitIndexerOptionsMut::from_ptr(
                core::ptr::addr_of_mut!(raw).cast::<ffi::git_indexer_options>(),
            )
        }
        .unwrap();
        git_indexer_options_init(&mut options, ffi::GIT_INDEXER_OPTIONS_VERSION).unwrap();
        assert_eq!(options.as_ref().version(), ffi::GIT_INDEXER_OPTIONS_VERSION);
        assert_eq!(options.as_ref().mode(), 0);
    }
}

/// Wraps: git_indexer_hash
/// Borrows the legacy 20-byte SHA-1 pack hash stored in an indexer.
///
/// The deprecated C function casts the indexer's raw checksum byte array to
/// `git_oid *`; current `git_oid` has an additional algorithm byte, so treating
/// that result as an `OidRef` would both shift the digest and read past the
/// legacy SHA-1 run. This wrapper preserves the actual storage contract.
#[must_use]
pub fn git_indexer_hash<'a>(indexer: IndexerRef<'a>) -> ffibox::CSlice<'a, u8> {
    // SAFETY: the live indexer owns its inline checksum for the input borrow;
    // C returns the start of that raw byte array despite its declared type.
    let hash = unsafe { ffi::git_indexer_hash(indexer.as_ptr()) }
        .cast_mut()
        .cast::<u8>();
    let hash = core::ptr::NonNull::new(hash).expect("a live indexer has an inline checksum");
    // SAFETY: this deprecated entry point is the SHA-1 API and exposes the
    // first 20 initialized checksum bytes, retained by `indexer` for `'a`.
    unsafe { ffibox::CSlice::from_raw_parts(hash, 20) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, TempDir};

    #[derive(Debug, Eq, PartialEq)]
    struct IndexerObservation {
        name: Vec<u8>,
        hash: Vec<u8>,
        total_objects: u32,
        indexed_objects: u32,
        received_objects: u32,
        total_deltas: u32,
        indexed_deltas: u32,
        received_bytes: usize,
        output_files: usize,
    }

    fn expand_delta_history(fixture: &HistoryFixture) {
        for revision in 0..24 {
            let mut content = String::with_capacity(80_000);
            for line in 0..1_500 {
                use core::fmt::Write as _;
                writeln!(
                    content,
                    "stable record {line:04}: revision marker {revision:02}"
                )
                .unwrap();
            }
            std::fs::write(fixture.directory.path().join("large-history.txt"), content).unwrap();
            let add = std::process::Command::new("git")
                .arg("-C")
                .arg(fixture.directory.path())
                .args(["add", "large-history.txt"])
                .status()
                .unwrap();
            assert!(add.success());
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(fixture.directory.path())
                .args([
                    "-c",
                    "user.name=Crustify",
                    "-c",
                    "user.email=crustify@example.com",
                    "commit",
                    "-q",
                    "-m",
                    &format!("large revision {revision}"),
                ])
                .env("GIT_AUTHOR_DATE", format!("1700001{revision:03} +0000"))
                .env("GIT_COMMITTER_DATE", format!("1700001{revision:03} +0000"))
                .status()
                .unwrap();
            assert!(status.success());
        }
    }

    fn pack_bytes(fixture: &HistoryFixture) -> Vec<u8> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args(["pack-objects", "--stdout", "--all", "--delta-base-offset"])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stdout.starts_with(b"PACK"));
        output.stdout
    }

    fn chunks(data: &[u8]) -> Vec<&[u8]> {
        let boundaries = [1usize, 3, 7, 12, 31, 64, 127, 255, 511, 1023];
        let mut offset = 0;
        let mut chunks = Vec::new();
        for length in boundaries {
            if offset >= data.len() {
                break;
            }
            let end = (offset + length).min(data.len());
            chunks.push(&data[offset..end]);
            offset = end;
        }
        if offset < data.len() {
            chunks.push(&data[offset..]);
        }
        chunks
    }

    unsafe fn raw_index(prefix: &TempDir, pack: &[u8]) -> IndexerObservation {
        let path = prefix.c_path();
        let mut indexer = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_indexer_new(&mut indexer, path.as_ptr(), core::ptr::null_mut()) },
            0
        );
        let mut stats = unsafe { core::mem::zeroed::<ffi::git_indexer_progress>() };
        for chunk in chunks(pack) {
            assert_eq!(
                unsafe {
                    ffi::git_indexer_append(indexer, chunk.as_ptr().cast(), chunk.len(), &mut stats)
                },
                0
            );
        }
        assert_eq!(unsafe { ffi::git_indexer_commit(indexer, &mut stats) }, 0);
        let name = unsafe { CStr::from_ptr(ffi::git_indexer_name(indexer)) }
            .to_bytes()
            .to_vec();
        let hash = unsafe {
            core::slice::from_raw_parts(ffi::git_indexer_hash(indexer).cast::<u8>(), 20).to_vec()
        };
        let output_files = std::fs::read_dir(prefix.path()).unwrap().count();
        unsafe { ffi::git_indexer_free(indexer) };
        IndexerObservation {
            name,
            hash,
            total_objects: stats.total_objects,
            indexed_objects: stats.indexed_objects,
            received_objects: stats.received_objects,
            total_deltas: stats.total_deltas,
            indexed_deltas: stats.indexed_deltas,
            received_bytes: stats.received_bytes,
            output_files,
        }
    }

    fn safe_index(prefix: &TempDir, pack: &[u8]) -> IndexerObservation {
        let path = prefix.c_path();
        let mut indexer = git_indexer_new(&path, None).unwrap();
        let mut stats = unsafe { core::mem::zeroed::<ffi::git_indexer_progress>() };
        let mut stats_handle =
            unsafe { IndexerProgressMut::from_ptr(core::ptr::from_mut(&mut stats)) }.unwrap();
        for chunk in chunks(pack) {
            git_indexer_append(&mut indexer.as_mut(), chunk, &mut stats_handle).unwrap();
        }
        git_indexer_commit(&mut indexer.as_mut(), &mut stats_handle).unwrap();
        let name = git_indexer_name(indexer.as_ref())
            .unwrap()
            .to_bytes()
            .to_vec();
        let hash = git_indexer_hash(indexer.as_ref()).elems().collect();
        let progress = stats_handle.as_ref();
        IndexerObservation {
            name,
            hash,
            total_objects: progress.total_objects(),
            indexed_objects: progress.indexed_objects(),
            received_objects: progress.received_objects(),
            total_deltas: progress.total_deltas(),
            indexed_deltas: progress.indexed_deltas(),
            received_bytes: progress.received_bytes(),
            output_files: std::fs::read_dir(prefix.path()).unwrap().count(),
        }
    }

    #[test]
    fn io_equiv_incremental_pack_indexing_and_finalization() {
        let _libgit2 = Libgit2Init::acquire();
        let fixture = HistoryFixture::new("indexer-pack-source");
        expand_delta_history(&fixture);
        let pack = pack_bytes(&fixture);
        let raw_output = TempDir::new("indexer-pack-raw");
        let safe_output = TempDir::new("indexer-pack-safe");
        let raw = unsafe { raw_index(&raw_output, &pack) };
        assert_eq!(raw, safe_index(&safe_output, &pack));
        assert!(raw.total_objects >= 10);
        assert_eq!(raw.output_files, 2);
    }
}
