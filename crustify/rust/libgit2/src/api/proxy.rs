//! Safe wrappers for libgit2 proxy APIs.

use core::ffi::{CStr, c_int, c_uint, c_void};
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::cert::{GitCertRef, GitTransportCertificateCheckCallback};
use crate::api::credential::GitCredentialAcquireCallback;
use crate::ffi;
use crate::proxy::{InvalidProxyType, ProxyType};
use crate::transports::credential::GitCredentialType;

/// Wraps: git_proxy_options
/// Layout-compatible proxy options borrowing their URL and callback state for
/// the options data lifetime.
#[repr(transparent)]
pub struct GitProxyOptions<'data> {
    inner: CType<ffi::git_proxy_options>,
    _data: PhantomData<&'data mut ()>,
}

/// Shared borrow of [`GitProxyOptions`].
#[repr(transparent)]
pub struct GitProxyOptionsRef<'object, 'data>(CPtr<'object, GitProxyOptions<'data>>);

impl Clone for GitProxyOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitProxyOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitProxyOptions`].
#[repr(transparent)]
pub struct GitProxyOptionsMut<'object, 'data>(GitProxyOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and never form references to C-visible
// storage, and the shared handle exposes no writes.
unsafe impl<'data> CCell for GitProxyOptions<'data> {
    type C = ffi::git_proxy_options;
    type Ref<'object>
        = GitProxyOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitProxyOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(p: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `p` is a live shared object.
        GitProxyOptionsRef(unsafe { CPtr::new(p) })
    }

    unsafe fn mut_from_raw<'object>(p: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitProxyOptionsMut(GitProxyOptionsRef(unsafe { CPtr::new(p) }))
    }
}

// SAFETY: values constructed by this safe public surface borrow every pointer
// field and own no resource, so disposing their inline storage does nothing.
unsafe impl CValued for GitProxyOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitProxyOptions<'data> {
    /// Constructs options equivalent to `GIT_PROXY_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every field of the bindgen struct admits the all-zero bit
        // pattern; the required version is written before returning it.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        options.as_mut().set_version(ffi::GIT_PROXY_OPTIONS_VERSION);
        options
    }
}

impl<'object, 'data> GitProxyOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify an initialized options value live for `'object`.
    /// Its URL and callback payload must remain valid for `'data`, and `'data`
    /// must outlive `'object`. Any callback and payload pair must agree on its
    /// concrete erased payload type.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_proxy_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitProxyOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_proxy_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_proxy_options.payload
    /// Returns whether callback state is installed.
    #[must_use]
    pub fn has_callback_payload(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized opaque pointer.
        !unsafe { addr_of!((*self.as_ptr()).payload).read() }.is_null()
    }

    /// Field: git_proxy_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_proxy_options.type
    /// Returns the configured proxy selection, rejecting unknown C values.
    pub fn proxy_type(&self) -> Result<ProxyType, InvalidProxyType> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let proxy_type = unsafe { addr_of!((*self.as_ptr()).type_).read() };
        ProxyType::try_from(proxy_type)
    }

    /// Field: git_proxy_options.url
    /// Borrows the optional proxy URL.
    #[must_use]
    pub fn url(&self) -> Option<&'object CStr> {
        // SAFETY: raw-place projection copies the initialized string pointer.
        let url = unsafe { addr_of!((*self.as_ptr()).url).read() };
        if url.is_null() {
            None
        } else {
            // SAFETY: the options contract keeps this NUL-terminated string
            // live and immutable for at least this object borrow.
            Some(unsafe { CStr::from_ptr(url) })
        }
    }

    /// Field: git_proxy_options.certificate_check
    /// Returns whether a certificate-check callback is installed.
    #[must_use]
    pub fn has_certificate_check(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { addr_of!((*self.as_ptr()).certificate_check).read() }.is_some()
    }

    /// Field: git_proxy_options.credentials
    /// Returns whether a credential callback is installed.
    #[must_use]
    pub fn has_credentials(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { addr_of!((*self.as_ptr()).credentials).read() }.is_some()
    }
}

impl<'object, 'data> GitProxyOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path to the
    /// options value may be used for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_proxy_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitProxyOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_proxy_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitProxyOptionsRef<'_, 'data> {
        GitProxyOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects how the proxy is discovered.
    pub fn set_proxy_type(&mut self, proxy_type: ProxyType) {
        // SAFETY: this exclusive handle permits the write, and `ProxyType`
        // contains only published C enum values.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).type_).write(proxy_type.into()) }
    }

    /// Stores an optional borrowed proxy URL.
    pub fn set_url(&mut self, url: Option<&'data CStr>) {
        let url = url.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write, and the
        // wrapper lifetime keeps every non-null URL alive and immutable.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).url).write(url) }
    }

    /// Installs only a typed credential callback, clearing the certificate
    /// callback so the shared payload has one unambiguous concrete type.
    pub fn set_credentials<C: GitCredentialAcquireCallback>(&mut self, callback: &'data mut C) {
        let options = self.as_mut_ptr();
        // SAFETY: both slots and their shared payload are updated coherently;
        // the wrapper's data lifetime reserves `callback` for every invocation.
        unsafe {
            addr_of_mut!((*options).credentials).write(Some(credential_trampoline::<C>));
            addr_of_mut!((*options).certificate_check).write(None);
            addr_of_mut!((*options).payload).write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Installs only a typed certificate callback, clearing the credential
    /// callback so the shared payload has one unambiguous concrete type.
    pub fn set_certificate_check<C: GitTransportCertificateCheckCallback>(
        &mut self,
        callback: &'data mut C,
    ) {
        let options = self.as_mut_ptr();
        // SAFETY: both slots and their shared payload are updated coherently;
        // the wrapper's data lifetime reserves `callback` for every invocation.
        unsafe {
            addr_of_mut!((*options).credentials).write(None);
            addr_of_mut!((*options).certificate_check).write(Some(certificate_trampoline::<C>));
            addr_of_mut!((*options).payload).write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Installs both callbacks on one state value implementing both callable
    /// contracts, matching the C structure's single shared payload slot.
    pub fn set_callbacks<C>(&mut self, callback: &'data mut C)
    where
        C: GitCredentialAcquireCallback + GitTransportCertificateCheckCallback,
    {
        let options = self.as_mut_ptr();
        // SAFETY: both callback monomorphizations agree on `C`, the writes are
        // coherent, and the data lifetime reserves the payload exclusively.
        unsafe {
            addr_of_mut!((*options).credentials).write(Some(credential_trampoline::<C>));
            addr_of_mut!((*options).certificate_check).write(Some(certificate_trampoline::<C>));
            addr_of_mut!((*options).payload).write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears both callbacks and their shared payload.
    pub fn clear_callbacks(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits all three coherent writes.
        unsafe {
            addr_of_mut!((*options).credentials).write(None);
            addr_of_mut!((*options).certificate_check).write(None);
            addr_of_mut!((*options).payload).write(core::ptr::null_mut());
        }
    }
}

unsafe extern "C" fn credential_trampoline<C: GitCredentialAcquireCallback>(
    out: *mut *mut ffi::git_credential,
    url: *const core::ffi::c_char,
    username_from_url: *const core::ffi::c_char,
    allowed_types: c_uint,
    payload: *mut c_void,
) -> c_int {
    let (Some(out), Some(url), Some(callback), Some(allowed_types)) = (
        NonNull::new(out),
        NonNull::new(url.cast_mut()),
        NonNull::new(payload.cast::<C>()),
        GitCredentialType::from_bits(allowed_types),
    ) else {
        return -1;
    };
    // SAFETY: libgit2 supplies a live NUL-terminated URL for this invocation.
    let url = unsafe { CStr::from_ptr(url.as_ptr()) };
    let username = NonNull::new(username_from_url.cast_mut()).map(|username| {
        // SAFETY: a non-null username supplied by libgit2 is NUL-terminated
        // and remains live for this callback invocation.
        unsafe { CStr::from_ptr(username.as_ptr()) }
    });
    // SAFETY: installation reserves this payload as one live `C` and libgit2
    // invokes a callback serially for one connection operation.
    let callback = unsafe { callback.as_ptr().as_mut() }.expect("a non-null pointer is valid");
    match callback.acquire(url, username, allowed_types) {
        Ok(credential) => {
            // SAFETY: the validated output slot is writable for one pointer;
            // ownership transfers to libgit2 on a successful callback.
            unsafe { out.as_ptr().write(credential.into_raw()) };
            0
        }
        Err(status) => status,
    }
}

unsafe extern "C" fn certificate_trampoline<C: GitTransportCertificateCheckCallback>(
    certificate: *mut ffi::git_cert,
    valid: c_int,
    host: *const core::ffi::c_char,
    payload: *mut c_void,
) -> c_int {
    // SAFETY: libgit2 supplies a live certificate for this callback only.
    let Some(certificate) = (unsafe { GitCertRef::from_ptr(certificate) }) else {
        return -1;
    };
    let (Some(host), Some(callback)) = (
        NonNull::new(host.cast_mut()),
        NonNull::new(payload.cast::<C>()),
    ) else {
        return -1;
    };
    // SAFETY: libgit2 supplies a live NUL-terminated host for this invocation.
    let host = unsafe { CStr::from_ptr(host.as_ptr()) };
    // SAFETY: installation reserves the payload as one live `C`, and callback
    // invocations for one connection do not overlap.
    let callback = unsafe { callback.as_ptr().as_mut() }.expect("a non-null pointer is valid");
    callback.call(certificate, valid != 0, host)
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use crate::api::cert::GitCertType;
    use crate::transports::credential::GitCredentialOwned;

    use super::*;

    #[test]
    fn proxy_options_preserve_layout_and_defaults() {
        assert_eq!(
            size_of::<GitProxyOptions<'static>>(),
            size_of::<ffi::git_proxy_options>()
        );
        assert_eq!(
            align_of::<GitProxyOptions<'static>>(),
            align_of::<ffi::git_proxy_options>()
        );
        assert_eq!(
            size_of::<GitProxyOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_proxy_options>()
        );

        let options = GitProxyOptions::new();
        let options = options.as_ref();
        assert_eq!(options.version(), ffi::GIT_PROXY_OPTIONS_VERSION);
        assert_eq!(options.proxy_type(), Ok(ProxyType::None));
        assert!(options.url().is_none());
        assert!(!options.has_credentials());
        assert!(!options.has_certificate_check());
        assert!(!options.has_callback_payload());
    }

    #[test]
    fn options_borrow_url_and_install_one_typed_callback_pair() {
        let mut calls = 0;
        let mut callback = |certificate: GitCertRef<'_>, valid: bool, host: &CStr| {
            calls += 1;
            assert_eq!(certificate.cert_type(), Ok(GitCertType::X509));
            assert!(valid);
            assert_eq!(host, c"proxy.example");
            7
        };
        let mut options = GitProxyOptions::new();
        {
            let mut options = options.as_mut();
            options.set_proxy_type(ProxyType::Specified);
            options.set_url(Some(c"http://proxy.example:8080"));
            options.set_certificate_check(&mut callback);
        }
        let options_ref = options.as_ref();
        assert_eq!(options_ref.proxy_type(), Ok(ProxyType::Specified));
        assert_eq!(options_ref.url(), Some(c"http://proxy.example:8080"));
        assert!(options_ref.has_certificate_check());
        assert!(!options_ref.has_credentials());
        assert!(options_ref.has_callback_payload());

        let mut certificate = ffi::git_cert {
            cert_type: ffi::git_cert_t_GIT_CERT_X509,
        };
        let raw = options_ref.as_ptr();
        // SAFETY: raw-place reads copy the coherently installed callback pair.
        let function = unsafe { addr_of!((*raw).certificate_check).read() }.unwrap();
        // SAFETY: as above; the opaque pointer is not dereferenced here.
        let payload = unsafe { addr_of!((*raw).payload).read() };
        // SAFETY: all callback inputs and the installed callback state remain
        // live for this invocation.
        let status =
            unsafe { function(&raw mut certificate, 1, c"proxy.example".as_ptr(), payload) };
        assert_eq!(status, 7);
        drop(options);
        assert_eq!(calls, 1);
    }

    #[test]
    fn credential_trampoline_rejects_unknown_allowed_bits() {
        let mut callback = |_: &CStr,
                            _: Option<&CStr>,
                            _: GitCredentialType|
         -> Result<GitCredentialOwned, i32> { Err(9) };
        let mut options = GitProxyOptions::new();
        options.as_mut().set_credentials(&mut callback);
        let raw = options.as_ref().as_ptr();
        // SAFETY: raw-place reads copy the coherently installed callback pair.
        let function = unsafe { addr_of!((*raw).credentials).read() }.unwrap();
        // SAFETY: as above; the opaque pointer is not dereferenced here.
        let payload = unsafe { addr_of!((*raw).payload).read() };
        let mut out = core::ptr::null_mut();
        // SAFETY: the output slot, strings and callback state remain live.
        let status = unsafe {
            function(
                &raw mut out,
                c"https://example".as_ptr(),
                core::ptr::null(),
                1 << 31,
                payload,
            )
        };
        assert_eq!(status, -1);
        assert!(out.is_null());
    }
}
