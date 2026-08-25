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

/// Wraps: git_diff_from_buffer_ext
/// Parses a patch using explicit parse options.
pub fn git_diff_from_buffer_ext(
    content: &[u8],
    options: &mut crate::api::diff::DiffParseOptionsMut<'_>,
) -> Result<DiffOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output is writable, content covers its exact byte length, and
    // options is exclusively borrowed for this non-retaining parse.
    let status = unsafe {
        ffi::git_diff_from_buffer_ext(
            &mut out,
            content.as_ptr().cast(),
            content.len(),
            options.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete owned diff.
    unsafe { DiffOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Holds one libgit2 initialization count for the duration of a test.
    ///
    /// Parsing reaches libgit2's allocator and thread-local error state,
    /// neither of which exists before `git_libgit2_init`; without this guard
    /// the test only survives when an unrelated test happens to hold a count.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and
            // refcounted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard, after every libgit2 owner has already been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    #[test]
    fn parses_a_minimal_patch_into_an_owned_diff() {
        let _libgit2 = Libgit2Init::acquire();
        let patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n";
        let mut diff = git_diff_from_buffer(patch).expect("valid patch");
        assert_eq!(crate::diff::git_diff_num_deltas(diff.as_ref()), 1);
        let options = crate::diff::git_diff_patchid_options_init().unwrap();
        let _oid = crate::diff::git_diff_patchid(&mut diff.as_mut(), Some(options)).unwrap();
    }
}
