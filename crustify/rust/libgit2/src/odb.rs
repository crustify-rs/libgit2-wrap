//! Safe wrappers for libgit2 odb APIs.

use core::ffi::{CStr, c_void};
use core::marker::PhantomData;
use core::ptr::NonNull;

use ffibox::{CBox, CCloned, CSlice, CSliceMut, CVal};

pub use crate::api::odb::GitOdbLookupFlags;
use crate::api::odb::{GitOdbExpandId, GitOdbOptions, GitOdbOptionsRef};
use crate::api::odb_backend::{
    GitOdbStreamMut, GitOdbStreamOwned, OdbWritepackMut, OdbWritepackOwned, OdbWritepackRef,
};
use crate::api::types::GitObjectType;
use crate::commit_graph::GitCommitGraphOwned;
use crate::ffi;
use crate::indexer::{GitIndexerProgressCallback, IndexerProgressRef};
use crate::oid::{Oid, OidRef};
use crate::sys::odb_backend::{GitOdbBackendOwned, GitOdbBackendRef};

ffibox::define_ctype!(
    /// Wraps: git_odb
    /// An opaque, reference-counted object database managed by libgit2.
    ///
    /// Each owned handle represents one reference count. Dropping it calls
    /// `git_odb_free`; libgit2 does not publish an operation for acquiring a
    /// second count directly, so the owner intentionally does not implement
    /// `Clone`.
    ///
    /// A database reached through `git_repository_odb` additionally carries
    /// the repository as its refcount owner, and `GIT_REFCOUNT_DEC` frees
    /// nothing while an owner is recorded. Dropping such a handle releases the
    /// count only; the repository still frees the database.
    GitOdb,
    GitOdbRef,
    GitOdbMut,
    ffi::git_odb
);

/// An owned reference to a libgit2 object database.
pub type GitOdbOwned = CBox<GitOdb>;

// SAFETY: `git_odb_free` consumes exactly one reference to a fully initialized
// object database and frees the object only when its reference count reaches
// zero and no repository owns it. It accepts null, although `CBox` supplies a
// live non-null object exactly once.
ffibox::impl_dropped!(GitOdb, ffi::git_odb, ffi::git_odb_free);

/// Wraps: git_odb_foreach_cb
/// Safe callable surface for object IDs visited by an object database.
pub trait GitOdbForeachCallback {
    /// Receives one transient object ID. Nonzero stops iteration.
    fn call(&mut self, oid: OidRef<'_>) -> i32;
}

impl<F> GitOdbForeachCallback for F
where
    F: for<'a> FnMut(OidRef<'a>) -> i32,
{
    fn call(&mut self, oid: OidRef<'_>) -> i32 {
        self(oid)
    }
}

/// Wraps: git_odb_hash
/// Hashes an in-memory object body with its Git object header.
pub fn git_odb_hash(data: &[u8], kind: GitObjectType) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable and `data` is a readable run of `len` bytes;
    // libgit2 retains neither pointer.
    let status = unsafe {
        ffi::git_odb_hash(
            core::ptr::addr_of_mut!(out).cast(),
            data.as_ptr().cast(),
            data.len(),
            kind.as_raw(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_odb_hashfile
/// Hashes the file at `path` as a Git object of `kind`.
pub fn git_odb_hashfile(path: &core::ffi::CStr, kind: GitObjectType) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable and `path` is a live C string retained only
    // for this call.
    let status = unsafe {
        ffi::git_odb_hashfile(
            core::ptr::addr_of_mut!(out).cast(),
            path.as_ptr(),
            kind.as_raw(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_odb_preserves_the_c_seam_and_drop_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitOdb>();
        assert_dropped::<GitOdb>();
        assert_eq!(size_of::<GitOdb>(), size_of::<ffi::git_odb>());
        assert_eq!(align_of::<GitOdb>(), align_of::<ffi::git_odb>());
        assert_eq!(size_of::<GitOdbRef<'_>>(), size_of::<*const ffi::git_odb>());
        assert_eq!(size_of::<GitOdbMut<'_>>(), size_of::<*mut ffi::git_odb>());
        assert_eq!(size_of::<GitOdbOwned>(), size_of::<*mut ffi::git_odb>());
    }

    #[test]
    fn null_odb_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitOdbRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitOdbMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitOdbOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// A stream whose backend remains protected by an exclusive ODB borrow.
pub struct GitOdbScopedStream<'odb> {
    inner: GitOdbStreamOwned,
    _odb: PhantomData<&'odb mut GitOdb>,
}

impl GitOdbScopedStream<'_> {
    /// Exclusively reborrows the underlying stream.
    pub fn as_mut(&mut self) -> GitOdbStreamMut<'_> {
        self.inner.as_mut()
    }
}

/// A successful C call returned an unknown object kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidOdbObjectType(pub ffi::git_object_t);

fn status_result(status: i32) -> Result<(), i32> {
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_odb_add_disk_alternate
pub fn git_odb_add_disk_alternate(odb: &mut GitOdbMut<'_>, path: &CStr) -> Result<(), i32> {
    // SAFETY: the ODB is exclusively borrowed while it installs the backend,
    // and it copies the live path during this call.
    status_result(unsafe { ffi::git_odb_add_disk_alternate(odb.as_mut_ptr(), path.as_ptr()) })
}

/// Wraps: git_odb_exists
#[must_use]
pub fn git_odb_exists(odb: &mut GitOdbMut<'_>, id: OidRef<'_>) -> bool {
    // SAFETY: the ODB is exclusively borrowed because lookup may refresh its
    // backend set; `id` remains live for the call.
    unsafe { ffi::git_odb_exists(odb.as_mut_ptr(), id.as_ptr()) != 0 }
}

/// Wraps: git_odb_exists_ext
#[must_use]
pub fn git_odb_exists_ext(
    odb: &mut GitOdbMut<'_>,
    id: OidRef<'_>,
    flags: GitOdbLookupFlags,
) -> bool {
    // SAFETY: the ODB is exclusively borrowed and `id` is live; `flags`
    // contains only published bits by construction.
    unsafe { ffi::git_odb_exists_ext(odb.as_mut_ptr(), id.as_ptr(), flags.bits()) != 0 }
}

/// Wraps: git_odb_exists_prefix
pub fn git_odb_exists_prefix(
    odb: &mut GitOdbMut<'_>,
    short_id: OidRef<'_>,
    hex_len: usize,
) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable, the ODB is exclusively borrowed because the
    // lookup may refresh it, and the abbreviated ID remains live.
    let status = unsafe {
        ffi::git_odb_exists_prefix(
            core::ptr::addr_of_mut!(out).cast(),
            odb.as_mut_ptr(),
            short_id.as_ptr(),
            hex_len,
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

unsafe extern "C" fn foreach_trampoline<C: GitOdbForeachCallback>(
    id: *const ffi::git_oid,
    payload: *mut core::ffi::c_void,
) -> i32 {
    if id.is_null() || payload.is_null() {
        return -1;
    }
    // SAFETY: the wrapper supplies the exact live callback object as payload
    // for this synchronous traversal.
    let callback = unsafe { &mut *payload.cast::<C>() };
    // SAFETY: libgit2 supplies one complete transient OID for this invocation.
    let id = unsafe { OidRef::from_ptr(id.cast_mut()) }.expect("callback ID is non-null");
    callback.call(id)
}

/// Wraps: git_odb_foreach
pub fn git_odb_foreach<C: GitOdbForeachCallback>(
    odb: &mut GitOdbMut<'_>,
    callback: &mut C,
) -> Result<(), i32> {
    // SAFETY: the ODB and callback remain exclusively live for the synchronous
    // traversal; the trampoline reconstructs the exact callback type.
    status_result(unsafe {
        ffi::git_odb_foreach(
            odb.as_mut_ptr(),
            Some(foreach_trampoline::<C>),
            (callback as *mut C).cast(),
        )
    })
}

/// Wraps: git_odb_new
pub fn git_odb_new() -> Result<GitOdbOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable slot for one fresh owned ODB count.
    let status = unsafe { ffi::git_odb_new(&mut out) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized ODB count.
    unsafe { GitOdbOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_odb_open_rstream
/// Opens a read stream tied to the database that owns its backend.
pub fn git_odb_open_rstream<'odb>(
    odb: &'odb mut GitOdbMut<'_>,
    id: OidRef<'_>,
) -> Result<(GitOdbScopedStream<'odb>, usize, GitObjectType), i32> {
    let mut stream = core::ptr::null_mut();
    let mut len = 0;
    let mut kind = ffi::git_object_t_GIT_OBJECT_INVALID;
    // SAFETY: all outputs are writable, the ODB is exclusively borrowed for
    // the returned stream lifetime, and `id` remains live for this call.
    let status = unsafe {
        ffi::git_odb_open_rstream(
            &mut stream,
            &mut len,
            &mut kind,
            odb.as_mut_ptr(),
            id.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    let Some(kind) = GitObjectType::from_raw(kind) else {
        // A successful constructor still owns the stream; adopt it before
        // returning an error so its destructor runs.
        // SAFETY: success transferred one complete stream allocation.
        drop(unsafe { GitOdbStreamOwned::from_raw(stream) });
        return Err(ffi::git_error_code_GIT_ERROR);
    };
    // SAFETY: success transfers one fully constructed stream allocation whose
    // backend remains alive through the exclusive ODB borrow carried below.
    let inner = unsafe { GitOdbStreamOwned::from_raw(stream) }
        .expect("successful stream construction returns non-null");
    Ok((
        GitOdbScopedStream {
            inner,
            _odb: PhantomData,
        },
        len,
        kind,
    ))
}

/// Wraps: git_odb_open_wstream
/// Opens a write stream tied to the database that owns its backend.
pub fn git_odb_open_wstream<'odb>(
    odb: &'odb mut GitOdbMut<'_>,
    size: ffi::git_object_size_t,
    kind: GitObjectType,
) -> Result<GitOdbScopedStream<'odb>, i32> {
    let mut stream = core::ptr::null_mut();
    // SAFETY: `stream` is writable, the ODB stays exclusively borrowed for
    // the returned stream lifetime, and `kind` is a checked C value.
    let status =
        unsafe { ffi::git_odb_open_wstream(&mut stream, odb.as_mut_ptr(), size, kind.as_raw()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully constructed stream allocation whose
    // borrowed backend is protected by the carried ODB lifetime.
    let inner = unsafe { GitOdbStreamOwned::from_raw(stream) }
        .expect("successful stream construction returns non-null");
    Ok(GitOdbScopedStream {
        inner,
        _odb: PhantomData,
    })
}

/// Wraps: git_odb_read_header
pub fn git_odb_read_header(
    odb: &mut GitOdbMut<'_>,
    id: OidRef<'_>,
) -> Result<(usize, GitObjectType), i32> {
    let mut len = 0;
    let mut kind = ffi::git_object_t_GIT_OBJECT_INVALID;
    // SAFETY: both outputs are writable, the ODB is exclusively borrowed, and
    // `id` remains live for this possibly refreshing lookup.
    let status =
        unsafe { ffi::git_odb_read_header(&mut len, &mut kind, odb.as_mut_ptr(), id.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    GitObjectType::from_raw(kind)
        .map(|kind| (len, kind))
        .ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_odb_refresh
pub fn git_odb_refresh(odb: &mut GitOdbMut<'_>) -> Result<(), i32> {
    // SAFETY: the ODB is exclusively borrowed while C refreshes its backends.
    status_result(unsafe { ffi::git_odb_refresh(odb.as_mut_ptr()) })
}

/// Wraps: git_odb_stream_finalize_write
pub fn git_odb_stream_finalize_write(stream: &mut GitOdbScopedStream<'_>) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    let mut stream = stream.as_mut();
    // SAFETY: `out` is writable and the scoped owner exclusively borrows a
    // live stream whose database/backend remains alive.
    let status = unsafe {
        ffi::git_odb_stream_finalize_write(core::ptr::addr_of_mut!(out).cast(), stream.as_mut_ptr())
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_odb_stream_read
pub fn git_odb_stream_read(
    stream: &mut GitOdbScopedStream<'_>,
    buffer: &mut [u8],
) -> Result<usize, i32> {
    let mut stream = stream.as_mut();
    // SAFETY: the scoped owner exclusively borrows a live stream and `buffer`
    // supplies exactly its length in writable bytes for the synchronous read.
    let status = unsafe {
        ffi::git_odb_stream_read(
            stream.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
        )
    };
    if status < 0 {
        Err(status)
    } else {
        let count = usize::try_from(status).expect("a nonnegative C int fits usize");
        if count <= buffer.len() {
            Ok(count)
        } else {
            Err(ffi::git_error_code_GIT_ERROR)
        }
    }
}

/// Wraps: git_odb_stream_write
pub fn git_odb_stream_write(stream: &mut GitOdbScopedStream<'_>, buffer: &[u8]) -> Result<(), i32> {
    let mut stream = stream.as_mut();
    // SAFETY: the scoped owner exclusively borrows a live write stream and
    // `buffer` supplies exactly its length in readable bytes.
    status_result(unsafe {
        ffi::git_odb_stream_write(stream.as_mut_ptr(), buffer.as_ptr().cast(), buffer.len())
    })
}

#[cfg(test)]
mod scheduled_wrapper_tests {
    use super::*;

    #[test]
    fn new_empty_odb_is_owned_and_refreshable() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after the ODB owner is dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut odb = git_odb_new().unwrap();
        git_odb_refresh(&mut odb.as_mut()).unwrap();
        drop(odb);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn lookup_flags_expose_only_published_bits() {
        assert_eq!(GitOdbLookupFlags::DEFAULT.bits(), 0);
        assert_eq!(
            GitOdbLookupFlags::NO_REFRESH.bits(),
            ffi::git_odb_lookup_flags_t_GIT_ODB_LOOKUP_NO_REFRESH
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_odb_object
    /// A reference-counted object-database value managed by libgit2's cache.
    ///
    /// The concrete layout remains C-compatible because cached values embed a
    /// `git_cached_obj` prefix. Safe consumers use the borrowed handles and
    /// [`GitOdbObjectOwned`] rather than references to that C-visible storage.
    GitOdbObject,
    GitOdbObjectRef,
    GitOdbObjectMut,
    ffi::git_odb_object
);

/// One independently owned reference to an object-database value.
pub type GitOdbObjectOwned = CBox<GitOdbObject>;

// SAFETY: `git_odb_object_free` consumes one cache reference to a fully
// initialized object. The final decrement releases the object's owned byte
// buffer and then its allocation; null is accepted although `CBox` is non-null.
ffibox::impl_dropped!(GitOdbObject, ffi::git_odb_object, ffi::git_odb_object_free);

// SAFETY: `git_odb_object_dup` increments the live source's cache reference
// count and publishes the same address in the output slot. The new reference
// is independently balanced by `CDropped` above.
unsafe impl CCloned for GitOdbObject {
    unsafe fn c_clone(object: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the trait caller supplies a live object and `duplicate` is a
        // writable output slot. The wrapper is transparent over the C layout.
        let status = unsafe {
            ffi::git_odb_object_dup(core::ptr::addr_of_mut!(duplicate), object.as_ptr().cast())
        };
        debug_assert_eq!(status, 0);
        (status == 0)
            .then(|| NonNull::new(duplicate.cast::<Self>()))
            .flatten()
    }
}

#[cfg(test)]
mod object_tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CCloned, CDropped};

    use super::*;

    #[test]
    fn object_wrapper_preserves_layout_and_refcount_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_refcounted<T: CDropped + CCloned>() {}

        assert_cell::<GitOdbObject>();
        assert_refcounted::<GitOdbObject>();
        assert_eq!(size_of::<GitOdbObject>(), size_of::<ffi::git_odb_object>());
        assert_eq!(
            align_of::<GitOdbObject>(),
            align_of::<ffi::git_odb_object>()
        );
        assert_eq!(
            size_of::<GitOdbObjectRef<'_>>(),
            size_of::<*const ffi::git_odb_object>()
        );
        assert_eq!(
            size_of::<GitOdbObjectMut<'_>>(),
            size_of::<*mut ffi::git_odb_object>()
        );
        assert_eq!(
            size_of::<Option<GitOdbObjectOwned>>(),
            size_of::<*mut ffi::git_odb_object>()
        );
    }

    #[test]
    fn null_object_seams_create_no_handle() {
        // SAFETY: each conversion explicitly accepts null and returns `None`
        // without borrowing or adopting any object.
        unsafe {
            assert!(GitOdbObjectRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitOdbObjectMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitOdbObjectOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// Wraps: git_odb_write
/// Writes one in-memory object and returns its object ID.
pub fn git_odb_write(odb: GitOdbRef<'_>, data: &[u8], kind: GitObjectType) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable, `odb` is live, and `data` describes exactly
    // the readable byte run passed to libgit2. No argument is retained.
    let status = unsafe {
        ffi::git_odb_write(
            core::ptr::addr_of_mut!(out).cast(),
            odb.as_ptr().cast_mut(),
            data.as_ptr().cast(),
            data.len(),
            kind.as_raw(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// An owned writepack together with the database and callback it may retain.
///
/// The writepack is dropped before the boxed callback, so its destructor can
/// still use the registered payload.
pub struct GitOdbWritepack<'db, C: GitIndexerProgressCallback> {
    inner: OdbWritepackOwned,
    callback: Box<C>,
    _database: PhantomData<GitOdbRef<'db>>,
}

impl<C: GitIndexerProgressCallback> GitOdbWritepack<'_, C> {
    /// Borrows the writepack.
    pub fn as_ref(&self) -> OdbWritepackRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the writepack exclusively.
    pub fn as_mut(&mut self) -> OdbWritepackMut<'_> {
        self.inner.as_mut()
    }

    /// Borrows the callback retained for later append and commit operations.
    pub fn callback(&self) -> &C {
        &self.callback
    }
}

unsafe extern "C" fn writepack_progress<C: GitIndexerProgressCallback>(
    stats: *const ffi::git_indexer_progress,
    payload: *mut c_void,
) -> i32 {
    if stats.is_null() || payload.is_null() {
        return -1;
    }
    // SAFETY: `git_odb_write_pack` receives this pointer from the stable box
    // stored in `GitOdbWritepack`, which outlives every callback invocation.
    let callback = unsafe { &mut *payload.cast::<C>() };
    // SAFETY: libgit2 supplies a live transient progress record for this call.
    let stats = unsafe { IndexerProgressRef::from_ptr(stats.cast_mut()) }
        .expect("the callback rejected null above");
    callback.call(stats)
}

/// Wraps: git_odb_write_pack
/// Opens a writepack whose progress callback remains valid for its lifetime.
pub fn git_odb_write_pack<'db, C>(
    odb: GitOdbRef<'db>,
    callback: C,
) -> Result<GitOdbWritepack<'db, C>, i32>
where
    C: GitIndexerProgressCallback,
{
    let mut out = core::ptr::null_mut();
    let mut callback = Box::new(callback);
    // SAFETY: the output slot is writable, `odb` remains live through the
    // returned wrapper, and the payload points into an address-stable box that
    // is retained on success and dropped only after the writepack on failure.
    let status = unsafe {
        ffi::git_odb_write_pack(
            core::ptr::addr_of_mut!(out),
            odb.as_ptr().cast_mut(),
            Some(writepack_progress::<C>),
            core::ptr::from_mut(callback.as_mut()).cast(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete writepack owner.
    let inner = unsafe { OdbWritepackOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(GitOdbWritepack {
        inner,
        callback,
        _database: PhantomData,
    })
}

/// Wraps: git_odb_add_backend
/// Transfers `backend` into the object database at the requested priority.
///
/// On failure, the backend is returned with the status code because libgit2
/// has not taken ownership of it.
///
/// The transfer consumes the owner, and this crate exposes no safe way to
/// borrow an installed backend back out of the database. A caller that must
/// keep operating on a concrete backend after installing it — the mempack
/// dump-and-reset cycle is the published example — recovers its typed handle
/// through the documented unsafe seam, such as
/// [`GitMempackBackendRef::from_odb_backend`](crate::odb_mempack::GitMempackBackendRef::from_odb_backend).
pub fn git_odb_add_backend(
    odb: &mut GitOdbMut<'_>,
    backend: GitOdbBackendOwned,
    priority: i32,
) -> Result<(), (i32, GitOdbBackendOwned)> {
    let raw = backend.into_raw();
    // SAFETY: `odb` is exclusively borrowed and `raw` transfers one complete
    // backend owner. Success stores it in the ODB; failure leaves it untouched.
    let status = unsafe { ffi::git_odb_add_backend(odb.as_mut_ptr(), raw, priority) };
    if status == 0 {
        Ok(())
    } else {
        // SAFETY: every failing path returns before backend insertion, so the
        // original uniquely owned complete backend is returned to Rust.
        let backend = unsafe { GitOdbBackendOwned::from_raw(raw) }
            .expect("the transferred backend pointer was non-null");
        Err((status, backend))
    }
}

/// Wraps: git_odb_object_data
/// Borrows the object's uncompressed raw bytes.
///
/// `None` is the valid representation of an empty object whose data pointer is
/// null. A nonempty live object always returns a counted view.
#[must_use]
pub fn git_odb_object_data<'a>(object: GitOdbObjectRef<'a>) -> Option<CSlice<'a, u8>> {
    // SAFETY: the live object is shared and both accessors only read fields.
    let (data, size) = unsafe {
        (
            ffi::git_odb_object_data(object.as_ptr().cast_mut()),
            ffi::git_odb_object_size(object.as_ptr().cast_mut()),
        )
    };
    let data = NonNull::new(data.cast_mut().cast::<u8>())?;
    // SAFETY: libgit2 guarantees the object owns `size` initialized bytes at
    // its non-null data pointer for the object's remaining lifetime.
    Some(unsafe { CSlice::from_raw_parts(data, size) })
}

/// Wraps: git_odb_object_free
/// Releases one independently owned object-cache reference.
pub fn git_odb_object_free(object: GitOdbObjectOwned) {
    drop(object);
}

/// Wraps: git_odb_object_id
/// Borrows the object identifier embedded in an object-database value.
#[must_use]
pub fn git_odb_object_id<'a>(object: GitOdbObjectRef<'a>) -> OidRef<'a> {
    // SAFETY: the shared live object is only read and always contains an
    // initialized inline cached-object identifier.
    let id = unsafe { ffi::git_odb_object_id(object.as_ptr().cast_mut()) };
    // SAFETY: C returns the non-null address of that inline identifier, whose
    // lifetime is exactly the object borrow carried into this function.
    unsafe { OidRef::from_ptr(id.cast_mut()) }.expect("an inline OID is non-null")
}

/// Wraps: git_odb_object_size
/// Returns the length of the object's raw data buffer.
#[must_use]
pub fn git_odb_object_size(object: GitOdbObjectRef<'_>) -> usize {
    // SAFETY: the shared live object is only read and no pointer is retained.
    unsafe { ffi::git_odb_object_size(object.as_ptr().cast_mut()) }
}

/// Wraps: git_odb_object_type
/// Returns the object's checked Git kind.
pub fn git_odb_object_type(
    object: GitOdbObjectRef<'_>,
) -> Result<GitObjectType, InvalidOdbObjectType> {
    // SAFETY: the shared live object is only read and no pointer is retained.
    let raw = unsafe { ffi::git_odb_object_type(object.as_ptr().cast_mut()) };
    GitObjectType::from_raw(raw).ok_or(InvalidOdbObjectType(raw))
}

/// Wraps: git_odb_read
/// Reads an object and returns one independently owned cache reference.
pub fn git_odb_read(odb: &mut GitOdbMut<'_>, id: OidRef<'_>) -> Result<GitOdbObjectOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the output slot is writable, the ODB is exclusively borrowed
    // because a cache miss may refresh it, and the ID is live for the call.
    let status = unsafe { ffi::git_odb_read(&mut out, odb.as_mut_ptr(), id.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one independently releasable object count.
    unsafe { GitOdbObjectOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod scheduled_object_api_tests {
    use super::*;

    #[test]
    fn add_backend_transfers_its_owner_to_the_odb() {
        // SAFETY: initialization is process-global and refcounted; shutdown is
        // balanced after the ODB has destroyed its installed backend.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut odb = git_odb_new().unwrap();
        let backend = crate::odb_mempack::GitMempackBackend::into_odb_backend(
            crate::odb_mempack::git_mempack_new().unwrap(),
        );
        git_odb_add_backend(&mut odb.as_mut(), backend, 999)
            .unwrap_or_else(|(status, _)| panic!("backend insertion failed: {status}"));
        drop(odb);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn a_read_object_is_an_independently_counted_cache_reference() {
        use crate::cache::{CacheStoreKind, GitCachedObjRef};

        // SAFETY: initialization is process-global and refcounted; shutdown is
        // balanced after every owner created here has been dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut odb = git_odb_new().unwrap();
        let backend = crate::odb_mempack::GitMempackBackend::into_odb_backend(
            crate::odb_mempack::git_mempack_new().unwrap(),
        );
        git_odb_add_backend(&mut odb.as_mut(), backend, 999)
            .unwrap_or_else(|(status, _)| panic!("backend insertion failed: {status}"));

        let body = b"crustify odb object";
        let mut id = git_odb_write(odb.as_ref(), body, GitObjectType::BLOB).unwrap();
        // SAFETY: the object ID lives on this stack frame for the whole test
        // and is only read through the borrowed handle.
        let id = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(id).cast()) }.unwrap();

        let object = git_odb_read(&mut odb.as_mut(), id).unwrap();
        assert_eq!(git_odb_object_size(object.as_ref()), body.len());
        assert_eq!(
            git_odb_object_type(object.as_ref()),
            Ok(GitObjectType::BLOB)
        );
        assert_eq!(
            git_odb_object_id(object.as_ref())
                .raw_bytes()
                .elems()
                .take(20)
                .collect::<Vec<_>>(),
            id.raw_bytes().elems().take(20).collect::<Vec<_>>()
        );
        let mut seen = vec![0u8; body.len()];
        assert!(
            git_odb_object_data(object.as_ref())
                .unwrap()
                .copy_to_slice(&mut seen)
        );
        assert_eq!(seen.as_slice(), body.as_slice());

        // `git_odb_object` is layout-compatible with the cache prefix it embeds
        // first, which is where its reference count lives.
        let count = |object: &GitOdbObjectOwned| {
            // SAFETY: `cached` is the first member of `git_odb_object`, so the
            // object's address is also the address of a live `git_cached_obj`,
            // and the handle only outlives this call.
            let cached =
                unsafe { GitCachedObjRef::from_ptr(object.as_ref().as_ptr().cast_mut().cast()) }
                    .expect("a live object has a non-null address");
            assert_eq!(cached.store_kind(), Ok(CacheStoreKind::Raw));
            cached.refcount()
        };

        // Cloning acquires another reference to the same object rather than
        // copying it, and dropping the clone leaves the original usable.
        let before = count(&object);
        let duplicate = object.clone();
        assert_eq!(duplicate.as_ref().as_ptr(), object.as_ref().as_ptr());
        assert_eq!(count(&object), before + 1);
        drop(duplicate);
        assert_eq!(count(&object), before);
        assert_eq!(git_odb_object_size(object.as_ref()), body.len());

        git_odb_object_free(object);
        drop(odb);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn object_borrows_are_tied_to_the_object_handle() {
        let _: for<'a> fn(GitOdbObjectRef<'a>) -> Option<CSlice<'a, u8>> = git_odb_object_data;
        let _: for<'a> fn(GitOdbObjectRef<'a>) -> OidRef<'a> = git_odb_object_id;
    }
}

/// Wraps: git_odb_expand_ids
/// Expands object-ID prefixes in place.
pub fn git_odb_expand_ids(
    odb: &mut GitOdbMut<'_>,
    mut ids: CSliceMut<'_, GitOdbExpandId>,
) -> Result<(), i32> {
    // SAFETY: `odb` is exclusively borrowed and `ids` describes exactly
    // `len` initialized, exclusively accessible query records.
    let status = unsafe { ffi::git_odb_expand_ids(odb.as_mut_ptr(), ids.as_mut_ptr(), ids.len()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_odb_new_ext
/// Creates an empty object database with optional configuration.
pub fn git_odb_new_ext(options: Option<GitOdbOptionsRef<'_>>) -> Result<GitOdbOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output is writable and `options` is null or live for this
    // call; success transfers one complete ODB owner.
    let status = unsafe {
        ffi::git_odb_new_ext(
            core::ptr::addr_of_mut!(raw),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one fully initialized owned ODB.
    unsafe { GitOdbOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_odb_num_backends
/// Returns the number of registered object-database backends.
#[must_use]
pub fn git_odb_num_backends(odb: &mut GitOdbMut<'_>) -> usize {
    // SAFETY: the exclusive handle permits the internal lock-backed query.
    unsafe { ffi::git_odb_num_backends(odb.as_mut_ptr()) }
}

/// Wraps: git_odb_open
/// Opens an object database rooted at `objects_dir`.
pub fn git_odb_open(objects_dir: &CStr) -> Result<GitOdbOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output is writable and the path is a live C string retained
    // only for this call.
    let status = unsafe { ffi::git_odb_open(core::ptr::addr_of_mut!(raw), objects_dir.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one complete owned ODB.
    unsafe { GitOdbOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_odb_open_ext
/// Opens an object database with optional configuration.
pub fn git_odb_open_ext(
    objects_dir: &CStr,
    options: Option<GitOdbOptionsRef<'_>>,
) -> Result<GitOdbOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output is writable, the path is live, and `options` is null
    // or a live shared value. No input pointer is retained.
    let status = unsafe {
        ffi::git_odb_open_ext(
            core::ptr::addr_of_mut!(raw),
            objects_dir.as_ptr(),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one complete owned ODB.
    unsafe { GitOdbOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_odb_options_init
/// Initializes object-database options for the requested ABI version.
pub fn git_odb_options_init(version: core::ffi::c_uint) -> Result<CVal<GitOdbOptions>, i32> {
    let mut options = GitOdbOptions::new();
    let status = {
        let mut output = options.as_mut();
        // SAFETY: `output` is writable layout-compatible storage and C does
        // not retain its address.
        unsafe { ffi::git_odb_options_init(output.as_mut_ptr(), version) }
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_odb_set_commit_graph
/// Transfers an optional commit graph into the object database.
///
/// On failure the supplied graph is reclaimed before the error is returned.
pub fn git_odb_set_commit_graph(
    odb: &mut GitOdbMut<'_>,
    graph: Option<GitCommitGraphOwned>,
) -> Result<(), i32> {
    let raw = graph.map_or(core::ptr::null_mut(), CBox::into_raw);
    // SAFETY: `odb` is exclusive and `raw` is null or one fully formed owner.
    // C takes that owner only when the call succeeds.
    let status = unsafe { ffi::git_odb_set_commit_graph(odb.as_mut_ptr(), raw) };
    if status == 0 {
        Ok(())
    } else {
        // SAFETY: on failure libgit2 did not store or consume `raw`; adopting
        // it back immediately ensures its allocation is released exactly once.
        drop(unsafe { GitCommitGraphOwned::from_raw(raw) });
        Err(status)
    }
}

/// Wraps: git_odb_write_multi_pack_index
/// Writes a multi-pack index for the database's non-alternate pack backends.
pub fn git_odb_write_multi_pack_index(odb: &mut GitOdbMut<'_>) -> Result<(), i32> {
    // SAFETY: the exclusive ODB handle permits backend traversal and writes.
    let status = unsafe { ffi::git_odb_write_multi_pack_index(odb.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_odb_add_alternate
/// Transfers an alternate backend into the object database.
///
/// On failure, libgit2 has not installed the backend, so its owner is returned.
pub fn git_odb_add_alternate(
    odb: &mut GitOdbMut<'_>,
    backend: GitOdbBackendOwned,
    priority: i32,
) -> Result<(), (i32, GitOdbBackendOwned)> {
    let raw = backend.into_raw();
    // SAFETY: the ODB is exclusively borrowed and `raw` transfers one complete
    // backend. Success stores it; failure leaves the allocation untouched.
    let status = unsafe { ffi::git_odb_add_alternate(odb.as_mut_ptr(), raw, priority) };
    if status == 0 {
        Ok(())
    } else {
        // SAFETY: every failure happens before insertion, so the transferred
        // complete backend remains uniquely owned by the caller.
        let backend = unsafe { GitOdbBackendOwned::from_raw(raw) }
            .expect("the transferred backend pointer was non-null");
        Err((status, backend))
    }
}

/// Wraps: git_odb_get_backend
/// Borrows the installed backend at `position` for the ODB reborrow.
pub fn git_odb_get_backend<'a>(
    odb: &'a mut GitOdbMut<'_>,
    position: usize,
) -> Result<GitOdbBackendRef<'a>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the output slot is writable and the ODB is exclusively
    // reborrowed while C locks and reads its backend vector.
    let status = unsafe { ffi::git_odb_get_backend(&mut out, odb.as_mut_ptr(), position) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns a non-owning backend kept alive by the ODB; the
    // returned handle cannot outlive this ODB reborrow.
    unsafe { GitOdbBackendRef::from_ptr(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_odb_read_prefix
/// Reads the unique object matching the first `hex_len` digits of `short_id`.
pub fn git_odb_read_prefix(
    odb: &mut GitOdbMut<'_>,
    short_id: OidRef<'_>,
    hex_len: usize,
) -> Result<GitOdbObjectOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the output is writable, the ODB is exclusively borrowed because
    // a miss may refresh it, and the object ID remains live for the call.
    let status =
        unsafe { ffi::git_odb_read_prefix(&mut out, odb.as_mut_ptr(), short_id.as_ptr(), hex_len) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one independently releasable cache reference.
    unsafe { GitOdbObjectOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, TempDir};

    #[derive(Debug, Eq, PartialEq)]
    struct OdbObservation {
        ids: Vec<Vec<u8>>,
        written_id: Vec<u8>,
        data: Vec<u8>,
        size: usize,
        kind: ffi::git_object_t,
    }

    unsafe fn raw_observation(repository: *mut ffi::git_repository) -> OdbObservation {
        unsafe extern "C" fn collect(
            id: *const ffi::git_oid,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            let ids = unsafe { &mut *payload.cast::<Vec<Vec<u8>>>() };
            ids.push(unsafe { (*id).id }.to_vec());
            0
        }

        let mut odb = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_repository_odb(&mut odb, repository) }, 0);
        let mut ids = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_odb_foreach(odb, Some(collect), core::ptr::from_mut(&mut ids).cast())
            },
            0
        );
        ids.sort();

        let content = b"an object written by the ODB equivalence test\n";
        let mut written = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_odb_write(
                    &mut written,
                    odb,
                    content.as_ptr().cast(),
                    content.len(),
                    ffi::git_object_t_GIT_OBJECT_BLOB,
                )
            },
            0
        );
        let mut object = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_odb_read(&mut object, odb, &written) }, 0);
        let size = unsafe { ffi::git_odb_object_size(object) };
        let kind = unsafe { ffi::git_odb_object_type(object) };
        let data =
            unsafe { core::slice::from_raw_parts(ffi::git_odb_object_data(object).cast(), size) }
                .to_vec();
        unsafe {
            ffi::git_odb_object_free(object);
            ffi::git_odb_free(odb);
        }
        OdbObservation {
            ids,
            written_id: written.id.to_vec(),
            data,
            size,
            kind,
        }
    }

    fn safe_observation(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
    ) -> OdbObservation {
        let mut odb = crate::repository::git_repository_odb(repository).unwrap();
        let mut ids = Vec::new();
        git_odb_foreach(&mut odb.as_mut(), &mut |id: OidRef<'_>| {
            ids.push(id.raw_bytes().elems().collect());
            0
        })
        .unwrap();
        ids.sort();

        let content = b"an object written by the ODB equivalence test\n";
        let mut written = git_odb_write(odb.as_ref(), content, GitObjectType::BLOB).unwrap();
        let written_ref =
            unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(written).cast()) }.unwrap();
        let object = git_odb_read(&mut odb.as_mut(), written_ref).unwrap();
        let size = git_odb_object_size(object.as_ref());
        let kind = git_odb_object_type(object.as_ref()).unwrap().as_raw();
        let data = git_odb_object_data(object.as_ref())
            .unwrap()
            .elems()
            .collect();
        OdbObservation {
            ids,
            written_id: written_ref.raw_bytes().elems().collect(),
            data,
            size,
            kind,
        }
    }

    #[test]
    fn io_equiv_odb_enumeration_write_and_read() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("odb-raw");
        let safe = HistoryFixture::new("odb-safe");
        let raw_observation = unsafe { raw_observation(raw.repository.as_ptr()) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(raw_observation, safe_observation(&mut safe_repository));
        assert_eq!(
            raw_observation.data,
            b"an object written by the ODB equivalence test\n"
        );
    }

    #[test]
    fn io_equiv_multi_pack_index_generation() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("midx-raw");
        let safe = HistoryFixture::new("midx-safe");
        for fixture in [&raw, &safe] {
            let status = std::process::Command::new("git")
                .args([
                    "-C",
                    fixture.directory.path().to_str().unwrap(),
                    "repack",
                    "-ad",
                ])
                .status()
                .unwrap();
            assert!(status.success());
            std::fs::write(
                fixture.directory.path().join("extra-object"),
                b"extra pack object\n",
            )
            .unwrap();
            let object_id = std::process::Command::new("git")
                .current_dir(fixture.directory.path())
                .args(["hash-object", "-w", "extra-object"])
                .output()
                .unwrap();
            assert!(object_id.status.success());
            let mut pack = std::process::Command::new("git")
                .current_dir(fixture.directory.path())
                .args(["pack-objects", ".git/objects/pack/pack-extra"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap();
            std::io::Write::write_all(pack.stdin.as_mut().unwrap(), &object_id.stdout).unwrap();
            drop(pack.stdin.take());
            assert!(pack.wait().unwrap().success());
        }

        let mut raw_repository = core::ptr::null_mut();
        let raw_path = raw.directory.c_path();
        assert_eq!(
            unsafe { ffi::git_repository_open(&mut raw_repository, raw_path.as_ptr()) },
            0
        );
        let mut raw_odb = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_odb(&mut raw_odb, raw_repository) },
            0
        );
        assert_eq!(unsafe { ffi::git_odb_write_multi_pack_index(raw_odb) }, 0);
        unsafe { ffi::git_odb_free(raw_odb) };
        unsafe { ffi::git_repository_free(raw_repository) };

        let safe_path = safe.directory.c_path();
        let mut safe_repository = crate::repository::git_repository_open(&safe_path).unwrap();
        let mut safe_odb =
            crate::repository::git_repository_odb(&mut safe_repository.as_mut()).unwrap();
        git_odb_write_multi_pack_index(&mut safe_odb.as_mut()).unwrap();
        drop(safe_odb);

        let relative = ".git/objects/pack/multi-pack-index";
        let raw_index = std::fs::read(raw.directory.path().join(relative)).unwrap();
        let safe_index = std::fs::read(safe.directory.path().join(relative)).unwrap();
        assert_eq!(raw_index, safe_index);
        assert!(raw_index.starts_with(b"MIDX"));
    }

    #[derive(Debug, Eq, PartialEq)]
    struct StreamObservation {
        header: (usize, ffi::git_object_t),
        streamed: Vec<u8>,
        exists: Vec<bool>,
        prefix: Vec<u8>,
        written: Vec<u8>,
        written_data: Vec<u8>,
        hash: Vec<u8>,
    }

    unsafe fn raw_stream_observation(repository: *mut ffi::git_repository) -> StreamObservation {
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut odb = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_repository_odb(&mut odb, repository) }, 0);
        let mut size = 0;
        let mut kind = ffi::git_object_t_GIT_OBJECT_INVALID;
        assert_eq!(
            unsafe { ffi::git_odb_read_header(&mut size, &mut kind, odb, &head) },
            0
        );
        let mut stream = core::ptr::null_mut();
        let mut stream_size = 0;
        let mut stream_kind = ffi::git_object_t_GIT_OBJECT_INVALID;
        assert_eq!(
            unsafe {
                ffi::git_odb_open_rstream(
                    &mut stream,
                    &mut stream_size,
                    &mut stream_kind,
                    odb,
                    &head,
                )
            },
            0
        );
        assert_eq!((stream_size, stream_kind), (size, kind));
        let mut streamed = Vec::new();
        loop {
            let mut chunk = [0u8; 17];
            let read =
                unsafe { ffi::git_odb_stream_read(stream, chunk.as_mut_ptr().cast(), chunk.len()) };
            assert!(read >= 0);
            if read == 0 {
                break;
            }
            streamed.extend_from_slice(&chunk[..read as usize]);
        }
        unsafe { ffi::git_odb_stream_free(stream) };

        let exists = vec![unsafe { ffi::git_odb_exists(odb, &head) != 0 }, unsafe {
            ffi::git_odb_exists_ext(
                odb,
                &head,
                ffi::git_odb_lookup_flags_t_GIT_ODB_LOOKUP_NO_REFRESH,
            ) != 0
        }];
        let mut prefix = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_odb_exists_prefix(&mut prefix, odb, &head, 8) },
            0
        );
        let content = b"streamed ODB object split across several writes\n";
        let mut writer = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_odb_open_wstream(
                    &mut writer,
                    odb,
                    content.len() as ffi::git_object_size_t,
                    ffi::git_object_t_GIT_OBJECT_BLOB,
                )
            },
            0
        );
        for chunk in content.chunks(7) {
            assert_eq!(
                unsafe { ffi::git_odb_stream_write(writer, chunk.as_ptr().cast(), chunk.len()) },
                0
            );
        }
        let mut written = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_odb_stream_finalize_write(&mut written, writer) },
            0
        );
        unsafe { ffi::git_odb_stream_free(writer) };
        assert_eq!(unsafe { ffi::git_odb_refresh(odb) }, 0);
        let mut object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_odb_read_prefix(&mut object, odb, &written, 10) },
            0
        );
        let object_size = unsafe { ffi::git_odb_object_size(object) };
        let written_data = unsafe {
            core::slice::from_raw_parts(ffi::git_odb_object_data(object).cast(), object_size)
        }
        .to_vec();
        unsafe { ffi::git_odb_object_free(object) };
        let mut hash = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_odb_hash(
                    &mut hash,
                    content.as_ptr().cast(),
                    content.len(),
                    ffi::git_object_t_GIT_OBJECT_BLOB,
                )
            },
            0
        );
        unsafe { ffi::git_odb_free(odb) };
        StreamObservation {
            header: (size, kind),
            streamed,
            exists,
            prefix: prefix.id.to_vec(),
            written: written.id.to_vec(),
            written_data,
            hash: hash.id.to_vec(),
        }
    }

    fn oid_bytes(mut id: Oid) -> Vec<u8> {
        let id = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(id).cast()) }.unwrap();
        id.raw_bytes().elems().collect()
    }

    fn safe_stream_observation(repository: *mut ffi::git_repository) -> StreamObservation {
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut odb = crate::repository::git_repository_odb(&mut repository).unwrap();
        let head_ref = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(head).cast()) }.unwrap();
        let (size, kind) = git_odb_read_header(&mut odb.as_mut(), head_ref).unwrap();
        let mut odb_borrow = odb.as_mut();
        let (mut reader, stream_size, stream_kind) =
            git_odb_open_rstream(&mut odb_borrow, head_ref).unwrap();
        assert_eq!((stream_size, stream_kind), (size, kind));
        let mut streamed = Vec::new();
        loop {
            let mut chunk = [0u8; 17];
            let read = git_odb_stream_read(&mut reader, &mut chunk).unwrap();
            if read == 0 {
                break;
            }
            streamed.extend_from_slice(&chunk[..read]);
        }
        drop(reader);
        drop(odb_borrow);
        let exists = vec![
            git_odb_exists(&mut odb.as_mut(), head_ref),
            git_odb_exists_ext(&mut odb.as_mut(), head_ref, GitOdbLookupFlags::NO_REFRESH),
        ];
        let prefix = oid_bytes(git_odb_exists_prefix(&mut odb.as_mut(), head_ref, 8).unwrap());
        let content = b"streamed ODB object split across several writes\n";
        let mut odb_borrow = odb.as_mut();
        let mut writer = git_odb_open_wstream(
            &mut odb_borrow,
            content.len() as ffi::git_object_size_t,
            GitObjectType::BLOB,
        )
        .unwrap();
        for chunk in content.chunks(7) {
            git_odb_stream_write(&mut writer, chunk).unwrap();
        }
        let written = git_odb_stream_finalize_write(&mut writer).unwrap();
        drop(writer);
        drop(odb_borrow);
        git_odb_refresh(&mut odb.as_mut()).unwrap();
        let mut written_for_ref = written;
        let written_ref =
            unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(written_for_ref).cast()) }.unwrap();
        let object = git_odb_read_prefix(&mut odb.as_mut(), written_ref, 10).unwrap();
        let written_data = git_odb_object_data(object.as_ref())
            .unwrap()
            .elems()
            .collect();
        StreamObservation {
            header: (size, kind.as_raw()),
            streamed,
            exists,
            prefix,
            written: written_ref.raw_bytes().elems().collect(),
            written_data,
            hash: oid_bytes(git_odb_hash(content, GitObjectType::BLOB).unwrap()),
        }
    }

    #[test]
    fn io_equiv_odb_streaming_prefix_hash_and_refresh() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("odb-stream-raw");
        let safe = HistoryFixture::new("odb-stream-safe");
        let raw_observation = unsafe { raw_stream_observation(raw.repository.as_ptr()) };
        assert_eq!(
            raw_observation,
            safe_stream_observation(safe.repository.as_ptr())
        );
        assert_eq!(
            raw_observation.written_data,
            b"streamed ODB object split across several writes\n"
        );
        assert_eq!(raw_observation.written, raw_observation.hash);
    }

    unsafe fn raw_open_alternate_and_hashfile(
        fixture: &HistoryFixture,
    ) -> (Vec<u8>, usize, u32, bool, usize, Vec<u8>) {
        let objects = std::ffi::CString::new(
            fixture
                .directory
                .path()
                .join(".git/objects")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let readme =
            std::ffi::CString::new(fixture.directory.path().join("README.md").to_str().unwrap())
                .unwrap();
        let mut hash = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_odb_hashfile(
                    &mut hash,
                    readme.as_ptr(),
                    ffi::git_object_t_GIT_OBJECT_BLOB,
                )
            },
            0
        );
        let mut opened = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_odb_open(&mut opened, objects.as_ptr()) },
            0
        );
        let opened_backends = unsafe { ffi::git_odb_num_backends(opened) };
        let mut backend = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_odb_get_backend(&mut backend, opened, 0) },
            0
        );
        let backend_version = unsafe { (*backend).version };

        let mut alternate = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_odb_new(&mut alternate) }, 0);
        assert_eq!(
            unsafe { ffi::git_odb_add_disk_alternate(alternate, objects.as_ptr()) },
            0
        );
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut head,
                    fixture.repository.as_ptr(),
                    c"HEAD".as_ptr(),
                )
            },
            0
        );
        let found = unsafe { ffi::git_odb_exists(alternate, &head) } != 0;
        let mut prefixed = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_odb_read_prefix(&mut prefixed, alternate, &head, 10) },
            0
        );
        let prefix_id = unsafe { (*ffi::git_odb_object_id(prefixed)).id }.to_vec();
        unsafe { ffi::git_odb_object_free(prefixed) };

        let mut options = unsafe { core::mem::zeroed::<ffi::git_odb_options>() };
        assert_eq!(
            unsafe { ffi::git_odb_options_init(&mut options, ffi::GIT_ODB_OPTIONS_VERSION) },
            0
        );
        let mut extended = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_odb_open_ext(&mut extended, objects.as_ptr(), &options) },
            0
        );
        let extended_backends = unsafe { ffi::git_odb_num_backends(extended) };
        assert_eq!(
            unsafe { ffi::git_odb_set_commit_graph(extended, core::ptr::null_mut()) },
            0
        );
        unsafe {
            ffi::git_odb_free(extended);
            ffi::git_odb_free(alternate);
            ffi::git_odb_free(opened);
        }
        (
            hash.id.to_vec(),
            opened_backends,
            backend_version,
            found,
            extended_backends,
            prefix_id,
        )
    }

    fn safe_open_alternate_and_hashfile(
        fixture: &HistoryFixture,
    ) -> (Vec<u8>, usize, u32, bool, usize, Vec<u8>) {
        let objects = std::ffi::CString::new(
            fixture
                .directory
                .path()
                .join(".git/objects")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let readme =
            std::ffi::CString::new(fixture.directory.path().join("README.md").to_str().unwrap())
                .unwrap();
        let hash = git_odb_hashfile(&readme, GitObjectType::BLOB).unwrap();
        let mut opened = git_odb_open(&objects).unwrap();
        let opened_backends = git_odb_num_backends(&mut opened.as_mut());
        let backend_version = git_odb_get_backend(&mut opened.as_mut(), 0)
            .unwrap()
            .version();

        let mut alternate = git_odb_new().unwrap();
        git_odb_add_disk_alternate(&mut alternate.as_mut(), &objects).unwrap();
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        let mut head = crate::refs::git_reference_name_to_id(&mut repository, c"HEAD").unwrap();
        let head = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(head).cast()) }.unwrap();
        let found = git_odb_exists(&mut alternate.as_mut(), head);
        let prefixed = git_odb_read_prefix(&mut alternate.as_mut(), head, 10).unwrap();
        let prefix_id = git_odb_object_id(prefixed.as_ref())
            .raw_bytes()
            .elems()
            .collect();

        let options = git_odb_options_init(ffi::GIT_ODB_OPTIONS_VERSION).unwrap();
        let mut extended = git_odb_open_ext(&objects, Some(options.as_ref())).unwrap();
        let extended_backends = git_odb_num_backends(&mut extended.as_mut());
        git_odb_set_commit_graph(&mut extended.as_mut(), None).unwrap();
        let mut hash = hash;
        let hash = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(hash).cast()) }.unwrap();
        (
            hash.raw_bytes().elems().collect(),
            opened_backends,
            backend_version,
            found,
            extended_backends,
            prefix_id,
        )
    }

    #[test]
    fn io_equiv_odb_open_disk_alternate_backend_hashfile_and_commit_graph_clear() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("odb-open-alternate-raw");
        let safe = HistoryFixture::new("odb-open-alternate-safe");
        let raw_observation = unsafe { raw_open_alternate_and_hashfile(&raw) };
        let safe_observation = safe_open_alternate_and_hashfile(&safe);
        assert_eq!(raw_observation, safe_observation);
        assert!(raw_observation.3);
        assert_eq!(raw_observation.5.len(), crate::oid::RAW_DIGEST_LEN);
    }

    fn delta_pack(fixture: &HistoryFixture) -> (Vec<u8>, ffi::git_oid) {
        for revision in 0..16 {
            let mut content = String::with_capacity(120_000);
            for line in 0..2_000 {
                use core::fmt::Write as _;
                writeln!(
                    content,
                    "writepack record {line:04}: stable payload with revision {revision:02}"
                )
                .unwrap();
            }
            std::fs::write(
                fixture.directory.path().join("writepack-history.txt"),
                content,
            )
            .unwrap();
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(fixture.directory.path())
                .args(["add", "writepack-history.txt"])
                .status()
                .unwrap();
            assert!(status.success());
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
                    &format!("writepack revision {revision}"),
                ])
                .env("GIT_AUTHOR_DATE", format!("170001{revision:04} +0000"))
                .env("GIT_COMMITTER_DATE", format!("170001{revision:04} +0000"))
                .status()
                .unwrap();
            assert!(status.success());
        }
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args(["pack-objects", "--stdout", "--all", "--delta-base-offset"])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stdout.starts_with(b"PACK"));
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut head,
                    fixture.repository.as_ptr(),
                    c"HEAD".as_ptr(),
                )
            },
            0
        );
        (output.stdout, head)
    }

    fn empty_objects(tag: &str) -> TempDir {
        let directory = TempDir::new(tag);
        std::fs::create_dir_all(directory.path().join("pack")).unwrap();
        std::fs::create_dir_all(directory.path().join("info")).unwrap();
        directory
    }

    #[derive(Debug, Eq, PartialEq)]
    struct WritepackObservation {
        total_objects: u32,
        indexed_objects: u32,
        total_deltas: u32,
        indexed_deltas: u32,
        callback_count: usize,
        head_exists: bool,
        pack_files: usize,
    }

    unsafe fn raw_writepack(
        objects: &TempDir,
        pack: &[u8],
        head: &ffi::git_oid,
    ) -> WritepackObservation {
        unsafe extern "C" fn progress(
            _: *const ffi::git_indexer_progress,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            unsafe { *payload.cast::<usize>() += 1 };
            0
        }

        let path = objects.c_path();
        let mut odb = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_odb_open(&mut odb, path.as_ptr()) }, 0);
        let mut callback_count = 0usize;
        let mut writepack = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_odb_write_pack(
                    &mut writepack,
                    odb,
                    Some(progress),
                    core::ptr::from_mut(&mut callback_count).cast(),
                )
            },
            0
        );
        let mut stats = unsafe { core::mem::zeroed::<ffi::git_indexer_progress>() };
        for chunk in pack.chunks(137) {
            let append = unsafe { (*writepack).append.unwrap() };
            assert_eq!(
                unsafe { append(writepack, chunk.as_ptr().cast(), chunk.len(), &mut stats,) },
                0
            );
        }
        let commit = unsafe { (*writepack).commit.unwrap() };
        assert_eq!(unsafe { commit(writepack, &mut stats) }, 0);
        let free = unsafe { (*writepack).free.unwrap() };
        unsafe { free(writepack) };
        let head_exists = unsafe { ffi::git_odb_exists(odb, head) } != 0;
        unsafe { ffi::git_odb_free(odb) };
        WritepackObservation {
            total_objects: stats.total_objects,
            indexed_objects: stats.indexed_objects,
            total_deltas: stats.total_deltas,
            indexed_deltas: stats.indexed_deltas,
            callback_count,
            head_exists,
            pack_files: std::fs::read_dir(objects.path().join("pack"))
                .unwrap()
                .count(),
        }
    }

    fn safe_writepack(
        objects: &TempDir,
        pack: &[u8],
        head: &mut ffi::git_oid,
    ) -> WritepackObservation {
        use std::cell::Cell;
        use std::rc::Rc;

        let path = objects.c_path();
        let mut odb = git_odb_open(&path).unwrap();
        let callback_count = Rc::new(Cell::new(0usize));
        let callback_view = Rc::clone(&callback_count);
        let mut writepack = git_odb_write_pack(odb.as_ref(), move |_: IndexerProgressRef<'_>| {
            callback_view.set(callback_view.get() + 1);
            0
        })
        .unwrap();
        let mut stats = unsafe { core::mem::zeroed::<ffi::git_indexer_progress>() };
        let mut stats_view =
            unsafe { crate::indexer::IndexerProgressMut::from_ptr(&raw mut stats) }.unwrap();
        for chunk in pack.chunks(137) {
            writepack.as_mut().append(chunk, &mut stats_view).unwrap();
        }
        writepack.as_mut().commit(&mut stats_view).unwrap();
        drop(writepack);
        let head = unsafe { OidRef::from_ptr(core::ptr::from_mut(head)) }.unwrap();
        let head_exists = git_odb_exists(&mut odb.as_mut(), head);
        WritepackObservation {
            total_objects: stats_view.as_ref().total_objects(),
            indexed_objects: stats_view.as_ref().indexed_objects(),
            total_deltas: stats_view.as_ref().total_deltas(),
            indexed_deltas: stats_view.as_ref().indexed_deltas(),
            callback_count: callback_count.get(),
            head_exists,
            pack_files: std::fs::read_dir(objects.path().join("pack"))
                .unwrap()
                .count(),
        }
    }

    #[test]
    fn io_equiv_odb_writepack_ingests_delta_pack_and_reports_progress() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("odb-writepack-source");
        let (pack, mut head) = delta_pack(&source);
        let raw_objects = empty_objects("odb-writepack-raw");
        let safe_objects = empty_objects("odb-writepack-safe");
        let raw = unsafe { raw_writepack(&raw_objects, &pack, &head) };
        assert_eq!(raw, safe_writepack(&safe_objects, &pack, &mut head));
        assert!(raw.head_exists);
        assert!(raw.total_deltas > 0);
        assert_eq!(raw.total_objects, raw.indexed_objects);
        assert_eq!(raw.total_deltas, raw.indexed_deltas);
        assert_eq!(raw.pack_files, 2);
    }
}

#[cfg(test)]
mod scheduled_alternate_and_prefix_tests {
    use super::*;
    use crate::oid::OidType;

    /// Holds one libgit2 initialization count for the duration of a test.
    ///
    /// Every allocation libgit2 performs goes through the allocator that
    /// initialization installs, so an unbracketed call corrupts memory rather
    /// than failing.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and reference
            // counted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the initialization this guard represents,
            // after every libgit2 owner in the test has been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// Creates one empty loose-object directory and its C path.
    fn objects_directory(tag: &str) -> (std::path::PathBuf, std::ffi::CString) {
        let directory = std::env::temp_dir().join(format!(
            "crustify-odb-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a fresh loose-object directory");
        let path = std::ffi::CString::new(
            directory
                .to_str()
                .expect("a temporary directory path is UTF-8"),
        )
        .expect("a temporary directory path holds no interior NUL");
        (directory, path)
    }

    /// Drives the three scheduled database operations against real backends.
    ///
    /// `git_odb_add_alternate` transfers the backend: `add_backend_internal`
    /// records it in the database's vector and `git_odb_free` later calls its
    /// `free` callback, so the owner has to be consumed. Every failing path
    /// in that helper returns before the vector insert, which is what lets the
    /// wrapper hand the still-owned backend back in the error case.
    ///
    /// `git_odb_get_backend` hands back a pointer the database keeps owning,
    /// so its handle is borrowed rather than owned, and bounded by the
    /// exclusive database reborrow that the C lock acquisition requires.
    #[test]
    fn an_alternate_backend_is_owned_by_the_database_and_answers_prefix_reads() {
        let _libgit2 = Libgit2Init::acquire();
        let (primary_directory, primary) = objects_directory("primary");
        let (alternate_directory, alternate) = objects_directory("alternate");

        // A SHA-256 database keeps every hash away from the bundled SHA1DC
        // implementation, whose unaligned 32-bit loads trip the C build's
        // UBSan on each object it hashes.
        let mut options = git_odb_options_init(ffi::GIT_ODB_OPTIONS_VERSION)
            .expect("the published database options version initializes");
        options.as_mut().set_oid_type(Some(OidType::Sha256));
        let mut odb = git_odb_new_ext(Some(options.as_ref())).expect("an empty database");

        let mut loose = crate::odb_loose::git_odb_backend_loose_options_init(
            ffi::GIT_ODB_BACKEND_LOOSE_OPTIONS_VERSION,
        )
        .expect("the published loose options version initializes");
        loose.as_mut().set_oid_type(Some(OidType::Sha256));

        let writable = crate::odb_loose::git_odb_backend_loose(&primary, Some(loose.as_ref()))
            .expect("a loose backend over the primary directory");
        git_odb_add_backend(&mut odb.as_mut(), writable, 1)
            .map_err(|(code, _backend)| code)
            .expect("the writable backend installs");

        let extra = crate::odb_loose::git_odb_backend_loose(&alternate, Some(loose.as_ref()))
            .expect("a loose backend over the alternate directory");
        git_odb_add_alternate(&mut odb.as_mut(), extra, 2)
            .map_err(|(code, _backend)| code)
            .expect("the alternate backend installs");

        assert_eq!(git_odb_num_backends(&mut odb.as_mut()), 2);
        assert_eq!(
            git_odb_get_backend(&mut odb.as_mut(), 0)
                .expect("the first slot is installed")
                .version(),
            1
        );
        assert_eq!(
            git_odb_get_backend(&mut odb.as_mut(), 1)
                .expect("the second slot is installed")
                .version(),
            1
        );
        assert_eq!(
            git_odb_get_backend(&mut odb.as_mut(), 2).err(),
            Some(ffi::git_error_code_GIT_ENOTFOUND),
            "an unused slot is a clean not-found rather than a null handle"
        );

        // Only the non-alternate backend accepts writes, so this lands in the
        // primary directory while the alternate stays readable and empty.
        let mut id = git_odb_write(odb.as_ref(), b"crustify", GitObjectType::BLOB)
            .expect("the writable loose backend stores the blob");
        // SAFETY: `id` is live, initialized, exclusively borrowed local
        // storage for the whole life of this handle.
        let id = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(id).cast()) }
            .expect("the address of a local value is non-null");

        // A short prefix takes the backend search; the full hex length takes
        // the cache fast path in `git_odb_read_prefix` instead.
        for hex_len in [8, 64] {
            let object = git_odb_read_prefix(&mut odb.as_mut(), id, hex_len)
                .expect("the stored blob resolves from its prefix");
            assert_eq!(git_odb_object_size(object.as_ref()), b"crustify".len());
            assert!(crate::oid::git_oid_equal(
                git_odb_object_id(object.as_ref()),
                id
            ));
        }

        assert_eq!(
            git_odb_read_prefix(&mut odb.as_mut(), id, 3).err(),
            Some(ffi::git_error_code_GIT_EAMBIGUOUS),
            "libgit2 refuses a prefix shorter than GIT_OID_MINPREFIXLEN"
        );

        // Dropping the database runs both installed `free` callbacks; the
        // sanitized C library reports a double free if either owner survived.
        drop(odb);
        let _ = std::fs::remove_dir_all(&primary_directory);
        let _ = std::fs::remove_dir_all(&alternate_directory);
    }
}
