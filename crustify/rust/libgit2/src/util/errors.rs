//! Safe wrappers for libgit2 errors APIs.

use core::{fmt, ptr::addr_of};
use std::ffi::{CStr, CString, NulError};

use crate::api::errors::{GitErrorClass, InvalidGitErrorClass};
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
    /// Field: git_error.message
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

    /// Field: git_error.klass
    /// Returns the validated libgit2 error class.
    pub fn klass(&self) -> Result<GitErrorClass, InvalidGitErrorClass> {
        // SAFETY: `self` carries a live shared borrow of an initialized
        // `git_error`; raw-place projection reads the scalar field without
        // forming a reference to C-visible object storage.
        let raw = unsafe { addr_of!((*self.as_ptr()).klass).read() };
        GitErrorClass::try_from(raw)
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
pub fn git_error_set_str(
    error_class: GitErrorClass,
    message: &CStr,
) -> Result<(), core::ffi::c_int> {
    // SAFETY: `message` is a live NUL-terminated string and libgit2 copies it
    // before returning rather than retaining the pointer.
    let status = unsafe { ffi::git_error_set_str(error_class.as_c_int(), message.as_ptr()) };
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
        assert_eq!(
            git_error_set_str(GitErrorClass::NoMemory, c"failure"),
            Ok(())
        );
        git_error_clear();
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn last_error_is_copied_out_of_thread_local_storage() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(
            git_error_set_str(GitErrorClass::Config, c"snapshot"),
            Ok(())
        );
        let snapshot = git_error_last();
        assert_eq!(snapshot.message.as_deref(), Some(c"snapshot"));
        assert_eq!(snapshot.klass, Ok(GitErrorClass::Config));
        git_error_clear();
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn variadic_error_setter_formats_safely_in_rust() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(
            git_error_set(
                GitErrorClass::Ssh,
                Some(format_args!("{} is {}% ready", "wrapper", 100))
            ),
            Ok(())
        );
        let snapshot = git_error_last();
        assert_eq!(snapshot.message.as_deref(), Some(c"wrapper is 100% ready"));
        assert_eq!(snapshot.klass, Ok(GitErrorClass::Ssh));
        assert_eq!(git_error_set(GitErrorClass::Filter, None), Ok(()));
        let snapshot = git_error_last();
        assert_eq!(snapshot.klass, Ok(GitErrorClass::Filter));
        // The null-format branch records the class over an emptied buffer.
        assert_eq!(snapshot.message.as_deref(), Some(c""));
        git_error_clear();
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn os_class_appends_the_platform_error_and_clears_errno() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let failure = std::fs::File::open("/crustify/no/such/path").unwrap_err();
        assert_eq!(failure.raw_os_error(), Some(2));
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(2));

        assert_eq!(
            git_error_set(GitErrorClass::Os, Some(format_args!("open failed"))),
            Ok(())
        );

        let snapshot = git_error_last();
        assert_eq!(snapshot.klass, Ok(GitErrorClass::Os));
        let message = snapshot.message.unwrap();
        let message = message.to_str().unwrap();
        assert_eq!(message, "open failed: No such file or directory");
        // libgit2 consumed the platform error state while formatting.
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(0));

        git_error_clear();
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn variadic_error_setter_rejects_an_interior_nul() {
        assert!(
            git_error_set(GitErrorClass::NoMemory, Some(format_args!("bad\0message"))).is_err()
        );
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
        assert_eq!(error.klass(), Ok(GitErrorClass::Config));
    }

    #[test]
    fn shared_handle_rejects_an_unknown_error_class() {
        let mut raw = ffi::git_error {
            message: core::ptr::null_mut(),
            klass: -1,
        };

        // SAFETY: `raw` is initialized and remains live, and no mutation
        // occurs while the handle is used.
        let error = unsafe { GitErrorRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(error.klass().unwrap_err().value(), -1);
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
    pub klass: Result<GitErrorClass, InvalidGitErrorClass>,
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

/// Wraps: git_error_set
/// Formats and copies a message into this thread's libgit2 error state.
///
/// Rust performs the formatting so callers never need to satisfy C variadic
/// argument rules. An interior NUL in the formatted message is rejected.
/// Passing `None` selects the explicit null-format branch of libgit2's
/// `git_error_vset`, which records `error_class` with an empty message.
///
/// For [`GitErrorClass::Os`] libgit2 samples the calling thread's `errno` and
/// appends its `strerror` text — behind `": "` when a message was supplied —
/// then resets `errno` to zero. A caller that still needs the platform error
/// must read it before calling. Rust renders the message before libgit2 takes
/// that sample, so a `Display` implementation performing system calls can
/// overwrite the `errno` libgit2 goes on to report.
///
/// libgit2 records nothing when it cannot allocate its thread-local error
/// state or grow the message buffer, so `Ok(())` reports that the message was
/// handed over, not that it was stored.
///
/// The process must have called `git_libgit2_init` before this function: the
/// thread-local error state is reached through a TLS key that initialization
/// allocates, and setting an error beforehand reads and writes through an
/// unallocated key.
pub fn git_error_set(
    error_class: GitErrorClass,
    message: Option<fmt::Arguments<'_>>,
) -> Result<(), NulError> {
    if let Some(message) = message {
        let message = CString::new(message.to_string())?;

        // SAFETY: the fixed format string consumes exactly one `const char *`
        // variadic argument. Both strings are live and NUL-terminated for the
        // duration of the call, and libgit2 copies the rendered message
        // instead of retaining either pointer.
        unsafe { ffi::git_error_set(error_class.as_c_int(), c"%s".as_ptr(), message.as_ptr()) };
    } else {
        // SAFETY: libgit2 explicitly accepts a null format and consequently
        // reads no variadic arguments; it records the class and, for OS
        // errors, obtains the message from the platform error state.
        unsafe { ffi::git_error_set(error_class.as_c_int(), core::ptr::null()) };
    }
    Ok(())
}
