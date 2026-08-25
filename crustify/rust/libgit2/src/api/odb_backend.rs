//! Safe wrappers for libgit2 odb_backend APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::NonNull;

use ffibox::{CBox, CVal, CValued};

use crate::ffi;
use crate::indexer::IndexerProgressMut;
use crate::oid::{InvalidOidType, OidRef, OidType};
use crate::sys::odb_backend::{GitOdbBackendMut, GitOdbBackendRef};

ffibox::define_ctype!(
    /// Wraps: git_odb_stream
    /// A layout-compatible polymorphic stream supplied by an object-database
    /// backend.
    ///
    /// Only the write path fills the header in full: `git_odb_open_wstream`
    /// assigns `oid_type`, `declared_size`, `received_bytes` and `hash_ctx`
    /// after the backend constructor has set `mode` to `GIT_STREAM_WRONLY`.
    /// `git_odb_open_rstream` assigns none of them, so a read stream keeps the
    /// zeroes its concrete allocation was calloc'd with. The validating
    /// getters below report that state instead of inventing a value for it.
    GitOdbStream,
    GitOdbStreamRef,
    GitOdbStreamMut,
    ffi::git_odb_stream
);

/// An exclusively owned ODB stream. Dropping it cleans its frontend hash
/// context and invokes the concrete backend stream's destructor.
pub type GitOdbStreamOwned = ffibox::CBox<GitOdbStream>;

/// Wraps: git_odb_stream_t
/// Published read/write capabilities of an ODB stream.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GitOdbStreamMode(ffi::git_odb_stream_t);

impl GitOdbStreamMode {
    /// The stream can only be read.
    pub const READ: Self = Self(ffi::git_odb_stream_t_GIT_STREAM_RDONLY);
    /// The stream can only be written.
    pub const WRITE: Self = Self(ffi::git_odb_stream_t_GIT_STREAM_WRONLY);
    /// The stream can be read and written.
    pub const READ_WRITE: Self = Self(ffi::git_odb_stream_t_GIT_STREAM_RW);

    /// Converts a C value when it is a nonempty set of published mode bits.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_odb_stream_t) -> Option<Self> {
        if bits != 0 && bits & !Self::READ_WRITE.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying C bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_odb_stream_t {
        self.0
    }

    /// Returns whether all capabilities in `other` are present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl From<GitOdbStreamMode> for ffi::git_odb_stream_t {
    fn from(mode: GitOdbStreamMode) -> Self {
        mode.bits()
    }
}

/// Identifies an unavailable operation on a mode-specific ODB stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitOdbStreamOperation {
    /// Reading bytes from the stream.
    Read,
    /// Writing bytes to the stream.
    Write,
    /// Finalizing a completed write.
    FinalizeWrite,
}

/// Failure returned while dispatching an ODB stream callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitOdbStreamError {
    /// The stream did not install the callback for this operation.
    Unsupported(GitOdbStreamOperation),
    /// The callback returned a libgit2 error code.
    Libgit2(core::ffi::c_int),
    /// A read callback claimed more initialized bytes than the supplied
    /// destination can hold.
    InvalidReadCount(usize),
    /// Finalization was requested before the stream received the byte count
    /// it declared, which `git_odb_stream_finalize_write` also refuses.
    IncompleteWrite {
        /// The stream's `received_bytes` field.
        received: ffi::git_object_size_t,
        /// The stream's `declared_size` field.
        declared: ffi::git_object_size_t,
    },
}

impl<'a> GitOdbStreamRef<'a> {
    /// Field: git_odb_stream.mode
    /// Returns the stream's published capability bits, or `None` if C stored
    /// an unknown or empty mode.
    ///
    /// `None` is the normal answer for a stream from `git_odb_open_rstream`:
    /// no read-stream constructor in libgit2 assigns `mode`, so the field
    /// keeps its zero. Only the write-stream constructors store
    /// `GIT_STREAM_WRONLY`.
    #[must_use]
    pub fn mode(&self) -> Option<GitOdbStreamMode> {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        let bits = unsafe { core::ptr::addr_of!((*stream).mode).read() };
        GitOdbStreamMode::from_bits(bits)
    }

    /// Field: git_odb_stream.oid_type
    /// Returns the object-ID algorithm after validating the C value.
    ///
    /// An error is the normal answer for a read stream: `git_oid_t` has no
    /// zero variant and only `git_odb_open_wstream` copies the database's
    /// `oid_type` into the header.
    pub fn oid_type(&self) -> Result<OidType, InvalidOidType> {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        let raw = unsafe { core::ptr::addr_of!((*stream).oid_type).read() };
        OidType::try_from(raw)
    }

    /// Field: git_odb_stream.backend
    /// Borrows the backend that created this stream, if it was installed.
    #[must_use]
    pub fn backend(&self) -> Option<GitOdbBackendRef<'a>> {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized pointer without dereferencing it.
        let backend = unsafe { core::ptr::addr_of!((*stream).backend).read() };
        // SAFETY: a non-null stream backend remains live until after the
        // stream is destroyed, so the typed borrow is bounded by this handle.
        unsafe { GitOdbBackendRef::from_ptr(backend) }
    }

    /// Field: git_odb_stream.received_bytes
    /// Returns the number of bytes accepted by this stream so far.
    ///
    /// The frontend `git_odb_stream_write` maintains this count. Dispatching
    /// [`GitOdbStreamMut::write`] does not, so a caller driving the field
    /// callbacks itself must maintain it with
    /// [`GitOdbStreamMut::set_received_bytes`].
    #[must_use]
    pub fn received_bytes(&self) -> ffi::git_object_size_t {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*stream).received_bytes).read() }
    }

    /// Field: git_odb_stream.hash_ctx
    /// Returns whether the frontend installed its owned hashing context.
    ///
    /// The erased context is owned by the stream and is deliberately not
    /// exposed: [`ffi::git_odb_stream_free`] performs its typed cleanup.
    #[must_use]
    pub fn has_hash_context(&self) -> bool {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized pointer without dereferencing it.
        !unsafe { core::ptr::addr_of!((*stream).hash_ctx).read() }.is_null()
    }

    /// Field: git_odb_stream.declared_size
    /// Returns the byte count promised when the write stream was opened.
    ///
    /// Zero on a read stream, which never has a promised size.
    #[must_use]
    pub fn declared_size(&self) -> ffi::git_object_size_t {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*stream).declared_size).read() }
    }
}

impl GitOdbStreamMut<'_> {
    /// Set the stream capability bits during concrete-stream initialization.
    pub fn set_mode(&mut self, mode: GitOdbStreamMode) {
        let stream = self.as_mut_ptr();
        // SAFETY: `stream` comes from this exclusive handle; raw-place
        // projection writes a validated scalar without forming a reference.
        unsafe { core::ptr::addr_of_mut!((*stream).mode).write(mode.bits()) }
    }

    /// Set the object-ID algorithm during stream initialization.
    pub fn set_oid_type(&mut self, oid_type: OidType) {
        let stream = self.as_mut_ptr();
        // SAFETY: `stream` comes from this exclusive handle; raw-place
        // projection writes a valid C enum value without forming a reference.
        unsafe { core::ptr::addr_of_mut!((*stream).oid_type).write(ffi::git_oid_t::from(oid_type)) }
    }

    /// Store the borrowed backend link during concrete-stream initialization.
    ///
    /// # Safety
    ///
    /// `backend` must remain live and at a stable address until this stream is
    /// destroyed, including for any callback dispatched through the stream.
    /// It must designate the backend that installed those callbacks.
    pub unsafe fn set_backend(&mut self, mut backend: GitOdbBackendMut<'_>) {
        let stream = self.as_mut_ptr();
        // SAFETY: the caller supplies the unexpressible backend lifetime and
        // identity guarantees; this exclusive handle permits the field write.
        unsafe { core::ptr::addr_of_mut!((*stream).backend).write(backend.as_mut_ptr()) }
    }

    /// Field: git_odb_stream.write
    /// Dispatches the concrete stream's synchronous write callback.
    ///
    /// This is the backend callback alone. The frontend `git_odb_stream_write`
    /// additionally hashes the bytes into `hash_ctx`, advances
    /// `received_bytes` and rejects a write past `declared_size`; none of that
    /// happens here. A caller driving the field callbacks on a frontend write
    /// stream must keep the count itself with
    /// [`set_received_bytes`](Self::set_received_bytes), or the object id
    /// computed by `git_odb_stream_finalize_write` will not match the bytes.
    pub fn write(&mut self, buffer: &[u8]) -> Result<(), GitOdbStreamError> {
        let stream = self.as_mut_ptr();
        // SAFETY: this exclusive handle addresses a live stream; raw-place
        // projection reads its initialized callback slot without a reference.
        let Some(write) = (unsafe { core::ptr::addr_of!((*stream).write).read() }) else {
            return Err(GitOdbStreamError::Unsupported(GitOdbStreamOperation::Write));
        };
        // SAFETY: the installed callback follows the public synchronous stream
        // contract. The slice supplies exactly `buffer.len()` readable bytes,
        // and this handle prevents another Rust invocation during the call.
        let result = unsafe {
            write(
                stream,
                buffer.as_ptr().cast::<core::ffi::c_char>(),
                buffer.len(),
            )
        };
        if result < 0 {
            Err(GitOdbStreamError::Libgit2(result))
        } else {
            Ok(())
        }
    }

    /// Field: git_odb_stream.read
    /// Dispatches the concrete stream's synchronous read callback.
    ///
    /// Returns the number of bytes the callback reported writing into
    /// `buffer`. The frontend `git_odb_stream_read` is this dispatch and
    /// nothing else, so the two agree.
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, GitOdbStreamError> {
        let stream = self.as_mut_ptr();
        // SAFETY: this exclusive handle addresses a live stream; raw-place
        // projection reads its initialized callback slot without a reference.
        let Some(read) = (unsafe { core::ptr::addr_of!((*stream).read).read() }) else {
            return Err(GitOdbStreamError::Unsupported(GitOdbStreamOperation::Read));
        };
        // SAFETY: the installed callback follows the public synchronous stream
        // contract. The slice supplies exactly `buffer.len()` writable bytes,
        // and this handle prevents another Rust invocation during the call.
        let result = unsafe {
            read(
                stream,
                buffer.as_mut_ptr().cast::<core::ffi::c_char>(),
                buffer.len(),
            )
        };
        if result < 0 {
            return Err(GitOdbStreamError::Libgit2(result));
        }
        let count = usize::try_from(result).expect("a nonnegative C int fits usize");
        if count > buffer.len() {
            Err(GitOdbStreamError::InvalidReadCount(count))
        } else {
            Ok(count)
        }
    }

    /// Field: git_odb_stream.finalize_write
    /// Dispatches the concrete stream's synchronous finalization callback
    /// once the stream has received the byte count it declared.
    ///
    /// The length equality is a precondition of the callback, not a courtesy:
    /// `init_fake_wstream` hands the backend a `git__malloc` buffer of
    /// `declared_size` bytes and `fake_wstream__fwrite` forwards all of them,
    /// so finalizing a short stream makes libgit2 read heap bytes nothing ever
    /// wrote. `git_odb_stream_finalize_write` refuses that case, and so does
    /// this dispatcher, with [`GitOdbStreamError::IncompleteWrite`].
    pub fn finalize_write(&mut self, oid: OidRef<'_>) -> Result<(), GitOdbStreamError> {
        let (received, declared) = {
            let shared = self.as_ref();
            (shared.received_bytes(), shared.declared_size())
        };
        if received != declared {
            return Err(GitOdbStreamError::IncompleteWrite { received, declared });
        }

        let stream = self.as_mut_ptr();
        // SAFETY: this exclusive handle addresses a live stream; raw-place
        // projection reads its initialized callback slot without a reference.
        let Some(finalize) = (unsafe { core::ptr::addr_of!((*stream).finalize_write).read() })
        else {
            return Err(GitOdbStreamError::Unsupported(
                GitOdbStreamOperation::FinalizeWrite,
            ));
        };
        // SAFETY: the installed callback follows the public synchronous stream
        // contract. `oid` keeps a complete object ID live for this call, and
        // this handle prevents another Rust invocation during the call.
        let result = unsafe { finalize(stream, oid.as_ptr()) };
        if result < 0 {
            Err(GitOdbStreamError::Libgit2(result))
        } else {
            Ok(())
        }
    }

    /// Set the byte count accepted so far.
    pub fn set_received_bytes(&mut self, received: ffi::git_object_size_t) {
        let stream = self.as_mut_ptr();
        // SAFETY: `stream` comes from this exclusive handle; raw-place
        // projection writes the scalar without forming a reference.
        unsafe { core::ptr::addr_of_mut!((*stream).received_bytes).write(received) }
    }

    /// Set the promised write size during stream initialization.
    pub fn set_declared_size(&mut self, size: ffi::git_object_size_t) {
        let stream = self.as_mut_ptr();
        // SAFETY: `stream` comes from this exclusive handle; raw-place
        // projection writes the scalar without forming a reference.
        unsafe { core::ptr::addr_of_mut!((*stream).declared_size).write(size) }
    }
}

/// Field: git_odb_stream.free
// SAFETY: adopting `GitOdbStreamOwned` requires a fully constructed stream
// allocation with a non-null concrete destructor installed, which
// `git_odb_stream_free` invokes unconditionally, and with `hash_ctx` either
// null or one libgit2-allocated `git_hash_ctx`, which it runs
// `git_hash_ctx_cleanup` and `git__free` over first. Both hold for every
// stream produced by `git_odb_open_wstream` or `git_odb_open_rstream`. The
// destructor runs exactly once; `CBox` never accesses the allocation
// afterward.
unsafe impl ffibox::CDropped for GitOdbStream {
    unsafe fn c_drop(stream: NonNull<Self>) {
        // SAFETY: the `CDropped` contract supplies one live, fully constructed,
        // uniquely owned stream allocation for its final release.
        unsafe { ffi::git_odb_stream_free(stream.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};

    static WRITTEN: AtomicUsize = AtomicUsize::new(0);
    static FINALIZED: AtomicUsize = AtomicUsize::new(0);
    static FREED: AtomicUsize = AtomicUsize::new(0);
    /// Counts finalizations for the short-stream test alone. Sharing
    /// `FINALIZED` would race the concurrently scheduled dispatch test, which
    /// resets and reads the same cell.
    static SHORT_FINALIZED: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn test_read(
        _stream: *mut ffi::git_odb_stream,
        buffer: *mut core::ffi::c_char,
        len: usize,
    ) -> core::ffi::c_int {
        let count = len.min(3);
        // SAFETY: the read-callback contract supplies `len` writable bytes;
        // `count <= len`, and the source has at least three bytes.
        unsafe { core::ptr::copy_nonoverlapping(b"abc".as_ptr(), buffer.cast(), count) };
        core::ffi::c_int::try_from(count).expect("count is at most three")
    }

    unsafe extern "C" fn test_write(
        _stream: *mut ffi::git_odb_stream,
        _buffer: *const core::ffi::c_char,
        len: usize,
    ) -> core::ffi::c_int {
        WRITTEN.store(len, Ordering::SeqCst);
        0
    }

    unsafe extern "C" fn test_finalize(
        _stream: *mut ffi::git_odb_stream,
        oid: *const ffi::git_oid,
    ) -> core::ffi::c_int {
        // SAFETY: the finalize-callback contract supplies a complete live OID
        // for the duration of this call.
        let first = unsafe { core::ptr::addr_of!((*oid).id).cast::<u8>().read() };
        FINALIZED.store(first.into(), Ordering::SeqCst);
        0
    }

    unsafe extern "C" fn test_short_finalize(
        _stream: *mut ffi::git_odb_stream,
        oid: *const ffi::git_oid,
    ) -> core::ffi::c_int {
        // SAFETY: the finalize-callback contract supplies a complete live OID
        // for the duration of this call.
        let first = unsafe { core::ptr::addr_of!((*oid).id).cast::<u8>().read() };
        SHORT_FINALIZED.store(first.into(), Ordering::SeqCst);
        0
    }

    unsafe extern "C" fn test_free(stream: *mut ffi::git_odb_stream) {
        FREED.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the test adopts exactly one allocation produced by
        // `Box::into_raw`, and the owned wrapper invokes this callback once.
        unsafe { drop(Box::from_raw(stream)) }
    }

    fn test_stream() -> ffi::git_odb_stream {
        ffi::git_odb_stream {
            backend: core::ptr::null_mut(),
            mode: ffi::git_odb_stream_t_GIT_STREAM_RW,
            hash_ctx: core::ptr::null_mut(),
            oid_type: ffi::git_oid_t_GIT_OID_SHA1,
            declared_size: 9,
            received_bytes: 2,
            read: Some(test_read),
            write: Some(test_write),
            finalize_write: Some(test_finalize),
            free: Some(test_free),
        }
    }

    #[test]
    fn wrapper_and_owner_preserve_the_c_layout() {
        assert_eq!(size_of::<GitOdbStream>(), size_of::<ffi::git_odb_stream>());
        assert_eq!(
            align_of::<GitOdbStream>(),
            align_of::<ffi::git_odb_stream>()
        );
        assert_eq!(
            size_of::<GitOdbStreamRef<'_>>(),
            size_of::<*const ffi::git_odb_stream>()
        );
        assert_eq!(
            size_of::<GitOdbStreamMut<'_>>(),
            size_of::<*mut ffi::git_odb_stream>()
        );
        assert_eq!(
            size_of::<GitOdbStreamOwned>(),
            size_of::<*mut ffi::git_odb_stream>()
        );
    }

    #[test]
    fn borrowed_handles_validate_and_update_scalar_fields() {
        let mut raw = test_stream();
        // SAFETY: `raw` is initialized and exclusively borrowed for the
        // handle's lifetime.
        let mut stream = unsafe { GitOdbStreamMut::from_ptr(&raw mut raw) }.unwrap();

        assert_eq!(stream.as_ref().mode(), Some(GitOdbStreamMode::READ_WRITE));
        assert!(stream.as_ref().backend().is_none());
        assert!(!stream.as_ref().has_hash_context());
        assert_eq!(stream.as_ref().oid_type(), Ok(OidType::Sha1));
        assert_eq!(stream.as_ref().declared_size(), 9);
        assert_eq!(stream.as_ref().received_bytes(), 2);

        stream.set_mode(GitOdbStreamMode::WRITE);
        stream.set_oid_type(OidType::Sha256);
        stream.set_declared_size(12);
        stream.set_received_bytes(4);
        assert_eq!(stream.as_ref().mode(), Some(GitOdbStreamMode::WRITE));
        assert_eq!(stream.as_ref().oid_type(), Ok(OidType::Sha256));
        assert_eq!(stream.as_ref().declared_size(), 12);
        assert_eq!(stream.as_ref().received_bytes(), 4);
    }

    #[test]
    fn mutable_handle_dispatches_mode_specific_callbacks() {
        WRITTEN.store(0, Ordering::SeqCst);
        FINALIZED.store(0, Ordering::SeqCst);
        let mut raw = test_stream();
        // SAFETY: `raw` is initialized and exclusively borrowed for the
        // handle's lifetime.
        let mut stream = unsafe { GitOdbStreamMut::from_ptr(&raw mut raw) }.unwrap();

        let mut read = [0; 4];
        assert_eq!(stream.read(&mut read), Ok(3));
        assert_eq!(&read[..3], b"abc");
        assert_eq!(stream.write(b"hello"), Ok(()));
        assert_eq!(WRITTEN.load(Ordering::SeqCst), 5);
        // The field-level dispatcher does not keep the frontend's count, so
        // the caller advances it before the stream may be finalized.
        assert_eq!(stream.as_ref().received_bytes(), 2);
        stream.set_received_bytes(stream.as_ref().declared_size());

        let mut oid = ffi::git_oid {
            id: [0; 32],
            type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
        };
        oid.id[0] = 7;
        // SAFETY: `oid` is initialized and remains live and shared for the
        // duration of the callback dispatch.
        let oid = unsafe { OidRef::from_ptr(&raw mut oid) }.unwrap();
        assert_eq!(stream.finalize_write(oid), Ok(()));
        assert_eq!(FINALIZED.load(Ordering::SeqCst), 7);
    }

    #[test]
    fn absent_callbacks_are_reported_without_invocation() {
        let mut raw = test_stream();
        raw.read = None;
        raw.write = None;
        raw.finalize_write = None;
        // SAFETY: `raw` is initialized and exclusively borrowed for the
        // handle's lifetime.
        let mut stream = unsafe { GitOdbStreamMut::from_ptr(&raw mut raw) }.unwrap();

        assert_eq!(
            stream.read(&mut []),
            Err(GitOdbStreamError::Unsupported(GitOdbStreamOperation::Read))
        );
        assert_eq!(
            stream.write(&[]),
            Err(GitOdbStreamError::Unsupported(GitOdbStreamOperation::Write))
        );
    }

    #[test]
    fn finalizing_a_short_stream_is_refused_without_dispatching() {
        SHORT_FINALIZED.store(0, Ordering::SeqCst);
        let mut raw = test_stream();
        raw.finalize_write = Some(test_short_finalize);
        // SAFETY: `raw` is initialized and exclusively borrowed for the
        // handle's lifetime.
        let mut stream = unsafe { GitOdbStreamMut::from_ptr(&raw mut raw) }.unwrap();

        let mut oid = ffi::git_oid {
            id: [0; 32],
            type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
        };
        oid.id[0] = 7;
        // SAFETY: `oid` is initialized and stays live and shared below.
        let oid = unsafe { OidRef::from_ptr(&raw mut oid) }.unwrap();

        // `test_stream` declares nine bytes and has received two, exactly the
        // state `git_odb_stream_finalize_write` rejects.
        assert_eq!(
            stream.finalize_write(oid),
            Err(GitOdbStreamError::IncompleteWrite {
                received: 2,
                declared: 9,
            })
        );
        assert_eq!(SHORT_FINALIZED.load(Ordering::SeqCst), 0);

        stream.set_received_bytes(9);
        assert_eq!(stream.finalize_write(oid), Ok(()));
        assert_eq!(SHORT_FINALIZED.load(Ordering::SeqCst), 7);
    }

    #[test]
    fn a_read_stream_header_reports_the_zeroes_libgit2_leaves_in_it() {
        // `git_odb_open_rstream` fills in only `backend`, `hash_ctx` and the
        // callbacks: no read-stream constructor assigns `mode`, `oid_type`,
        // `declared_size` or `received_bytes`, so they keep the calloc'd zero.
        let mut raw = test_stream();
        raw.mode = 0;
        raw.oid_type = 0;
        raw.declared_size = 0;
        raw.received_bytes = 0;
        raw.write = None;
        raw.finalize_write = None;
        // SAFETY: `raw` is initialized and exclusively borrowed for the
        // handle's lifetime.
        let mut stream = unsafe { GitOdbStreamMut::from_ptr(&raw mut raw) }.unwrap();

        assert_eq!(stream.as_ref().mode(), None);
        assert!(stream.as_ref().oid_type().is_err());
        assert_eq!(stream.as_ref().declared_size(), 0);
        assert_eq!(stream.as_ref().received_bytes(), 0);
        assert!(stream.as_ref().backend().is_none());

        let mut read = [0; 4];
        assert_eq!(stream.read(&mut read), Ok(3));
        assert_eq!(
            stream.write(b"x"),
            Err(GitOdbStreamError::Unsupported(GitOdbStreamOperation::Write))
        );
    }

    #[test]
    fn owned_stream_uses_the_complete_public_destructor() {
        FREED.store(0, Ordering::SeqCst);
        let raw = Box::into_raw(Box::new(test_stream()));
        // SAFETY: `raw` is a unique, fully initialized stream allocation. Its
        // hash context is null and its installed free callback reclaims the
        // allocation exactly once.
        let stream = unsafe { GitOdbStreamOwned::from_raw(raw) }.unwrap();
        drop(stream);
        assert_eq!(FREED.load(Ordering::SeqCst), 1);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_odb_writepack
    /// A polymorphic sink that incrementally writes a packfile into an object
    /// database backend.
    OdbWritepack,
    OdbWritepackRef,
    OdbWritepackMut,
    ffi::git_odb_writepack
);

/// An exclusively owned writepack whose installed `free` callback releases
/// its concrete allocation.
pub type OdbWritepackOwned = CBox<OdbWritepack>;

impl<'a> OdbWritepackRef<'a> {
    /// Field: git_odb_writepack.backend
    /// Borrows the backend that created this writepack, if one is recorded.
    #[must_use]
    pub fn backend(&self) -> Option<GitOdbBackendRef<'a>> {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized pointer field without forming a reference to the backend
        // or to C-visible writepack storage.
        let backend = unsafe { core::ptr::addr_of!((*self.as_ptr()).backend).read() };
        // SAFETY: a non-null originating backend outlives this writepack.
        unsafe { GitOdbBackendRef::from_ptr(backend) }
    }
}

impl OdbWritepackMut<'_> {
    /// Field: git_odb_writepack.append
    /// Synchronously appends a chunk of packfile bytes and updates `stats`.
    pub fn append(
        &mut self,
        data: &[u8],
        stats: &mut IndexerProgressMut<'_>,
    ) -> Result<(), core::ffi::c_int> {
        let writepack = self.as_mut_ptr();
        // SAFETY: this live exclusive handle covers a fully constructed
        // writepack, and raw-place projection copies its initialized callback
        // without forming a reference to C-visible memory.
        let append = unsafe { core::ptr::addr_of!((*writepack).append).read() }
            .expect("a valid git_odb_writepack has an append callback");
        // SAFETY: `append` is installed for this live writepack. `data` keeps
        // exactly its readable byte run alive for the synchronous call, and
        // both writepack and progress have exclusive handles during it.
        let status = unsafe {
            append(
                writepack,
                data.as_ptr().cast::<core::ffi::c_void>(),
                data.len(),
                stats.as_mut_ptr(),
            )
        };
        if status < 0 { Err(status) } else { Ok(()) }
    }

    /// Field: git_odb_writepack.commit
    /// Finalizes the packfile and updates `stats`.
    pub fn commit(&mut self, stats: &mut IndexerProgressMut<'_>) -> Result<(), core::ffi::c_int> {
        let writepack = self.as_mut_ptr();
        // SAFETY: as in `append`, raw-place projection copies the callback from
        // this live, exclusively borrowed, fully constructed writepack.
        let commit = unsafe { core::ptr::addr_of!((*writepack).commit).read() }
            .expect("a valid git_odb_writepack has a commit callback");
        // SAFETY: `commit` is installed for this writepack and both mutable
        // arguments are exclusively borrowed for the duration of the call.
        let status = unsafe { commit(writepack, stats.as_mut_ptr()) };
        if status < 0 { Err(status) } else { Ok(()) }
    }
}

/// Field: git_odb_writepack.free
// SAFETY: adopting `OdbWritepackOwned` requires a unique, fully constructed
// concrete allocation whose installed callback releases that allocation.
// `CBox` invokes the callback exactly once and never accesses the pointer again.
unsafe impl ffibox::CDropped for OdbWritepack {
    unsafe fn c_drop(writepack: NonNull<Self>) {
        let writepack = writepack.as_ptr().cast::<ffi::git_odb_writepack>();
        // SAFETY: the `CDropped` contract supplies a live writepack, and
        // raw-place projection copies its initialized callback without forming
        // a reference to C-visible memory.
        let free = unsafe { core::ptr::addr_of!((*writepack).free).read() }
            .expect("a valid git_odb_writepack has a free callback");
        // SAFETY: the callback is the concrete destructor installed for this
        // uniquely owned writepack, and this is its one final invocation.
        unsafe { free(writepack) }
    }
}

#[cfg(test)]
mod writepack_tests {
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use crate::indexer::IndexerProgressMut;

    use super::*;

    static APPENDED: AtomicUsize = AtomicUsize::new(0);
    static COMMITS: AtomicUsize = AtomicUsize::new(0);
    static FREES: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn test_append(
        _writepack: *mut ffi::git_odb_writepack,
        data: *const core::ffi::c_void,
        size: usize,
        stats: *mut ffi::git_indexer_progress,
    ) -> core::ffi::c_int {
        if size != 0 {
            // SAFETY: the callback contract provides `size` readable bytes, so
            // a nonempty call permits reading the first one.
            assert_eq!(unsafe { data.cast::<u8>().read() }, b'p');
        }
        APPENDED.store(size, Ordering::SeqCst);
        // SAFETY: the callback contract supplies an exclusive live progress
        // value, and raw-place projection writes its scalar field.
        unsafe { core::ptr::addr_of_mut!((*stats).received_bytes).write(size) };
        0
    }

    unsafe extern "C" fn test_commit(
        _writepack: *mut ffi::git_odb_writepack,
        _stats: *mut ffi::git_indexer_progress,
    ) -> core::ffi::c_int {
        COMMITS.fetch_add(1, Ordering::SeqCst);
        0
    }

    unsafe extern "C" fn test_free(writepack: *mut ffi::git_odb_writepack) {
        FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the test adopts exactly one allocation created by
        // `Box::into_raw`, and the owning wrapper calls this function once.
        unsafe { drop(Box::from_raw(writepack)) }
    }

    #[test]
    fn wrapper_preserves_the_c_layout() {
        assert_eq!(
            size_of::<OdbWritepack>(),
            size_of::<ffi::git_odb_writepack>()
        );
        assert_eq!(
            align_of::<OdbWritepack>(),
            align_of::<ffi::git_odb_writepack>()
        );
        assert_eq!(
            size_of::<OdbWritepackRef<'_>>(),
            size_of::<*const ffi::git_odb_writepack>()
        );
        assert_eq!(
            size_of::<OdbWritepackMut<'_>>(),
            size_of::<*mut ffi::git_odb_writepack>()
        );
    }

    #[test]
    fn owned_writepack_dispatches_and_frees() {
        APPENDED.store(0, Ordering::SeqCst);
        COMMITS.store(0, Ordering::SeqCst);
        FREES.store(0, Ordering::SeqCst);

        let raw = Box::into_raw(Box::new(ffi::git_odb_writepack {
            backend: core::ptr::null_mut(),
            append: Some(test_append),
            commit: Some(test_commit),
            free: Some(test_free),
        }));
        // SAFETY: `raw` is a unique fully initialized allocation whose
        // installed destructor releases that exact allocation once.
        let mut writepack =
            unsafe { OdbWritepackOwned::from_raw(raw) }.expect("Box::into_raw is non-null");

        // SAFETY: the bindgen progress record contains only integer fields, so
        // all-zero is a valid initialized value.
        let mut raw_stats: ffi::git_indexer_progress = unsafe { core::mem::zeroed() };
        // SAFETY: `raw_stats` is initialized, live, and exclusively borrowed
        // for this handle's use below.
        let mut stats = unsafe { IndexerProgressMut::from_ptr(&raw mut raw_stats) }
            .expect("a stack address is non-null");

        assert!(writepack.as_ref().backend().is_none());
        assert_eq!(writepack.as_mut().append(b"pack", &mut stats), Ok(()));
        assert_eq!(stats.as_ref().received_bytes(), 4);
        assert_eq!(writepack.as_mut().commit(&mut stats), Ok(()));
        assert_eq!(APPENDED.load(Ordering::SeqCst), 4);
        assert_eq!(COMMITS.load(Ordering::SeqCst), 1);

        drop(writepack);
        assert_eq!(FREES.load(Ordering::SeqCst), 1);
    }
}

/// Wraps: git_odb_backend_loose_flag_t
/// A checked set of flags configuring a loose-object backend.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitOdbBackendLooseFlags(ffi::git_odb_backend_loose_flag_t);

impl GitOdbBackendLooseFlags {
    /// Use the backend's default durability behavior.
    pub const NONE: Self = Self(0);
    /// Flush loose-object data to durable storage when writing.
    pub const FSYNC: Self = Self(ffi::git_odb_backend_loose_flag_t_GIT_ODB_BACKEND_LOOSE_FSYNC);
    /// Every loose-backend flag published by this libgit2 version.
    pub const ALL: Self = Self(Self::FSYNC.0);

    /// Converts raw bits when every bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_odb_backend_loose_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_odb_backend_loose_flag_t {
        self.0
    }

    /// Returns whether no optional behavior is selected.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitOdbBackendLooseFlags> for ffi::git_odb_backend_loose_flag_t {
    fn from(flags: GitOdbBackendLooseFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_odb_backend_loose_flag_t> for GitOdbBackendLooseFlags {
    type Error = ffi::git_odb_backend_loose_flag_t;

    fn try_from(bits: ffi::git_odb_backend_loose_flag_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitOdbBackendLooseFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitOdbBackendLooseFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitOdbBackendLooseFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitOdbBackendLooseFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitOdbBackendLooseFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod loose_flag_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_loose_backend_flags_validate_and_convert() {
        assert_eq!(
            GitOdbBackendLooseFlags::from_bits(0),
            Some(GitOdbBackendLooseFlags::NONE)
        );
        assert!(GitOdbBackendLooseFlags::NONE.is_empty());
        assert!(GitOdbBackendLooseFlags::FSYNC.contains(GitOdbBackendLooseFlags::FSYNC));
        assert!(GitOdbBackendLooseFlags::FSYNC.intersects(GitOdbBackendLooseFlags::FSYNC));
        assert_eq!(
            ffi::git_odb_backend_loose_flag_t::from(GitOdbBackendLooseFlags::FSYNC),
            GitOdbBackendLooseFlags::FSYNC.bits()
        );
    }

    #[test]
    fn unknown_loose_backend_flags_are_rejected() {
        let unknown = GitOdbBackendLooseFlags::ALL.bits() << 1;
        assert_eq!(GitOdbBackendLooseFlags::from_bits(unknown), None);
        assert_eq!(GitOdbBackendLooseFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn loose_backend_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitOdbBackendLooseFlags>(),
            size_of::<ffi::git_odb_backend_loose_flag_t>()
        );
        assert_eq!(
            align_of::<GitOdbBackendLooseFlags>(),
            align_of::<ffi::git_odb_backend_loose_flag_t>()
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_odb_backend_loose_options
    /// Layout-compatible configuration for a loose-object backend.
    GitOdbBackendLooseOptions,
    GitOdbBackendLooseOptionsRef,
    GitOdbBackendLooseOptionsMut,
    ffi::git_odb_backend_loose_options
);

// SAFETY: this options value contains only scalar configuration and owns no
// resource, so disposing inline storage requires no action.
unsafe impl CValued for GitOdbBackendLooseOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitOdbBackendLooseOptions {
    /// Constructs options equivalent to `GIT_ODB_BACKEND_LOOSE_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_ODB_BACKEND_LOOSE_OPTIONS_VERSION);
            view.set_compression_level(-1);
        }
        options
    }
}

impl GitOdbBackendLooseOptionsRef<'_> {
    /// Field: git_odb_backend_loose_options.flags
    /// Returns the checked loose-backend option flags.
    pub fn flags(&self) -> Result<GitOdbBackendLooseFlags, ffi::git_odb_backend_loose_flag_t> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let bits = unsafe { core::ptr::addr_of!((*self.as_ptr()).flags).read() };
        GitOdbBackendLooseFlags::try_from(bits)
    }

    /// Field: git_odb_backend_loose_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_odb_backend_loose_options.oid_type
    /// Returns the selected object-ID format, or `None` for libgit2's default.
    pub fn oid_type(&self) -> Result<Option<OidType>, InvalidOidType> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let raw = unsafe { core::ptr::addr_of!((*self.as_ptr()).oid_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            OidType::try_from(raw).map(Some)
        }
    }

    /// Field: git_odb_backend_loose_options.file_mode
    /// Returns the requested file permissions, or zero for defaults.
    #[must_use]
    pub fn file_mode(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).file_mode).read() }
    }

    /// Field: git_odb_backend_loose_options.dir_mode
    /// Returns the requested directory permissions, or zero for defaults.
    #[must_use]
    pub fn dir_mode(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).dir_mode).read() }
    }

    /// Field: git_odb_backend_loose_options.compression_level
    /// Returns the zlib compression level, with `-1` selecting the default.
    #[must_use]
    pub fn compression_level(&self) -> core::ffi::c_int {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).compression_level).read() }
    }
}

impl GitOdbBackendLooseOptionsMut<'_> {
    /// Replaces the loose-backend option flags.
    pub fn set_flags(&mut self, flags: GitOdbBackendLooseFlags) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects an object-ID format, or the libgit2 default with `None`.
    pub fn set_oid_type(&mut self, oid_type: Option<OidType>) {
        let raw = oid_type.map_or(0, Into::into);
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).oid_type).write(raw) }
    }

    /// Selects file permissions, or backend defaults with zero.
    pub fn set_file_mode(&mut self, mode: core::ffi::c_uint) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).file_mode).write(mode) }
    }

    /// Selects directory permissions, or backend defaults with zero.
    pub fn set_dir_mode(&mut self, mode: core::ffi::c_uint) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).dir_mode).write(mode) }
    }

    /// Selects the zlib compression level (`-1` for the backend default).
    pub fn set_compression_level(&mut self, level: core::ffi::c_int) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).compression_level).write(level) }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_odb_backend_pack_options
    /// Layout-compatible configuration for packfile object backends.
    GitOdbBackendPackOptions,
    GitOdbBackendPackOptionsRef,
    GitOdbBackendPackOptionsMut,
    ffi::git_odb_backend_pack_options
);

// SAFETY: this options value contains only scalar configuration and owns no
// resource, so disposing inline storage requires no action.
unsafe impl CValued for GitOdbBackendPackOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitOdbBackendPackOptions {
    /// Constructs options equivalent to `GIT_ODB_BACKEND_PACK_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        options
            .as_mut()
            .set_version(ffi::GIT_ODB_BACKEND_PACK_OPTIONS_VERSION);
        options
    }
}

impl GitOdbBackendPackOptionsRef<'_> {
    /// Field: git_odb_backend_pack_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_odb_backend_pack_options.oid_type
    /// Returns the selected object-ID format, or `None` for libgit2's default.
    pub fn oid_type(&self) -> Result<Option<OidType>, InvalidOidType> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let raw = unsafe { core::ptr::addr_of!((*self.as_ptr()).oid_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            OidType::try_from(raw).map(Some)
        }
    }
}

impl GitOdbBackendPackOptionsMut<'_> {
    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects an object-ID format, or the libgit2 default with `None`.
    pub fn set_oid_type(&mut self, oid_type: Option<OidType>) {
        let raw = oid_type.map_or(0, Into::into);
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).oid_type).write(raw) }
    }
}

#[cfg(test)]
mod backend_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn loose_options_preserve_layout_defaults_and_mutation() {
        assert_eq!(
            size_of::<GitOdbBackendLooseOptions>(),
            size_of::<ffi::git_odb_backend_loose_options>()
        );
        assert_eq!(
            align_of::<GitOdbBackendLooseOptions>(),
            align_of::<ffi::git_odb_backend_loose_options>()
        );
        assert_eq!(
            size_of::<GitOdbBackendLooseOptionsRef<'_>>(),
            size_of::<*const ffi::git_odb_backend_loose_options>()
        );

        let mut options = GitOdbBackendLooseOptions::new();
        let mut view = options.as_mut();
        assert_eq!(
            view.as_ref().version(),
            ffi::GIT_ODB_BACKEND_LOOSE_OPTIONS_VERSION
        );
        assert_eq!(view.as_ref().flags(), Ok(GitOdbBackendLooseFlags::NONE));
        assert_eq!(view.as_ref().compression_level(), -1);
        assert_eq!(view.as_ref().dir_mode(), 0);
        assert_eq!(view.as_ref().file_mode(), 0);
        assert_eq!(view.as_ref().oid_type(), Ok(None));

        view.set_flags(GitOdbBackendLooseFlags::FSYNC);
        view.set_compression_level(9);
        view.set_dir_mode(0o755);
        view.set_file_mode(0o644);
        view.set_oid_type(Some(OidType::Sha256));

        assert_eq!(view.as_ref().flags(), Ok(GitOdbBackendLooseFlags::FSYNC));
        assert_eq!(view.as_ref().compression_level(), 9);
        assert_eq!(view.as_ref().dir_mode(), 0o755);
        assert_eq!(view.as_ref().file_mode(), 0o644);
        assert_eq!(view.as_ref().oid_type(), Ok(Some(OidType::Sha256)));
    }

    #[test]
    fn pack_options_preserve_layout_defaults_and_mutation() {
        assert_eq!(
            size_of::<GitOdbBackendPackOptions>(),
            size_of::<ffi::git_odb_backend_pack_options>()
        );
        assert_eq!(
            align_of::<GitOdbBackendPackOptions>(),
            align_of::<ffi::git_odb_backend_pack_options>()
        );

        let mut options = GitOdbBackendPackOptions::new();
        let mut view = options.as_mut();
        assert_eq!(
            view.as_ref().version(),
            ffi::GIT_ODB_BACKEND_PACK_OPTIONS_VERSION
        );
        assert_eq!(view.as_ref().oid_type(), Ok(None));
        view.set_oid_type(Some(OidType::Sha1));
        assert_eq!(view.as_ref().oid_type(), Ok(Some(OidType::Sha1)));
    }
}
