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
mod unit_tests {
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

#[cfg(test)]
mod io_equiv {
    use super::*;
    use crate::io_equiv_support::Libgit2Init;

    #[test]
    fn io_equiv_git_message_trailers() {
        let _init = Libgit2Init::acquire();
        let message = c"subject\n\nSigned-off-by: A. U. Thor\nReviewed-by: R. Viewer\n";
        let mut raw = ffi::git_message_trailer_array {
            trailers: core::ptr::null_mut(),
            count: 0,
            _trailer_block: core::ptr::null_mut(),
        };
        // SAFETY: `raw` is a writable empty header and `message` is a live C
        // string; successful parsing transfers the array fields to `raw`.
        let raw_status = unsafe { ffi::git_message_trailers(&mut raw, message.as_ptr()) };
        assert_eq!(raw_status, 0);

        let raw_observation: Vec<(Vec<u8>, Vec<u8>)> = (0..raw.count)
            .map(|index| {
                // SAFETY: success initialized `count` descriptors and their
                // two non-null C strings in the array-owned trailer block.
                let trailer = unsafe { &*raw.trailers.add(index) };
                // SAFETY: both pointers are non-null array-owned C strings.
                let (key, value) = unsafe {
                    (
                        CStr::from_ptr(trailer.key).to_bytes().to_vec(),
                        CStr::from_ptr(trailer.value).to_bytes().to_vec(),
                    )
                };
                (key, value)
            })
            .collect();

        let safe = git_message_trailers(message).unwrap();
        let safe_view = safe.as_ref();
        let safe_trailers = safe_view.trailers().unwrap();
        let safe_observation: Vec<(Vec<u8>, Vec<u8>)> = (0..safe_view.len())
            .map(|index| {
                let trailer = safe_trailers.get(index).unwrap();
                (
                    trailer.key().to_bytes().to_vec(),
                    trailer.value().to_bytes().to_vec(),
                )
            })
            .collect();

        assert_eq!(safe_observation, raw_observation);
        drop(safe);
        // SAFETY: releases the two allocations transferred into `raw` once.
        unsafe { ffi::git_message_trailer_array_free(&mut raw) };
    }
}
