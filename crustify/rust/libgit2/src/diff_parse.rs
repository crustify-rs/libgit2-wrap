//! Safe wrappers for libgit2 diff_parse APIs.

use crate::diff::DiffOwned;
use crate::ffi;

/// Wraps: git_diff_from_buffer
/// Parses a Git patch from a counted byte buffer.
pub fn git_diff_from_buffer(content: &[u8]) -> Result<DiffOwned, i32> {
    let mut out = core::ptr::null_mut();
    let content_ptr = if content.is_empty() {
        b"".as_ptr()
    } else {
        content.as_ptr()
    };
    // SAFETY: `content` supplies exactly `content.len()` readable bytes and C
    // retains no pointer into it. `out` is a writable result slot.
    let status = unsafe { ffi::git_diff_from_buffer(&mut out, content_ptr.cast(), content.len()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one complete owned diff reference whose drop
    // contract calls `git_diff_free`.
    unsafe { DiffOwned::from_raw(out) }.ok_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_minimal_patch_into_an_owned_diff() {
        let patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n";
        let mut diff = git_diff_from_buffer(patch).expect("valid patch");
        assert_eq!(crate::diff::git_diff_num_deltas(diff.as_ref()), 1);
        let options = crate::diff::git_diff_patchid_options_init().unwrap();
        let oid = crate::diff::git_diff_patchid(&mut diff.as_mut(), Some(options)).unwrap();
        drop(oid);
    }
}
