//! Safe wrappers for libgit2 credential_helpers APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::ffi;

/// Wraps: git_credential_userpass_payload
/// Layout-compatible payload borrowing optional username and password strings.
///
/// The string lifetime is invariant because the mutable handle can replace
/// either stored pointer. This prevents safe code from shrinking the lifetime
/// parameter and then storing a shorter-lived string in a longer-lived value.
///
/// ```compile_fail
/// use libgit2::api::credential_helpers::GitCredentialUserpassPayloadMut;
///
/// fn shrink<'object, 'short>(
///     payload: GitCredentialUserpassPayloadMut<'object, 'static>,
/// ) -> GitCredentialUserpassPayloadMut<'object, 'short> {
///     payload
/// }
/// ```
#[repr(transparent)]
pub struct GitCredentialUserpassPayload<'data> {
    inner: CType<ffi::git_credential_userpass_payload>,
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitCredentialUserpassPayload`].
#[repr(transparent)]
pub struct GitCredentialUserpassPayloadRef<'object, 'data>(
    CPtr<'object, GitCredentialUserpassPayload<'data>>,
);

impl Clone for GitCredentialUserpassPayloadRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitCredentialUserpassPayloadRef<'_, '_> {}

/// Exclusive borrow of [`GitCredentialUserpassPayload`].
#[repr(transparent)]
pub struct GitCredentialUserpassPayloadMut<'object, 'data>(
    GitCredentialUserpassPayloadRef<'object, 'data>,
);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitCredentialUserpassPayload<'data> {
    type C = ffi::git_credential_userpass_payload;
    type Ref<'object>
        = GitCredentialUserpassPayloadRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitCredentialUserpassPayloadMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitCredentialUserpassPayloadRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitCredentialUserpassPayloadMut(GitCredentialUserpassPayloadRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: the payload borrows both strings and owns no resource, so disposing
// its inline storage requires no action.
unsafe impl CValued for GitCredentialUserpassPayload<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitCredentialUserpassPayload<'data> {
    /// Constructs a payload borrowing the supplied strings.
    #[must_use]
    pub fn new(username: Option<&'data CStr>, password: Option<&'data CStr>) -> CVal<Self> {
        let raw = ffi::git_credential_userpass_payload {
            username: username.map_or(core::ptr::null(), CStr::as_ptr),
            password: password.map_or(core::ptr::null(), CStr::as_ptr),
        };
        // The invariant lifetime marker records that both initialized raw
        // pointers remain valid for `'data`.
        let inner = CType::new(raw);
        CVal::new(Self {
            inner,
            _data: PhantomData,
        })
    }
}

impl<'object, 'data> GitCredentialUserpassPayloadRef<'object, 'data> {
    /// Borrows a raw C payload pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify an initialized payload live for `'object`. Each
    /// non-null string pointer must remain NUL-terminated and live for `'data`,
    /// and `'data` must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_credential_userpass_payload) -> Option<Self> {
        NonNull::new(ptr.cast::<GitCredentialUserpassPayload<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_credential_userpass_payload {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_credential_userpass_payload.username
    /// Borrows the optional NUL-terminated username.
    #[must_use]
    pub fn username(&self) -> Option<&'data CStr> {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        let username = unsafe { addr_of!((*self.as_ptr()).username).read() };
        if username.is_null() {
            None
        } else {
            // SAFETY: the payload contract guarantees a non-null username is
            // NUL-terminated and live for the type's invariant `'data`.
            Some(unsafe { CStr::from_ptr(username) })
        }
    }

    /// Field: git_credential_userpass_payload.password
    /// Borrows the optional NUL-terminated password.
    #[must_use]
    pub fn password(&self) -> Option<&'data CStr> {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        let password = unsafe { addr_of!((*self.as_ptr()).password).read() };
        if password.is_null() {
            None
        } else {
            // SAFETY: the payload contract guarantees a non-null password is
            // NUL-terminated and live for the type's invariant `'data`.
            Some(unsafe { CStr::from_ptr(password) })
        }
    }
}

impl<'object, 'data> GitCredentialUserpassPayloadMut<'object, 'data> {
    /// Exclusively borrows a raw C payload pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path may use
    /// the payload value for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_credential_userpass_payload) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible payload.
        unsafe { GitCredentialUserpassPayloadRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the C pointer for mutable FFI calls.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_credential_userpass_payload {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this payload as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitCredentialUserpassPayloadRef<'_, 'data> {
        GitCredentialUserpassPayloadRef(self.0.0)
    }

    /// Stores an optional borrowed username.
    pub fn set_username(&mut self, username: Option<&'data CStr>) {
        let username = username.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the raw-place write, and the
        // type's invariant `'data` guarantees the stored string remains live.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).username).write(username) }
    }

    /// Stores an optional borrowed password.
    pub fn set_password(&mut self, password: Option<&'data CStr>) {
        let password = password.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the raw-place write, and the
        // type's invariant `'data` guarantees the stored string remains live.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).password).write(password) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn payload_round_trips_borrowed_strings_and_nulls() {
        let username = c"alice";
        let password = c"secret";
        let replacement = c"new-secret";
        let mut payload = GitCredentialUserpassPayload::new(Some(username), Some(password));

        assert_eq!(payload.as_ref().username(), Some(username));
        assert_eq!(payload.as_ref().password(), Some(password));

        payload.as_mut().set_username(None);
        payload.as_mut().set_password(Some(replacement));
        assert_eq!(payload.as_ref().username(), None);
        assert_eq!(payload.as_ref().password(), Some(replacement));
    }

    #[test]
    fn payload_preserves_the_c_layout() {
        assert_eq!(
            size_of::<GitCredentialUserpassPayload<'static>>(),
            size_of::<ffi::git_credential_userpass_payload>()
        );
        assert_eq!(
            align_of::<GitCredentialUserpassPayload<'static>>(),
            align_of::<ffi::git_credential_userpass_payload>()
        );
        assert_eq!(
            size_of::<GitCredentialUserpassPayloadRef<'static, 'static>>(),
            size_of::<*mut ffi::git_credential_userpass_payload>()
        );
    }
}
