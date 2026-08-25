//! Safe wrappers for libgit2 odb_pack APIs.

use ffibox::CVal;

use crate::api::odb_backend::{GitOdbBackendPackOptions, GitOdbBackendPackOptionsRef};
use crate::ffi;
use crate::sys::odb_backend::GitOdbBackendOwned;

/// Wraps: git_odb_backend_pack_options_init
/// Initializes pack-backend options for the requested ABI version.
pub fn git_odb_backend_pack_options_init(
    version: core::ffi::c_uint,
) -> Result<CVal<GitOdbBackendPackOptions>, i32> {
    let mut options = GitOdbBackendPackOptions::new();
    let status = {
        let mut output = options.as_mut();
        // SAFETY: `output` is writable layout-compatible storage and C does
        // not retain its address.
        unsafe { ffi::git_odb_backend_pack_options_init(output.as_mut_ptr(), version) }
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_odb_backend_one_pack
/// Creates a backend for the pack described by `index_file`.
pub fn git_odb_backend_one_pack(
    index_file: &core::ffi::CStr,
    options: Option<GitOdbBackendPackOptionsRef<'_>>,
) -> Result<GitOdbBackendOwned, i32> {
    backend_result(|out| {
        // SAFETY: `out` is writable and every optional input is live for this
        // non-retaining constructor call.
        unsafe {
            ffi::git_odb_backend_one_pack(
                out,
                index_file.as_ptr(),
                options.map_or(core::ptr::null(), |value| value.as_ptr()),
            )
        }
    })
}

/// Wraps: git_odb_backend_pack
/// Creates a backend for the pack directory below `objects_dir`.
pub fn git_odb_backend_pack(
    objects_dir: &core::ffi::CStr,
    options: Option<GitOdbBackendPackOptionsRef<'_>>,
) -> Result<GitOdbBackendOwned, i32> {
    backend_result(|out| {
        // SAFETY: `out` is writable and every optional input is live for this
        // non-retaining constructor call.
        unsafe {
            ffi::git_odb_backend_pack(
                out,
                objects_dir.as_ptr(),
                options.map_or(core::ptr::null(), |value| value.as_ptr()),
            )
        }
    })
}

fn backend_result(
    construct: impl FnOnce(*mut *mut ffi::git_odb_backend) -> i32,
) -> Result<GitOdbBackendOwned, i32> {
    let mut out = core::ptr::null_mut();
    let status = construct(&mut out);
    if status != 0 {
        return Err(status);
    }
    // SAFETY: either constructor returns one complete backend with its
    // concrete destructor installed when it reports success.
    unsafe { GitOdbBackendOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_initializer_sets_pack_backend_version() {
        let options = git_odb_backend_pack_options_init(ffi::GIT_ODB_BACKEND_PACK_OPTIONS_VERSION)
            .expect("the published version initializes");
        assert_eq!(options.as_ref().version(), 1);
        assert_eq!(options.as_ref().oid_type(), Ok(None));
    }

    #[test]
    fn constructor_helper_rejects_null_success_output() {
        assert!(matches!(
            backend_result(|_| 0),
            Err(ffi::git_error_code_GIT_ERROR)
        ));
    }
}
