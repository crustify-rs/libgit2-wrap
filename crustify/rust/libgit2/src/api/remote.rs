//! Safe wrappers for libgit2 remote APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::api::buffer::GitBufMut;
use crate::remote::GitPushUpdateRef;
use crate::util::net::Direction;

/// Wraps: git_url_resolve_cb
/// Safe callable surface for the deprecated remote URL resolver.
pub trait GitUrlResolveCallback {
    /// Writes the resolved URL to `output`.
    ///
    /// Return zero on success, `GIT_PASSTHROUGH`, or another libgit2 error.
    fn resolve(&mut self, output: &mut GitBufMut<'_>, url: &CStr, direction: Direction) -> i32;
}

impl<F> GitUrlResolveCallback for F
where
    F: FnMut(&mut GitBufMut<'_>, &CStr, Direction) -> i32,
{
    fn resolve(&mut self, output: &mut GitBufMut<'_>, url: &CStr, direction: Direction) -> i32 {
        self(output, url, direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_implement_the_url_resolver_surface() {
        fn accepts<C: GitUrlResolveCallback>(_callback: C) {}
        accepts(|_: &mut GitBufMut<'_>, _: &CStr, _: Direction| 0);
    }
}

/// A borrowed pointer-array view supplied to push negotiation callbacks.
#[derive(Clone, Copy)]
pub struct GitPushUpdates<'a> {
    ptr: NonNull<*const crate::ffi::git_push_update>,
    len: usize,
    _borrow: PhantomData<GitPushUpdateRef<'a>>,
}

impl<'a> GitPushUpdates<'a> {
    /// Constructs the transient view used by a callback trampoline.
    ///
    /// # Safety
    /// `ptr` must address `len` readable pointers to live push updates for
    /// `'a`; every element must be non-null.
    #[allow(dead_code)]
    pub(crate) unsafe fn from_raw(
        ptr: *const *const crate::ffi::git_push_update,
        len: usize,
    ) -> Option<Self> {
        let ptr = if len == 0 && ptr.is_null() {
            NonNull::dangling()
        } else {
            NonNull::new(ptr.cast_mut())?
        };
        Some(Self {
            ptr,
            len,
            _borrow: PhantomData,
        })
    }

    /// Returns the number of updates.
    #[must_use]
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns whether there are no updates.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrows one update by position.
    #[must_use]
    pub fn get(self, index: usize) -> Option<GitPushUpdateRef<'a>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: construction guarantees a readable `len`-element pointer
        // array, and the bounds check selects one initialized element.
        let update = unsafe { self.ptr.as_ptr().add(index).read() };
        // SAFETY: construction requires each element to name a live update for
        // `'a`; a null element is defensively rejected.
        unsafe { GitPushUpdateRef::from_ptr(update.cast_mut()) }
    }
}

/// Wraps: git_push_negotiation
/// Safe callable surface for the transient update array negotiated by a push.
pub trait GitPushNegotiationCallback {
    /// Returns zero to proceed or a nonzero libgit2 status to cancel.
    fn call(&mut self, updates: GitPushUpdates<'_>) -> i32;
}

impl<F> GitPushNegotiationCallback for F
where
    F: for<'a> FnMut(GitPushUpdates<'a>) -> i32,
{
    fn call(&mut self, updates: GitPushUpdates<'_>) -> i32 {
        self(updates)
    }
}

#[cfg(test)]
mod push_negotiation_tests {
    use core::mem::MaybeUninit;

    use super::*;

    #[test]
    fn pointer_array_view_checks_bounds_without_forming_a_slice() {
        let storage = Box::new(MaybeUninit::<crate::ffi::git_push_update>::zeroed());
        let update = Box::into_raw(storage).cast::<crate::ffi::git_push_update>();
        let pointers = [update.cast_const()];
        // SAFETY: the local array and opaque allocation remain live for this
        // view, and its sole element is non-null.
        let updates = unsafe { GitPushUpdates::from_raw(pointers.as_ptr(), 1) }.unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates.get(0).unwrap().as_ptr(), update.cast_const());
        assert!(updates.get(1).is_none());
        let mut callback = |values: GitPushUpdates<'_>| values.len() as i32;
        assert_eq!(GitPushNegotiationCallback::call(&mut callback, updates), 1);
        // SAFETY: no handle survives and this recovers the original allocation.
        drop(unsafe { Box::from_raw(update.cast::<MaybeUninit<crate::ffi::git_push_update>>()) });
    }
}

/// A single typed receiver for every callback in [`GitRemoteCallbacks`].
///
/// Libgit2 stores one payload pointer for the whole table, so all methods share
/// one exclusively borrowed receiver. Defaults preserve the behavior of an
/// unconfigured callback table.
pub trait GitRemoteCallbackHandler {
    /// Receives textual sideband progress bytes.
    fn sideband_progress(&mut self, _message: &[u8]) -> i32 {
        0
    }

    /// Reports completion of one remote phase.
    fn completion(&mut self, _completion: crate::remote::GitRemoteCompletion) -> i32 {
        0
    }

    /// Acquires an owned credential for an authentication attempt.
    fn credentials(
        &mut self,
        _url: &CStr,
        _username: Option<&CStr>,
        _allowed: crate::transports::credential::GitCredentialType,
    ) -> Result<crate::transports::credential::GitCredentialOwned, i32> {
        Err(crate::api::errors::GitErrorCode::Passthrough.into())
    }

    /// Makes the final certificate decision for a connection.
    fn certificate_check(
        &mut self,
        _certificate: crate::api::cert::GitCertRef<'_>,
        _valid: bool,
        _host: &CStr,
    ) -> i32 {
        crate::api::errors::GitErrorCode::Passthrough.into()
    }

    /// Receives download/indexing progress.
    fn transfer_progress(&mut self, _progress: crate::indexer::IndexerProgressRef<'_>) -> i32 {
        0
    }

    /// Receives the deprecated per-reference update notification.
    fn update_tips(
        &mut self,
        _refname: &CStr,
        _old: crate::oid::OidRef<'_>,
        _new: crate::oid::OidRef<'_>,
    ) -> i32 {
        0
    }

    /// Receives packbuilder progress.
    fn pack_progress(
        &mut self,
        _stage: crate::api::pack::GitPackbuilderStage,
        _current: u32,
        _total: u32,
    ) -> i32 {
        0
    }

    /// Receives push upload progress.
    fn push_transfer_progress(&mut self, _current: u32, _total: u32, _bytes: usize) -> i32 {
        0
    }

    /// Receives one remote-side reference update result.
    fn push_update_reference(&mut self, _refname: &CStr, _status: Option<&CStr>) -> i32 {
        0
    }

    /// Receives the updates negotiated for a push.
    fn push_negotiation(&mut self, _updates: GitPushUpdates<'_>) -> i32 {
        0
    }

    /// Optionally creates a custom transport. `Ok(None)` selects libgit2's
    /// registered transport for the URL.
    fn transport(
        &mut self,
        _remote: crate::remote::GitRemoteMut<'_>,
    ) -> Result<Option<crate::sys::transport::GitTransportOwned>, i32> {
        Ok(None)
    }

    /// Runs immediately before connecting a remote.
    fn remote_ready(
        &mut self,
        _remote: crate::remote::GitRemoteMut<'_>,
        _direction: Direction,
    ) -> i32 {
        0
    }

    /// Resolves a remote URL through the deprecated callback.
    fn resolve_url(
        &mut self,
        _output: &mut GitBufMut<'_>,
        _url: &CStr,
        _direction: Direction,
    ) -> i32 {
        crate::api::errors::GitErrorCode::Passthrough.into()
    }

    /// Receives the preferred local-reference update notification.
    fn update_refs(
        &mut self,
        _refname: &CStr,
        _old: crate::oid::OidRef<'_>,
        _new: crate::oid::OidRef<'_>,
        _spec: crate::refspec::GitRefspecRef<'_>,
    ) -> i32 {
        0
    }
}

/// Wraps: git_remote_callbacks
/// Layout-compatible remote callback table borrowing one typed receiver for
/// the table's data lifetime.
#[repr(transparent)]
pub struct GitRemoteCallbacks<'data> {
    inner: ffibox::CType<crate::ffi::git_remote_callbacks>,
    _data: PhantomData<&'data mut ()>,
}

/// Shared borrow of [`GitRemoteCallbacks`].
#[repr(transparent)]
pub struct GitRemoteCallbacksRef<'object, 'data>(ffibox::CPtr<'object, GitRemoteCallbacks<'data>>);

impl Clone for GitRemoteCallbacksRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitRemoteCallbacksRef<'_, '_> {}

/// Exclusive borrow of [`GitRemoteCallbacks`].
#[repr(transparent)]
pub struct GitRemoteCallbacksMut<'object, 'data>(GitRemoteCallbacksRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> ffibox::CCell for GitRemoteCallbacks<'data> {
    type C = crate::ffi::git_remote_callbacks;
    type Ref<'object>
        = GitRemoteCallbacksRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitRemoteCallbacksMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(p: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `p` is a live shared object.
        GitRemoteCallbacksRef(unsafe { ffibox::CPtr::new(p) })
    }

    unsafe fn mut_from_raw<'object>(p: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitRemoteCallbacksMut(GitRemoteCallbacksRef(unsafe { ffibox::CPtr::new(p) }))
    }
}

// SAFETY: the table owns no data; function pointers and the payload are
// borrowed and are never released when inline table storage is disposed.
unsafe impl ffibox::CValued for GitRemoteCallbacks<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitRemoteCallbacks<'data> {
    /// Constructs a table equivalent to `GIT_REMOTE_CALLBACKS_INIT`.
    #[must_use]
    pub fn new() -> ffibox::CVal<Self> {
        // SAFETY: every raw field admits zero; the ABI version is installed
        // before the initialized wrapper is returned.
        let inner = unsafe { ffibox::CType::zeroed() };
        let mut callbacks = ffibox::CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        callbacks
            .as_mut()
            .set_version(crate::ffi::GIT_REMOTE_CALLBACKS_VERSION);
        callbacks
    }
}

impl<'object, 'data> GitRemoteCallbacksRef<'object, 'data> {
    /// Borrows a raw callback table, returning `None` for null.
    ///
    /// # Safety
    /// `ptr` must identify an initialized table live for `'object`. Every
    /// installed callback and its shared payload must remain valid and
    /// exclusively reserved for `'data`, which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut crate::ffi::git_remote_callbacks) -> Option<Self> {
        NonNull::new(ptr.cast::<GitRemoteCallbacks<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared table.
            Self(unsafe { ffibox::CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const crate::ffi::git_remote_callbacks {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_remote_callbacks.payload
    /// Reports whether shared callback state is installed.
    #[must_use]
    pub fn has_payload(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized opaque pointer.
        !unsafe { core::ptr::addr_of!((*self.as_ptr()).payload).read() }.is_null()
    }

    /// Field: git_remote_callbacks.version
    /// Returns the callback-table ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_remote_callbacks.reserved
    /// Reports the unavailable hard-deprecation reserved slot. This build
    /// publishes `resolve_url` in the same ABI position instead.
    #[must_use]
    pub const fn has_reserved(&self) -> bool {
        false
    }

    /// Field: git_remote_callbacks.transport
    #[must_use]
    pub fn has_transport(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).transport).read() }.is_some()
    }

    /// Field: git_remote_callbacks.update_refs
    #[must_use]
    pub fn has_update_refs(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).update_refs).read() }.is_some()
    }

    /// Field: git_remote_callbacks.resolve_url
    #[must_use]
    pub fn has_resolve_url(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).resolve_url).read() }.is_some()
    }

    /// Field: git_remote_callbacks.remote_ready
    #[must_use]
    pub fn has_remote_ready(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).remote_ready).read() }.is_some()
    }

    /// Field: git_remote_callbacks.push_negotiation
    #[must_use]
    pub fn has_push_negotiation(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).push_negotiation).read() }.is_some()
    }

    /// Field: git_remote_callbacks.push_update_reference
    #[must_use]
    pub fn has_push_update_reference(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).push_update_reference).read() }.is_some()
    }

    /// Field: git_remote_callbacks.push_transfer_progress
    #[must_use]
    pub fn has_push_transfer_progress(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).push_transfer_progress).read() }.is_some()
    }

    /// Field: git_remote_callbacks.pack_progress
    #[must_use]
    pub fn has_pack_progress(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).pack_progress).read() }.is_some()
    }

    /// Field: git_remote_callbacks.update_tips
    #[must_use]
    pub fn has_update_tips(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).update_tips).read() }.is_some()
    }

    /// Field: git_remote_callbacks.transfer_progress
    #[must_use]
    pub fn has_transfer_progress(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).transfer_progress).read() }.is_some()
    }

    /// Field: git_remote_callbacks.certificate_check
    #[must_use]
    pub fn has_certificate_check(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).certificate_check).read() }.is_some()
    }

    /// Field: git_remote_callbacks.credentials
    #[must_use]
    pub fn has_credentials(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).credentials).read() }.is_some()
    }

    /// Field: git_remote_callbacks.completion
    #[must_use]
    pub fn has_completion(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).completion).read() }.is_some()
    }

    /// Field: git_remote_callbacks.sideband_progress
    #[must_use]
    pub fn has_sideband_progress(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).sideband_progress).read() }.is_some()
    }

    /// Field: git_remote_callbacks.reserved_update_tips
    /// Reports the unavailable hard-deprecation reserved slot. This build
    /// publishes `update_tips` in the same ABI position instead.
    #[must_use]
    pub const fn has_reserved_update_tips(&self) -> bool {
        false
    }
}

impl<'object, 'data> GitRemoteCallbacksMut<'object, 'data> {
    /// Exclusively borrows a raw callback table, returning `None` for null.
    ///
    /// # Safety
    /// The shared-handle requirements apply, and no other access path may use
    /// the table for `'object`.
    pub unsafe fn from_ptr(ptr: *mut crate::ffi::git_remote_callbacks) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible table.
        unsafe { GitRemoteCallbacksRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut crate::ffi::git_remote_callbacks {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitRemoteCallbacksRef<'_, 'data> {
        GitRemoteCallbacksRef(self.0.0)
    }

    /// Replaces the callback-table ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Installs every callback trampoline over one typed receiver.
    ///
    /// # Safety
    /// Libgit2 may copy this table into longer-lived connection and push
    /// state. Every copy must stop being used before `handler` is released,
    /// and callback invocations must not overlap or reenter unless `H`
    /// internally prevents aliasing its mutable state.
    pub unsafe fn set_handler<H: GitRemoteCallbackHandler>(&mut self, handler: &'data mut H) {
        let table = self.as_mut_ptr();
        let payload = core::ptr::from_mut(handler).cast();
        // SAFETY: every trampoline agrees on payload type `H`; the wrapper
        // lifetime reserves that value exclusively, and this handle permits
        // the coherent callback-table writes.
        unsafe {
            core::ptr::addr_of_mut!((*table).sideband_progress).write(Some(sideband::<H>));
            core::ptr::addr_of_mut!((*table).completion).write(Some(completion::<H>));
            core::ptr::addr_of_mut!((*table).credentials).write(Some(credentials::<H>));
            core::ptr::addr_of_mut!((*table).certificate_check).write(Some(certificate::<H>));
            core::ptr::addr_of_mut!((*table).transfer_progress).write(Some(transfer::<H>));
            core::ptr::addr_of_mut!((*table).update_tips).write(Some(update_tips::<H>));
            core::ptr::addr_of_mut!((*table).pack_progress).write(Some(pack::<H>));
            core::ptr::addr_of_mut!((*table).push_transfer_progress)
                .write(Some(push_transfer::<H>));
            core::ptr::addr_of_mut!((*table).push_update_reference).write(Some(push_update::<H>));
            core::ptr::addr_of_mut!((*table).push_negotiation).write(Some(push_negotiation::<H>));
            core::ptr::addr_of_mut!((*table).transport).write(Some(transport::<H>));
            core::ptr::addr_of_mut!((*table).remote_ready).write(Some(remote_ready::<H>));
            core::ptr::addr_of_mut!((*table).resolve_url).write(Some(resolve_url::<H>));
            core::ptr::addr_of_mut!((*table).update_refs).write(Some(update_refs::<H>));
            core::ptr::addr_of_mut!((*table).payload).write(payload);
        }
    }

    /// Clears every callback and the shared receiver pointer together.
    pub fn clear_handler(&mut self) {
        let table = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits all coherent field writes.
        unsafe {
            core::ptr::addr_of_mut!((*table).sideband_progress).write(None);
            core::ptr::addr_of_mut!((*table).completion).write(None);
            core::ptr::addr_of_mut!((*table).credentials).write(None);
            core::ptr::addr_of_mut!((*table).certificate_check).write(None);
            core::ptr::addr_of_mut!((*table).transfer_progress).write(None);
            core::ptr::addr_of_mut!((*table).update_tips).write(None);
            core::ptr::addr_of_mut!((*table).pack_progress).write(None);
            core::ptr::addr_of_mut!((*table).push_transfer_progress).write(None);
            core::ptr::addr_of_mut!((*table).push_update_reference).write(None);
            core::ptr::addr_of_mut!((*table).push_negotiation).write(None);
            core::ptr::addr_of_mut!((*table).transport).write(None);
            core::ptr::addr_of_mut!((*table).remote_ready).write(None);
            core::ptr::addr_of_mut!((*table).resolve_url).write(None);
            core::ptr::addr_of_mut!((*table).update_refs).write(None);
            core::ptr::addr_of_mut!((*table).payload).write(core::ptr::null_mut());
        }
    }
}

unsafe extern "C" fn sideband<H: GitRemoteCallbackHandler>(
    message: *const core::ffi::c_char,
    len: core::ffi::c_int,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let Ok(len) = usize::try_from(len) else {
        return -1;
    };
    let message = if len == 0 {
        &[]
    } else if message.is_null() {
        return -1;
    } else {
        // SAFETY: libgit2 supplies exactly `len` transient readable bytes.
        unsafe { core::slice::from_raw_parts(message.cast::<u8>(), len) }
    };
    handler.sideband_progress(message)
}

unsafe extern "C" fn completion<H: GitRemoteCallbackHandler>(
    value: crate::ffi::git_remote_completion_t,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let Ok(value) = crate::remote::GitRemoteCompletion::try_from(value) else {
        return -1;
    };
    handler.completion(value)
}

unsafe extern "C" fn credentials<H: GitRemoteCallbackHandler>(
    out: *mut *mut crate::ffi::git_credential,
    url: *const core::ffi::c_char,
    username: *const core::ffi::c_char,
    allowed: core::ffi::c_uint,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let (Some(out), Some(url), Some(allowed)) = (
        NonNull::new(out),
        NonNull::new(url.cast_mut()),
        crate::transports::credential::GitCredentialType::from_bits(allowed),
    ) else {
        return -1;
    };
    // SAFETY: libgit2 supplies transient NUL-terminated callback strings.
    let url = unsafe { CStr::from_ptr(url.as_ptr()) };
    let username = NonNull::new(username.cast_mut()).map(|value| {
        // SAFETY: a non-null username is transient and NUL-terminated.
        unsafe { CStr::from_ptr(value.as_ptr()) }
    });
    match handler.credentials(url, username, allowed) {
        Ok(credential) => {
            // SAFETY: the validated output slot accepts the transferred owner.
            unsafe { out.as_ptr().write(credential.into_raw()) };
            0
        }
        Err(status) => status,
    }
}

unsafe extern "C" fn certificate<H: GitRemoteCallbackHandler>(
    certificate: *mut crate::ffi::git_cert,
    valid: core::ffi::c_int,
    host: *const core::ffi::c_char,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let (Some(certificate), Some(host)) = (
        // SAFETY: libgit2 supplies a live certificate for this invocation.
        unsafe { crate::api::cert::GitCertRef::from_ptr(certificate) },
        NonNull::new(host.cast_mut()),
    ) else {
        return -1;
    };
    // SAFETY: libgit2 supplies a transient NUL-terminated host.
    let host = unsafe { CStr::from_ptr(host.as_ptr()) };
    handler.certificate_check(certificate, valid != 0, host)
}

unsafe extern "C" fn transfer<H: GitRemoteCallbackHandler>(
    progress: *const crate::ffi::git_indexer_progress,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    // SAFETY: libgit2 supplies a live progress record for this invocation.
    let Some(progress) =
        (unsafe { crate::indexer::IndexerProgressRef::from_ptr(progress.cast_mut()) })
    else {
        return -1;
    };
    handler.transfer_progress(progress)
}

unsafe extern "C" fn update_tips<H: GitRemoteCallbackHandler>(
    refname: *const core::ffi::c_char,
    old: *const crate::ffi::git_oid,
    new: *const crate::ffi::git_oid,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let (Some(refname), Some(old), Some(new)) = (
        NonNull::new(refname.cast_mut()),
        // SAFETY: libgit2 supplies live OIDs for this callback invocation.
        unsafe { crate::oid::OidRef::from_ptr(old.cast_mut()) },
        // SAFETY: libgit2 supplies live OIDs for this callback invocation.
        unsafe { crate::oid::OidRef::from_ptr(new.cast_mut()) },
    ) else {
        return -1;
    };
    // SAFETY: libgit2 supplies a transient NUL-terminated reference name.
    handler.update_tips(unsafe { CStr::from_ptr(refname.as_ptr()) }, old, new)
}

unsafe extern "C" fn pack<H: GitRemoteCallbackHandler>(
    stage: core::ffi::c_int,
    current: u32,
    total: u32,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let Ok(stage) = crate::api::pack::GitPackbuilderStage::try_from(
        stage as crate::ffi::git_packbuilder_stage_t,
    ) else {
        return -1;
    };
    handler.pack_progress(stage, current, total)
}

unsafe extern "C" fn push_transfer<H: GitRemoteCallbackHandler>(
    current: u32,
    total: u32,
    bytes: usize,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    handler.push_transfer_progress(current, total, bytes)
}

unsafe extern "C" fn push_update<H: GitRemoteCallbackHandler>(
    refname: *const core::ffi::c_char,
    status: *const core::ffi::c_char,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let Some(refname) = NonNull::new(refname.cast_mut()) else {
        return -1;
    };
    // SAFETY: libgit2 supplies transient NUL-terminated callback strings.
    let refname = unsafe { CStr::from_ptr(refname.as_ptr()) };
    let status = NonNull::new(status.cast_mut()).map(|value| {
        // SAFETY: a non-null status is transient and NUL-terminated.
        unsafe { CStr::from_ptr(value.as_ptr()) }
    });
    handler.push_update_reference(refname, status)
}

unsafe extern "C" fn push_negotiation<H: GitRemoteCallbackHandler>(
    updates: *mut *const crate::ffi::git_push_update,
    len: usize,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    // SAFETY: libgit2 supplies a readable pointer run with live elements.
    let Some(updates) = (unsafe { GitPushUpdates::from_raw(updates.cast_const(), len) }) else {
        return -1;
    };
    handler.push_negotiation(updates)
}

unsafe extern "C" fn transport<H: GitRemoteCallbackHandler>(
    out: *mut *mut crate::ffi::git_transport,
    remote: *mut crate::ffi::git_remote,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let (Some(out), Some(remote)) = (
        NonNull::new(out),
        // SAFETY: libgit2 grants exclusive access for the callback invocation.
        unsafe { crate::remote::GitRemoteMut::from_ptr(remote) },
    ) else {
        return -1;
    };
    match handler.transport(remote) {
        Ok(transport) => {
            let raw = transport.map_or(core::ptr::null_mut(), ffibox::CBox::into_raw);
            // SAFETY: the validated output slot accepts null or the transferred owner.
            unsafe { out.as_ptr().write(raw) };
            0
        }
        Err(status) => status,
    }
}

unsafe extern "C" fn remote_ready<H: GitRemoteCallbackHandler>(
    remote: *mut crate::ffi::git_remote,
    direction: core::ffi::c_int,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let (Some(remote), Ok(direction)) = (
        // SAFETY: libgit2 grants exclusive access for the callback invocation.
        unsafe { crate::remote::GitRemoteMut::from_ptr(remote) },
        Direction::try_from(direction as crate::ffi::git_direction),
    ) else {
        return -1;
    };
    handler.remote_ready(remote, direction)
}

unsafe extern "C" fn resolve_url<H: GitRemoteCallbackHandler>(
    output: *mut crate::ffi::git_buf,
    url: *const core::ffi::c_char,
    direction: core::ffi::c_int,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let (Some(mut output), Some(url), Ok(direction)) = (
        // SAFETY: libgit2 supplies an exclusively writable output header.
        unsafe { GitBufMut::from_ptr(output) },
        NonNull::new(url.cast_mut()),
        Direction::try_from(direction as crate::ffi::git_direction),
    ) else {
        return -1;
    };
    // SAFETY: libgit2 supplies a transient NUL-terminated URL.
    handler.resolve_url(
        &mut output,
        unsafe { CStr::from_ptr(url.as_ptr()) },
        direction,
    )
}

unsafe extern "C" fn update_refs<H: GitRemoteCallbackHandler>(
    refname: *const core::ffi::c_char,
    old: *const crate::ffi::git_oid,
    new: *const crate::ffi::git_oid,
    spec: *mut crate::ffi::git_refspec,
    payload: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    let Some(mut handler) = NonNull::new(payload.cast::<H>()) else {
        return -1;
    };
    // SAFETY: the unsafe installation contract reserves this live `H`
    // exclusively and forbids overlapping or reentrant callback calls.
    let handler = unsafe { handler.as_mut() };
    let (Some(refname), Some(old), Some(new), Some(spec)) = (
        NonNull::new(refname.cast_mut()),
        // SAFETY: libgit2 supplies live callback records for this invocation.
        unsafe { crate::oid::OidRef::from_ptr(old.cast_mut()) },
        // SAFETY: libgit2 supplies live callback records for this invocation.
        unsafe { crate::oid::OidRef::from_ptr(new.cast_mut()) },
        // SAFETY: libgit2 supplies a live refspec for this invocation.
        unsafe { crate::refspec::GitRefspecRef::from_ptr(spec) },
    ) else {
        return -1;
    };
    // SAFETY: libgit2 supplies a transient NUL-terminated reference name.
    handler.update_refs(unsafe { CStr::from_ptr(refname.as_ptr()) }, old, new, spec)
}

#[cfg(test)]
mod remote_callbacks_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    struct Handler {
        bytes: usize,
    }

    impl GitRemoteCallbackHandler for Handler {
        fn sideband_progress(&mut self, message: &[u8]) -> i32 {
            self.bytes += message.len();
            self.bytes as i32
        }
    }

    #[test]
    fn callback_table_preserves_layout_and_defaults() {
        assert_eq!(
            size_of::<GitRemoteCallbacks<'static>>(),
            size_of::<crate::ffi::git_remote_callbacks>()
        );
        assert_eq!(
            align_of::<GitRemoteCallbacks<'static>>(),
            align_of::<crate::ffi::git_remote_callbacks>()
        );
        assert_eq!(
            size_of::<GitRemoteCallbacksRef<'static, 'static>>(),
            size_of::<*const crate::ffi::git_remote_callbacks>()
        );
        let callbacks = GitRemoteCallbacks::new();
        let callbacks = callbacks.as_ref();
        assert_eq!(
            callbacks.version(),
            crate::ffi::GIT_REMOTE_CALLBACKS_VERSION
        );
        assert!(!callbacks.has_payload());
        assert!(!callbacks.has_sideband_progress());
        assert!(!callbacks.has_reserved());
        assert!(!callbacks.has_reserved_update_tips());
    }

    #[test]
    fn typed_handler_is_installed_and_called_through_the_c_table() {
        let mut handler = Handler { bytes: 0 };
        let mut callbacks = GitRemoteCallbacks::new();
        // SAFETY: this local table is the only copy, calls are sequential, and
        // it is dropped before `handler` is inspected or released.
        unsafe { callbacks.as_mut().set_handler(&mut handler) };
        let view = callbacks.as_ref();
        assert!(view.has_payload());
        assert!(view.has_sideband_progress());
        let raw = view.as_ptr();
        // SAFETY: raw-place reads copy the coherently installed callback pair.
        let callback = unsafe { core::ptr::addr_of!((*raw).sideband_progress).read() }.unwrap();
        // SAFETY: as above; this does not dereference the opaque pointer.
        let payload = unsafe { core::ptr::addr_of!((*raw).payload).read() };
        // SAFETY: the message and typed receiver remain live for the call.
        assert_eq!(unsafe { callback(b"abc".as_ptr().cast(), 3, payload) }, 3);
        drop(callbacks);
        assert_eq!(handler.bytes, 3);
    }
}
