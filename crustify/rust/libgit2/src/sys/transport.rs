//! Safe wrappers for libgit2 transport APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CDropped};

use crate::ffi;
use crate::remote::GitRemoteMut;

/// Wraps: git_smart_service_t
/// An operation requested from a smart subtransport.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitSmartService {
    /// Discover references available from `git-upload-pack`.
    UploadPackLs = ffi::git_smart_service_t_GIT_SERVICE_UPLOADPACK_LS,
    /// Exchange data with `git-upload-pack`.
    UploadPack = ffi::git_smart_service_t_GIT_SERVICE_UPLOADPACK,
    /// Discover references available from `git-receive-pack`.
    ReceivePackLs = ffi::git_smart_service_t_GIT_SERVICE_RECEIVEPACK_LS,
    /// Exchange data with `git-receive-pack`.
    ReceivePack = ffi::git_smart_service_t_GIT_SERVICE_RECEIVEPACK,
}

/// An integer that is not a published [`GitSmartService`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitSmartService(ffi::git_smart_service_t);

impl InvalidGitSmartService {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_smart_service_t {
        self.0
    }
}

impl From<GitSmartService> for ffi::git_smart_service_t {
    fn from(service: GitSmartService) -> Self {
        service as Self
    }
}

impl TryFrom<ffi::git_smart_service_t> for GitSmartService {
    type Error = InvalidGitSmartService;

    fn try_from(service: ffi::git_smart_service_t) -> Result<Self, Self::Error> {
        match service {
            ffi::git_smart_service_t_GIT_SERVICE_UPLOADPACK_LS => Ok(Self::UploadPackLs),
            ffi::git_smart_service_t_GIT_SERVICE_UPLOADPACK => Ok(Self::UploadPack),
            ffi::git_smart_service_t_GIT_SERVICE_RECEIVEPACK_LS => Ok(Self::ReceivePackLs),
            ffi::git_smart_service_t_GIT_SERVICE_RECEIVEPACK => Ok(Self::ReceivePack),
            value => Err(InvalidGitSmartService(value)),
        }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_smart_subtransport
    /// A polymorphic transport that creates streams for Git protocol actions.
    GitSmartSubtransport,
    GitSmartSubtransportRef,
    GitSmartSubtransportMut,
    ffi::git_smart_subtransport
);

/// An exclusively owned smart subtransport.
pub type GitSmartSubtransportOwned = CBox<GitSmartSubtransport>;

/// An owned subtransport that cannot outlive the smart transport it stores.
pub struct GitSmartSubtransportWithTransport<'transport> {
    inner: GitSmartSubtransportOwned,
    _transport: core::marker::PhantomData<GitTransportMut<'transport>>,
}

impl<'transport> GitSmartSubtransportWithTransport<'transport> {
    /// Couples a newly owned subtransport to its owner transport.
    #[must_use]
    pub fn from_owned(
        inner: GitSmartSubtransportOwned,
        owner: GitTransportMut<'transport>,
    ) -> Self {
        let _ = owner;
        Self {
            inner,
            _transport: core::marker::PhantomData,
        }
    }

    /// Borrows the subtransport shared.
    #[must_use]
    pub fn as_ref(&self) -> GitSmartSubtransportRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the subtransport exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitSmartSubtransportMut<'_> {
        self.inner.as_mut()
    }

    pub(crate) fn into_raw(self) -> *mut ffi::git_smart_subtransport {
        self.inner.into_raw()
    }
}

/// Wraps: git_smart_subtransport_cb
/// Safe callable surface for a factory that transfers one new subtransport
/// to its owner transport.
pub trait GitSmartSubtransportCallback {
    /// Creates a subtransport whose lifetime remains coupled to `owner`.
    fn call<'transport>(
        &mut self,
        owner: GitTransportMut<'transport>,
    ) -> Result<GitSmartSubtransportWithTransport<'transport>, i32>;
}

impl<F> GitSmartSubtransportCallback for F
where
    F: for<'transport> FnMut(
        GitTransportMut<'transport>,
    ) -> Result<GitSmartSubtransportWithTransport<'transport>, i32>,
{
    fn call<'transport>(
        &mut self,
        owner: GitTransportMut<'transport>,
    ) -> Result<GitSmartSubtransportWithTransport<'transport>, i32> {
        self(owner)
    }
}

/// Failure returned while dispatching a smart-subtransport callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitSmartSubtransportError {
    /// The concrete callback returned a libgit2 error code.
    Libgit2(core::ffi::c_int),
    /// A successful action did not publish its required stream.
    MissingStream,
}

impl GitSmartSubtransportMut<'_> {
    /// Field: git_smart_subtransport.action
    /// Dispatches an action and borrows the stream it publishes.
    ///
    /// This method deliberately does not adopt the returned allocation:
    /// stateless subtransports return a newly owned stream, while stateful
    /// continuations may return the stream from the preceding action. The
    /// borrowed result safely represents both contracts without manufacturing
    /// a second owner. Its borrow also prevents another Rust action or close
    /// call through this handle while the stream is in use.
    pub fn action<'a>(
        &'a mut self,
        url: &core::ffi::CStr,
        service: GitSmartService,
    ) -> Result<GitSmartSubtransportStreamMut<'a>, GitSmartSubtransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: this exclusive handle addresses a live, fully constructed
        // subtransport; raw-place projection reads its initialized callback
        // without forming a reference to C-visible storage.
        let action = unsafe { core::ptr::addr_of!((*transport).action).read() }
            .expect("a valid smart subtransport has an action callback");
        let mut stream = core::ptr::null_mut();
        // SAFETY: `action` is installed for this live subtransport; `url` is a
        // readable NUL-terminated string, the output slot is writable, and the
        // callback retains neither of those two pointers.
        let result = unsafe { action(&raw mut stream, transport, url.as_ptr(), service.into()) };
        if result < 0 {
            return Err(GitSmartSubtransportError::Libgit2(result));
        }
        // SAFETY: a successful action publishes a live stream associated with
        // `transport`. The returned exclusive handle is bounded by this
        // subtransport's exclusive reborrow, preventing another safe callback
        // dispatch until it expires. Ownership is intentionally not adopted.
        unsafe { GitSmartSubtransportStreamMut::from_ptr(stream) }
            .ok_or(GitSmartSubtransportError::MissingStream)
    }

    /// Field: git_smart_subtransport.close
    /// Closes the current subtransport session.
    pub fn close(&mut self) -> Result<(), GitSmartSubtransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: this exclusive handle addresses a live, fully constructed
        // subtransport; raw-place projection reads its initialized callback
        // without forming a reference to C-visible storage.
        let close = unsafe { core::ptr::addr_of!((*transport).close).read() }
            .expect("a valid smart subtransport has a close callback");
        // SAFETY: `close` is installed for this live subtransport and the
        // exclusive handle prevents another Rust callback invocation during
        // this synchronous call.
        let result = unsafe { close(transport) };
        if result < 0 {
            Err(GitSmartSubtransportError::Libgit2(result))
        } else {
            Ok(())
        }
    }
}

/// Field: git_smart_subtransport.free
// SAFETY: adopting `GitSmartSubtransportOwned` requires a fully constructed
// concrete subtransport allocation with its required destructor callback
// installed. `CBox` invokes that callback exactly once and never touches the
// allocation afterward.
unsafe impl CDropped for GitSmartSubtransport {
    unsafe fn c_drop(transport: NonNull<Self>) {
        let transport = transport.as_ptr().cast::<ffi::git_smart_subtransport>();
        // SAFETY: the `CDropped` contract supplies a live, fully constructed
        // subtransport; raw-place projection reads its callback without
        // forming a reference to C-visible storage.
        let free = unsafe { core::ptr::addr_of!((*transport).free).read() }
            .expect("a valid smart subtransport has a free callback");
        // SAFETY: `free` is the concrete destructor installed for this owned
        // subtransport and the `CDropped` contract grants its final call.
        unsafe { free(transport) }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_smart_subtransport_stream
    /// A polymorphic stream supplied by a smart subtransport.
    ///
    /// The stream borrows its originating subtransport and dispatches I/O
    /// synchronously through callbacks installed by its concrete
    /// implementation. Owned stream allocations use
    /// [`GitSmartSubtransportStreamOwned`].
    GitSmartSubtransportStream,
    GitSmartSubtransportStreamRef,
    GitSmartSubtransportStreamMut,
    ffi::git_smart_subtransport_stream
);

/// An exclusively owned smart-subtransport stream.
pub type GitSmartSubtransportStreamOwned = CBox<GitSmartSubtransportStream>;

/// Failure returned while dispatching a smart-subtransport stream callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitSmartSubtransportStreamError {
    /// The concrete callback returned a libgit2 error code.
    Libgit2(core::ffi::c_int),
    /// A read callback reported more initialized bytes than the destination
    /// can hold.
    InvalidReadCount(usize),
}

impl<'a> GitSmartSubtransportStreamRef<'a> {
    /// Field: git_smart_subtransport_stream.subtransport
    /// Borrows the subtransport that owns this stream's session.
    #[must_use]
    pub fn subtransport(&self) -> GitSmartSubtransportRef<'a> {
        let stream = self.as_ptr();
        // SAFETY: `stream` comes from this live shared handle; raw-place
        // projection reads the initialized pointer without dereferencing it.
        let subtransport = unsafe { core::ptr::addr_of!((*stream).subtransport).read() };
        // SAFETY: every fully constructed stream stores its non-null
        // originating subtransport, which remains live until after the stream
        // is destroyed. The shared view cannot dispatch mutating callbacks.
        unsafe { GitSmartSubtransportRef::from_ptr(subtransport) }
            .expect("a valid smart-subtransport stream has an owner")
    }
}

impl GitSmartSubtransportStreamMut<'_> {
    /// Stores the originating subtransport during concrete-stream
    /// initialization.
    ///
    /// # Safety
    ///
    /// `subtransport` must designate the subtransport that installed this
    /// stream's callbacks. It must remain live at a stable address until the
    /// stream is destroyed and must not be mutably accessed concurrently.
    pub unsafe fn set_subtransport(&mut self, mut subtransport: GitSmartSubtransportMut<'_>) {
        let stream = self.as_mut_ptr();
        // SAFETY: the caller supplies the unexpressible referent lifetime and
        // identity guarantees; this exclusive handle permits the field write.
        unsafe {
            core::ptr::addr_of_mut!((*stream).subtransport).write(subtransport.as_mut_ptr());
        }
    }

    /// Field: git_smart_subtransport_stream.read
    /// Reads available bytes synchronously into `buffer`.
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, GitSmartSubtransportStreamError> {
        let stream = self.as_mut_ptr();
        // SAFETY: this exclusive handle addresses a live, fully constructed
        // stream; raw-place projection reads its initialized callback slot
        // without forming a reference to C-visible storage.
        let read = unsafe { core::ptr::addr_of!((*stream).read).read() }
            .expect("a valid smart-subtransport stream has a read callback");
        let mut bytes_read = 0;
        // SAFETY: `read` is the callback installed for this live stream. The
        // slice supplies exactly `buffer.len()` writable bytes and
        // `bytes_read` is a writable scalar output slot; this exclusive handle
        // prevents another Rust invocation during the synchronous call.
        let result = unsafe {
            read(
                stream,
                buffer.as_mut_ptr().cast::<core::ffi::c_char>(),
                buffer.len(),
                &raw mut bytes_read,
            )
        };
        if result < 0 {
            return Err(GitSmartSubtransportStreamError::Libgit2(result));
        }
        if bytes_read > buffer.len() {
            Err(GitSmartSubtransportStreamError::InvalidReadCount(
                bytes_read,
            ))
        } else {
            Ok(bytes_read)
        }
    }

    /// Field: git_smart_subtransport_stream.write
    /// Writes all bytes in `buffer` synchronously.
    pub fn write(&mut self, buffer: &[u8]) -> Result<(), GitSmartSubtransportStreamError> {
        let stream = self.as_mut_ptr();
        // SAFETY: this exclusive handle addresses a live, fully constructed
        // stream; raw-place projection reads its initialized callback slot
        // without forming a reference to C-visible storage.
        let write = unsafe { core::ptr::addr_of!((*stream).write).read() }
            .expect("a valid smart-subtransport stream has a write callback");
        // SAFETY: `write` is the callback installed for this live stream. The
        // slice supplies exactly `buffer.len()` readable bytes, and this
        // exclusive handle prevents another Rust invocation during the
        // synchronous call.
        let result = unsafe {
            write(
                stream,
                buffer.as_ptr().cast::<core::ffi::c_char>(),
                buffer.len(),
            )
        };
        if result < 0 {
            Err(GitSmartSubtransportStreamError::Libgit2(result))
        } else {
            Ok(())
        }
    }
}

/// Field: git_smart_subtransport_stream.free
// SAFETY: adopting `GitSmartSubtransportStreamOwned` requires a fully
// constructed concrete stream allocation with its required destructor
// callback installed. `CBox` invokes that callback exactly once and never
// touches the allocation afterward.
unsafe impl CDropped for GitSmartSubtransportStream {
    unsafe fn c_drop(stream: NonNull<Self>) {
        let stream = stream.as_ptr().cast::<ffi::git_smart_subtransport_stream>();
        // SAFETY: the `CDropped` contract supplies a live, fully constructed
        // stream; raw-place projection reads its callback without forming a
        // reference to C-visible storage.
        let free = unsafe { core::ptr::addr_of!((*stream).free).read() }
            .expect("a valid smart-subtransport stream has a free callback");
        // SAFETY: `free` is the concrete destructor installed for this owned
        // stream and the `CDropped` contract grants its one final invocation.
        unsafe { free(stream) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use ffibox::CCell;

    use super::*;

    static WRITTEN_LEN: AtomicUsize = AtomicUsize::new(0);
    static FIRST_BYTE: AtomicUsize = AtomicUsize::new(0);
    static FREES: AtomicUsize = AtomicUsize::new(0);
    static ACTIONS: AtomicUsize = AtomicUsize::new(0);
    static CLOSES: AtomicUsize = AtomicUsize::new(0);
    static SUBTRANSPORT_FREES: AtomicUsize = AtomicUsize::new(0);
    static LAST_SERVICE: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn test_read(
        _stream: *mut ffi::git_smart_subtransport_stream,
        buffer: *mut core::ffi::c_char,
        size: usize,
        bytes_read: *mut usize,
    ) -> core::ffi::c_int {
        let count = size.min(3);
        // SAFETY: the callback contract supplies `size` writable bytes and a
        // valid scalar output slot; `count <= size`, and the source has three
        // bytes.
        unsafe {
            core::ptr::copy_nonoverlapping(b"abc".as_ptr(), buffer.cast(), count);
            bytes_read.write(count);
        }
        0
    }

    unsafe extern "C" fn invalid_read_count(
        _stream: *mut ffi::git_smart_subtransport_stream,
        _buffer: *mut core::ffi::c_char,
        size: usize,
        bytes_read: *mut usize,
    ) -> core::ffi::c_int {
        // SAFETY: the callback contract supplies a valid scalar output slot;
        // this deliberately invalid count exercises wrapper validation.
        unsafe { bytes_read.write(size.saturating_add(1)) };
        0
    }

    unsafe extern "C" fn test_write(
        _stream: *mut ffi::git_smart_subtransport_stream,
        buffer: *const core::ffi::c_char,
        size: usize,
    ) -> core::ffi::c_int {
        WRITTEN_LEN.store(size, Ordering::SeqCst);
        if size != 0 {
            // SAFETY: the callback contract supplies `size` readable bytes and
            // this branch establishes that the first byte exists.
            FIRST_BYTE.store(
                unsafe { buffer.cast::<u8>().read() } as usize,
                Ordering::SeqCst,
            );
        }
        0
    }

    unsafe extern "C" fn test_free(stream: *mut ffi::git_smart_subtransport_stream) {
        FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the lifecycle test transfers one `Box` allocation to the
        // owning stream and this callback is its only final release.
        drop(unsafe { Box::from_raw(stream) });
    }

    unsafe extern "C" fn no_free(_stream: *mut ffi::git_smart_subtransport_stream) {}

    unsafe extern "C" fn action_stream_free(stream: *mut ffi::git_smart_subtransport_stream) {
        // SAFETY: `test_action` transfers one fresh `Box` allocation and this
        // callback performs its only final release.
        drop(unsafe { Box::from_raw(stream) });
    }

    unsafe extern "C" fn test_action(
        out: *mut *mut ffi::git_smart_subtransport_stream,
        transport: *mut ffi::git_smart_subtransport,
        url: *const core::ffi::c_char,
        service: ffi::git_smart_service_t,
    ) -> core::ffi::c_int {
        // SAFETY: the callback contract supplies a readable NUL-terminated
        // URL for the duration of this call.
        assert_eq!(unsafe { core::ffi::CStr::from_ptr(url) }, c"git://example");
        ACTIONS.fetch_add(1, Ordering::SeqCst);
        LAST_SERVICE.store(service as usize, Ordering::SeqCst);
        let stream = Box::into_raw(Box::new(ffi::git_smart_subtransport_stream {
            subtransport: transport,
            read: Some(test_read),
            write: Some(test_write),
            free: Some(action_stream_free),
        }));
        // SAFETY: the callback contract supplies a writable output slot and
        // ownership of this new stream allocation passes to its caller.
        unsafe { out.write(stream) };
        0
    }

    unsafe extern "C" fn missing_stream_action(
        out: *mut *mut ffi::git_smart_subtransport_stream,
        _transport: *mut ffi::git_smart_subtransport,
        _url: *const core::ffi::c_char,
        _service: ffi::git_smart_service_t,
    ) -> core::ffi::c_int {
        // SAFETY: the callback contract supplies a writable output slot; null
        // deliberately violates the success contract for wrapper validation.
        unsafe { out.write(core::ptr::null_mut()) };
        0
    }

    unsafe extern "C" fn test_close(
        _transport: *mut ffi::git_smart_subtransport,
    ) -> core::ffi::c_int {
        CLOSES.fetch_add(1, Ordering::SeqCst);
        0
    }

    unsafe extern "C" fn no_subtransport_free(_transport: *mut ffi::git_smart_subtransport) {}

    unsafe extern "C" fn test_subtransport_free(transport: *mut ffi::git_smart_subtransport) {
        SUBTRANSPORT_FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the lifecycle test transfers one `Box` allocation to the
        // owning subtransport and this callback is its only final release.
        drop(unsafe { Box::from_raw(transport) });
    }

    fn raw_subtransport(
        action: unsafe extern "C" fn(
            *mut *mut ffi::git_smart_subtransport_stream,
            *mut ffi::git_smart_subtransport,
            *const core::ffi::c_char,
            ffi::git_smart_service_t,
        ) -> core::ffi::c_int,
    ) -> ffi::git_smart_subtransport {
        ffi::git_smart_subtransport {
            action: Some(action),
            close: Some(test_close),
            free: Some(no_subtransport_free),
        }
    }

    fn raw_stream(
        read: unsafe extern "C" fn(
            *mut ffi::git_smart_subtransport_stream,
            *mut core::ffi::c_char,
            usize,
            *mut usize,
        ) -> core::ffi::c_int,
    ) -> ffi::git_smart_subtransport_stream {
        ffi::git_smart_subtransport_stream {
            subtransport: core::ptr::null_mut(),
            read: Some(read),
            write: Some(test_write),
            free: Some(no_free),
        }
    }

    #[test]
    fn smart_services_round_trip_and_reject_unknown_values() {
        let services = [
            GitSmartService::UploadPackLs,
            GitSmartService::UploadPack,
            GitSmartService::ReceivePackLs,
            GitSmartService::ReceivePack,
        ];
        for service in services {
            let raw = ffi::git_smart_service_t::from(service);
            assert_eq!(GitSmartService::try_from(raw), Ok(service));
        }

        let invalid = ffi::git_smart_service_t_GIT_SERVICE_RECEIVEPACK + 1;
        assert_eq!(
            GitSmartService::try_from(invalid).unwrap_err().value(),
            invalid
        );
        assert_eq!(
            size_of::<GitSmartService>(),
            size_of::<ffi::git_smart_service_t>()
        );
        assert_eq!(
            align_of::<GitSmartService>(),
            align_of::<ffi::git_smart_service_t>()
        );
    }

    #[test]
    fn stream_wrapper_matches_the_c_layout() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitSmartSubtransportStream>();
        assert_dropped::<GitSmartSubtransportStream>();
        assert_eq!(
            size_of::<GitSmartSubtransportStream>(),
            size_of::<ffi::git_smart_subtransport_stream>()
        );
        assert_eq!(
            align_of::<GitSmartSubtransportStream>(),
            align_of::<ffi::git_smart_subtransport_stream>()
        );
        assert_eq!(
            size_of::<GitSmartSubtransportStreamRef<'_>>(),
            size_of::<*const ffi::git_smart_subtransport_stream>()
        );
        assert_eq!(
            size_of::<GitSmartSubtransportStreamMut<'_>>(),
            size_of::<*mut ffi::git_smart_subtransport_stream>()
        );
        assert_eq!(
            size_of::<Option<GitSmartSubtransportStreamOwned>>(),
            size_of::<*mut ffi::git_smart_subtransport_stream>()
        );
    }

    #[test]
    fn stream_dispatches_read_and_write_callbacks() {
        let mut raw = raw_stream(test_read);
        // SAFETY: `raw` remains live and this handle is its only borrow for the
        // scope; all callback slots are initialized with compatible functions.
        let mut stream = unsafe { GitSmartSubtransportStreamMut::from_ptr(&raw mut raw) }.unwrap();

        let mut buffer = [0; 8];
        assert_eq!(stream.read(&mut buffer), Ok(3));
        assert_eq!(&buffer[..3], b"abc");
        assert_eq!(stream.write(b"xyz"), Ok(()));
        assert_eq!(WRITTEN_LEN.load(Ordering::SeqCst), 3);
        assert_eq!(FIRST_BYTE.load(Ordering::SeqCst), b'x' as usize);
    }

    #[test]
    fn stream_rejects_an_invalid_read_count() {
        let mut raw = raw_stream(invalid_read_count);
        // SAFETY: `raw` remains live and this handle is its only borrow for the
        // scope; all callback slots are initialized with compatible functions.
        let mut stream = unsafe { GitSmartSubtransportStreamMut::from_ptr(&raw mut raw) }.unwrap();
        let mut buffer = [0; 4];
        assert_eq!(
            stream.read(&mut buffer),
            Err(GitSmartSubtransportStreamError::InvalidReadCount(5))
        );
    }

    #[test]
    fn subtransport_link_can_be_initialized_without_becoming_publicly_raw() {
        let mut raw = raw_stream(test_read);
        let mut subtransport = raw_subtransport(test_action);
        // SAFETY: `raw` remains live and is exclusively borrowed for this
        // scope; `subtransport` also remains live at a stable address.
        let mut stream = unsafe { GitSmartSubtransportStreamMut::from_ptr(&raw mut raw) }.unwrap();
        // SAFETY: `subtransport` remains live and this is its only active
        // handle while it is transferred into the stream's lifetime contract.
        let subtransport =
            unsafe { GitSmartSubtransportMut::from_ptr(&raw mut subtransport) }.unwrap();
        // SAFETY: the referenced storage remains live and stable through the
        // stream handle, and no competing access occurs in this test.
        unsafe { stream.set_subtransport(subtransport) };
        let owner = stream.as_ref().subtransport();
        assert_eq!(owner.as_ptr(), raw.subtransport.cast_const());
    }

    #[test]
    fn owned_stream_invokes_its_concrete_destructor_once() {
        let before = FREES.load(Ordering::SeqCst);
        let raw = Box::into_raw(Box::new(ffi::git_smart_subtransport_stream {
            subtransport: core::ptr::null_mut(),
            read: Some(test_read),
            write: Some(test_write),
            free: Some(test_free),
        }));
        // SAFETY: `raw` is a unique, fully initialized heap allocation whose
        // installed callback reclaims that exact allocation.
        let stream = unsafe { GitSmartSubtransportStreamOwned::from_raw(raw) }.unwrap();
        drop(stream);
        assert_eq!(FREES.load(Ordering::SeqCst), before + 1);
    }

    #[test]
    fn subtransport_wrapper_matches_the_c_layout() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitSmartSubtransport>();
        assert_dropped::<GitSmartSubtransport>();
        assert_eq!(
            size_of::<GitSmartSubtransport>(),
            size_of::<ffi::git_smart_subtransport>()
        );
        assert_eq!(
            align_of::<GitSmartSubtransport>(),
            align_of::<ffi::git_smart_subtransport>()
        );
        assert_eq!(
            size_of::<GitSmartSubtransportRef<'_>>(),
            size_of::<*const ffi::git_smart_subtransport>()
        );
        assert_eq!(
            size_of::<GitSmartSubtransportMut<'_>>(),
            size_of::<*mut ffi::git_smart_subtransport>()
        );
        assert_eq!(
            size_of::<Option<GitSmartSubtransportOwned>>(),
            size_of::<*mut ffi::git_smart_subtransport>()
        );
    }

    #[test]
    fn subtransport_dispatches_action_and_close_callbacks() {
        let before_actions = ACTIONS.load(Ordering::SeqCst);
        let before_closes = CLOSES.load(Ordering::SeqCst);
        let mut raw = raw_subtransport(test_action);
        // SAFETY: `raw` remains live and this handle is its only access path
        // for the scope; all required callbacks are installed.
        let mut subtransport = unsafe { GitSmartSubtransportMut::from_ptr(&raw mut raw) }.unwrap();

        let stream_ptr = {
            let mut stream = subtransport
                .action(c"git://example", GitSmartService::UploadPackLs)
                .unwrap();
            assert_eq!(stream.write(b"request"), Ok(()));
            stream.as_mut_ptr()
        };
        // SAFETY: `test_action` returned a fresh allocation and no handle to it
        // remains; its installed destructor performs the one final release.
        unsafe { action_stream_free(stream_ptr) };

        assert_eq!(ACTIONS.load(Ordering::SeqCst), before_actions + 1);
        assert_eq!(
            LAST_SERVICE.load(Ordering::SeqCst),
            ffi::git_smart_service_t_GIT_SERVICE_UPLOADPACK_LS as usize
        );
        assert_eq!(subtransport.close(), Ok(()));
        assert_eq!(CLOSES.load(Ordering::SeqCst), before_closes + 1);
    }

    #[test]
    fn subtransport_rejects_a_missing_success_stream() {
        let mut raw = raw_subtransport(missing_stream_action);
        // SAFETY: `raw` remains live and exclusively borrowed, with compatible
        // callbacks installed; the action's bad output is validated safely.
        let mut subtransport = unsafe { GitSmartSubtransportMut::from_ptr(&raw mut raw) }.unwrap();
        assert!(matches!(
            subtransport.action(c"git://example", GitSmartService::UploadPackLs),
            Err(GitSmartSubtransportError::MissingStream)
        ));
    }

    #[test]
    fn owned_subtransport_invokes_its_concrete_destructor_once() {
        let before = SUBTRANSPORT_FREES.load(Ordering::SeqCst);
        let raw = Box::into_raw(Box::new(ffi::git_smart_subtransport {
            action: Some(test_action),
            close: Some(test_close),
            free: Some(test_subtransport_free),
        }));
        // SAFETY: `raw` is a unique, fully initialized heap allocation whose
        // installed callback reclaims that exact allocation.
        let subtransport = unsafe { GitSmartSubtransportOwned::from_raw(raw) }.unwrap();
        drop(subtransport);
        assert_eq!(SUBTRANSPORT_FREES.load(Ordering::SeqCst), before + 1);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_fetch_negotiation
    /// Layout-compatible state passed to a fetch-negotiation callback.
    GitFetchNegotiation,
    GitFetchNegotiationRef,
    GitFetchNegotiationMut,
    ffi::git_fetch_negotiation
);

/// A borrowed pointer array of advertised remote heads.
#[derive(Clone, Copy)]
pub struct GitFetchNegotiationRefs<'a> {
    ptr: *const *const ffi::git_remote_head,
    len: usize,
    _borrow: core::marker::PhantomData<crate::util::net::RemoteHeadRef<'a>>,
}

impl<'a> GitFetchNegotiationRefs<'a> {
    /// Returns the number of pointer slots.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether there are no pointer slots.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Borrows advertised head `index`, or `None` for an out-of-range or null pointer.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<crate::util::net::RemoteHeadRef<'a>> {
        if index >= self.len || self.ptr.is_null() {
            return None;
        }
        // SAFETY: `index < len`; a valid negotiation supplies that many
        // initialized pointer slots. Reading one forms no reference.
        let head = unsafe { self.ptr.add(index).read() };
        // SAFETY: a non-null advertised head remains live with the remote for
        // the lifetime of the negotiation view.
        unsafe { crate::util::net::RemoteHeadRef::from_ptr(head.cast_mut()) }
    }

    /// Iterates over the non-null advertised heads.
    pub fn iter(&self) -> impl Iterator<Item = crate::util::net::RemoteHeadRef<'a>> + use<'a> {
        let view = *self;
        (0..view.len).filter_map(move |index| view.get(index))
    }
}

impl<'a> GitFetchNegotiationRef<'a> {
    /// Field: git_fetch_negotiation.depth
    /// Returns the requested shallow-fetch depth, or zero for full history.
    #[must_use]
    pub fn depth(&self) -> i32 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).depth).read() }
    }

    /// Field: git_fetch_negotiation.refs_len
    /// Returns the number of advertised-head pointer slots.
    #[must_use]
    pub fn refs_len(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).refs_len).read() }
    }

    /// Field: git_fetch_negotiation.refs
    /// Borrows the counted advertised-head pointer array.
    #[must_use]
    pub fn refs(&self) -> GitFetchNegotiationRefs<'a> {
        let p = self.as_ptr();
        // SAFETY: both initialized fields are copied by raw-place projection.
        let (ptr, len) = unsafe {
            (
                core::ptr::addr_of!((*p).refs).read(),
                core::ptr::addr_of!((*p).refs_len).read(),
            )
        };
        GitFetchNegotiationRefs {
            ptr,
            len,
            _borrow: core::marker::PhantomData,
        }
    }

    /// Field: git_fetch_negotiation.shallow_roots_len
    /// Returns the number of shallow-root object IDs.
    #[must_use]
    pub fn shallow_roots_len(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).shallow_roots_len).read() }
    }

    /// Field: git_fetch_negotiation.shallow_roots
    /// Borrows the owned shallow-root run without exposing its pointer.
    #[must_use]
    pub fn shallow_roots(&self) -> Option<ffibox::CSlice<'a, crate::oid::Oid>> {
        let p = self.as_ptr();
        // SAFETY: both initialized fields are copied by raw-place projection.
        let (roots, len) = unsafe {
            (
                core::ptr::addr_of!((*p).shallow_roots).read(),
                core::ptr::addr_of!((*p).shallow_roots_len).read(),
            )
        };
        let roots = NonNull::new(roots.cast::<crate::oid::Oid>())?;
        // SAFETY: a valid non-null run contains `len` initialized OIDs and
        // remains live for this handle's lifetime.
        Some(unsafe { ffibox::CSlice::from_raw_parts(roots, len) })
    }
}

impl GitFetchNegotiationMut<'_> {
    /// Sets the requested shallow-fetch depth.
    pub fn set_depth(&mut self, depth: i32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).depth).write(depth) }
    }

    /// Stores a borrowed pointer array of advertised remote heads.
    ///
    /// # Safety
    /// The pointer array and every head must remain live and immutable until
    /// no C code can inspect this negotiation.
    pub unsafe fn set_refs<'a>(&mut self, refs: &'a [crate::util::net::RemoteHeadRef<'a>]) {
        let p = self.as_mut_ptr();
        // SAFETY: caller supplies the referent lifetime; transparent handles
        // have the same one-pointer representation as the C array elements.
        unsafe {
            core::ptr::addr_of_mut!((*p).refs).write(refs.as_ptr().cast());
            core::ptr::addr_of_mut!((*p).refs_len).write(refs.len());
        }
    }

    /// Borrows the shallow-root run exclusively.
    #[must_use]
    pub fn shallow_roots_mut(&mut self) -> Option<ffibox::CSliceMut<'_, crate::oid::Oid>> {
        let p = self.as_mut_ptr();
        // SAFETY: both initialized fields are copied while access is exclusive.
        let (roots, len) = unsafe {
            (
                core::ptr::addr_of!((*p).shallow_roots).read(),
                core::ptr::addr_of!((*p).shallow_roots_len).read(),
            )
        };
        let roots = NonNull::new(roots.cast::<crate::oid::Oid>())?;
        // SAFETY: this handle excludes other Rust views and the valid run has
        // `len` initialized elements.
        Some(unsafe { ffibox::CSliceMut::from_raw_parts(roots, len) })
    }

    /// Detaches and returns the owned shallow-root allocation.
    #[must_use]
    pub fn take_shallow_roots(&mut self) -> Option<crate::oidarray::OidArrayIds> {
        let p = self.as_mut_ptr();
        // SAFETY: exclusive access permits moving out the pair and restoring
        // the canonical empty state immediately.
        let (roots, len) = unsafe {
            let roots = core::ptr::addr_of!((*p).shallow_roots).read();
            let len = core::ptr::addr_of!((*p).shallow_roots_len).read();
            core::ptr::addr_of_mut!((*p).shallow_roots).write(core::ptr::null_mut());
            core::ptr::addr_of_mut!((*p).shallow_roots_len).write(0);
            (roots, len)
        };
        // SAFETY: the field transfers its unique configured-allocator run.
        unsafe { crate::oidarray::OidArrayIds::from_raw_parts(roots.cast(), len) }
    }

    /// Replaces the owned shallow-root allocation, returning the old one.
    pub fn replace_shallow_roots(
        &mut self,
        roots: Option<crate::oidarray::OidArrayIds>,
    ) -> Option<crate::oidarray::OidArrayIds> {
        let old = self.take_shallow_roots();
        let (roots, len) = roots.map_or((core::ptr::null_mut(), 0), |v| v.into_raw_parts());
        let p = self.as_mut_ptr();
        // SAFETY: the new owner was consumed and this handle has exclusive access.
        unsafe {
            core::ptr::addr_of_mut!((*p).shallow_roots).write(roots.cast());
            core::ptr::addr_of_mut!((*p).shallow_roots_len).write(len);
        }
        old
    }
}

#[cfg(test)]
mod fetch_negotiation_tests {
    use super::*;
    use core::mem::{align_of, size_of};
    use ffibox::CCell;

    #[test]
    fn wrapper_and_handles_match_the_c_seam() {
        fn assert_cell<T: CCell>() {}
        assert_cell::<GitFetchNegotiation>();
        assert_eq!(
            size_of::<GitFetchNegotiation>(),
            size_of::<ffi::git_fetch_negotiation>()
        );
        assert_eq!(
            align_of::<GitFetchNegotiation>(),
            align_of::<ffi::git_fetch_negotiation>()
        );
        assert_eq!(
            size_of::<GitFetchNegotiationRef<'_>>(),
            size_of::<*const ffi::git_fetch_negotiation>()
        );
        assert_eq!(
            size_of::<GitFetchNegotiationMut<'_>>(),
            size_of::<*mut ffi::git_fetch_negotiation>()
        );
    }

    #[test]
    fn getters_preserve_counts_and_typed_elements() {
        let mut head = crate::util::net::RemoteHead::zeroed();
        // SAFETY: the initialized head remains live throughout the test.
        let head = unsafe {
            crate::util::net::RemoteHeadRef::from_ptr(core::ptr::addr_of_mut!(head).cast()).unwrap()
        };
        let heads = [head];
        let mut roots = [crate::oid::Oid::zeroed()];
        let mut raw = ffi::git_fetch_negotiation {
            refs: heads.as_ptr().cast(),
            refs_len: 1,
            shallow_roots: roots.as_mut_ptr().cast(),
            shallow_roots_len: 1,
            depth: 3,
        };
        // SAFETY: raw and all its borrowed referents remain live and immutable.
        let view = unsafe { GitFetchNegotiationRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(view.depth(), 3);
        assert_eq!(view.refs_len(), 1);
        assert_eq!(view.refs().get(0).unwrap().as_ptr(), head.as_ptr());
        assert_eq!(view.shallow_roots_len(), 1);
        assert_eq!(view.shallow_roots().unwrap().len(), 1);
    }

    #[test]
    fn owned_shallow_roots_detach_into_a_single_raii_owner() {
        // SAFETY: process-global initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        // SAFETY: libgit2 is initialized, so its configured allocator is live.
        let roots =
            unsafe { ffi::crustify_git__malloc(size_of::<ffi::git_oid>()) }.cast::<ffi::git_oid>();
        assert!(!roots.is_null());
        // SAFETY: the fresh allocation has room for one object ID; zero is a
        // valid initialized C object-ID representation.
        unsafe { roots.write(core::mem::zeroed()) };
        let mut raw = ffi::git_fetch_negotiation {
            refs: core::ptr::null(),
            refs_len: 0,
            shallow_roots: roots,
            shallow_roots_len: 1,
            depth: 0,
        };
        // SAFETY: `raw` is initialized, live, and exclusively accessed here.
        let mut negotiation = unsafe { GitFetchNegotiationMut::from_ptr(&raw mut raw) }.unwrap();
        let owned = negotiation
            .take_shallow_roots()
            .expect("the non-null allocation transfers");
        assert_eq!(owned.count(), 1);
        assert!(negotiation.as_ref().shallow_roots().is_none());
        assert_eq!(negotiation.as_ref().shallow_roots_len(), 0);
        drop(owned);
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_transport
    /// A polymorphic connection to a local or remote Git repository.
    ///
    /// Callback dispatch requires an exclusive handle because libgit2
    /// transports are stateful and their implementations synchronize neither
    /// concurrent calls nor access to callback outputs.
    GitTransport,
    GitTransportRef,
    GitTransportMut,
    ffi::git_transport
);

/// An exclusively owned transport allocation.
pub type GitTransportOwned = CBox<GitTransport>;

/// An owned transport that cannot outlive the remote pointer it retains.
pub struct GitTransportWithRemote<'remote> {
    inner: GitTransportOwned,
    _remote: core::marker::PhantomData<GitRemoteMut<'remote>>,
}

impl<'remote> GitTransportWithRemote<'remote> {
    /// Couples a newly owned transport to the remote it stores.
    #[must_use]
    pub fn from_owned(inner: GitTransportOwned, remote: GitRemoteMut<'remote>) -> Self {
        let _ = remote;
        Self {
            inner,
            _remote: core::marker::PhantomData,
        }
    }

    /// Borrows the transport shared.
    #[must_use]
    pub fn as_ref(&self) -> GitTransportRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the transport exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitTransportMut<'_> {
        self.inner.as_mut()
    }
}

/// A transport callback that may be absent from a custom implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitTransportCallback {
    Connect,
    SetConnectOptions,
    Capabilities,
    OidType,
    List,
    Push,
    NegotiateFetch,
    ShallowRoots,
    DownloadPack,
    IsConnected,
    Cancel,
    Close,
}

/// Failure returned while dispatching a transport callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitTransportError {
    /// The concrete transport does not implement the requested callback.
    Unsupported(GitTransportCallback),
    /// The callback returned a libgit2 error code.
    Libgit2(core::ffi::c_int),
    /// A callback produced an invalid object-ID type.
    InvalidOidType(crate::oid::InvalidOidType),
    /// A successful callback returned a null pointer with a nonzero count.
    InvalidOutput,
}

/// A transport-owned list of advertised remote heads.
///
/// The list remains valid only until the next callback on its transport. Its
/// lifetime therefore holds an exclusive reborrow of the transport even
/// though individual heads are exposed as shared handles.
#[derive(Clone, Copy)]
pub struct GitTransportHeads<'a> {
    ptr: *mut *const ffi::git_remote_head,
    len: usize,
    _transport: core::marker::PhantomData<GitTransportMut<'a>>,
}

impl<'a> GitTransportHeads<'a> {
    /// Returns the number of advertised heads.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the transport advertised no heads.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Borrows advertised head `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<crate::util::net::RemoteHeadRef<'a>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: construction rejects a null pointer for a nonempty list, and
        // the callback promised `len` initialized pointer slots owned by the
        // transport for this view's lifetime.
        let head = unsafe { self.ptr.add(index).read() };
        // SAFETY: each non-null entry is transport-owned and remains live for
        // the exclusive reborrow represented by `'a`.
        unsafe { crate::util::net::RemoteHeadRef::from_ptr(head.cast_mut()) }
    }

    /// Iterates over the non-null advertised heads.
    pub fn iter(&self) -> impl Iterator<Item = crate::util::net::RemoteHeadRef<'a>> + use<'a> {
        let view = *self;
        (0..view.len).filter_map(move |index| view.get(index))
    }
}

impl GitTransportRef<'_> {
    /// Field: git_transport.version
    /// Returns the public transport-vtable version.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference over C-visible storage.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).version).read() }
    }
}

impl GitTransportMut<'_> {
    /// Sets the public transport-vtable version during initialization.
    pub fn set_version(&mut self, version: u32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Field: git_transport.connect
    /// Connects using libgit2's default connection options.
    ///
    /// The wrapper initializes a non-null default options record so custom
    /// transports receive the same valid input as libgit2's own callers. A
    /// future wrapper for
    /// `git_remote_connect_options` can add the configured variant without
    /// exposing that higher-layer type as a raw pointer here.
    pub fn connect(
        &mut self,
        url: &core::ffi::CStr,
        direction: crate::util::net::Direction,
    ) -> Result<(), GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer
        // from a live, exclusively borrowed transport.
        let callback = unsafe { core::ptr::addr_of!((*transport).connect).read() }.ok_or(
            GitTransportError::Unsupported(GitTransportCallback::Connect),
        )?;
        let options = default_connect_options()?;
        // SAFETY: the callback belongs to this live transport; `url` is a
        // readable NUL-terminated string, `options` is a valid initialized
        // default record, and all inputs remain live for this synchronous call.
        let result = unsafe {
            callback(
                transport,
                url.as_ptr(),
                ffi::git_direction::from(direction) as core::ffi::c_int,
                core::ptr::addr_of!(options),
            )
        };
        callback_result(result)
    }

    /// Field: git_transport.set_connect_opts
    /// Resets a connected transport to libgit2's default connection options.
    pub fn reset_connect_options(&mut self) -> Result<(), GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer
        // from a live, exclusively borrowed transport.
        let callback = unsafe { core::ptr::addr_of!((*transport).set_connect_opts).read() }.ok_or(
            GitTransportError::Unsupported(GitTransportCallback::SetConnectOptions),
        )?;
        let options = default_connect_options()?;
        // SAFETY: the callback belongs to this live transport; `options` is a
        // valid initialized default record and the call is synchronous.
        callback_result(unsafe { callback(transport, core::ptr::addr_of!(options)) })
    }

    /// Field: git_transport.capabilities
    /// Returns the concrete transport's published capability bitmask.
    pub fn capabilities(&mut self) -> Result<u32, GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).capabilities).read() }.ok_or(
            GitTransportError::Unsupported(GitTransportCallback::Capabilities),
        )?;
        let mut capabilities = 0;
        // SAFETY: the callback belongs to this live transport and receives a
        // writable scalar output slot for the synchronous call.
        callback_result(unsafe { callback(&raw mut capabilities, transport) })?;
        Ok(capabilities)
    }

    /// Field: git_transport.oid_type
    /// Returns the object-ID type used by the connected repository.
    pub fn oid_type(&mut self) -> Result<crate::oid::OidType, GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).oid_type).read() }.ok_or(
            GitTransportError::Unsupported(GitTransportCallback::OidType),
        )?;
        let mut oid_type = 0;
        // SAFETY: the callback belongs to this live transport and receives a
        // writable scalar output slot for the synchronous call.
        callback_result(unsafe { callback(&raw mut oid_type, transport) })?;
        crate::oid::OidType::try_from(oid_type).map_err(GitTransportError::InvalidOidType)
    }

    /// Field: git_transport.ls
    /// Borrows the transport-owned advertised heads.
    pub fn ls<'a>(&'a mut self) -> Result<GitTransportHeads<'a>, GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).ls).read() }
            .ok_or(GitTransportError::Unsupported(GitTransportCallback::List))?;
        let mut heads = core::ptr::null_mut();
        let mut len = 0;
        // SAFETY: both output slots are writable for the synchronous callback;
        // the transport is exclusively borrowed for the returned view.
        callback_result(unsafe { callback(&raw mut heads, &raw mut len, transport) })?;
        if heads.is_null() && len != 0 {
            return Err(GitTransportError::InvalidOutput);
        }
        Ok(GitTransportHeads {
            ptr: heads,
            len,
            _transport: core::marker::PhantomData,
        })
    }

    /// Field: git_transport.push
    /// Executes a push through this transport.
    pub fn push(
        &mut self,
        push: &mut crate::push::GitPushMut<'_>,
    ) -> Result<(), GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).push).read() }
            .ok_or(GitTransportError::Unsupported(GitTransportCallback::Push))?;
        // SAFETY: both exclusive handles designate live objects for the
        // synchronous callback, which retains neither pointer.
        callback_result(unsafe { callback(transport, push.as_mut_ptr()) })
    }

    /// Field: git_transport.negotiate_fetch
    /// Negotiates the requested fetch with the remote repository.
    pub fn negotiate_fetch(
        &mut self,
        repository: &mut crate::repository::GitRepositoryMut<'_>,
        fetch: GitFetchNegotiationRef<'_>,
    ) -> Result<(), GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).negotiate_fetch).read() }.ok_or(
            GitTransportError::Unsupported(GitTransportCallback::NegotiateFetch),
        )?;
        // SAFETY: the handles designate live inputs with the mutability in the
        // C signature, and the callback retains none of them.
        callback_result(unsafe { callback(transport, repository.as_mut_ptr(), fetch.as_ptr()) })
    }

    /// Field: git_transport.shallow_roots
    /// Returns an inline owner for the connected remote's shallow roots.
    pub fn shallow_roots(
        &mut self,
    ) -> Result<ffibox::CVal<crate::oidarray::OidArray>, GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).shallow_roots).read() }.ok_or(
            GitTransportError::Unsupported(GitTransportCallback::ShallowRoots),
        )?;
        let mut roots = crate::oidarray::OidArray::new();
        // SAFETY: `roots` is an initialized empty inline owner whose exclusive
        // output pointer is writable for the synchronous callback.
        callback_result(unsafe { callback(roots.as_mut().as_mut_ptr(), transport) })?;
        Ok(roots)
    }

    /// Field: git_transport.download_pack
    /// Downloads and indexes a packfile into `repository`.
    pub fn download_pack(
        &mut self,
        repository: &mut crate::repository::GitRepositoryMut<'_>,
        progress: &mut crate::indexer::IndexerProgressMut<'_>,
    ) -> Result<(), GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).download_pack).read() }.ok_or(
            GitTransportError::Unsupported(GitTransportCallback::DownloadPack),
        )?;
        // SAFETY: all three exclusive handles designate live objects for the
        // synchronous callback, which retains none of the pointers.
        callback_result(unsafe {
            callback(transport, repository.as_mut_ptr(), progress.as_mut_ptr())
        })
    }

    /// Field: git_transport.is_connected
    /// Reports whether this transport currently has a live connection.
    pub fn is_connected(&mut self) -> Result<bool, GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).is_connected).read() }.ok_or(
            GitTransportError::Unsupported(GitTransportCallback::IsConnected),
        )?;
        // SAFETY: the callback belongs to this live exclusively borrowed
        // transport and is synchronous.
        Ok(unsafe { callback(transport) } != 0)
    }

    /// Field: git_transport.cancel
    /// Cancels any outstanding operation on this transport.
    pub fn cancel(&mut self) -> Result<(), GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).cancel).read() }
            .ok_or(GitTransportError::Unsupported(GitTransportCallback::Cancel))?;
        // SAFETY: the callback belongs to this live exclusively borrowed
        // transport and is synchronous.
        unsafe { callback(transport) };
        Ok(())
    }

    /// Field: git_transport.close
    /// Closes this transport's current connection.
    pub fn close(&mut self) -> Result<(), GitTransportError> {
        let transport = self.as_mut_ptr();
        // SAFETY: raw-place projection copies the nullable static code pointer.
        let callback = unsafe { core::ptr::addr_of!((*transport).close).read() }
            .ok_or(GitTransportError::Unsupported(GitTransportCallback::Close))?;
        // SAFETY: the callback belongs to this live exclusively borrowed
        // transport and is synchronous.
        callback_result(unsafe { callback(transport) })
    }
}

fn callback_result(result: core::ffi::c_int) -> Result<(), GitTransportError> {
    if result < 0 {
        Err(GitTransportError::Libgit2(result))
    } else {
        Ok(())
    }
}

fn default_connect_options() -> Result<ffi::git_remote_connect_options, GitTransportError> {
    let mut options = core::mem::MaybeUninit::uninit();
    // SAFETY: `options` is a writable correctly aligned output slot and the
    // published version asks libgit2 to initialize its entire record.
    let result = unsafe {
        ffi::git_remote_connect_options_init(
            options.as_mut_ptr(),
            ffi::GIT_REMOTE_CONNECT_OPTIONS_VERSION,
        )
    };
    callback_result(result)?;
    // SAFETY: a successful initializer wrote the complete options record.
    Ok(unsafe { options.assume_init() })
}

/// Field: git_transport.free
// SAFETY: adopting `GitTransportOwned` requires a unique, fully constructed
// concrete transport allocation with its destructor callback installed.
// `CBox` invokes that callback exactly once as its final access to the object.
unsafe impl CDropped for GitTransport {
    unsafe fn c_drop(transport: NonNull<Self>) {
        let transport = transport.as_ptr().cast::<ffi::git_transport>();
        // SAFETY: the `CDropped` contract supplies a live, fully constructed
        // transport; raw-place projection copies its static callback pointer.
        let free = unsafe { core::ptr::addr_of!((*transport).free).read() }
            .expect("an owned transport has a free callback");
        // SAFETY: `free` is this concrete allocation's destructor and the
        // `CDropped` contract grants its one final invocation.
        unsafe { free(transport) }
    }
}

#[cfg(test)]
mod transport_tests {
    use super::*;
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};
    use ffibox::CCell;

    static CANCELS: AtomicUsize = AtomicUsize::new(0);
    static FREES: AtomicUsize = AtomicUsize::new(0);

    fn raw_transport() -> ffi::git_transport {
        ffi::git_transport {
            version: 1,
            connect: None,
            set_connect_opts: None,
            capabilities: None,
            oid_type: None,
            ls: None,
            push: None,
            negotiate_fetch: None,
            shallow_roots: None,
            download_pack: None,
            is_connected: None,
            cancel: None,
            close: None,
            free: None,
        }
    }

    unsafe extern "C" fn test_connect(
        _transport: *mut ffi::git_transport,
        url: *const core::ffi::c_char,
        direction: core::ffi::c_int,
        options: *const ffi::git_remote_connect_options,
    ) -> core::ffi::c_int {
        assert!(!url.is_null());
        assert_eq!(direction, ffi::git_direction_GIT_DIRECTION_FETCH as i32);
        assert!(!options.is_null());
        0
    }

    unsafe extern "C" fn test_reset_options(
        _transport: *mut ffi::git_transport,
        options: *const ffi::git_remote_connect_options,
    ) -> core::ffi::c_int {
        assert!(!options.is_null());
        0
    }

    unsafe extern "C" fn test_capabilities(
        capabilities: *mut u32,
        _transport: *mut ffi::git_transport,
    ) -> core::ffi::c_int {
        // SAFETY: the wrapper supplies a live writable scalar output slot.
        unsafe { capabilities.write(7) };
        0
    }

    unsafe extern "C" fn test_oid_type(
        oid_type: *mut ffi::git_oid_t,
        _transport: *mut ffi::git_transport,
    ) -> core::ffi::c_int {
        // SAFETY: the wrapper supplies a live writable scalar output slot.
        unsafe { oid_type.write(ffi::git_oid_t_GIT_OID_SHA256) };
        0
    }

    unsafe extern "C" fn test_connected(_transport: *mut ffi::git_transport) -> core::ffi::c_int {
        1
    }

    unsafe extern "C" fn test_cancel(_transport: *mut ffi::git_transport) {
        CANCELS.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn test_close(_transport: *mut ffi::git_transport) -> core::ffi::c_int {
        0
    }

    unsafe extern "C" fn test_free(transport: *mut ffi::git_transport) {
        FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: this callback is installed only on the unique allocation
        // produced by `Box::into_raw` in the ownership test.
        drop(unsafe { Box::from_raw(transport) });
    }

    #[repr(C)]
    struct TransportWithHead {
        parent: ffi::git_transport,
        head: ffi::git_remote_head,
        heads: [*const ffi::git_remote_head; 1],
    }

    unsafe extern "C" fn test_ls(
        out: *mut *mut *const ffi::git_remote_head,
        len: *mut usize,
        transport: *mut ffi::git_transport,
    ) -> core::ffi::c_int {
        let concrete = transport.cast::<TransportWithHead>();
        // SAFETY: `transport` is the first field of the live concrete test
        // object, and both caller-provided output slots are writable.
        unsafe {
            out.write(core::ptr::addr_of_mut!((*concrete).heads).cast());
            len.write(1);
        }
        0
    }

    #[test]
    fn wrapper_and_handles_match_the_c_seam() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitTransport>();
        assert_dropped::<GitTransport>();
        assert_eq!(size_of::<GitTransport>(), size_of::<ffi::git_transport>());
        assert_eq!(align_of::<GitTransport>(), align_of::<ffi::git_transport>());
        assert_eq!(
            size_of::<GitTransportRef<'_>>(),
            size_of::<*const ffi::git_transport>()
        );
        assert_eq!(
            size_of::<GitTransportMut<'_>>(),
            size_of::<*mut ffi::git_transport>()
        );
        assert_eq!(
            size_of::<GitTransportOwned>(),
            size_of::<*mut ffi::git_transport>()
        );
    }

    #[test]
    fn scalar_callbacks_dispatch_through_the_exclusive_handle() {
        let before_cancels = CANCELS.load(Ordering::SeqCst);
        let mut raw = raw_transport();
        raw.connect = Some(test_connect);
        raw.set_connect_opts = Some(test_reset_options);
        raw.capabilities = Some(test_capabilities);
        raw.oid_type = Some(test_oid_type);
        raw.is_connected = Some(test_connected);
        raw.cancel = Some(test_cancel);
        raw.close = Some(test_close);

        // SAFETY: `raw` is initialized, remains live, and is exclusively
        // accessed through this handle for the test's duration.
        let mut transport = unsafe { GitTransportMut::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(transport.as_ref().version(), 1);
        transport.set_version(2);
        assert_eq!(transport.as_ref().version(), 2);
        assert_eq!(
            transport.connect(c"file:///tmp/repo", crate::util::net::Direction::Fetch),
            Ok(())
        );
        assert_eq!(transport.reset_connect_options(), Ok(()));
        assert_eq!(transport.capabilities(), Ok(7));
        assert_eq!(transport.oid_type(), Ok(crate::oid::OidType::Sha256));
        assert_eq!(transport.is_connected(), Ok(true));
        assert_eq!(transport.cancel(), Ok(()));
        assert_eq!(transport.close(), Ok(()));
        assert_eq!(CANCELS.load(Ordering::SeqCst), before_cancels + 1);
    }

    #[test]
    fn list_view_keeps_transport_owned_heads_typed() {
        let mut concrete = Box::new(TransportWithHead {
            parent: raw_transport(),
            // SAFETY: every field in the raw head accepts the all-zero C
            // representation used for this pointer-identity test.
            head: unsafe { core::mem::zeroed() },
            heads: [core::ptr::null()],
        });
        concrete.parent.ls = Some(test_ls);
        concrete.heads[0] = core::ptr::addr_of!(concrete.head);
        let expected = concrete.heads[0];
        // SAFETY: the boxed concrete object has a stable address, its parent
        // is the first field, and this handle has exclusive access.
        let mut transport =
            unsafe { GitTransportMut::from_ptr(core::ptr::addr_of_mut!(concrete.parent)) }.unwrap();
        let heads = transport.ls().unwrap();
        assert_eq!(heads.len(), 1);
        assert!(!heads.is_empty());
        assert_eq!(heads.get(0).unwrap().as_ptr(), expected);
        assert!(heads.get(1).is_none());
        assert_eq!(heads.iter().count(), 1);
    }

    #[test]
    fn unsupported_callbacks_are_reported_without_calling_null() {
        let mut raw = raw_transport();
        // SAFETY: `raw` is initialized, live, and exclusively accessed here.
        let mut transport = unsafe { GitTransportMut::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(
            transport.close(),
            Err(GitTransportError::Unsupported(GitTransportCallback::Close))
        );
    }

    #[test]
    fn owned_transport_invokes_its_concrete_destructor_once() {
        let before = FREES.load(Ordering::SeqCst);
        let mut raw = raw_transport();
        raw.free = Some(test_free);
        let raw = Box::into_raw(Box::new(raw));
        // SAFETY: `raw` is a unique fully initialized allocation whose
        // installed destructor reclaims that exact allocation.
        let transport = unsafe { GitTransportOwned::from_raw(raw) }.unwrap();
        drop(transport);
        assert_eq!(FREES.load(Ordering::SeqCst), before + 1);
    }
}
