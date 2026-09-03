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

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    fn pack_paths(
        fixture: &HistoryFixture,
    ) -> (std::ffi::CString, std::ffi::CString, ffi::git_oid) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args(["gc", "--prune=now"])
            .status()
            .unwrap();
        assert!(status.success());
        let objects = fixture.directory.path().join(".git/objects");
        let index = std::fs::read_dir(objects.join("pack"))
            .unwrap()
            .map(Result::unwrap)
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|extension| extension == "idx"))
            .unwrap();
        let objects = std::ffi::CString::new(objects.to_str().unwrap()).unwrap();
        let index = std::ffi::CString::new(index.to_str().unwrap()).unwrap();
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut head,
                    fixture.repository.as_ptr(),
                    c"HEAD".as_ptr(),
                )
            },
            0
        );
        (objects, index, head)
    }

    unsafe fn raw_pack_backend(
        path: &core::ffi::CStr,
        head: &ffi::git_oid,
        one_pack: bool,
    ) -> (i32, usize, Vec<u8>) {
        let mut options = unsafe { core::mem::zeroed::<ffi::git_odb_backend_pack_options>() };
        assert_eq!(
            unsafe {
                ffi::git_odb_backend_pack_options_init(
                    &mut options,
                    ffi::GIT_ODB_BACKEND_PACK_OPTIONS_VERSION,
                )
            },
            0
        );
        options.oid_type = ffi::git_oid_t_GIT_OID_SHA1;
        let mut backend = core::ptr::null_mut();
        let status = if one_pack {
            unsafe { ffi::git_odb_backend_one_pack(&mut backend, path.as_ptr(), &options) }
        } else {
            unsafe { ffi::git_odb_backend_pack(&mut backend, path.as_ptr(), &options) }
        };
        assert_eq!(status, 0);
        let mut odb = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_odb_new(&mut odb) }, 0);
        assert_eq!(unsafe { ffi::git_odb_add_backend(odb, backend, 10) }, 0);
        let mut object = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_odb_read(&mut object, odb, head) }, 0);
        let kind = unsafe { ffi::git_odb_object_type(object) };
        let size = unsafe { ffi::git_odb_object_size(object) };
        let prefix = unsafe {
            core::slice::from_raw_parts(ffi::git_odb_object_data(object).cast::<u8>(), size.min(32))
                .to_vec()
        };
        unsafe {
            ffi::git_odb_object_free(object);
            ffi::git_odb_free(odb);
        }
        (kind, size, prefix)
    }

    fn safe_pack_backend(
        path: &core::ffi::CStr,
        head: &mut ffi::git_oid,
        one_pack: bool,
    ) -> (i32, usize, Vec<u8>) {
        let mut options =
            git_odb_backend_pack_options_init(ffi::GIT_ODB_BACKEND_PACK_OPTIONS_VERSION).unwrap();
        options
            .as_mut()
            .set_oid_type(Some(crate::oid::OidType::Sha1));
        let backend = if one_pack {
            git_odb_backend_one_pack(path, Some(options.as_ref())).unwrap()
        } else {
            git_odb_backend_pack(path, Some(options.as_ref())).unwrap()
        };
        let mut odb = crate::odb::git_odb_new().unwrap();
        crate::odb::git_odb_add_backend(&mut odb.as_mut(), backend, 10).unwrap();
        let head = unsafe { crate::oid::OidRef::from_ptr(head) }.unwrap();
        let object = crate::odb::git_odb_read(&mut odb.as_mut(), head).unwrap();
        (
            crate::odb::git_odb_object_type(object.as_ref())
                .unwrap()
                .as_raw(),
            crate::odb::git_odb_object_size(object.as_ref()),
            crate::odb::git_odb_object_data(object.as_ref())
                .unwrap()
                .elems()
                .take(32)
                .collect(),
        )
    }

    #[test]
    fn io_equiv_pack_directory_and_single_pack_backends_read_objects() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("odb-pack-backend-raw");
        let safe = HistoryFixture::new("odb-pack-backend-safe");
        let (raw_objects, raw_index, raw_head) = pack_paths(&raw);
        let (safe_objects, safe_index, mut safe_head) = pack_paths(&safe);
        let raw_one = unsafe { raw_pack_backend(&raw_index, &raw_head, true) };
        let safe_one = safe_pack_backend(&safe_index, &mut safe_head, true);
        assert_eq!(raw_one, safe_one);
        let raw_directory = unsafe { raw_pack_backend(&raw_objects, &raw_head, false) };
        let safe_directory = safe_pack_backend(&safe_objects, &mut safe_head, false);
        assert_eq!(raw_directory, safe_directory);
        assert_eq!(raw_one, raw_directory);
        assert_eq!(raw_one.0, ffi::git_object_t_GIT_OBJECT_COMMIT);
    }
}
