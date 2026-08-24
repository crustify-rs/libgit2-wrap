//! Safe wrappers for libgit2 diff_print APIs.

use ffibox::CVal;

use crate::api::buffer::GitBuf;
use crate::ffi;
use crate::patch::GitPatchMut;

/// Wraps: git_patch_to_buf
/// Serializes a patch into a newly owned libgit2 buffer.
pub fn git_patch_to_buf(patch: &mut GitPatchMut<'_>) -> Result<CVal<GitBuf>, i32> {
    let mut output = GitBuf::new();
    let status = {
        let mut output = output.as_mut();
        // SAFETY: `output` is an empty, exclusively borrowed buffer header and
        // `patch` is exclusively borrowed for any lazy formatting updates.
        // On success the buffer owns its allocation; on error it remains valid
        // for `CVal` to dispose.
        unsafe { ffi::git_patch_to_buf(output.as_mut_ptr(), patch.as_mut_ptr()) }
    };
    if status == 0 { Ok(output) } else { Err(status) }
}
