//! Safe wrappers for libgit2 tag APIs.

use crate::ffi;

/// Wraps: git_tag_name_is_valid
/// Checks whether `name` is a valid tag shorthand.
///
/// `None` mirrors libgit2's accepted null input and is reported as invalid.
pub fn git_tag_name_is_valid(name: Option<&core::ffi::CStr>) -> Result<bool, i32> {
    let mut valid = 0;
    let name = name.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `valid` is writable and `name` is null or a live C string.
    let status = unsafe { ffi::git_tag_name_is_valid(&mut valid, name) };
    if status == 0 {
        Ok(valid != 0)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_names_are_checked_through_the_safe_surface() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(git_tag_name_is_valid(Some(c"v1.0")), Ok(true));
        assert_eq!(git_tag_name_is_valid(None), Ok(false));
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
