//! Safe wrappers for FETCH_HEAD traversal.

use core::ffi::{CStr, c_char, c_void};

use crate::ffi;
use crate::oid::OidRef;
use crate::repository::{GitRepositoryFetchheadForeachCallback, GitRepositoryRef};

/// Wraps: git_repository_fetchhead_foreach
/// Visits each transient entry parsed from a repository's `FETCH_HEAD`.
pub fn git_repository_fetchhead_foreach<C>(
    repository: GitRepositoryRef<'_>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitRepositoryFetchheadForeachCallback,
{
    unsafe extern "C" fn trampoline<C: GitRepositoryFetchheadForeachCallback>(
        reference_name: *const c_char,
        remote_url: *const c_char,
        oid: *const ffi::git_oid,
        is_merge: u32,
        payload: *mut c_void,
    ) -> i32 {
        if oid.is_null() || payload.is_null() {
            return -1;
        }
        // SAFETY: the synchronous wrapper supplies this live exclusive
        // callback payload for the entire traversal.
        let callback = unsafe { &mut *payload.cast::<C>() };
        let string = |value: *const c_char| {
            if value.is_null() {
                None
            } else {
                // SAFETY: libgit2 documents each non-null callback string as
                // NUL-terminated and live for this invocation.
                Some(unsafe { CStr::from_ptr(value) })
            }
        };
        // SAFETY: the non-null OID is a live transient value for this call.
        let oid = unsafe { OidRef::from_ptr(oid.cast_mut()) }.expect("checked non-null");
        callback.call(
            string(reference_name),
            string(remote_url),
            oid,
            is_merge != 0,
        )
    }

    // SAFETY: the repository and callback remain live for this synchronous
    // traversal, and the trampoline reconstructs exactly `C` from the payload.
    let status = unsafe {
        ffi::git_repository_fetchhead_foreach(
            repository.as_ptr().cast_mut(),
            Some(trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}
