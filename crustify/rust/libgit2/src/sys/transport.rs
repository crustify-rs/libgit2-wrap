//! Safe wrappers for libgit2 transport APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CDropped};

use crate::ffi;

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
