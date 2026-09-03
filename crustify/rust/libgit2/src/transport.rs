//! Safe wrappers for libgit2 transport APIs.

use core::ffi::{CStr, c_void};

use crate::api::transport::GitTransportCallback;
use crate::remote::GitRemoteMut;

/// A process-global custom transport registration.
///
/// Dropping this value unregisters its scheme before releasing the callback.
pub struct GitTransportRegistration<C> {
    scheme: std::ffi::CString,
    callback: Option<Box<std::sync::Mutex<C>>>,
}

impl<C> Drop for GitTransportRegistration<C> {
    fn drop(&mut self) {
        // SAFETY: the registration constructor's caller promises to serialize
        // registration lifetime and drop against all transport instantiation.
        // The stored scheme is the exact live string used for registration.
        let status = unsafe { crate::ffi::git_transport_unregister(self.scheme.as_ptr()) };
        if status != 0 {
            // Unregistration can fail while allocating its temporary lookup
            // string. The C registry then still holds this address, so leaking
            // is the only sound fallback; freeing it would leave a dangling
            // callback payload reachable from safe libgit2 calls.
            core::mem::forget(self.callback.take());
        }
    }
}

unsafe extern "C" fn registered_transport<C>(
    out: *mut *mut crate::ffi::git_transport,
    owner: *mut crate::ffi::git_remote,
    payload: *mut c_void,
) -> core::ffi::c_int
where
    C: GitTransportCallback + Send + 'static,
{
    if out.is_null() || owner.is_null() || payload.is_null() {
        return crate::ffi::git_error_code_GIT_EINVALID;
    }

    // SAFETY: `out` is non-null and belongs to the C caller for this callback.
    unsafe { out.write(core::ptr::null_mut()) };
    // SAFETY: libgit2 supplies a live remote exclusively for factory setup.
    let Some(owner) = (unsafe { GitRemoteMut::from_ptr(owner) }) else {
        return crate::ffi::git_error_code_GIT_EINVALID;
    };
    // SAFETY: the registration stores a boxed `Mutex<C>` at this exact
    // pointer until after unregister completes. The mutex synchronizes all
    // concurrent invocations before yielding exclusive callback access.
    let callback = unsafe { &*payload.cast::<std::sync::Mutex<C>>() };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut callback = callback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        callback.call(owner)
    }));

    match result {
        Ok(Ok(transport)) => {
            // SAFETY: `out` is the writable slot checked above. Ownership moves
            // to libgit2, which attaches the transport to `owner` or frees it.
            unsafe { out.write(transport.into_raw()) };
            0
        }
        Ok(Err(status)) => status,
        Err(_) => crate::ffi::git_error_code_GIT_ERROR,
    }
}

/// Wraps: git_transport_message_cb
/// Safe callable surface for length-delimited transport messages.
pub trait GitTransportMessageCallback {
    /// Receives transient message bytes, which need not be NUL-terminated.
    fn call(&mut self, message: &[u8]) -> i32;
}

impl<F> GitTransportMessageCallback for F
where
    F: FnMut(&[u8]) -> i32,
{
    fn call(&mut self, message: &[u8]) -> i32 {
        self(message)
    }
}

/// Wraps: git_transport_register
/// Registers a Rust transport factory until the returned guard is dropped.
///
/// # Safety
///
/// The caller must serialize both this call and destruction of the returned
/// guard against every libgit2 operation that may instantiate a transport.
/// The same scheme must not be unregistered through another API while the
/// guard lives.
pub unsafe fn git_transport_register<C>(
    scheme: &CStr,
    callback: C,
) -> Result<GitTransportRegistration<C>, i32>
where
    C: GitTransportCallback + Send + 'static,
{
    let scheme = scheme.to_owned();
    let callback = Box::new(std::sync::Mutex::new(callback));
    let payload = (&*callback as *const std::sync::Mutex<C>).cast_mut().cast();
    // SAFETY: the callback and payload have the ABI expected by libgit2. The
    // box has a stable address and remains stored in the returned guard until
    // after its scheme is unregistered.
    let status = unsafe {
        crate::ffi::git_transport_register(
            scheme.as_ptr(),
            Some(registered_transport::<C>),
            payload,
        )
    };
    if status == 0 {
        Ok(GitTransportRegistration {
            scheme,
            callback: Some(callback),
        })
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::sys::transport::GitTransportWithRemote;

    struct RejectFactory;

    impl GitTransportCallback for RejectFactory {
        fn call<'remote>(
            &mut self,
            _owner: GitRemoteMut<'remote>,
        ) -> Result<GitTransportWithRemote<'remote>, i32> {
            Err(-1)
        }
    }

    #[test]
    fn message_callback_preserves_non_nul_bytes() {
        let mut callback = |message: &[u8]| message.len() as i32;
        assert_eq!(GitTransportMessageCallback::call(&mut callback, b"a\0b"), 3);
    }

    #[test]
    fn registered_factory_rejects_invalid_c_callback_arguments() {
        // SAFETY: null arguments are intentionally passed to exercise the
        // trampoline's validation path; it dereferences none of them.
        let status = unsafe {
            registered_transport::<RejectFactory>(
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            )
        };
        assert_eq!(status, crate::ffi::git_error_code_GIT_EINVALID);
    }
}
