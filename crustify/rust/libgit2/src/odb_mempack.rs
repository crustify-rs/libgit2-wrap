//! Safe wrappers for libgit2 odb_mempack APIs.

use crate::api::buffer::GitBufMut;
use crate::ffi;
use crate::repository::GitRepositoryMut;
use crate::sys::odb_backend::{GitOdbBackendMut, GitOdbBackendOwned, GitOdbBackendRef};

/// Wraps: git_mempack_dump
/// Writes the mempack's queued objects to a thin packfile buffer.
pub fn git_mempack_dump(
    pack: &mut GitBufMut<'_>,
    repository: &mut GitRepositoryMut<'_>,
    backend: GitOdbBackendRef<'_>,
) -> Result<(), i32> {
    // SAFETY: the output and repository are exclusively borrowed, while the
    // backend is read for this synchronous operation. No pointer is retained.
    let status = unsafe {
        ffi::git_mempack_dump(
            pack.as_mut_ptr(),
            repository.as_mut_ptr(),
            backend.as_ptr().cast_mut(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_mempack_new
/// Creates a new in-memory object-database backend.
pub fn git_mempack_new() -> Result<GitOdbBackendOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable slot for the newly allocated backend.
    let status = unsafe { ffi::git_mempack_new(&mut out) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully constructed concrete backend with
    // its required destructor callback installed.
    unsafe { GitOdbBackendOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_mempack_reset
/// Frees all queued objects while retaining the backend for reuse.
pub fn git_mempack_reset(backend: &mut GitOdbBackendMut<'_>) -> Result<(), i32> {
    // SAFETY: the exclusive live backend handle permits clearing its private
    // object map and commit array; C retains no pointer.
    let status = unsafe { ffi::git_mempack_reset(backend.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mempack_can_be_reset_and_dropped() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after the backend and all of its allocations have been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut backend = git_mempack_new().unwrap();
        git_mempack_reset(&mut backend.as_mut()).unwrap();
        drop(backend);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
