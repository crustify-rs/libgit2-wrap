//! Safe wrappers for libgit2 credential APIs.

/// Wraps: git_credential_acquire_cb
/// Safe callable surface for acquiring transport credentials.
pub trait GitCredentialAcquireCallback {
    /// Returns a newly owned credential, or a callback/error status.
    fn acquire(
        &mut self,
        url: &core::ffi::CStr,
        username_from_url: Option<&core::ffi::CStr>,
        allowed_types: crate::transports::credential::GitCredentialType,
    ) -> Result<crate::transports::credential::GitCredentialOwned, i32>;
}

impl<F> GitCredentialAcquireCallback for F
where
    F: FnMut(
        &core::ffi::CStr,
        Option<&core::ffi::CStr>,
        crate::transports::credential::GitCredentialType,
    ) -> Result<crate::transports::credential::GitCredentialOwned, i32>,
{
    fn acquire(
        &mut self,
        url: &core::ffi::CStr,
        username_from_url: Option<&core::ffi::CStr>,
        allowed_types: crate::transports::credential::GitCredentialType,
    ) -> Result<crate::transports::credential::GitCredentialOwned, i32> {
        self(url, username_from_url, allowed_types)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_implement_credential_acquisition() {
        fn accepts<C: GitCredentialAcquireCallback>(_callback: C) {}
        accepts(
            |_: &core::ffi::CStr,
             _: Option<&core::ffi::CStr>,
             _: crate::transports::credential::GitCredentialType| Err(-7),
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: _LIBSSH2_SESSION
    /// Opaque libssh2 session state borrowed by custom credential callbacks.
    ///
    /// Libssh2 owns this incomplete type. The wrapper therefore exposes only
    /// lifetime-bound shared and exclusive handles and no owning lifecycle.
    Libssh2Session,
    Libssh2SessionRef,
    Libssh2SessionMut,
    crate::ffi::_LIBSSH2_SESSION
);

ffibox::define_ctype!(
    /// Wraps: _LIBSSH2_USERAUTH_KBDINT_PROMPT
    /// Opaque keyboard-interactive prompt data borrowed for one callback.
    ///
    /// The public libgit2 header deliberately leaves the libssh2 layout
    /// incomplete, so consumers can pass this value on but cannot inspect it.
    Libssh2UserauthKbdintPrompt,
    Libssh2UserauthKbdintPromptRef,
    Libssh2UserauthKbdintPromptMut,
    crate::ffi::_LIBSSH2_USERAUTH_KBDINT_PROMPT
);

ffibox::define_ctype!(
    /// Wraps: _LIBSSH2_USERAUTH_KBDINT_RESPONSE
    /// Opaque keyboard-interactive response data borrowed for one callback.
    ///
    /// Libssh2 owns the response array and libgit2 lends it exclusively to the
    /// callback while answers are populated.
    Libssh2UserauthKbdintResponse,
    Libssh2UserauthKbdintResponseRef,
    Libssh2UserauthKbdintResponseMut,
    crate::ffi::_LIBSSH2_USERAUTH_KBDINT_RESPONSE
);

/// An opaque run of keyboard-interactive prompts for one callback invocation.
///
/// Libgit2 deliberately exposes no prompt layout when libssh2 headers are not
/// present, so this token reports cardinality without pretending that Rust can
/// calculate an element stride for the incomplete C type.
#[derive(Clone, Copy)]
pub struct Libssh2PromptBatch<'a> {
    _ptr: core::ptr::NonNull<crate::ffi::_LIBSSH2_USERAUTH_KBDINT_PROMPT>,
    len: usize,
    _borrow: core::marker::PhantomData<&'a ()>,
}

impl Libssh2PromptBatch<'_> {
    /// Returns the number of opaque prompt records.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether there are no prompts.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// An exclusive opaque run of keyboard-interactive responses.
pub struct Libssh2ResponseBatch<'a> {
    _ptr: core::ptr::NonNull<crate::ffi::_LIBSSH2_USERAUTH_KBDINT_RESPONSE>,
    len: usize,
    _borrow: core::marker::PhantomData<&'a mut ()>,
}

impl Libssh2ResponseBatch<'_> {
    /// Returns the number of opaque response records.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether there are no responses.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod libssh2_type_tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::CCell;

    use super::*;

    #[test]
    fn opaque_libssh2_types_preserve_the_binding_layout_and_handle_shape() {
        fn assert_cell<T: CCell>() {}

        assert_cell::<Libssh2Session>();
        assert_cell::<Libssh2UserauthKbdintPrompt>();
        assert_cell::<Libssh2UserauthKbdintResponse>();

        assert_eq!(
            size_of::<Libssh2Session>(),
            size_of::<crate::ffi::_LIBSSH2_SESSION>()
        );
        assert_eq!(
            align_of::<Libssh2Session>(),
            align_of::<crate::ffi::_LIBSSH2_SESSION>()
        );
        assert_eq!(
            size_of::<Libssh2SessionRef<'_>>(),
            size_of::<*mut crate::ffi::_LIBSSH2_SESSION>()
        );
        assert_eq!(
            size_of::<Libssh2SessionMut<'_>>(),
            size_of::<*mut crate::ffi::_LIBSSH2_SESSION>()
        );
        assert_eq!(
            size_of::<Libssh2UserauthKbdintPrompt>(),
            size_of::<crate::ffi::_LIBSSH2_USERAUTH_KBDINT_PROMPT>()
        );
        assert_eq!(
            size_of::<Libssh2UserauthKbdintPromptRef<'_>>(),
            size_of::<*mut crate::ffi::_LIBSSH2_USERAUTH_KBDINT_PROMPT>()
        );
        assert_eq!(
            size_of::<Libssh2UserauthKbdintResponse>(),
            size_of::<crate::ffi::_LIBSSH2_USERAUTH_KBDINT_RESPONSE>()
        );
        assert_eq!(
            size_of::<Libssh2UserauthKbdintResponseMut<'_>>(),
            size_of::<*mut crate::ffi::_LIBSSH2_USERAUTH_KBDINT_RESPONSE>()
        );
    }

    #[test]
    fn null_libssh2_pointers_create_no_borrowed_handle() {
        // SAFETY: the borrowed-handle seam accepts null and returns `None`
        // without constructing a borrow.
        unsafe {
            assert!(Libssh2SessionRef::from_ptr(ptr::null_mut()).is_none());
            assert!(Libssh2SessionMut::from_ptr(ptr::null_mut()).is_none());
            assert!(Libssh2UserauthKbdintPromptRef::from_ptr(ptr::null_mut()).is_none());
            assert!(Libssh2UserauthKbdintResponseMut::from_ptr(ptr::null_mut()).is_none());
        }
    }
}

/// Wraps: git_credential_sign_cb
/// Safe callable surface for a custom SSH signing operation.
pub trait GitCredentialSignCallback {
    /// Signs `data`; the returned bytes are transferred by the eventual C
    /// trampoline using libgit2's configured allocator.
    fn sign(&mut self, session: Libssh2SessionMut<'_>, data: &[u8]) -> Result<Vec<u8>, i32>;
}

impl<F> GitCredentialSignCallback for F
where
    F: FnMut(Libssh2SessionMut<'_>, &[u8]) -> Result<Vec<u8>, i32>,
{
    fn sign(&mut self, session: Libssh2SessionMut<'_>, data: &[u8]) -> Result<Vec<u8>, i32> {
        self(session, data)
    }
}

/// Wraps: git_credential_ssh_interactive_cb
/// Safe callable surface for SSH keyboard-interactive prompts.
pub trait GitCredentialSshInteractiveCallback {
    /// Fills the response records for the corresponding prompt records.
    fn respond(
        &mut self,
        name: &[u8],
        instruction: &[u8],
        prompts: Libssh2PromptBatch<'_>,
        responses: Libssh2ResponseBatch<'_>,
    );
}

impl<F> GitCredentialSshInteractiveCallback for F
where
    F: FnMut(&[u8], &[u8], Libssh2PromptBatch<'_>, Libssh2ResponseBatch<'_>),
{
    fn respond(
        &mut self,
        name: &[u8],
        instruction: &[u8],
        prompts: Libssh2PromptBatch<'_>,
        responses: Libssh2ResponseBatch<'_>,
    ) {
        self(name, instruction, prompts, responses)
    }
}
