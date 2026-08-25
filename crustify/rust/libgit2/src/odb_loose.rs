//! Safe wrappers for libgit2 odb_loose APIs.

use ffibox::CVal;

use crate::api::odb_backend::GitOdbBackendLooseOptions;
use crate::ffi;

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
}
