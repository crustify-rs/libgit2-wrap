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

/// Wraps: git_diff_find_similar
/// Detects renames and copies, transforming `diff` in place.
pub fn git_diff_find_similar(
    diff: &mut DiffMut<'_>,
    options: Option<crate::api::diff::DiffFindOptionsRef<'_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the diff is exclusively borrowed and the optional options and
    // custom metric it names remain live for the synchronous transformation.
    let status = unsafe { ffi::git_diff_find_similar(diff.as_mut_ptr(), options) };
    if status == 0 { Ok(()) } else { Err(status) }
}
