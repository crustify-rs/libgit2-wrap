//! Safe wrappers for libgit2 odb_backend APIs.

use core::ptr::NonNull;

use crate::ffi;
use crate::oid::{InvalidOidType, OidRef, OidType};

ffibox::define_ctype!(
    /// Wraps: git_odb_stream
    /// A layout-compatible polymorphic stream supplied by an object-database
    /// backend.
    GitOdbStream,
    GitOdbStreamRef,
    GitOdbStreamMut,
    ffi::git_odb_stream
);

/// An exclusively owned ODB stream. Dropping it cleans its frontend hash
/// context and invokes the concrete backend stream's destructor.
pub type GitOdbStreamOwned = ffibox::CBox<GitOdbStream>;

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
}

impl GitOdbStreamRef<'_> {
    /// Wraps: git_odb_stream.mode
    /// Returns the stream's published capability bits, or `None` if C stored
    /// an unknown or empty mode.
    #[must_use]
    pub fn mode(&self) -> Option<GitOdbStreamMode> {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        let bits = unsafe { core::ptr::addr_of!((*stream).mode).read() };
        GitOdbStreamMode::from_bits(bits)
    }

    /// Wraps: git_odb_stream.oid_type
    /// Returns the object-ID algorithm after validating the C value.
    pub fn oid_type(&self) -> Result<OidType, InvalidOidType> {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        let raw = unsafe { core::ptr::addr_of!((*stream).oid_type).read() };
        OidType::try_from(raw)
    }

    /// Wraps: git_odb_stream.backend
    /// Returns whether the stream has its required borrowed backend link.
    ///
    /// The backend type has not yet been wrapped, so this intentionally does
    /// not expose the temporary raw dependency.
    #[must_use]
    pub fn has_backend(&self) -> bool {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized pointer without dereferencing it.
        !unsafe { core::ptr::addr_of!((*stream).backend).read() }.is_null()
    }

    /// Wraps: git_odb_stream.received_bytes
    /// Returns the number of bytes accepted by this stream so far.
    #[must_use]
    pub fn received_bytes(&self) -> ffi::git_object_size_t {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*stream).received_bytes).read() }
    }

    /// Wraps: git_odb_stream.hash_ctx
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

    /// Wraps: git_odb_stream.declared_size
    /// Returns the byte count promised when the write stream was opened.
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
    pub unsafe fn set_backend(&mut self, backend: NonNull<ffi::git_odb_backend>) {
        let stream = self.as_mut_ptr();
        // SAFETY: the caller supplies the unexpressible backend lifetime and
        // identity guarantees; this exclusive handle permits the field write.
        unsafe { core::ptr::addr_of_mut!((*stream).backend).write(backend.as_ptr()) }
    }

    /// Wraps: git_odb_stream.write
    /// Dispatches the concrete stream's synchronous write callback.
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

    /// Wraps: git_odb_stream.read
    /// Dispatches the concrete stream's synchronous read callback.
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

    /// Wraps: git_odb_stream.finalize_write
    /// Dispatches the concrete stream's synchronous finalization callback.
    pub fn finalize_write(&mut self, oid: OidRef<'_>) -> Result<(), GitOdbStreamError> {
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

/// Wraps: git_odb_stream.free
// SAFETY: adopting `GitOdbStreamOwned` requires a fully constructed stream
// allocation with its concrete destructor installed. The public free routine
// first disposes the stream-owned hash context and then invokes that destructor
// exactly once; `CBox` never accesses the allocation afterward.
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

    unsafe extern "C" fn test_free(stream: *mut ffi::git_odb_stream) {
        FREED.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the test adopts exactly one allocation produced by
        // `Box::into_raw`, and the owned wrapper invokes this callback once.
        unsafe { drop(Box::from_raw(stream)) }
    }

    fn test_stream() -> ffi::git_odb_stream {
        ffi::git_odb_stream {
            backend: NonNull::<ffi::git_odb_backend>::dangling().as_ptr(),
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
        assert!(stream.as_ref().has_backend());
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
