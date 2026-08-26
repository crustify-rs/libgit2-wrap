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

#[cfg(test)]
mod scheduled_constructor_tests {
    use super::*;

    /// Holds one libgit2 initialization count for the duration of a test.
    ///
    /// The pack backends reach the process-global mwindow registry and its
    /// mutex, both of which only initialization creates.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and reference
            // counted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the initialization this guard represents,
            // after every libgit2 owner in the test has been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    fn temporary_directory(tag: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "crustify-odb-pack-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a fresh temporary directory");
        directory
    }

    fn c_path(path: &std::path::Path) -> std::ffi::CString {
        std::ffi::CString::new(path.to_str().expect("a temporary path is UTF-8"))
            .expect("a temporary path holds no interior NUL")
    }

    /// The directory constructor succeeds whether or not a `pack` folder is
    /// present: `git_odb_backend_pack` only refreshes when `objects_dir/pack`
    /// is a directory, and reports the allocated backend either way.
    #[test]
    fn the_directory_constructor_owns_a_backend_with_or_without_a_pack_folder() {
        let _libgit2 = Libgit2Init::acquire();
        let directory = temporary_directory("dir");

        let options = git_odb_backend_pack_options_init(ffi::GIT_ODB_BACKEND_PACK_OPTIONS_VERSION)
            .expect("the published version initializes");

        let backend = git_odb_backend_pack(&c_path(&directory), Some(options.as_ref()))
            .expect("an objects directory without a pack folder still allocates");
        assert_eq!(backend.as_ref().version(), 1);
        drop(backend);

        std::fs::create_dir_all(directory.join("pack")).expect("an empty pack folder");
        let backend = git_odb_backend_pack(&c_path(&directory), None)
            .expect("an empty pack folder refreshes to no packs");
        assert_eq!(backend.as_ref().version(), 1);
        // Dropping runs `pack_backend__free`, which releases the pack vectors
        // and the detached folder path the refresh above installed.
        drop(backend);

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// The single-pack constructor resolves its index path eagerly, so a
    /// missing pack is an error rather than an empty backend, and the partly
    /// built backend is released by C before the wrapper ever sees it.
    #[test]
    fn the_single_pack_constructor_reports_a_missing_pack() {
        let _libgit2 = Libgit2Init::acquire();
        let directory = temporary_directory("one");
        let missing = c_path(&directory.join("pack-absent.idx"));

        assert!(
            git_odb_backend_one_pack(&missing, None).is_err(),
            "a nonexistent index file cannot back a pack"
        );

        let options = git_odb_backend_pack_options_init(ffi::GIT_ODB_BACKEND_PACK_OPTIONS_VERSION)
            .expect("the published version initializes");
        assert!(
            git_odb_backend_one_pack(&missing, Some(options.as_ref())).is_err(),
            "supplying options does not change the missing-pack outcome"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }
}
