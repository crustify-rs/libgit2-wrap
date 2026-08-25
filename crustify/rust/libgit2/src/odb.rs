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
use crate::sys::odb_backend::GitOdbBackendOwned;

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
mod tests {
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
