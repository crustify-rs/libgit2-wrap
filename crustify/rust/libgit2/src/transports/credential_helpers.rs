//! Safe wrappers for libgit2 credential_helpers APIs.

use crate::ffi;

/// Wraps: git_credential_userpass
/// Acquires a plaintext credential from a typed user/password payload.
pub fn git_credential_userpass(
    url: &core::ffi::CStr,
    username_from_url: Option<&core::ffi::CStr>,
    allowed_types: crate::transports::credential::GitCredentialType,
    mut payload: crate::api::credential_helpers::GitCredentialUserpassPayloadMut<'_, '_>,
) -> Result<crate::transports::credential::GitCredentialOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output is writable; strings and typed payload are live for this
    // synchronous callback helper, which copies values into any result.
    let status = unsafe {
        ffi::git_credential_userpass(
            &mut out,
            url.as_ptr(),
            username_from_url.map_or(core::ptr::null(), core::ffi::CStr::as_ptr),
            allowed_types.bits(),
            payload.as_mut_ptr().cast(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete credential allocation.
    unsafe { crate::transports::credential::GitCredentialOwned::from_raw(out) }
        .ok_or(ffi::git_error_code_GIT_ERROR)
}
