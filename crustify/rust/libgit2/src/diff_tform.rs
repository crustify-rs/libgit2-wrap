//! Safe wrappers for libgit2 diff_tform APIs.

use crate::diff::{DiffMut, DiffRef};
use crate::ffi;

/// Wraps: git_diff_merge
/// Merges every delta from `from` into `onto`.
pub fn git_diff_merge(onto: &mut DiffMut<'_>, from: DiffRef<'_>) -> Result<(), i32> {
    // SAFETY: `onto` is exclusively borrowed, `from` is shared and distinct,
    // and C copies all retained data into `onto` before returning.
    let status = unsafe { ffi::git_diff_merge(onto.as_mut_ptr(), from.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}
