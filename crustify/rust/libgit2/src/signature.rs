//! Safe constructors for libgit2 signatures.

use core::ffi::CStr;

use crate::api::types::GitSignatureOwned;
use crate::ffi;
use crate::repository::GitRepositoryMut;

/// Wraps: git_signature_default
/// Builds the current-time signature from repository configuration.
pub fn git_signature_default(
    repository: &mut GitRepositoryMut<'_>,
) -> Result<GitSignatureOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and the exclusive repository handle
    // permits configuration snapshot access. Success transfers one owner.
    let status = unsafe {
        ffi::git_signature_default(core::ptr::addr_of_mut!(output), repository.as_mut_ptr())
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete signature allocation.
    unsafe { GitSignatureOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_signature_default_from_env
/// Builds the requested author and/or committer signatures from environment
/// variables, falling back to repository configuration.
pub fn git_signature_default_from_env(
    repository: &mut GitRepositoryMut<'_>,
    author: bool,
    committer: bool,
) -> Result<(Option<GitSignatureOwned>, Option<GitSignatureOwned>), i32> {
    if !author && !committer {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    let mut author_output = core::ptr::null_mut();
    let mut committer_output = core::ptr::null_mut();
    // SAFETY: each selected output slot is writable, each unselected slot is
    // null as permitted by C, and the repository is exclusively borrowed.
    let status = unsafe {
        ffi::git_signature_default_from_env(
            if author {
                core::ptr::addr_of_mut!(author_output)
            } else {
                core::ptr::null_mut()
            },
            if committer {
                core::ptr::addr_of_mut!(committer_output)
            } else {
                core::ptr::null_mut()
            },
            repository.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: selected successful outputs transfer one complete owner and
    // unselected outputs remain null.
    let author = unsafe { GitSignatureOwned::from_raw(author_output) };
    // SAFETY: as above, for the independent committer output.
    let committer = unsafe { GitSignatureOwned::from_raw(committer_output) };
    Ok((author, committer))
}

/// Wraps: git_signature_new
/// Creates a signature at an explicit timestamp and timezone offset.
pub fn git_signature_new(
    name: &CStr,
    email: &CStr,
    time: ffi::git_time_t,
    offset: i32,
) -> Result<GitSignatureOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and both C strings are live for the call;
    // libgit2 copies their contents into the new allocation.
    let status = unsafe {
        ffi::git_signature_new(
            core::ptr::addr_of_mut!(output),
            name.as_ptr(),
            email.as_ptr(),
            time,
            offset,
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete signature allocation.
    unsafe { GitSignatureOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_signature_now
/// Creates a signature with the current time and local timezone offset.
pub fn git_signature_now(name: &CStr, email: &CStr) -> Result<GitSignatureOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and libgit2 copies both live C strings.
    let status = unsafe {
        ffi::git_signature_now(
            core::ptr::addr_of_mut!(output),
            name.as_ptr(),
            email.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete signature allocation.
    unsafe { GitSignatureOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_signature_constructor_copies_and_trims_inputs() {
        // SAFETY: process initialization is reference counted and balanced
        // after the returned signature has been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let signature =
            git_signature_new(c"  Ada  ", c"ada@example.com", 123, -60).expect("valid identity");
        let view = signature.as_ref();
        assert_eq!(view.name(), c"Ada");
        assert_eq!(view.email(), c"ada@example.com");
        assert_eq!(view.when().time(), 123);
        assert_eq!(view.when().offset(), -60);
        drop(signature);
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn current_time_constructor_rejects_empty_identity() {
        // SAFETY: process initialization is reference counted and balanced.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert!(git_signature_now(c"", c"ada@example.com").is_err());
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
