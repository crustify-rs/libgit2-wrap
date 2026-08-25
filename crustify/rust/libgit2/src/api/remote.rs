//! Safe wrappers for libgit2 remote APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

use crate::api::buffer::GitBufMut;
use crate::ffi;
use crate::remote::GitPushUpdateRef;
use crate::repository::GitRepositoryRef;
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
    ///
    /// Libgit2 calls the `update_tips` slot only when `update_refs` is unset,
    /// and [`GitRemoteCallbacksMut::set_handler`] installs every slot. The
    /// default [`Self::update_refs`] therefore forwards here, so a handler
    /// that implements only this method still observes every update, exactly
    /// as a C table that sets only the deprecated slot does.
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

    /// Optionally creates a custom transport for `remote`. `Ok(None)` selects
    /// libgit2's registered transport for the URL.
    ///
    /// A returned transport is transferred to libgit2, which attaches it to
    /// `remote` and releases it with the remote. Transports built for a remote
    /// retain it — `git_transport_smart` stores the owner in `transport_smart`
    /// and dereferences `owner->repo` on every connect — so the result is
    /// coupled to `'remote` rather than to a free-standing owner. This is the
    /// same contract as [`GitTransportCallback`](crate::api::transport::GitTransportCallback),
    /// which wraps the identical `git_transport_cb` slot on the registration
    /// side.
    fn transport<'remote>(
        &mut self,
        _remote: crate::remote::GitRemoteMut<'remote>,
    ) -> Result<Option<crate::sys::transport::GitTransportWithRemote<'remote>>, i32> {
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
    ///
    /// The default forwards to [`Self::update_tips`], because libgit2 prefers
    /// this slot whenever both are installed and `set_handler` always installs
    /// both. Overriding this method supersedes `update_tips`, matching a C
    /// table that sets both.
    fn update_refs(
        &mut self,
        refname: &CStr,
        old: crate::oid::OidRef<'_>,
        new: crate::oid::OidRef<'_>,
        _spec: crate::refspec::GitRefspecRef<'_>,
    ) -> i32 {
        self.update_tips(refname, old, new)
    }
}

/// Wraps: git_remote_callbacks
/// Layout-compatible remote callback table borrowing one typed receiver for
/// the table's data lifetime.
///
/// `'data` is invariant. [`GitRemoteCallbacksMut::set_handler`] stores a
/// `&'data mut` receiver in the payload slot, and libgit2 copies the whole
/// table into connection and push state that outlives the call. A covariant
/// `'data` would let safe code shrink the parameter on the exclusive handle,
/// install a shorter-lived receiver, and hand a table still typed at the
/// longer lifetime to an operation that then calls back into released state.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::remote::GitRemoteCallbacksMut;
///
/// fn shrink<'object, 'short>(
///     callbacks: GitRemoteCallbacksMut<'object, 'static>,
/// ) -> GitRemoteCallbacksMut<'object, 'short> {
///     callbacks
/// }
/// ```
#[repr(transparent)]
pub struct GitRemoteCallbacks<'data> {
    inner: ffibox::CType<crate::ffi::git_remote_callbacks>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
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
    /// Every slot is filled, so libgit2's preference for `update_refs` over
    /// the deprecated `update_tips` always applies; see
    /// [`GitRemoteCallbackHandler::update_tips`] for how the trait keeps both
    /// reachable.
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
            let raw = transport.map_or(
                core::ptr::null_mut(),
                crate::sys::transport::GitTransportWithRemote::into_raw,
            );
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
    use core::sync::atomic::{AtomicUsize, Ordering};

    use crate::sys::transport::{GitTransportOwned, GitTransportWithRemote};

    use super::*;

    static FACTORY_TRANSPORT_FREES: AtomicUsize = AtomicUsize::new(0);

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

    unsafe extern "C" fn free_factory_transport(transport: *mut crate::ffi::git_transport) {
        FACTORY_TRANSPORT_FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: this destructor is installed only on the unique `Box`
        // allocation the factory below hands to the C seam.
        drop(unsafe { Box::from_raw(transport) });
    }

    /// Builds a transport whose Rust type records the remote it is coupled to,
    /// exactly as `git_transport_smart` records `t->owner`.
    struct TransportFactory;

    impl GitRemoteCallbackHandler for TransportFactory {
        fn transport<'remote>(
            &mut self,
            remote: crate::remote::GitRemoteMut<'remote>,
        ) -> Result<Option<GitTransportWithRemote<'remote>>, i32> {
            // SAFETY: every field of the bindgen vtable admits the all-zero bit
            // pattern; each callback slot is a nullable `Option<fn>`.
            let mut raw: crate::ffi::git_transport = unsafe { core::mem::zeroed() };
            raw.version = crate::ffi::GIT_TRANSPORT_VERSION;
            raw.free = Some(free_factory_transport);
            let raw = Box::into_raw(Box::new(raw));
            // SAFETY: `raw` is a unique fully initialized transport whose
            // installed destructor reclaims that exact allocation.
            let owned = unsafe { GitTransportOwned::from_raw(raw) }.unwrap();
            Ok(Some(GitTransportWithRemote::from_owned(owned, remote)))
        }
    }

    /// Opaque stand-in storage for the remote the trampoline borrows. Nothing
    /// dereferences it: the factory only records the borrow.
    fn remote_storage() -> *mut crate::ffi::git_remote {
        Box::into_raw(Box::new(
            core::mem::MaybeUninit::<crate::ffi::git_remote>::zeroed(),
        ))
        .cast()
    }

    /// Releases the storage `remote_storage` produced.
    ///
    /// # Safety
    ///
    /// `raw` must be a pointer `remote_storage` returned and not yet released,
    /// with no surviving handle over it.
    unsafe fn drop_remote_storage(raw: *mut crate::ffi::git_remote) {
        // SAFETY: the caller supplies exactly that allocation, which this
        // recovers without interpreting its opaque contents.
        drop(unsafe {
            Box::from_raw(raw.cast::<core::mem::MaybeUninit<crate::ffi::git_remote>>())
        });
    }

    /// Opaque stand-in storage for the refspec the update trampoline borrows.
    /// Nothing dereferences it: `update_refs` only forwards the handle.
    fn refspec_storage() -> *mut crate::ffi::git_refspec {
        Box::into_raw(Box::new(
            core::mem::MaybeUninit::<crate::ffi::git_refspec>::zeroed(),
        ))
        .cast()
    }

    /// Releases the storage `refspec_storage` produced.
    ///
    /// # Safety
    ///
    /// `raw` must be a pointer `refspec_storage` returned and not yet
    /// released, with no surviving handle over it.
    unsafe fn drop_refspec_storage(raw: *mut crate::ffi::git_refspec) {
        // SAFETY: the caller supplies exactly that allocation, which this
        // recovers without interpreting its opaque contents.
        drop(unsafe {
            Box::from_raw(raw.cast::<core::mem::MaybeUninit<crate::ffi::git_refspec>>())
        });
    }

    /// A handler that implements only the deprecated notification, as a C
    /// caller filling just `update_tips` would.
    struct DeprecatedTipsOnly {
        tips: Vec<std::ffi::CString>,
    }

    impl GitRemoteCallbackHandler for DeprecatedTipsOnly {
        fn update_tips(
            &mut self,
            refname: &CStr,
            _old: crate::oid::OidRef<'_>,
            _new: crate::oid::OidRef<'_>,
        ) -> i32 {
            self.tips.push(refname.to_owned());
            0
        }
    }

    #[test]
    fn update_refs_reaches_a_handler_that_only_implements_update_tips() {
        // `set_handler` fills both slots and libgit2 then calls `update_refs`
        // alone, so the deprecated method is reachable only by delegation.
        let mut handler = DeprecatedTipsOnly { tips: Vec::new() };
        let raw_spec = refspec_storage();
        let old = crate::oid::Oid::zeroed();
        let new = crate::oid::Oid::zeroed();
        // SAFETY: both IDs and the opaque refspec storage are live for the
        // call, the name is NUL-terminated, and `handler` is the live typed
        // receiver this trampoline is instantiated for.
        let status = unsafe {
            update_refs::<DeprecatedTipsOnly>(
                c"refs/heads/main".as_ptr(),
                core::ptr::addr_of!(old).cast(),
                core::ptr::addr_of!(new).cast(),
                raw_spec,
                core::ptr::addr_of_mut!(handler).cast(),
            )
        };
        assert_eq!(status, 0);
        assert_eq!(
            handler.tips,
            [std::ffi::CString::new("refs/heads/main").unwrap()]
        );
        // SAFETY: no handle to the refspec storage survives.
        unsafe { drop_refspec_storage(raw_spec) };
    }

    #[test]
    fn transport_trampoline_transfers_a_remote_coupled_owner() {
        let before = FACTORY_TRANSPORT_FREES.load(Ordering::SeqCst);
        let mut handler = TransportFactory;
        let raw_remote = remote_storage();
        let mut out: *mut crate::ffi::git_transport = core::ptr::null_mut();
        // SAFETY: the output slot is writable, the opaque remote storage is
        // live and exclusively borrowed for the call, and `handler` is the
        // live typed receiver this trampoline is instantiated for.
        let status = unsafe {
            transport::<TransportFactory>(
                core::ptr::addr_of_mut!(out),
                raw_remote,
                core::ptr::addr_of_mut!(handler).cast(),
            )
        };
        assert_eq!(status, 0);
        assert!(!out.is_null());
        // SAFETY: the trampoline surrendered exactly one owner through `out`;
        // libgit2 would take it here, so this test releases it instead.
        drop(unsafe { GitTransportOwned::from_raw(out) }.unwrap());
        assert_eq!(FACTORY_TRANSPORT_FREES.load(Ordering::SeqCst), before + 1);
        // SAFETY: no handle to the remote storage survives.
        unsafe { drop_remote_storage(raw_remote) };
    }

    #[test]
    fn transport_trampoline_defers_to_libgit2_without_a_factory() {
        let mut handler = Handler { bytes: 0 };
        let raw_remote = remote_storage();
        let mut sentinel = 0u8;
        // A non-null starting value: the trampoline must overwrite the slot,
        // not merely leave whatever libgit2 had there.
        let mut out: *mut crate::ffi::git_transport = core::ptr::addr_of_mut!(sentinel).cast();
        // SAFETY: as above; the default handler builds no transport and the
        // trampoline must still initialize the output slot.
        let status = unsafe {
            transport::<Handler>(
                core::ptr::addr_of_mut!(out),
                raw_remote,
                core::ptr::addr_of_mut!(handler).cast(),
            )
        };
        assert_eq!(status, 0);
        assert!(out.is_null());
        // SAFETY: no handle to the remote storage survives.
        unsafe { drop_remote_storage(raw_remote) };
    }
}

/// Wraps: git_fetch_options
/// Layout-compatible fetch options borrowing their nested callback, proxy and
/// string-array data for `'data`.
///
/// `'data` is invariant. [`GitFetchOptionsMut::set_custom_headers`] stores a
/// `&'data` string run in the C struct, and the embedded callback and proxy
/// headers reached through [`GitFetchOptionsMut::callbacks_mut`] and
/// [`GitFetchOptionsMut::proxy_options_mut`] inherit `'data` and store their
/// own referents the same way. Every one of those is read back out through a
/// shared handle, so a covariant `'data` would let safe code shrink the
/// parameter on the exclusive handle, install a shorter-lived proxy URL,
/// header array or callback receiver, and then read the released storage back
/// through a handle still typed at the longer lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::remote::GitFetchOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitFetchOptionsMut<'object, 'static>,
/// ) -> GitFetchOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitFetchOptions<'data> {
    inner: ffibox::CType<crate::ffi::git_fetch_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitFetchOptions`].
#[repr(transparent)]
pub struct GitFetchOptionsRef<'object, 'data>(ffibox::CPtr<'object, GitFetchOptions<'data>>);

impl Clone for GitFetchOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitFetchOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitFetchOptions`].
#[repr(transparent)]
pub struct GitFetchOptionsMut<'object, 'data>(GitFetchOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> ffibox::CCell for GitFetchOptions<'data> {
    type C = crate::ffi::git_fetch_options;
    type Ref<'object>
        = GitFetchOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitFetchOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(p: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `p` is a live shared object.
        GitFetchOptionsRef(unsafe { ffibox::CPtr::new(p) })
    }

    unsafe fn mut_from_raw<'object>(p: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitFetchOptionsMut(GitFetchOptionsRef(unsafe { ffibox::CPtr::new(p) }))
    }
}

// SAFETY: fetch options own no resource. Their embedded callback, proxy and
// string-array headers borrow data that the wrapper lifetime keeps alive.
unsafe impl ffibox::CValued for GitFetchOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitFetchOptions<'data> {
    /// Constructs options equivalent to `GIT_FETCH_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> ffibox::CVal<Self> {
        // SAFETY: every raw field admits zero. Required versions and nonzero
        // defaults are installed before the initialized wrapper is returned.
        let inner = unsafe { ffibox::CType::zeroed() };
        let mut options = ffibox::CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(crate::ffi::GIT_FETCH_OPTIONS_VERSION as core::ffi::c_int);
            // `GIT_REMOTE_UPDATE_FETCHHEAD` is bit zero of the public update
            // flags stored in this historically scalar field.
            view.set_update_flags(1);
            view.callbacks_mut()
                .set_version(crate::ffi::GIT_REMOTE_CALLBACKS_VERSION);
            view.proxy_options_mut()
                .set_version(crate::ffi::GIT_PROXY_OPTIONS_VERSION);
        }
        options
    }
}

impl<'object, 'data> GitFetchOptionsRef<'object, 'data> {
    /// Borrows a raw options pointer, returning `None` for null.
    ///
    /// # Safety
    /// `ptr` must identify initialized options live for `'object`. Every
    /// callback payload, proxy URL and string reachable through the options
    /// must remain live for `'data`, which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut crate::ffi::git_fetch_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitFetchOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { ffibox::CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const crate::ffi::git_fetch_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_fetch_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_int {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_fetch_options.depth
    /// Returns the requested history depth, rejecting negative C values.
    pub fn depth(&self) -> Result<GitFetchDepth, core::ffi::c_int> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let depth = unsafe { core::ptr::addr_of!((*self.as_ptr()).depth).read() };
        GitFetchDepth::try_from(depth)
    }

    /// Field: git_fetch_options.callbacks
    /// Borrows the embedded callback table.
    #[must_use]
    pub fn callbacks(&self) -> GitRemoteCallbacksRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { core::ptr::addr_of!((*self.as_ptr()).callbacks).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { GitRemoteCallbacksRef::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Field: git_fetch_options.custom_headers
    /// Borrows the counted array of extra HTTP headers.
    #[must_use]
    pub fn custom_headers(&self) -> crate::strarray::GitStrArrayRef<'object> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { core::ptr::addr_of!((*self.as_ptr()).custom_headers).cast_mut() };
        // SAFETY: the projected header and every entry live for the enclosing
        // options borrow by the wrapper's data-lifetime contract.
        unsafe { crate::strarray::GitStrArrayRef::from_ptr(field) }
            .expect("an inline field is non-null")
    }

    /// Field: git_fetch_options.follow_redirects
    /// Returns the redirect policy, with `None` meaning consult configuration.
    pub fn follow_redirects(
        &self,
    ) -> Result<Option<crate::remote::GitRemoteRedirect>, crate::ffi::git_remote_redirect_t> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let value = unsafe { core::ptr::addr_of!((*self.as_ptr()).follow_redirects).read() };
        crate::remote::GitRemoteRedirect::from_field(value)
    }

    /// Field: git_fetch_options.proxy_opts
    /// Borrows the embedded proxy options.
    #[must_use]
    pub fn proxy_options(&self) -> crate::api::proxy::GitProxyOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { core::ptr::addr_of!((*self.as_ptr()).proxy_opts).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { crate::api::proxy::GitProxyOptionsRef::from_ptr(field) }
            .expect("an inline field is non-null")
    }

    /// Field: git_fetch_options.download_tags
    /// Returns the tag-download policy, rejecting unknown C values.
    pub fn download_tags(
        &self,
    ) -> Result<crate::remote::GitRemoteAutotagOption, crate::ffi::git_remote_autotag_option_t>
    {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let value = unsafe { core::ptr::addr_of!((*self.as_ptr()).download_tags).read() };
        crate::remote::GitRemoteAutotagOption::try_from(value)
    }

    /// Field: git_fetch_options.update_fetchhead
    /// Returns the `git_remote_update_flags` bit set for reference updates.
    #[must_use]
    pub fn update_flags(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).update_fetchhead).read() }
    }

    /// Field: git_fetch_options.prune
    /// Returns the pruning policy, rejecting unknown C values.
    pub fn prune(&self) -> Result<crate::remote::GitFetchPrune, crate::ffi::git_fetch_prune_t> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let value = unsafe { core::ptr::addr_of!((*self.as_ptr()).prune).read() };
        crate::remote::GitFetchPrune::try_from(value)
    }
}

impl<'object, 'data> GitFetchOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw options pointer, returning `None` for null.
    ///
    /// # Safety
    /// The shared-handle requirements apply, and no other access path may use
    /// the options for `'object`.
    pub unsafe fn from_ptr(ptr: *mut crate::ffi::git_fetch_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitFetchOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut crate::ffi::git_fetch_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitFetchOptionsRef<'_, 'data> {
        GitFetchOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_int) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Replaces the requested history depth.
    pub fn set_depth(&mut self, depth: GitFetchDepth) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).depth).write(depth.value()) }
    }

    /// Replaces the `git_remote_update_flags` bit set.
    pub fn set_update_flags(&mut self, flags: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).update_fetchhead).write(flags) }
    }

    /// Replaces the tag-download policy.
    pub fn set_download_tags(&mut self, option: crate::remote::GitRemoteAutotagOption) {
        // SAFETY: this exclusive handle permits the write, and the Rust enum
        // contains only published C values.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).download_tags).write(option.into()) }
    }

    /// Replaces the pruning policy.
    pub fn set_prune(&mut self, prune: crate::remote::GitFetchPrune) {
        // SAFETY: this exclusive handle permits the write, and the Rust enum
        // contains only published C values.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).prune).write(prune.into()) }
    }

    /// Replaces the redirect policy; `None` selects configuration lookup.
    pub fn set_follow_redirects(&mut self, redirects: Option<crate::remote::GitRemoteRedirect>) {
        let redirects = crate::remote::GitRemoteRedirect::to_field(redirects);
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).follow_redirects).write(redirects) }
    }

    /// Copies a borrowed string-array header into the custom-header field.
    pub fn set_custom_headers(&mut self, headers: crate::strarray::GitStrArrayRef<'data>) {
        // SAFETY: the source is an initialized header; copying it transfers no
        // ownership, and the outer data lifetime keeps its storage live.
        let headers = unsafe { headers.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).custom_headers).write(headers) }
    }

    /// Copies a callback table into the inline field.
    ///
    /// # Safety
    /// No callback invocation may overlap between the source table, this copy,
    /// or any further C copies. The payload remains exclusively reserved for
    /// `'data`.
    pub unsafe fn set_callbacks(&mut self, callbacks: GitRemoteCallbacksRef<'_, 'data>) {
        // SAFETY: the source identifies an initialized borrowed table.
        let callbacks = unsafe { callbacks.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).callbacks).write(callbacks) }
    }

    /// Copies proxy options into the inline field.
    ///
    /// # Safety
    /// If callbacks are configured, invocations through this and all copied
    /// headers must not overlap. Their shared payload remains exclusively
    /// reserved for `'data`.
    pub unsafe fn set_proxy_options(
        &mut self,
        options: crate::api::proxy::GitProxyOptionsRef<'_, 'data>,
    ) {
        // SAFETY: the source identifies an initialized borrowed header.
        let options = unsafe { options.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).proxy_opts).write(options) }
    }

    /// Exclusively borrows the embedded callback table.
    #[must_use]
    pub fn callbacks_mut(&mut self) -> GitRemoteCallbacksMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).callbacks) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime.
        unsafe { GitRemoteCallbacksMut::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Exclusively borrows the embedded custom-header array.
    #[must_use]
    pub fn custom_headers_mut(&mut self) -> crate::strarray::GitStrArrayMut<'_> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).custom_headers) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime.
        unsafe { crate::strarray::GitStrArrayMut::from_ptr(field) }
            .expect("an inline field is non-null")
    }

    /// Exclusively borrows the embedded proxy options.
    #[must_use]
    pub fn proxy_options_mut(&mut self) -> crate::api::proxy::GitProxyOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).proxy_opts) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime.
        unsafe { crate::api::proxy::GitProxyOptionsMut::from_ptr(field) }
            .expect("an inline field is non-null")
    }
}

/// Wraps: git_push_options
/// Layout-compatible push options borrowing their nested callback, proxy and
/// string-array data for `'data`.
///
/// `'data` is invariant, for the reason given on [`GitFetchOptions`].
/// [`GitPushOptionsMut::set_custom_headers`] and
/// [`GitPushOptionsMut::set_remote_push_options`] each store a `&'data` string
/// run, the embedded callback and proxy headers inherit `'data`, and the
/// shared handle hands all of them back out.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::remote::GitPushOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitPushOptionsMut<'object, 'static>,
/// ) -> GitPushOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitPushOptions<'data> {
    inner: ffibox::CType<crate::ffi::git_push_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitPushOptions`].
#[repr(transparent)]
pub struct GitPushOptionsRef<'object, 'data>(ffibox::CPtr<'object, GitPushOptions<'data>>);

impl Clone for GitPushOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitPushOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitPushOptions`].
#[repr(transparent)]
pub struct GitPushOptionsMut<'object, 'data>(GitPushOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> ffibox::CCell for GitPushOptions<'data> {
    type C = crate::ffi::git_push_options;
    type Ref<'object>
        = GitPushOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitPushOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(p: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `p` is a live shared object.
        GitPushOptionsRef(unsafe { ffibox::CPtr::new(p) })
    }

    unsafe fn mut_from_raw<'object>(p: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitPushOptionsMut(GitPushOptionsRef(unsafe { ffibox::CPtr::new(p) }))
    }
}

// SAFETY: push options own no resource. Their embedded callback, proxy and
// string-array headers borrow data that the wrapper lifetime keeps alive.
unsafe impl ffibox::CValued for GitPushOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitPushOptions<'data> {
    /// Constructs options equivalent to `GIT_PUSH_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> ffibox::CVal<Self> {
        // SAFETY: every raw field admits zero. Required versions and nonzero
        // defaults are installed before the initialized wrapper is returned.
        let inner = unsafe { ffibox::CType::zeroed() };
        let mut options = ffibox::CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(crate::ffi::GIT_PUSH_OPTIONS_VERSION);
            view.set_packbuilder_parallelism(1);
            view.callbacks_mut()
                .set_version(crate::ffi::GIT_REMOTE_CALLBACKS_VERSION);
            view.proxy_options_mut()
                .set_version(crate::ffi::GIT_PROXY_OPTIONS_VERSION);
        }
        options
    }
}

impl<'object, 'data> GitPushOptionsRef<'object, 'data> {
    /// Borrows a raw options pointer, returning `None` for null.
    ///
    /// # Safety
    /// `ptr` must identify initialized options live for `'object`. Every
    /// callback payload, proxy URL and string reachable through the options
    /// must remain live for `'data`, which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut crate::ffi::git_push_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitPushOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { ffibox::CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const crate::ffi::git_push_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_push_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_push_options.callbacks
    /// Borrows the embedded callback table.
    #[must_use]
    pub fn callbacks(&self) -> GitRemoteCallbacksRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { core::ptr::addr_of!((*self.as_ptr()).callbacks).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { GitRemoteCallbacksRef::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Field: git_push_options.pb_parallelism
    /// Returns the packbuilder worker count, with zero selecting auto-detect.
    #[must_use]
    pub fn packbuilder_parallelism(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).pb_parallelism).read() }
    }

    /// Field: git_push_options.remote_push_options
    /// Borrows the options delivered to the remote server.
    #[must_use]
    pub fn remote_push_options(&self) -> crate::strarray::GitStrArrayRef<'object> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { core::ptr::addr_of!((*self.as_ptr()).remote_push_options).cast_mut() };
        // SAFETY: the projected header and every entry live for the enclosing
        // options borrow by the wrapper's data-lifetime contract.
        unsafe { crate::strarray::GitStrArrayRef::from_ptr(field) }
            .expect("an inline field is non-null")
    }

    /// Field: git_push_options.custom_headers
    /// Borrows the counted array of extra HTTP headers.
    #[must_use]
    pub fn custom_headers(&self) -> crate::strarray::GitStrArrayRef<'object> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { core::ptr::addr_of!((*self.as_ptr()).custom_headers).cast_mut() };
        // SAFETY: the projected header and every entry live for the enclosing
        // options borrow by the wrapper's data-lifetime contract.
        unsafe { crate::strarray::GitStrArrayRef::from_ptr(field) }
            .expect("an inline field is non-null")
    }

    /// Field: git_push_options.follow_redirects
    /// Returns the redirect policy, with `None` meaning consult configuration.
    pub fn follow_redirects(
        &self,
    ) -> Result<Option<crate::remote::GitRemoteRedirect>, crate::ffi::git_remote_redirect_t> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let value = unsafe { core::ptr::addr_of!((*self.as_ptr()).follow_redirects).read() };
        crate::remote::GitRemoteRedirect::from_field(value)
    }

    /// Field: git_push_options.proxy_opts
    /// Borrows the embedded proxy options.
    #[must_use]
    pub fn proxy_options(&self) -> crate::api::proxy::GitProxyOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { core::ptr::addr_of!((*self.as_ptr()).proxy_opts).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { crate::api::proxy::GitProxyOptionsRef::from_ptr(field) }
            .expect("an inline field is non-null")
    }
}

impl<'object, 'data> GitPushOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw options pointer, returning `None` for null.
    ///
    /// # Safety
    /// The shared-handle requirements apply, and no other access path may use
    /// the options for `'object`.
    pub unsafe fn from_ptr(ptr: *mut crate::ffi::git_push_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitPushOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut crate::ffi::git_push_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitPushOptionsRef<'_, 'data> {
        GitPushOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Replaces the packbuilder worker count; zero selects auto-detect.
    pub fn set_packbuilder_parallelism(&mut self, parallelism: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).pb_parallelism).write(parallelism) }
    }

    /// Replaces the redirect policy; `None` selects configuration lookup.
    pub fn set_follow_redirects(&mut self, redirects: Option<crate::remote::GitRemoteRedirect>) {
        let redirects = crate::remote::GitRemoteRedirect::to_field(redirects);
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).follow_redirects).write(redirects) }
    }

    /// Copies a borrowed string-array header into the custom-header field.
    pub fn set_custom_headers(&mut self, headers: crate::strarray::GitStrArrayRef<'data>) {
        // SAFETY: the source is an initialized header; copying it transfers no
        // ownership, and the outer data lifetime keeps its storage live.
        let headers = unsafe { headers.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).custom_headers).write(headers) }
    }

    /// Copies a borrowed string-array header into the remote-option field.
    pub fn set_remote_push_options(&mut self, options: crate::strarray::GitStrArrayRef<'data>) {
        // SAFETY: the source is an initialized header; copying it transfers no
        // ownership, and the outer data lifetime keeps its storage live.
        let options = unsafe { options.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).remote_push_options).write(options) }
    }

    /// Copies a callback table into the inline field.
    ///
    /// # Safety
    /// No callback invocation may overlap between the source table, this copy,
    /// or any further C copies. The payload remains exclusively reserved for
    /// `'data`.
    pub unsafe fn set_callbacks(&mut self, callbacks: GitRemoteCallbacksRef<'_, 'data>) {
        // SAFETY: the source identifies an initialized borrowed table.
        let callbacks = unsafe { callbacks.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).callbacks).write(callbacks) }
    }

    /// Copies proxy options into the inline field.
    ///
    /// # Safety
    /// If callbacks are configured, invocations through this and all copied
    /// headers must not overlap. Their shared payload remains exclusively
    /// reserved for `'data`.
    pub unsafe fn set_proxy_options(
        &mut self,
        options: crate::api::proxy::GitProxyOptionsRef<'_, 'data>,
    ) {
        // SAFETY: the source identifies an initialized borrowed header.
        let options = unsafe { options.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).proxy_opts).write(options) }
    }

    /// Exclusively borrows the embedded callback table.
    #[must_use]
    pub fn callbacks_mut(&mut self) -> GitRemoteCallbacksMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).callbacks) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime.
        unsafe { GitRemoteCallbacksMut::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Exclusively borrows the embedded custom-header array.
    #[must_use]
    pub fn custom_headers_mut(&mut self) -> crate::strarray::GitStrArrayMut<'_> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).custom_headers) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime.
        unsafe { crate::strarray::GitStrArrayMut::from_ptr(field) }
            .expect("an inline field is non-null")
    }

    /// Exclusively borrows the embedded remote push-option array.
    #[must_use]
    pub fn remote_push_options_mut(&mut self) -> crate::strarray::GitStrArrayMut<'_> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).remote_push_options) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime.
        unsafe { crate::strarray::GitStrArrayMut::from_ptr(field) }
            .expect("an inline field is non-null")
    }

    /// Exclusively borrows the embedded proxy options.
    #[must_use]
    pub fn proxy_options_mut(&mut self) -> crate::api::proxy::GitProxyOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).proxy_opts) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime.
        unsafe { crate::api::proxy::GitProxyOptionsMut::from_ptr(field) }
            .expect("an inline field is non-null")
    }
}

#[cfg(test)]
mod fetch_and_push_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn option_wrappers_preserve_the_c_layout() {
        assert_eq!(
            size_of::<GitFetchOptions<'static>>(),
            size_of::<crate::ffi::git_fetch_options>()
        );
        assert_eq!(
            align_of::<GitFetchOptions<'static>>(),
            align_of::<crate::ffi::git_fetch_options>()
        );
        assert_eq!(
            size_of::<GitPushOptions<'static>>(),
            size_of::<crate::ffi::git_push_options>()
        );
        assert_eq!(
            align_of::<GitPushOptions<'static>>(),
            align_of::<crate::ffi::git_push_options>()
        );
    }

    #[test]
    fn constructors_install_every_published_default() {
        let fetch = GitFetchOptions::new();
        let fetch = fetch.as_ref();
        assert_eq!(
            fetch.version(),
            crate::ffi::GIT_FETCH_OPTIONS_VERSION as core::ffi::c_int
        );
        assert_eq!(fetch.depth(), Ok(GitFetchDepth::FULL));
        assert_eq!(fetch.prune(), Ok(crate::remote::GitFetchPrune::Unspecified));
        assert_eq!(
            fetch.download_tags(),
            Ok(crate::remote::GitRemoteAutotagOption::Unspecified)
        );
        assert_eq!(fetch.update_flags(), 1);
        assert_eq!(fetch.follow_redirects(), Ok(None));
        assert_eq!(
            fetch.callbacks().version(),
            crate::ffi::GIT_REMOTE_CALLBACKS_VERSION
        );
        assert_eq!(
            fetch.proxy_options().version(),
            crate::ffi::GIT_PROXY_OPTIONS_VERSION
        );
        assert_eq!(fetch.custom_headers().count(), 0);

        let push = GitPushOptions::new();
        let push = push.as_ref();
        assert_eq!(push.version(), crate::ffi::GIT_PUSH_OPTIONS_VERSION);
        assert_eq!(push.packbuilder_parallelism(), 1);
        assert_eq!(push.follow_redirects(), Ok(None));
        assert_eq!(
            push.callbacks().version(),
            crate::ffi::GIT_REMOTE_CALLBACKS_VERSION
        );
        assert_eq!(
            push.proxy_options().version(),
            crate::ffi::GIT_PROXY_OPTIONS_VERSION
        );
        assert_eq!(push.custom_headers().count(), 0);
        assert_eq!(push.remote_push_options().count(), 0);
    }

    #[test]
    fn exclusive_handles_update_scalars_and_project_nested_headers() {
        let headers = crate::strarray::GitStrArray::new();
        let mut fetch = GitFetchOptions::new();
        {
            let mut view = fetch.as_mut();
            view.set_depth(GitFetchDepth::new(7).unwrap());
            view.set_prune(crate::remote::GitFetchPrune::Prune);
            view.set_download_tags(crate::remote::GitRemoteAutotagOption::All);
            view.set_update_flags(2);
            view.set_follow_redirects(Some(crate::remote::GitRemoteRedirect::Initial));
            view.set_custom_headers(headers.as_ref());
            view.callbacks_mut().set_version(17);
            view.proxy_options_mut().set_version(19);
            assert_eq!(view.custom_headers_mut().as_ref().count(), 0);
        }
        let view = fetch.as_ref();
        assert_eq!(view.depth(), Ok(GitFetchDepth::new(7).unwrap()));
        assert_eq!(view.prune(), Ok(crate::remote::GitFetchPrune::Prune));
        assert_eq!(
            view.download_tags(),
            Ok(crate::remote::GitRemoteAutotagOption::All)
        );
        assert_eq!(view.update_flags(), 2);
        assert_eq!(
            view.follow_redirects(),
            Ok(Some(crate::remote::GitRemoteRedirect::Initial))
        );
        assert_eq!(view.callbacks().version(), 17);
        assert_eq!(view.proxy_options().version(), 19);

        let push_headers = crate::strarray::GitStrArray::new();
        let remote_options = crate::strarray::GitStrArray::new();
        let mut push = GitPushOptions::new();
        {
            let mut view = push.as_mut();
            view.set_packbuilder_parallelism(8);
            view.set_follow_redirects(Some(crate::remote::GitRemoteRedirect::All));
            view.set_custom_headers(push_headers.as_ref());
            view.set_remote_push_options(remote_options.as_ref());
            assert_eq!(view.remote_push_options_mut().as_ref().count(), 0);
        }
        let view = push.as_ref();
        assert_eq!(view.packbuilder_parallelism(), 8);
        assert_eq!(
            view.follow_redirects(),
            Ok(Some(crate::remote::GitRemoteRedirect::All))
        );
        assert_eq!(view.custom_headers().count(), 0);
        assert_eq!(view.remote_push_options().count(), 0);
    }

    #[test]
    fn borrowed_data_survives_for_the_pinned_options_lifetime() {
        let header = c"X-Crustify: 1";
        let url = c"http://proxy.invalid/";
        let mut strings = [header.as_ptr().cast_mut()];
        let mut raw = crate::ffi::git_strarray {
            strings: strings.as_mut_ptr(),
            count: strings.len(),
        };
        // SAFETY: `raw` is an initialized header whose single entry addresses a
        // `'static` NUL-terminated string, and nothing else borrows it here.
        let borrowed =
            unsafe { crate::strarray::GitStrArrayRef::from_ptr(core::ptr::addr_of_mut!(raw)) }
                .expect("a stack header is non-null");

        let mut fetch = GitFetchOptions::new();
        {
            let mut view = fetch.as_mut();
            view.set_custom_headers(borrowed);
            view.proxy_options_mut().set_url(Some(url));
        }
        let view = fetch.as_ref();
        assert_eq!(view.custom_headers().count(), 1);
        assert_eq!(
            view.custom_headers().strings().and_then(|run| run.get(0)),
            Some(header)
        );
        assert_eq!(view.proxy_options().url(), Some(url));

        let mut push = GitPushOptions::new();
        {
            let mut view = push.as_mut();
            view.set_custom_headers(borrowed);
            view.set_remote_push_options(borrowed);
            view.proxy_options_mut().set_url(Some(url));
        }
        let view = push.as_ref();
        assert_eq!(
            view.custom_headers().strings().and_then(|run| run.get(0)),
            Some(header)
        );
        assert_eq!(
            view.remote_push_options()
                .strings()
                .and_then(|run| run.get(0)),
            Some(header)
        );
        assert_eq!(view.proxy_options().url(), Some(url));
    }

    /// The pinned `'data` still accepts a referent that merely outlives the
    /// options, so the invariance fix does not force `'static` on callers.
    #[test]
    fn pinned_options_accept_a_scoped_referent() {
        let url = std::ffi::CString::new("http://scoped.invalid/").unwrap();
        let mut fetch = GitFetchOptions::new();
        fetch.as_mut().proxy_options_mut().set_url(Some(&url));
        assert_eq!(fetch.as_ref().proxy_options().url(), Some(url.as_c_str()));

        let mut push = GitPushOptions::new();
        push.as_mut().proxy_options_mut().set_url(Some(&url));
        assert_eq!(push.as_ref().proxy_options().url(), Some(url.as_c_str()));
    }
}

/// Wraps: git_remote_create_flags
/// A checked set of options controlling how a remote is created.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitRemoteCreateFlags(crate::ffi::git_remote_create_flags);

impl GitRemoteCreateFlags {
    /// Apply libgit2's normal remote-creation behavior.
    pub const EMPTY: Self = Self(0);
    /// Ignore `url.<base>.insteadOf` configuration while resolving the URL.
    pub const SKIP_INSTEAD_OF: Self =
        Self(crate::ffi::git_remote_create_flags_GIT_REMOTE_CREATE_SKIP_INSTEADOF);
    /// Do not derive a default fetch refspec from the remote name.
    pub const SKIP_DEFAULT_FETCHSPEC: Self =
        Self(crate::ffi::git_remote_create_flags_GIT_REMOTE_CREATE_SKIP_DEFAULT_FETCHSPEC);
    /// Every remote-creation option published by this libgit2 version.
    pub const ALL: Self = Self(Self::SKIP_INSTEAD_OF.0 | Self::SKIP_DEFAULT_FETCHSPEC.0);

    /// Converts raw bits when every bit is a published remote-creation option.
    #[must_use]
    pub const fn from_bits(bits: crate::ffi::git_remote_create_flags) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> crate::ffi::git_remote_create_flags {
        self.0
    }

    /// Returns whether no remote-creation option is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any option in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitRemoteCreateFlags> for crate::ffi::git_remote_create_flags {
    fn from(flags: GitRemoteCreateFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<crate::ffi::git_remote_create_flags> for GitRemoteCreateFlags {
    type Error = crate::ffi::git_remote_create_flags;

    fn try_from(bits: crate::ffi::git_remote_create_flags) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitRemoteCreateFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitRemoteCreateFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitRemoteCreateFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitRemoteCreateFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitRemoteCreateFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

/// Wraps: git_remote_update_flags
/// A checked set of options controlling reference updates after a fetch.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitRemoteUpdateFlags(crate::ffi::git_remote_update_flags);

impl GitRemoteUpdateFlags {
    /// Do not request either optional reference-update behavior.
    pub const EMPTY: Self = Self(0);
    /// Write fetched references to `FETCH_HEAD`.
    pub const FETCH_HEAD: Self =
        Self(crate::ffi::git_remote_update_flags_GIT_REMOTE_UPDATE_FETCHHEAD);
    /// Report unchanged tips to the update-reference callback.
    pub const REPORT_UNCHANGED: Self =
        Self(crate::ffi::git_remote_update_flags_GIT_REMOTE_UPDATE_REPORT_UNCHANGED);
    /// Every reference-update option published by this libgit2 version.
    pub const ALL: Self = Self(Self::FETCH_HEAD.0 | Self::REPORT_UNCHANGED.0);

    /// Converts raw bits when every bit is a published reference-update option.
    #[must_use]
    pub const fn from_bits(bits: crate::ffi::git_remote_update_flags) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> crate::ffi::git_remote_update_flags {
        self.0
    }

    /// Returns whether no reference-update option is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any option in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitRemoteUpdateFlags> for crate::ffi::git_remote_update_flags {
    fn from(flags: GitRemoteUpdateFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<crate::ffi::git_remote_update_flags> for GitRemoteUpdateFlags {
    type Error = crate::ffi::git_remote_update_flags;

    fn try_from(bits: crate::ffi::git_remote_update_flags) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitRemoteUpdateFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitRemoteUpdateFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitRemoteUpdateFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitRemoteUpdateFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitRemoteUpdateFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod remote_flag_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn remote_create_flags_compose_and_validate() {
        let flags =
            GitRemoteCreateFlags::SKIP_INSTEAD_OF | GitRemoteCreateFlags::SKIP_DEFAULT_FETCHSPEC;
        assert_eq!(flags, GitRemoteCreateFlags::ALL);
        assert!(flags.contains(GitRemoteCreateFlags::SKIP_INSTEAD_OF));
        assert!(flags.intersects(GitRemoteCreateFlags::SKIP_DEFAULT_FETCHSPEC));
        assert_eq!(GitRemoteCreateFlags::from_bits(flags.bits()), Some(flags));
        assert_eq!(GitRemoteCreateFlags::from_bits(1 << 2), None);
        assert_eq!(!GitRemoteCreateFlags::EMPTY, GitRemoteCreateFlags::ALL);
    }

    #[test]
    fn remote_update_flags_compose_and_validate() {
        let mut flags = GitRemoteUpdateFlags::EMPTY;
        assert!(flags.is_empty());
        flags |= GitRemoteUpdateFlags::FETCH_HEAD;
        flags |= GitRemoteUpdateFlags::REPORT_UNCHANGED;
        assert_eq!(flags, GitRemoteUpdateFlags::ALL);
        assert_eq!(GitRemoteUpdateFlags::try_from(flags.bits()), Ok(flags));
        assert_eq!(GitRemoteUpdateFlags::try_from(1 << 2), Err(1 << 2));
    }

    #[test]
    fn remote_flag_sets_match_their_c_enum_layouts() {
        assert_eq!(
            size_of::<GitRemoteCreateFlags>(),
            size_of::<crate::ffi::git_remote_create_flags>()
        );
        assert_eq!(
            align_of::<GitRemoteCreateFlags>(),
            align_of::<crate::ffi::git_remote_create_flags>()
        );
        assert_eq!(
            size_of::<GitRemoteUpdateFlags>(),
            size_of::<crate::ffi::git_remote_update_flags>()
        );
        assert_eq!(
            align_of::<GitRemoteUpdateFlags>(),
            align_of::<crate::ffi::git_remote_update_flags>()
        );
    }
}

/// Wraps: git_fetch_depth_t
/// A valid history depth for a fetch operation.
///
/// Zero requests complete history, values below [`Self::UNSHALLOW`] request
/// that many commits, and `UNSHALLOW` requests all history missing from an
/// already-shallow repository.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitFetchDepth(ffi::git_fetch_depth_t);

impl GitFetchDepth {
    /// Fetch complete history instead of making a shallow clone.
    pub const FULL: Self = Self(ffi::git_fetch_depth_t_GIT_FETCH_DEPTH_FULL);
    /// Fetch all history missing from an already-shallow repository.
    pub const UNSHALLOW: Self = Self(ffi::git_fetch_depth_t_GIT_FETCH_DEPTH_UNSHALLOW);

    /// Constructs a valid fetch depth from a commit count.
    ///
    /// Zero is [`Self::FULL`]. The `UNSHALLOW` sentinel is also accepted;
    /// larger values cannot be represented by libgit2's signed options field.
    #[must_use]
    pub const fn new(commits: ffi::git_fetch_depth_t) -> Option<Self> {
        if commits <= Self::UNSHALLOW.0 {
            Some(Self(commits))
        } else {
            None
        }
    }

    /// Returns the signed value stored in `git_fetch_options.depth`.
    #[must_use]
    pub const fn value(self) -> core::ffi::c_int {
        self.0 as core::ffi::c_int
    }

    /// Returns the underlying C enum value.
    #[must_use]
    pub const fn as_raw(self) -> ffi::git_fetch_depth_t {
        self.0
    }

    /// Returns whether this requests complete, non-shallow history.
    #[must_use]
    pub const fn is_full(self) -> bool {
        self.0 == Self::FULL.0
    }
}

impl From<GitFetchDepth> for ffi::git_fetch_depth_t {
    fn from(depth: GitFetchDepth) -> Self {
        depth.as_raw()
    }
}

impl TryFrom<ffi::git_fetch_depth_t> for GitFetchDepth {
    type Error = ffi::git_fetch_depth_t;

    fn try_from(depth: ffi::git_fetch_depth_t) -> Result<Self, Self::Error> {
        Self::new(depth).ok_or(depth)
    }
}

impl TryFrom<core::ffi::c_int> for GitFetchDepth {
    type Error = core::ffi::c_int;

    fn try_from(depth: core::ffi::c_int) -> Result<Self, Self::Error> {
        if depth < 0 {
            Err(depth)
        } else {
            Ok(Self(depth as ffi::git_fetch_depth_t))
        }
    }
}

#[cfg(test)]
mod fetch_depth_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn fetch_depths_cover_full_shallow_and_unshallow_requests() {
        assert!(GitFetchDepth::FULL.is_full());
        assert_eq!(GitFetchDepth::new(7).unwrap().value(), 7);
        assert_eq!(GitFetchDepth::UNSHALLOW.value(), core::ffi::c_int::MAX);
        assert_eq!(GitFetchDepth::try_from(-1), Err(-1));
        assert_eq!(
            GitFetchDepth::new(GitFetchDepth::UNSHALLOW.as_raw() + 1),
            None
        );
    }

    #[test]
    fn fetch_depth_preserves_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitFetchDepth>(),
            size_of::<ffi::git_fetch_depth_t>()
        );
        assert_eq!(
            align_of::<GitFetchDepth>(),
            align_of::<ffi::git_fetch_depth_t>()
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_remote_create_options
    /// Borrowing options that control remote creation.
    GitRemoteCreateOptions,
    GitRemoteCreateOptionsRef,
    GitRemoteCreateOptionsMut,
    ffi::git_remote_create_options
);

// SAFETY: the options record borrows its repository and strings and owns no
// resource, so disposing an inline value releases nothing.
unsafe impl CValued for GitRemoteCreateOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitRemoteCreateOptions {
    /// Constructs options equivalent to `GIT_REMOTE_CREATE_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        options
            .as_mut()
            .set_version(ffi::GIT_REMOTE_CREATE_OPTIONS_VERSION);
        options
    }
}

impl<'a> GitRemoteCreateOptionsRef<'a> {
    /// Field: git_remote_create_options.flags
    /// Returns the checked remote-creation flags.
    pub fn flags(&self) -> Result<GitRemoteCreateFlags, ffi::git_remote_create_flags> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let raw = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitRemoteCreateFlags::try_from(raw)
    }

    /// Field: git_remote_create_options.version
    /// Returns the ABI version stored in this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_remote_create_options.name
    /// Borrows the optional NUL-terminated remote name.
    #[must_use]
    pub fn name(&self) -> Option<&'a CStr> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to C-visible storage.
        let name = unsafe { addr_of!((*self.as_ptr()).name).read() };
        if name.is_null() {
            None
        } else {
            // SAFETY: a valid options record keeps a non-null name live,
            // immutable and NUL-terminated for this handle's lifetime.
            Some(unsafe { CStr::from_ptr(name) })
        }
    }

    /// Field: git_remote_create_options.fetchspec
    /// Borrows the optional NUL-terminated fetch refspec.
    #[must_use]
    pub fn fetchspec(&self) -> Option<&'a CStr> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to C-visible storage.
        let fetchspec = unsafe { addr_of!((*self.as_ptr()).fetchspec).read() };
        if fetchspec.is_null() {
            None
        } else {
            // SAFETY: a valid options record keeps a non-null refspec live,
            // immutable and NUL-terminated for this handle's lifetime.
            Some(unsafe { CStr::from_ptr(fetchspec) })
        }
    }

    /// Field: git_remote_create_options.repository
    /// Borrows the optional repository that will own the created remote.
    #[must_use]
    pub fn repository(&self) -> Option<GitRepositoryRef<'a>> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to C-visible storage.
        let repository = unsafe { addr_of!((*self.as_ptr()).repository).read() };
        // SAFETY: a non-null repository in a valid options record remains live
        // for the enclosing options borrow.
        unsafe { GitRepositoryRef::from_ptr(repository) }
    }
}

impl GitRemoteCreateOptionsMut<'_> {
    /// Sets the remote-creation flags.
    pub fn set_flags(&mut self, flags: GitRemoteCreateFlags) {
        // SAFETY: this exclusive handle permits a raw-place scalar write, and
        // the checked flag set contains only published bits.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Sets the ABI version expected by libgit2.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Stores a borrowed NUL-terminated remote name.
    ///
    /// # Safety
    ///
    /// A non-null `name` must remain live and immutable for every later use
    /// of this options value, including uses after this handle is released.
    pub unsafe fn set_borrowed_name(&mut self, name: Option<&CStr>) {
        let name = name.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the field write, and the
        // caller upholds the stored borrow's lifetime and validity.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).name).write(name) }
    }

    /// Clears the optional remote name.
    pub fn clear_name(&mut self) {
        // SAFETY: a null pointer creates no stored lifetime obligation.
        unsafe { self.set_borrowed_name(None) }
    }

    /// Stores a borrowed NUL-terminated fetch refspec.
    ///
    /// # Safety
    ///
    /// A non-null `fetchspec` must remain live and immutable for every later
    /// use of this options value, including uses after this handle is released.
    pub unsafe fn set_borrowed_fetchspec(&mut self, fetchspec: Option<&CStr>) {
        let fetchspec = fetchspec.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the field write, and the
        // caller upholds the stored borrow's lifetime and validity.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).fetchspec).write(fetchspec) }
    }

    /// Clears the optional fetch refspec.
    pub fn clear_fetchspec(&mut self) {
        // SAFETY: a null pointer creates no stored lifetime obligation.
        unsafe { self.set_borrowed_fetchspec(None) }
    }

    /// Stores a borrowed repository for the created remote.
    ///
    /// # Safety
    ///
    /// A non-null `repository` must remain live for every later use of this
    /// options value and for every remote created from it. The caller must
    /// also respect libgit2's mutation and synchronization requirements while
    /// the repository is used through this pointer.
    pub unsafe fn set_borrowed_repository(&mut self, repository: Option<GitRepositoryRef<'_>>) {
        let repository = repository.map_or(core::ptr::null_mut(), |repository| {
            repository.as_ptr().cast_mut()
        });
        // SAFETY: this exclusive handle permits the field write, and the
        // caller upholds the stored repository's lifetime and access rules.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).repository).write(repository) }
    }

    /// Clears the optional repository.
    pub fn clear_repository(&mut self) {
        // SAFETY: a null pointer creates no stored lifetime or aliasing
        // obligation.
        unsafe { self.set_borrowed_repository(None) }
    }
}

#[cfg(test)]
mod remote_create_options_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn remote_create_options_match_the_c_layout() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<GitRemoteCreateOptions>();
        assert_valued::<GitRemoteCreateOptions>();
        assert_eq!(
            size_of::<GitRemoteCreateOptions>(),
            size_of::<ffi::git_remote_create_options>()
        );
        assert_eq!(
            align_of::<GitRemoteCreateOptions>(),
            align_of::<ffi::git_remote_create_options>()
        );
        assert_eq!(
            size_of::<GitRemoteCreateOptionsRef<'_>>(),
            size_of::<*const ffi::git_remote_create_options>()
        );
        assert_eq!(
            size_of::<GitRemoteCreateOptionsMut<'_>>(),
            size_of::<*mut ffi::git_remote_create_options>()
        );
    }

    #[test]
    fn default_and_borrowed_fields_round_trip_through_handles() {
        let name = c"origin";
        let fetchspec = c"+refs/heads/*:refs/remotes/origin/*";
        let mut options = GitRemoteCreateOptions::new();

        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_REMOTE_CREATE_OPTIONS_VERSION
        );
        assert_eq!(options.as_ref().flags(), Ok(GitRemoteCreateFlags::EMPTY));
        assert_eq!(options.as_ref().name(), None);
        assert_eq!(options.as_ref().fetchspec(), None);
        assert!(options.as_ref().repository().is_none());

        options.as_mut().set_flags(GitRemoteCreateFlags::ALL);
        // SAFETY: both static C strings outlive every use of `options` below.
        unsafe {
            options.as_mut().set_borrowed_name(Some(name));
            options.as_mut().set_borrowed_fetchspec(Some(fetchspec));
        }
        assert_eq!(options.as_ref().flags(), Ok(GitRemoteCreateFlags::ALL));
        assert_eq!(options.as_ref().name(), Some(name));
        assert_eq!(options.as_ref().fetchspec(), Some(fetchspec));

        options.as_mut().clear_name();
        options.as_mut().clear_fetchspec();
        options.as_mut().clear_repository();
        assert_eq!(options.as_ref().name(), None);
        assert_eq!(options.as_ref().fetchspec(), None);
        assert!(options.as_ref().repository().is_none());
    }

    #[test]
    fn flags_getter_rejects_unknown_bits() {
        let mut raw = ffi::git_remote_create_options {
            version: ffi::GIT_REMOTE_CREATE_OPTIONS_VERSION,
            repository: core::ptr::null_mut(),
            name: core::ptr::null(),
            fetchspec: core::ptr::null(),
            flags: 1 << 2,
        };
        // SAFETY: `raw` remains live and initialized for this sole handle.
        let options = unsafe { GitRemoteCreateOptionsRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(options.flags(), Err(1 << 2));
    }
}
