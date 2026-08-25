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
