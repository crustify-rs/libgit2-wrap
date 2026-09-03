//! Safe wrappers for libgit2 smart APIs.

use crate::ffi;
use crate::remote::GitRemoteMut;
use crate::sys::transport::{
    GitSmartSubtransportCallback, GitSmartSubtransportDefinition, GitTransportOwned,
    GitTransportWithRemote,
};

/// Wraps: git_transport_smart
/// Creates a smart transport with a typed synchronous subtransport factory.
pub fn git_transport_smart<'remote, C>(
    mut owner: GitRemoteMut<'remote>,
    rpc: bool,
    callback: &mut C,
) -> Result<GitTransportWithRemote<'remote>, i32>
where
    C: GitSmartSubtransportCallback,
{
    let mut raw = core::ptr::null_mut();
    let mut definition = GitSmartSubtransportDefinition::new(rpc, callback);
    // SAFETY: `raw` is writable; `owner` is live and exclusive, and the
    // temporary definition and callback remain live until this synchronous
    // constructor returns. The returned transport retains only `owner`.
    let status = unsafe {
        ffi::git_transport_smart(
            core::ptr::addr_of_mut!(raw),
            owner.as_mut_ptr(),
            definition.as_mut().as_mut_ptr().cast(),
        )
    };
    if status != 0 {
        debug_assert!(raw.is_null());
        return Err(status);
    }
    // SAFETY: success publishes one fully initialized owned transport.
    let transport =
        unsafe { GitTransportOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(GitTransportWithRemote::from_owned(transport, owner))
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::sys::transport::{
        GitSmartSubtransportOwned, GitSmartSubtransportWithTransport, GitTransportMut,
    };

    unsafe extern "C" fn action(
        _out: *mut *mut ffi::git_smart_subtransport_stream,
        _transport: *mut ffi::git_smart_subtransport,
        _url: *const core::ffi::c_char,
        _action: ffi::git_smart_service_t,
    ) -> i32 {
        -1
    }

    unsafe extern "C" fn close(_transport: *mut ffi::git_smart_subtransport) -> i32 {
        0
    }

    unsafe extern "C" fn free(transport: *mut ffi::git_smart_subtransport) {
        // SAFETY: the test factory transfers the unique `Box` allocation to
        // libgit2, which invokes this installed callback exactly once.
        drop(unsafe { Box::from_raw(transport) });
    }

    fn factory<'transport>(
        owner: GitTransportMut<'transport>,
    ) -> Result<GitSmartSubtransportWithTransport<'transport>, i32> {
        let raw = Box::into_raw(Box::new(ffi::git_smart_subtransport {
            action: Some(action),
            close: Some(close),
            free: Some(free),
        }));
        // SAFETY: `raw` is a unique fully initialized subtransport with its
        // required destructor callback installed.
        let inner = unsafe { GitSmartSubtransportOwned::from_raw(raw) }.unwrap();
        Ok(GitSmartSubtransportWithTransport::from_owned(inner, owner))
    }

    #[test]
    fn smart_transport_invokes_typed_factory_and_owns_its_result() {
        // SAFETY: libgit2 initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let storage = Box::new(core::mem::MaybeUninit::<ffi::git_remote>::zeroed());
        let raw_remote = Box::into_raw(storage).cast::<ffi::git_remote>();
        {
            // SAFETY: the opaque storage remains live and exclusively held for
            // this handle and the transport's lifetime marker.
            let remote = unsafe { GitRemoteMut::from_ptr(raw_remote) }.unwrap();
            let mut transport = git_transport_smart(remote, false, &mut factory).unwrap();
            assert!(!transport.as_ref().as_ptr().is_null());
            assert!(!transport.as_mut().as_mut_ptr().is_null());
        }
        // SAFETY: the transport and its remote borrow are gone; recover the
        // exact allocation without interpreting its opaque contents.
        drop(unsafe {
            Box::from_raw(raw_remote.cast::<core::mem::MaybeUninit<ffi::git_remote>>())
        });
        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
