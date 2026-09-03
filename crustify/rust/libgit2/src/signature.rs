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
mod unit_tests {
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

/// Wraps: git_signature_from_buffer
/// Parses a complete signature from its canonical textual representation.
pub fn git_signature_from_buffer(buffer: &CStr) -> Result<GitSignatureOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and `buffer` is a live NUL-terminated
    // string retained only for this call.
    let status =
        unsafe { ffi::git_signature_from_buffer(core::ptr::addr_of_mut!(output), buffer.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete signature allocation.
    unsafe { GitSignatureOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod from_buffer_tests {
    use super::*;

    #[test]
    fn parses_the_documented_signature_format() {
        let signature = git_signature_from_buffer(c"Ada <ada@example.com> 123 +0100")
            .expect("documented signature text parses");
        assert_eq!(signature.as_ref().name(), c"Ada");
        assert_eq!(signature.as_ref().when().time(), 123);
    }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::api::types::GitSignatureRef;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    type Signature = (Vec<u8>, Vec<u8>, ffi::git_time_t, i32);

    unsafe fn raw_value(signature: *const ffi::git_signature) -> Signature {
        (
            unsafe { CStr::from_ptr((*signature).name) }
                .to_bytes()
                .to_vec(),
            unsafe { CStr::from_ptr((*signature).email) }
                .to_bytes()
                .to_vec(),
            unsafe { (*signature).when.time },
            unsafe { (*signature).when.offset },
        )
    }

    fn safe_value(signature: GitSignatureRef<'_>) -> Signature {
        (
            signature.name().to_bytes().to_vec(),
            signature.email().to_bytes().to_vec(),
            signature.when().time(),
            signature.when().offset(),
        )
    }

    unsafe fn raw_signatures(repository: *mut ffi::git_repository) -> Vec<Signature> {
        let mut output = Vec::new();
        let mut signature = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_signature_default(&mut signature, repository) },
            0
        );
        let mut value = unsafe { raw_value(signature) };
        value.2 = 0;
        output.push(value);
        unsafe { ffi::git_signature_free(signature) };

        let mut author = core::ptr::null_mut();
        let mut committer = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_signature_default_from_env(&mut author, &mut committer, repository) },
            0
        );
        let mut author_value = unsafe { raw_value(author) };
        let mut committer_value = unsafe { raw_value(committer) };
        author_value.2 = 0;
        committer_value.2 = 0;
        output.extend([author_value, committer_value]);
        unsafe {
            ffi::git_signature_free(author);
            ffi::git_signature_free(committer);
        }

        signature = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_signature_new(
                    &mut signature,
                    c"  Explicit Person  ".as_ptr(),
                    c"explicit@example.com".as_ptr(),
                    1_700_123_456,
                    -90,
                )
            },
            0
        );
        output.push(unsafe { raw_value(signature) });
        unsafe { ffi::git_signature_free(signature) };

        signature = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_signature_from_buffer(
                    &mut signature,
                    c"Buffered Person <buffered@example.com> 1700123457 +0230".as_ptr(),
                )
            },
            0
        );
        output.push(unsafe { raw_value(signature) });
        unsafe { ffi::git_signature_free(signature) };
        output
    }

    fn safe_signatures(repository: *mut ffi::git_repository) -> Vec<Signature> {
        let mut repository = unsafe { GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut output = Vec::new();
        let signature = git_signature_default(&mut repository).unwrap();
        let mut value = safe_value(signature.as_ref());
        value.2 = 0;
        output.push(value);
        let (author, committer) =
            git_signature_default_from_env(&mut repository, true, true).unwrap();
        for signature in [author.unwrap(), committer.unwrap()] {
            let mut value = safe_value(signature.as_ref());
            value.2 = 0;
            output.push(value);
        }
        let signature = git_signature_new(
            c"  Explicit Person  ",
            c"explicit@example.com",
            1_700_123_456,
            -90,
        )
        .unwrap();
        output.push(safe_value(signature.as_ref()));
        let signature =
            git_signature_from_buffer(c"Buffered Person <buffered@example.com> 1700123457 +0230")
                .unwrap();
        output.push(safe_value(signature.as_ref()));
        output
    }

    #[test]
    fn io_equiv_config_environment_explicit_and_buffered_signatures() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("signature-raw");
        let safe = HistoryFixture::new("signature-safe");
        for fixture in [&raw, &safe] {
            for (key, value) in [
                ("user.name", "Configured Person"),
                ("user.email", "configured@example.com"),
            ] {
                let status = std::process::Command::new("git")
                    .arg("-C")
                    .arg(fixture.directory.path())
                    .args(["config", key, value])
                    .status()
                    .unwrap();
                assert!(status.success());
            }
        }
        unsafe {
            std::env::set_var("GIT_AUTHOR_NAME", "Environment Author");
            std::env::set_var("GIT_AUTHOR_EMAIL", "author@example.com");
            std::env::set_var("GIT_AUTHOR_DATE", "1700123400 +0000");
            std::env::set_var("GIT_COMMITTER_NAME", "Environment Committer");
            std::env::set_var("GIT_COMMITTER_EMAIL", "committer@example.com");
            std::env::set_var("GIT_COMMITTER_DATE", "1700123401 +0000");
        }
        let raw_values = unsafe { raw_signatures(raw.repository.as_ptr()) };
        assert_eq!(raw_values, safe_signatures(safe.repository.as_ptr()));
        unsafe {
            for name in [
                "GIT_AUTHOR_NAME",
                "GIT_AUTHOR_EMAIL",
                "GIT_AUTHOR_DATE",
                "GIT_COMMITTER_NAME",
                "GIT_COMMITTER_EMAIL",
                "GIT_COMMITTER_DATE",
            ] {
                std::env::remove_var(name);
            }
        }
        assert_eq!(raw_values[1].0, b"Environment Author");
        assert_eq!(raw_values[2].0, b"Environment Committer");
    }
}
