//! Safe wrappers for libgit2 trailer APIs.

use core::ffi::CStr;

use ffibox::CVal;

use crate::ffi;
use crate::message::GitMessageTrailerArray;

/// Wraps: git_message_trailers
/// Parses the final trailer block into an owned trailer array.
pub fn git_message_trailers(message: &CStr) -> Result<CVal<GitMessageTrailerArray>, i32> {
    let mut trailers = GitMessageTrailerArray::new();
    let status = {
        let mut out = trailers.as_mut();
        // SAFETY: `out` is an empty exclusively borrowed owner header and
        // `message` is a live C string; parsing copies all retained bytes.
        unsafe { ffi::git_message_trailers(out.as_mut_ptr(), message.as_ptr()) }
    };
    if status == 0 {
        Ok(trailers)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_owns_message_trailers() {
        // SAFETY: libgit2 initialization is refcounted and balanced after the
        // returned trailer owner has been dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let trailers = git_message_trailers(c"subject\n\nSigned-off-by: A. U. Thor\n").unwrap();
        assert_eq!(trailers.as_ref().len(), 1);
        let view = trailers.as_ref().trailers().unwrap();
        assert_eq!(view.get(0).unwrap().key(), c"Signed-off-by");
        assert_eq!(view.get(0).unwrap().value(), c"A. U. Thor");
        let _ = view;
        drop(trailers);
        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
