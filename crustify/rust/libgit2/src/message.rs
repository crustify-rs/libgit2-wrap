//! Safe wrappers for libgit2 message APIs.

use core::ffi::CStr;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_message_trailer
    /// A key/value pair borrowed from an owning message-trailer array.
    ///
    /// Both strings point into the array's private trailer block. A borrowed
    /// handle must therefore never outlive the array that produced it.
    GitMessageTrailer,
    GitMessageTrailerRef,
    GitMessageTrailerMut,
    ffi::git_message_trailer
);

impl<'a> GitMessageTrailerRef<'a> {
    /// Wraps: git_message_trailer.key
    /// Returns the trailer key borrowed from the containing trailer array.
    #[must_use]
    pub fn key(&self) -> &'a CStr {
        let trailer = self.as_ptr();
        // SAFETY: a live trailer produced by `git_message_trailers` has a
        // non-null NUL-terminated key in its array-owned trailer block. The
        // handle's lifetime is bounded by the owner that keeps that block
        // alive, and raw-place projection forms no reference to C storage.
        unsafe { CStr::from_ptr(core::ptr::addr_of!((*trailer).key).read()) }
    }

    /// Wraps: git_message_trailer.value
    /// Returns the trailer value borrowed from the containing trailer array.
    #[must_use]
    pub fn value(&self) -> &'a CStr {
        let trailer = self.as_ptr();
        // SAFETY: a live trailer produced by `git_message_trailers` has a
        // non-null NUL-terminated value in its array-owned trailer block. The
        // handle's lifetime is bounded by the owner that keeps that block
        // alive, and raw-place projection forms no reference to C storage.
        unsafe { CStr::from_ptr(core::ptr::addr_of!((*trailer).value).read()) }
    }
}

impl GitMessageTrailerMut<'_> {
    /// Stores a borrowed trailer key.
    ///
    /// # Safety
    ///
    /// `key` must remain alive and NUL-terminated for every subsequent use of
    /// the trailer, including uses after this mutable handle is released.
    pub unsafe fn set_key(&mut self, key: &CStr) {
        let trailer = self.as_mut_ptr();
        // SAFETY: this handle provides exclusive access to the pointer field,
        // raw-place projection forms no reference to C storage, and the caller
        // guarantees the stored string's required lifetime.
        unsafe { core::ptr::addr_of_mut!((*trailer).key).write(key.as_ptr()) }
    }

    /// Stores a borrowed trailer value.
    ///
    /// # Safety
    ///
    /// `value` must remain alive and NUL-terminated for every subsequent use
    /// of the trailer, including uses after this mutable handle is released.
    pub unsafe fn set_value(&mut self, value: &CStr) {
        let trailer = self.as_mut_ptr();
        // SAFETY: this handle provides exclusive access to the pointer field,
        // raw-place projection forms no reference to C storage, and the caller
        // guarantees the stored string's required lifetime.
        unsafe { core::ptr::addr_of_mut!((*trailer).value).write(value.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn trailer_wrapper_preserves_the_c_layout() {
        assert_eq!(
            size_of::<GitMessageTrailer>(),
            size_of::<ffi::git_message_trailer>()
        );
        assert_eq!(
            align_of::<GitMessageTrailer>(),
            align_of::<ffi::git_message_trailer>()
        );
        assert_eq!(
            size_of::<GitMessageTrailerRef<'_>>(),
            size_of::<*const ffi::git_message_trailer>()
        );
        assert_eq!(
            size_of::<GitMessageTrailerMut<'_>>(),
            size_of::<*mut ffi::git_message_trailer>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_replace_both_strings() {
        let first_key = c"Signed-off-by";
        let first_value = c"A. U. Thor";
        let second_key = c"Reviewed-by";
        let second_value = c"R. Viewer";
        let mut raw = ffi::git_message_trailer {
            key: first_key.as_ptr(),
            value: first_value.as_ptr(),
        };

        // SAFETY: `raw` and all four static C string literals remain alive for
        // the handles' complete use, and the mutable handle is exclusive.
        let mut trailer = unsafe { GitMessageTrailerMut::from_ptr(&raw mut raw) }
            .expect("address of a local trailer is non-null");
        assert_eq!(trailer.as_ref().key(), first_key);
        assert_eq!(trailer.as_ref().value(), first_value);
        // SAFETY: the replacement strings have static storage duration.
        unsafe {
            trailer.set_key(second_key);
            trailer.set_value(second_value);
        }
        assert_eq!(trailer.as_ref().key(), second_key);
        assert_eq!(trailer.as_ref().value(), second_value);
    }
}
