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

/// Wraps: git_error_clear
/// Clears this thread's last libgit2 error and platform error state.
pub fn git_error_clear() {
    // SAFETY: this function has no pointer arguments and only resets
    // thread-local error state owned by libgit2.
    unsafe { ffi::git_error_clear() }
}

/// Wraps: git_error_set_str
/// Copies `message` into this thread's libgit2 error state.
pub fn git_error_set_str(error_class: i32, message: &CStr) -> Result<(), i32> {
    // SAFETY: `message` is a live NUL-terminated string and libgit2 copies it
    // before returning rather than retaining the pointer.
    let status = unsafe { ffi::git_error_set_str(error_class, message.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn thread_error_can_be_set_and_cleared_from_safe_rust() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(git_error_set_str(1, c"failure"), Ok(()));
        git_error_clear();
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn last_error_is_copied_out_of_thread_local_storage() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(git_error_set_str(7, c"snapshot"), Ok(()));
        let snapshot = git_error_last();
        assert_eq!(snapshot.message.as_deref(), Some(c"snapshot"));
        assert_eq!(snapshot.klass, 7);
        git_error_clear();
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

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

/// An owned copy of libgit2's thread-local last-error record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitErrorSnapshot {
    /// The optional copied error message.
    pub message: Option<CString>,
    /// The libgit2 error class.
    pub klass: i32,
}

/// Wraps: git_error_last
/// Copies the current thread's last-error record before another libgit2 call
/// can replace its thread-local storage.
#[must_use]
pub fn git_error_last() -> GitErrorSnapshot {
    // SAFETY: libgit2 documents a non-null pointer to an initialized
    // thread-local record. The handle is kept private and used only until both
    // fields have been copied below.
    let error = unsafe { GitErrorRef::from_ptr(ffi::git_error_last().cast_mut()) }
        .expect("git_error_last is documented never to return null");
    GitErrorSnapshot {
        message: error.message(),
        klass: error.klass(),
    }
}
