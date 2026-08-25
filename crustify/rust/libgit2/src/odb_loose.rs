//! Safe wrappers for libgit2 odb_loose APIs.

use ffibox::CVal;

use crate::api::odb_backend::{GitOdbBackendLooseOptions, GitOdbBackendLooseOptionsRef};
use crate::ffi;
use crate::sys::odb_backend::GitOdbBackendOwned;

/// Wraps: git_odb_backend_loose_options_init
/// Initializes loose-backend options for the requested ABI version.
pub fn git_odb_backend_loose_options_init(
    version: core::ffi::c_uint,
) -> Result<CVal<GitOdbBackendLooseOptions>, i32> {
    let mut options = GitOdbBackendLooseOptions::new();
    let status = {
        let mut output = options.as_mut();
        // SAFETY: `output` is writable layout-compatible storage and C does
        // not retain its address.
        unsafe { ffi::git_odb_backend_loose_options_init(output.as_mut_ptr(), version) }
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_odb_backend_loose
/// Creates a loose-object backend rooted at `objects_dir`.
pub fn git_odb_backend_loose(
    objects_dir: &core::ffi::CStr,
    options: Option<GitOdbBackendLooseOptionsRef<'_>>,
) -> Result<GitOdbBackendOwned, i32> {
    if objects_dir.is_empty() {
        // C indexes `objects_dir[len - 1]` while adding a trailing slash, so
        // an empty path cannot safely cross the boundary.
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    let mut out = core::ptr::null_mut();
    // SAFETY: the output slot is writable, the directory and optional options
    // remain live for this call, and C copies rather than retains them. Source
    // inspection confirms that the legacy non-const options pointer is read only.
    let status = unsafe {
        ffi::git_odb_backend_loose(
            &mut out,
            objects_dir.as_ptr(),
            options.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut()),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully constructed backend whose callback
    // table contains its concrete destructor.
    unsafe { GitOdbBackendOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_initializer_sets_loose_backend_defaults() {
        let options =
            git_odb_backend_loose_options_init(ffi::GIT_ODB_BACKEND_LOOSE_OPTIONS_VERSION)
                .expect("the published version initializes");
        assert_eq!(options.as_ref().compression_level(), -1);
        assert_eq!(options.as_ref().version(), 1);
    }

    #[test]
    fn loose_constructor_returns_a_backend_owner() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after the backend owner has invoked its concrete destructor.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let backend = git_odb_backend_loose(c".", None).expect("loose backend allocation");
        assert_eq!(backend.as_ref().version(), 1);
        drop(backend);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn loose_constructor_rejects_an_empty_directory_before_ffi() {
        assert!(matches!(
            git_odb_backend_loose(c"", None),
            Err(ffi::git_error_code_GIT_EINVALID)
        ));
    }
}
