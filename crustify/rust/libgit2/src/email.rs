//! Safe wrappers for libgit2 email APIs.

use core::ffi::CStr;

use crate::api::buffer::GitBufMut;
use crate::api::email::GitEmailCreateOptionsRef;
use crate::api::types::GitSignatureRef;
use crate::commit::GitCommitRef;
use crate::diff::DiffRef;
use crate::ffi;
use crate::oid::OidRef;

/// Wraps: git_email_create_from_commit
/// Replaces `out` with an mbox-formatted patch for `commit`.
pub fn git_email_create_from_commit(
    out: &mut GitBufMut<'_>,
    commit: GitCommitRef<'_>,
    options: Option<GitEmailCreateOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: `out` is exclusive and all other typed inputs are live and
    // read-only for the synchronous formatting operation.
    let status = unsafe {
        ffi::git_email_create_from_commit(out.as_mut_ptr(), commit.as_ptr().cast_mut(), options)
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_email_create_from_diff
/// Replaces `out` with one mbox-formatted patch from `diff`.
#[allow(clippy::too_many_arguments)]
pub fn git_email_create_from_diff(
    out: &mut GitBufMut<'_>,
    diff: DiffRef<'_>,
    patch_index: usize,
    patch_count: usize,
    commit_id: OidRef<'_>,
    summary: &CStr,
    body: Option<&CStr>,
    author: GitSignatureRef<'_>,
    options: Option<GitEmailCreateOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: `out` is exclusive; all other pointers come from live shared
    // handles or C strings and are retained only for this call.
    let status = unsafe {
        ffi::git_email_create_from_diff(
            out.as_mut_ptr(),
            diff.as_ptr().cast_mut(),
            patch_index,
            patch_count,
            commit_id.as_ptr(),
            summary.as_ptr(),
            body.map_or(core::ptr::null(), CStr::as_ptr),
            author.as_ptr(),
            options,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}
