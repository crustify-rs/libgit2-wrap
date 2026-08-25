//! Safe wrappers for libgit2 odb_backend APIs.

use core::ffi::{c_int, c_void};
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, null_mut};

use ffibox::{CBox, CVec};

use crate::api::odb_backend::{
    GitOdbStream, GitOdbStreamMut, GitOdbStreamOwned, GitOdbStreamRef, OdbWritepack,
    OdbWritepackMut, OdbWritepackOwned, OdbWritepackRef,
};
use crate::api::types::GitObjectType;
use crate::ffi;
use crate::indexer::{GitIndexerProgressCallback, IndexerProgressRef};
use crate::odb::{GitOdbForeachCallback, GitOdbMut, GitOdbRef};
use crate::oid::{Oid, OidRef};
use crate::util::alloc::GitMallocFree;

macro_rules! backend_callback {
    ($this:expr, $backend:expr, $field:ident, $operation:expr) => {{
        // SAFETY: `$backend` comes from the live exclusive handle at every
        // invocation; raw-place projection forms no reference to its storage.
        let field = unsafe { addr_of!((*$backend).$field) };
        $this.callback(field, $operation)
    }};
}

ffibox::define_ctype!(
    /// Wraps: git_odb_backend
    /// A layout-compatible polymorphic object-database backend.
    ///
    /// The callback table is installed by the concrete backend. Safe methods
    /// dispatch those callbacks without exposing their raw pointer contracts.
    /// An owned handle represents a backend not yet transferred to an object
    /// database; dropping it invokes its required concrete destructor.
    GitOdbBackend,
    GitOdbBackendRef,
    GitOdbBackendMut,
    ffi::git_odb_backend
);

/// An exclusively owned backend allocation.
pub type GitOdbBackendOwned = CBox<GitOdbBackend>;

/// One backend operation that a concrete implementation may omit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitOdbBackendOperation {
    /// Read a complete object.
    Read,
    /// Read a complete object found by an ID prefix.
    ReadPrefix,
    /// Read only an object's header.
    ReadHeader,
    /// Write a complete object.
    Write,
    /// Open a streaming object writer.
    WriteStream,
    /// Open a streaming object reader.
    ReadStream,
    /// Test for an object.
    Exists,
    /// Resolve an ID prefix.
    ExistsPrefix,
    /// Refresh backend state.
    Refresh,
    /// Visit every object ID.
    Foreach,
    /// Open a pack writer.
    WritePack,
    /// Write a multi-pack index.
    WriteMultiPackIndex,
    /// Freshen an existing object.
    Freshen,
}

/// Failure while dispatching an object-database backend callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitOdbBackendError {
    /// The concrete backend did not install the requested callback.
    Unsupported(GitOdbBackendOperation),
    /// The callback returned a libgit2 status code.
    Libgit2(c_int),
    /// A successful read returned a nonempty run without storage.
    MissingData,
    /// A successful stream constructor returned no stream.
    MissingStream,
    /// A successful pack-writer constructor returned no writepack.
    MissingWritepack,
    /// A successful callback returned a non-published object kind.
    InvalidObjectType(ffi::git_object_t),
}

/// A uniquely owned byte run returned by a backend read callback.
pub struct GitOdbBackendData {
    bytes: Option<CVec<u8, GitMallocFree>>,
}

/// An owned stream tethered to the backend whose callbacks it retains.
pub struct GitOdbBackendStream<'backend> {
    inner: GitOdbStreamOwned,
    _backend: PhantomData<GitOdbBackendRef<'backend>>,
}

impl GitOdbBackendStream<'_> {
    /// Borrows the stream shared.
    #[must_use]
    pub fn as_ref(&self) -> GitOdbStreamRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the stream exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitOdbStreamMut<'_> {
        self.inner.as_mut()
    }
}

/// An owned writepack tethered to its backend, database and progress state.
///
/// The writepack drops first, freeing the indexer that retains these pointers;
/// only then is the optional callback state released.
pub struct GitOdbBackendWritepack<'owners, C = ()> {
    inner: OdbWritepackOwned,
    _progress: Option<Box<C>>,
    _owners: PhantomData<(GitOdbBackendRef<'owners>, GitOdbRef<'owners>)>,
}

impl<C> GitOdbBackendWritepack<'_, C> {
    /// Borrows the writepack shared.
    #[must_use]
    pub fn as_ref(&self) -> OdbWritepackRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the writepack exclusively for appending or committing.
    #[must_use]
    pub fn as_mut(&mut self) -> OdbWritepackMut<'_> {
        self.inner.as_mut()
    }
}

impl GitOdbBackendData {
    /// Borrows the bytes returned by the backend.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_ref().map_or(&[], CVec::as_slice)
    }

    /// Returns the byte count supplied by the backend.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.as_ref().map_or(0, CVec::count)
    }

    /// Returns whether the backend returned an empty object body.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn from_callback_bytes(
        bytes: Option<CVec<u8, GitMallocFree>>,
        len: usize,
    ) -> Result<Self, GitOdbBackendError> {
        if bytes.is_none() {
            return if len == 0 {
                Ok(Self { bytes: None })
            } else {
                Err(GitOdbBackendError::MissingData)
            };
        }
        Ok(Self { bytes })
    }
}

impl<'a> GitOdbBackendRef<'a> {
    /// Field: git_odb_backend.version
    /// Returns the public backend ABI version stored in this callback table.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_odb_backend.odb
    /// Borrows the object database that owns this backend, if it has been
    /// installed. A newly constructed, unattached backend stores null.
    #[must_use]
    pub fn odb(&self) -> Option<GitOdbRef<'a>> {
        // SAFETY: raw-place projection copies the initialized back-reference
        // without forming a reference over the backend or database.
        let odb = unsafe { addr_of!((*self.as_ptr()).odb).read() };
        // SAFETY: when non-null, the owning database destroys this backend
        // before freeing itself, so it remains live for the backend borrow.
        unsafe { GitOdbRef::from_ptr(odb) }
    }
}

impl GitOdbBackendMut<'_> {
    fn callback<T: Copy>(
        &self,
        field: *const Option<T>,
        operation: GitOdbBackendOperation,
    ) -> Result<T, GitOdbBackendError> {
        // SAFETY: every call site projects `field` from this live backend and
        // names the exact initialized callback slot. Copying it forms no C
        // object reference.
        unsafe { field.read() }.ok_or(GitOdbBackendError::Unsupported(operation))
    }

    /// Field: git_odb_backend.read
    /// Reads and takes ownership of a complete object body.
    ///
    /// The callback transfers its `git_odb_backend_data_alloc` buffer only
    /// when it succeeds; a buffer published alongside a failure stays with the
    /// backend, exactly as `odb_read_1` treats it.
    pub fn read(
        &mut self,
        oid: OidRef<'_>,
    ) -> Result<(GitOdbBackendData, GitObjectType), GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let read = backend_callback!(self, backend, read, GitOdbBackendOperation::Read)?;
        let mut data = null_mut();
        let mut len = 0;
        let mut kind = ffi::git_object_t_GIT_OBJECT_INVALID;
        // SAFETY: output slots are initialized and writable, `oid` is live,
        // and this exclusive handle serializes the synchronous callback. A
        // successful callback transfers its data allocation to the caller.
        let status = unsafe {
            read(
                &raw mut data,
                &raw mut len,
                &raw mut kind,
                backend,
                oid.as_ptr(),
            )
        };
        if status != 0 {
            // A failing callback transfers nothing: `odb_read_1` returns the
            // status without touching a buffer the callback may have published,
            // so releasing one here could free storage the backend still owns.
            return Err(GitOdbBackendError::Libgit2(status));
        }
        // SAFETY: the successful backend callback transfers a unique run of
        // `len` bytes allocated with `git_odb_backend_data_alloc`, which uses
        // libgit2's configured allocator and therefore matches this strategy.
        let data = unsafe { CVec::from_raw_parts(data.cast::<u8>(), len) };
        let data = GitOdbBackendData::from_callback_bytes(data, len)?;
        let kind =
            GitObjectType::from_raw(kind).ok_or(GitOdbBackendError::InvalidObjectType(kind))?;
        Ok((data, kind))
    }

    /// Field: git_odb_backend.read_prefix
    /// Resolves an ID prefix and reads the matching object body.
    ///
    /// Ownership of the object body transfers on success only, as in
    /// [`read`](Self::read).
    pub fn read_prefix(
        &mut self,
        short_id: OidRef<'_>,
        prefix_len: usize,
    ) -> Result<(Oid, GitOdbBackendData, GitObjectType), GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let read = backend_callback!(
            self,
            backend,
            read_prefix,
            GitOdbBackendOperation::ReadPrefix
        )?;
        let mut oid = Oid::zeroed();
        let mut data = null_mut();
        let mut len = 0;
        let mut kind = ffi::git_object_t_GIT_OBJECT_INVALID;
        // SAFETY: all outputs are initialized writable storage, `short_id` is
        // live, and this handle exclusively drives the synchronous callback.
        let status = unsafe {
            read(
                (&raw mut oid).cast(),
                &raw mut data,
                &raw mut len,
                &raw mut kind,
                backend,
                short_id.as_ptr(),
                prefix_len,
            )
        };
        if status != 0 {
            // As in `read`: `read_prefix_1` leaves a buffer published by a
            // failing callback alone, so this wrapper does not release it.
            return Err(GitOdbBackendError::Libgit2(status));
        }
        // SAFETY: the successful backend callback transfers a unique run of
        // `len` bytes allocated with `git_odb_backend_data_alloc`, which uses
        // libgit2's configured allocator and therefore matches this strategy.
        let data = unsafe { CVec::from_raw_parts(data.cast::<u8>(), len) };
        let data = GitOdbBackendData::from_callback_bytes(data, len)?;
        let kind =
            GitObjectType::from_raw(kind).ok_or(GitOdbBackendError::InvalidObjectType(kind))?;
        Ok((oid, data, kind))
    }

    /// Field: git_odb_backend.read_header
    /// Reads the byte length and kind of an object without requiring its body.
    pub fn read_header(
        &mut self,
        oid: OidRef<'_>,
    ) -> Result<(usize, GitObjectType), GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let read_header = backend_callback!(
            self,
            backend,
            read_header,
            GitOdbBackendOperation::ReadHeader
        )?;
        let mut len = 0;
        let mut kind = ffi::git_object_t_GIT_OBJECT_INVALID;
        // SAFETY: scalar outputs are writable, `oid` is live, and this handle
        // exclusively drives the synchronous callback.
        let status = unsafe { read_header(&raw mut len, &raw mut kind, backend, oid.as_ptr()) };
        if status != 0 {
            return Err(GitOdbBackendError::Libgit2(status));
        }
        let kind =
            GitObjectType::from_raw(kind).ok_or(GitOdbBackendError::InvalidObjectType(kind))?;
        Ok((len, kind))
    }

    /// Field: git_odb_backend.write
    /// Writes a complete object whose identifier has already been calculated.
    pub fn write(
        &mut self,
        oid: OidRef<'_>,
        data: &[u8],
        kind: GitObjectType,
    ) -> Result<(), GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let write = backend_callback!(self, backend, write, GitOdbBackendOperation::Write)?;
        // SAFETY: `oid` and the byte slice remain live for this synchronous
        // callback, which this handle invokes exclusively and which retains
        // neither borrowed input.
        let status = unsafe {
            write(
                backend,
                oid.as_ptr(),
                data.as_ptr().cast(),
                data.len(),
                kind.as_raw(),
            )
        };
        status_result(status)
    }

    /// Field: git_odb_backend.writestream
    /// Opens and takes ownership of a stream for writing one object.
    pub fn write_stream<'backend>(
        &'backend mut self,
        size: ffi::git_object_size_t,
        kind: GitObjectType,
    ) -> Result<GitOdbBackendStream<'backend>, GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let open = backend_callback!(
            self,
            backend,
            writestream,
            GitOdbBackendOperation::WriteStream
        )?;
        let mut stream = null_mut();
        // SAFETY: the output slot is writable and this handle exclusively
        // drives the synchronous constructor. Success transfers one complete
        // stream allocation to the caller.
        let status = unsafe { open(&raw mut stream, backend, size, kind.as_raw()) };
        if status != 0 {
            return Err(GitOdbBackendError::Libgit2(status));
        }
        // SAFETY: success transfers a fully constructed stream whose public
        // destructor matches `GitOdbStreamOwned`.
        let inner = unsafe { CBox::<GitOdbStream>::from_raw(stream) }
            .ok_or(GitOdbBackendError::MissingStream)?;
        Ok(GitOdbBackendStream {
            inner,
            _backend: PhantomData,
        })
    }

    /// Field: git_odb_backend.readstream
    /// Opens and takes ownership of a stream for reading one object.
    pub fn read_stream<'backend>(
        &'backend mut self,
        oid: OidRef<'_>,
    ) -> Result<(GitOdbBackendStream<'backend>, usize, GitObjectType), GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let open = backend_callback!(
            self,
            backend,
            readstream,
            GitOdbBackendOperation::ReadStream
        )?;
        let mut stream = null_mut();
        let mut len = 0;
        let mut kind = ffi::git_object_t_GIT_OBJECT_INVALID;
        // SAFETY: outputs are writable, `oid` is live, and this exclusive
        // handle drives the synchronous constructor. Success transfers the
        // published stream allocation.
        let status = unsafe {
            open(
                &raw mut stream,
                &raw mut len,
                &raw mut kind,
                backend,
                oid.as_ptr(),
            )
        };
        if status != 0 {
            return Err(GitOdbBackendError::Libgit2(status));
        }
        // SAFETY: success transfers a fully constructed stream with the
        // registered public destructor.
        let inner = unsafe { CBox::<GitOdbStream>::from_raw(stream) }
            .ok_or(GitOdbBackendError::MissingStream)?;
        let kind =
            GitObjectType::from_raw(kind).ok_or(GitOdbBackendError::InvalidObjectType(kind))?;
        Ok((
            GitOdbBackendStream {
                inner,
                _backend: PhantomData,
            },
            len,
            kind,
        ))
    }

    /// Field: git_odb_backend.exists
    /// Tests whether this backend contains an object.
    pub fn exists(&mut self, oid: OidRef<'_>) -> Result<bool, GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let exists = backend_callback!(self, backend, exists, GitOdbBackendOperation::Exists)?;
        // SAFETY: `oid` is live for the synchronous call and this handle
        // exclusively invokes the installed backend callback.
        let status = unsafe { exists(backend, oid.as_ptr()) };
        if status < 0 {
            Err(GitOdbBackendError::Libgit2(status))
        } else {
            Ok(status != 0)
        }
    }

    /// Field: git_odb_backend.exists_prefix
    /// Resolves a unique object identifier from a prefix.
    pub fn exists_prefix(
        &mut self,
        short_id: OidRef<'_>,
        prefix_len: usize,
    ) -> Result<Oid, GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let exists = backend_callback!(
            self,
            backend,
            exists_prefix,
            GitOdbBackendOperation::ExistsPrefix
        )?;
        let mut oid = Oid::zeroed();
        // SAFETY: the output is writable, `short_id` is live, and this handle
        // exclusively invokes the synchronous callback.
        let status = unsafe {
            exists(
                (&raw mut oid).cast(),
                backend,
                short_id.as_ptr(),
                prefix_len,
            )
        };
        if status == 0 {
            Ok(oid)
        } else {
            Err(GitOdbBackendError::Libgit2(status))
        }
    }

    /// Field: git_odb_backend.refresh
    /// Refreshes the concrete backend's storage view.
    pub fn refresh(&mut self) -> Result<(), GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let refresh = backend_callback!(self, backend, refresh, GitOdbBackendOperation::Refresh)?;
        // SAFETY: this handle supplies exclusive access for the complete
        // synchronous callback.
        status_result(unsafe { refresh(backend) })
    }

    /// Field: git_odb_backend.foreach
    /// Visits each object ID and stops when `callback` returns nonzero.
    pub fn foreach<C: GitOdbForeachCallback>(
        &mut self,
        callback: &mut C,
    ) -> Result<(), GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let foreach = backend_callback!(self, backend, foreach, GitOdbBackendOperation::Foreach)?;
        // SAFETY: the trampoline and payload have matching `C`; `callback`
        // remains live and exclusively borrowed throughout this synchronous
        // dispatch, and neither pointer is retained afterward.
        let status = unsafe {
            foreach(
                backend,
                Some(foreach_trampoline::<C>),
                (callback as *mut C).cast(),
            )
        };
        status_result(status)
    }

    /// Field: git_odb_backend.writepack
    /// Opens a pack writer and reports indexer progress to `progress`.
    pub fn write_pack<'owners, C: GitIndexerProgressCallback>(
        &'owners mut self,
        odb: &'owners mut GitOdbMut<'_>,
        progress: C,
    ) -> Result<GitOdbBackendWritepack<'owners, C>, GitOdbBackendError> {
        let mut progress = Box::new(progress);
        let inner =
            self.write_pack_impl(odb, Some(progress.as_mut()), Some(progress_trampoline::<C>))?;
        Ok(GitOdbBackendWritepack {
            inner,
            _progress: Some(progress),
            _owners: PhantomData,
        })
    }

    /// Opens a pack writer without progress notifications.
    pub fn write_pack_without_progress<'owners>(
        &'owners mut self,
        odb: &'owners mut GitOdbMut<'_>,
    ) -> Result<GitOdbBackendWritepack<'owners>, GitOdbBackendError> {
        let inner = self.write_pack_impl::<()>(odb, None, None)?;
        Ok(GitOdbBackendWritepack {
            inner,
            _progress: None,
            _owners: PhantomData,
        })
    }

    fn write_pack_impl<C>(
        &mut self,
        odb: &mut GitOdbMut<'_>,
        progress_state: Option<&mut C>,
        progress: ffi::git_indexer_progress_cb,
    ) -> Result<OdbWritepackOwned, GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let open = backend_callback!(self, backend, writepack, GitOdbBackendOperation::WritePack)?;
        let mut writepack = null_mut();
        let payload = progress_state.map_or(null_mut(), |state| (state as *mut C).cast());
        // SAFETY: the output is writable and both typed handles are exclusively
        // borrowed. Any progress payload is paired with its trampoline; the
        // public constructors tether that stable payload and both handles to
        // the returned writepack for as long as the C indexer may retain them.
        let status = unsafe {
            open(
                &raw mut writepack,
                backend,
                odb.as_mut_ptr(),
                progress,
                payload,
            )
        };
        if status != 0 {
            return Err(GitOdbBackendError::Libgit2(status));
        }
        // SAFETY: success transfers one fully constructed concrete writepack
        // whose installed `free` callback is the registered drop contract.
        unsafe { CBox::<OdbWritepack>::from_raw(writepack) }
            .ok_or(GitOdbBackendError::MissingWritepack)
    }

    /// Field: git_odb_backend.writemidx
    /// Writes a multi-pack index for this backend.
    pub fn write_multi_pack_index(&mut self) -> Result<(), GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let write = backend_callback!(
            self,
            backend,
            writemidx,
            GitOdbBackendOperation::WriteMultiPackIndex
        )?;
        // SAFETY: this handle exclusively invokes the synchronous callback.
        status_result(unsafe { write(backend) })
    }

    /// Field: git_odb_backend.freshen
    /// Returns whether this backend found and freshened the object.
    pub fn freshen(&mut self, oid: OidRef<'_>) -> Result<bool, GitOdbBackendError> {
        let backend = self.as_mut_ptr();
        let freshen = backend_callback!(self, backend, freshen, GitOdbBackendOperation::Freshen)?;
        // SAFETY: `oid` is live for this synchronous callback and the backend
        // handle is exclusively borrowed for its duration.
        Ok(unsafe { freshen(backend, oid.as_ptr()) } == 0)
    }
}

/// Field: git_odb_backend.free
// SAFETY: adopting `GitOdbBackendOwned` requires a fully constructed concrete
// backend with its required destructor installed. `CBox` invokes that callback
// exactly once and accesses no part of the allocation afterward.
unsafe impl ffibox::CDropped for GitOdbBackend {
    unsafe fn c_drop(backend: NonNull<Self>) {
        let backend = backend.as_ptr().cast::<ffi::git_odb_backend>();
        // SAFETY: raw-place projection reads the initialized required callback
        // from the live backend supplied by the `CDropped` contract.
        let free = unsafe { addr_of!((*backend).free).read() }
            .expect("a complete git_odb_backend has a free callback");
        // SAFETY: this is the concrete backend's installed destructor and the
        // owner grants its one final invocation.
        unsafe { free(backend) }
    }
}

fn status_result(status: c_int) -> Result<(), GitOdbBackendError> {
    if status == 0 {
        Ok(())
    } else {
        Err(GitOdbBackendError::Libgit2(status))
    }
}

unsafe extern "C" fn foreach_trampoline<C: GitOdbForeachCallback>(
    oid: *const ffi::git_oid,
    payload: *mut c_void,
) -> c_int {
    // SAFETY: the backend callback contract supplies a live OID for this call.
    let Some(oid) = (unsafe { OidRef::from_ptr(oid.cast_mut()) }) else {
        return -1;
    };
    if payload.is_null() {
        return -1;
    }
    // SAFETY: `foreach` passes the address of its exclusively borrowed `C`
    // and the backend callback retains it only for the synchronous dispatch.
    let callback = unsafe { &mut *payload.cast::<C>() };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback.call(oid))).unwrap_or(-1)
}

unsafe extern "C" fn progress_trampoline<C: GitIndexerProgressCallback>(
    progress: *const ffi::git_indexer_progress,
    payload: *mut c_void,
) -> c_int {
    // SAFETY: the callback contract supplies a live progress record for this call.
    let Some(progress) = (unsafe { IndexerProgressRef::from_ptr(progress.cast_mut()) }) else {
        return -1;
    };
    if payload.is_null() {
        return -1;
    }
    // SAFETY: `write_pack` boxes `C` at a stable address, pairs it with this
    // monomorphized trampoline, and retains it exclusively until the writepack
    // (and therefore the C indexer that may call us) has been destroyed.
    let callback = unsafe { &mut *payload.cast::<C>() };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback.call(progress))).unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

    use ffibox::{CCell, CDropped};

    use super::*;

    static FREES: AtomicUsize = AtomicUsize::new(0);
    static WRITES: AtomicUsize = AtomicUsize::new(0);
    static PUBLISHED: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

    unsafe extern "C" fn test_free(backend: *mut ffi::git_odb_backend) {
        FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the test transfers one `Box` allocation to the owner and
        // this callback performs its only final release.
        drop(unsafe { Box::from_raw(backend) });
    }

    unsafe extern "C" fn test_exists(
        _backend: *mut ffi::git_odb_backend,
        _oid: *const ffi::git_oid,
    ) -> c_int {
        1
    }

    unsafe extern "C" fn test_write(
        _backend: *mut ffi::git_odb_backend,
        _oid: *const ffi::git_oid,
        _data: *const c_void,
        len: usize,
        _kind: ffi::git_object_t,
    ) -> c_int {
        WRITES.store(len, Ordering::SeqCst);
        0
    }

    /// A failing read that also publishes a buffer, to prove the wrapper does
    /// not adopt one the C protocol never transfers.
    unsafe extern "C" fn test_failing_read(
        data: *mut *mut c_void,
        len: *mut usize,
        kind: *mut ffi::git_object_t,
        _backend: *mut ffi::git_odb_backend,
        _oid: *const ffi::git_oid,
    ) -> c_int {
        // SAFETY: the wrapper supplies three writable output slots, and this
        // callback keeps ownership of the leaked buffer it publishes.
        unsafe {
            *data = PUBLISHED.load(Ordering::SeqCst);
            *len = 3;
            *kind = ffi::git_object_t_GIT_OBJECT_BLOB;
        }
        -1
    }

    unsafe extern "C" fn test_failing_read_prefix(
        oid: *mut ffi::git_oid,
        data: *mut *mut c_void,
        len: *mut usize,
        kind: *mut ffi::git_object_t,
        backend: *mut ffi::git_odb_backend,
        short_id: *const ffi::git_oid,
        _prefix_len: usize,
    ) -> c_int {
        // SAFETY: the wrapper supplies a writable object-ID slot alongside the
        // three outputs the shared read callback fills in.
        unsafe { core::ptr::write_bytes(oid, 0, 1) };
        // SAFETY: the wrapper's outputs and backend handle are those the read
        // callback expects, and it retains none of them.
        unsafe { test_failing_read(data, len, kind, backend, short_id) }
    }

    fn test_backend() -> ffi::git_odb_backend {
        ffi::git_odb_backend {
            version: 1,
            odb: null_mut(),
            read: None,
            read_prefix: None,
            read_header: None,
            write: Some(test_write),
            writestream: None,
            readstream: None,
            exists: Some(test_exists),
            exists_prefix: None,
            refresh: None,
            foreach: None,
            writepack: None,
            writemidx: None,
            freshen: None,
            free: Some(test_free),
        }
    }

    #[test]
    fn backend_wrapper_preserves_layout_and_drop_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitOdbBackend>();
        assert_dropped::<GitOdbBackend>();
        assert_eq!(
            size_of::<GitOdbBackend>(),
            size_of::<ffi::git_odb_backend>()
        );
        assert_eq!(
            align_of::<GitOdbBackend>(),
            align_of::<ffi::git_odb_backend>()
        );
        assert_eq!(
            size_of::<GitOdbBackendRef<'_>>(),
            size_of::<*const ffi::git_odb_backend>()
        );
        assert_eq!(
            size_of::<GitOdbBackendMut<'_>>(),
            size_of::<*mut ffi::git_odb_backend>()
        );
        assert_eq!(
            size_of::<Option<GitOdbBackendOwned>>(),
            size_of::<*mut ffi::git_odb_backend>()
        );
    }

    #[test]
    fn getters_and_callback_dispatch_are_typed() {
        WRITES.store(0, Ordering::SeqCst);
        let mut raw = test_backend();
        let oid = Oid::zeroed();
        // SAFETY: `raw` is fully initialized and exclusively borrowed by this
        // handle; `oid` stays live for every synchronous callback.
        let mut backend = unsafe { GitOdbBackendMut::from_ptr(&raw mut raw) }.unwrap();
        // SAFETY: `oid` remains live and immutable for this scope; callbacks in
        // this test do not inspect its deliberately zeroed contents.
        let oid = unsafe { OidRef::from_ptr((&raw const oid).cast_mut().cast()) }.unwrap();

        assert_eq!(backend.as_ref().version(), 1);
        assert!(backend.as_ref().odb().is_none());
        assert_eq!(backend.exists(oid), Ok(true));
        assert_eq!(backend.write(oid, b"abc", GitObjectType::BLOB), Ok(()));
        assert_eq!(WRITES.load(Ordering::SeqCst), 3);
        assert_eq!(
            backend.refresh(),
            Err(GitOdbBackendError::Unsupported(
                GitOdbBackendOperation::Refresh
            ))
        );
    }

    #[test]
    fn backend_data_keeps_missing_storage_typed() {
        let empty = GitOdbBackendData::from_callback_bytes(None, 0);
        assert!(matches!(empty, Ok(data) if data.is_empty()));

        let missing = GitOdbBackendData::from_callback_bytes(None, 1);
        assert!(matches!(missing, Err(GitOdbBackendError::MissingData)));
    }

    #[test]
    fn a_failing_read_leaves_a_published_buffer_with_the_backend() {
        let bytes = Box::into_raw(Box::new(*b"abc"));
        PUBLISHED.store(bytes.cast(), Ordering::SeqCst);

        let mut raw = test_backend();
        raw.read = Some(test_failing_read);
        raw.read_prefix = Some(test_failing_read_prefix);
        let oid = Oid::zeroed();
        // SAFETY: `raw` is fully initialized and exclusively borrowed here.
        let mut backend = unsafe { GitOdbBackendMut::from_ptr(&raw mut raw) }.unwrap();
        // SAFETY: `oid` stays live and immutable for both synchronous calls.
        let oid = unsafe { OidRef::from_ptr((&raw const oid).cast_mut().cast()) }.unwrap();

        assert_eq!(
            backend.read(oid).err(),
            Some(GitOdbBackendError::Libgit2(-1))
        );
        assert_eq!(
            backend.read_prefix(oid, 2).err(),
            Some(GitOdbBackendError::Libgit2(-1))
        );

        // The buffer was neither freed nor adopted, so the callback's owner
        // still reclaims it. Freeing it twice would fail under the sanitizers.
        // SAFETY: this reclaims the single leaked allocation published above.
        assert_eq!(*unsafe { Box::from_raw(bytes) }, *b"abc");
        PUBLISHED.store(null_mut(), Ordering::SeqCst);
    }

    #[test]
    fn owning_backend_invokes_installed_destructor_once() {
        FREES.store(0, Ordering::SeqCst);
        let raw = Box::into_raw(Box::new(test_backend()));
        // SAFETY: `raw` is a fresh, fully initialized backend allocation whose
        // installed callback uniquely frees that same allocation.
        let backend = unsafe { GitOdbBackendOwned::from_raw(raw) }.unwrap();
        drop(backend);
        assert_eq!(FREES.load(Ordering::SeqCst), 1);
    }
}
