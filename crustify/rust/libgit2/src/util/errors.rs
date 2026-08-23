//! Safe wrappers for libgit2 errors APIs.

use core::ptr::addr_of;
use std::ffi::{CStr, CString};

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_error
    /// Details for an error reported by libgit2.
    ///
    /// The object and its message may be owned by libgit2. Borrowed handles do
    /// not take ownership and must not outlive the storage that supplied them.
    GitError,
    GitErrorRef,
    GitErrorMut,
    ffi::git_error
);

impl<'a> GitErrorRef<'a> {
    /// Wraps: git_error.message
    /// Copies the optional NUL-terminated error message.
    ///
    /// libgit2 may replace its thread-local message during a later call, so
    /// this accessor returns an owned snapshot instead of lending that buffer.
    #[must_use]
    pub fn message(&self) -> Option<CString> {
        // SAFETY: `self` carries a live shared borrow of an initialized
        // `git_error`; raw-place projection reads the pointer field without
        // forming a reference to C-visible object storage.
        let message = unsafe { addr_of!((*self.as_ptr()).message).read() };

        if message.is_null() {
            None
        } else {
            // SAFETY: a live `git_error` with a non-null message points to a
            // NUL-terminated string. The current thread cannot replace its
            // thread-local error buffer during this accessor, and other
            // threads use distinct buffers. Copying prevents the result from
            // outliving libgit2's storage.
            Some(unsafe { CStr::from_ptr(message) }.to_owned())
        }
    }

    /// Wraps: git_error.klass
    /// Returns the raw libgit2 error class value.
    #[must_use]
    pub fn klass(&self) -> i32 {
        // SAFETY: `self` carries a live shared borrow of an initialized
        // `git_error`; raw-place projection reads the scalar field without
        // forming a reference to C-visible object storage.
        unsafe { addr_of!((*self.as_ptr()).klass).read() }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn error_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<GitError>(), size_of::<ffi::git_error>());
        assert_eq!(align_of::<GitError>(), align_of::<ffi::git_error>());
        assert_eq!(
            size_of::<GitErrorRef<'_>>(),
            size_of::<*mut ffi::git_error>()
        );
        assert_eq!(
            size_of::<GitErrorMut<'_>>(),
            size_of::<*mut ffi::git_error>()
        );
    }

    #[test]
    fn shared_handle_reads_error_fields() {
        let mut raw = ffi::git_error {
            message: c"failure".as_ptr().cast_mut(),
            klass: 7,
        };

        // SAFETY: `raw` is initialized and remains live, the static message
        // outlives the handle, and no mutation occurs while it is used.
        let error = unsafe { GitErrorRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(error.message().as_deref(), Some(c"failure"));
        assert_eq!(error.klass(), 7);
    }

    #[test]
    fn shared_handle_accepts_a_null_message() {
        let mut raw = ffi::git_error {
            message: core::ptr::null_mut(),
            klass: 0,
        };

        // SAFETY: `raw` is initialized and remains live, and no mutation
        // occurs while the handle is used.
        let error = unsafe { GitErrorRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(error.message(), None);
    }
}
