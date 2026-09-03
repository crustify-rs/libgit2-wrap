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
mod unit_tests {
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

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{Libgit2Init, TempDir};

    unsafe fn raw_loose(path: &core::ffi::CStr) -> (Vec<u8>, Vec<u8>, usize) {
        let mut odb = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_odb_new(&mut odb) }, 0);
        let mut options = unsafe { core::mem::zeroed::<ffi::git_odb_backend_loose_options>() };
        assert_eq!(
            unsafe {
                ffi::git_odb_backend_loose_options_init(
                    &mut options,
                    ffi::GIT_ODB_BACKEND_LOOSE_OPTIONS_VERSION,
                )
            },
            0
        );
        options.compression_level = 1;
        options.dir_mode = 0o755;
        options.file_mode = 0o644;
        let mut backend = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_odb_backend_loose(&mut backend, path.as_ptr(), &mut options) },
            0
        );
        assert_eq!(unsafe { ffi::git_odb_add_backend(odb, backend, 10) }, 0);
        let payload = b"loose backend equivalence payload\n";
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_odb_write(
                    &mut id,
                    odb,
                    payload.as_ptr().cast(),
                    payload.len(),
                    ffi::git_object_t_GIT_OBJECT_BLOB,
                )
            },
            0
        );
        let mut object = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_odb_read(&mut object, odb, &id) }, 0);
        let size = unsafe { ffi::git_odb_object_size(object) };
        let data = unsafe {
            core::slice::from_raw_parts(ffi::git_odb_object_data(object).cast::<u8>(), size)
                .to_vec()
        };
        unsafe {
            ffi::git_odb_object_free(object);
            ffi::git_odb_free(odb);
        }
        (id.id.to_vec(), data, size)
    }

    fn safe_loose(path: &core::ffi::CStr) -> (Vec<u8>, Vec<u8>, usize) {
        let mut odb = crate::odb::git_odb_new().unwrap();
        let mut options =
            git_odb_backend_loose_options_init(ffi::GIT_ODB_BACKEND_LOOSE_OPTIONS_VERSION).unwrap();
        options.as_mut().set_compression_level(1);
        options.as_mut().set_dir_mode(0o755);
        options.as_mut().set_file_mode(0o644);
        let backend = git_odb_backend_loose(path, Some(options.as_ref())).unwrap();
        crate::odb::git_odb_add_backend(&mut odb.as_mut(), backend, 10).unwrap();
        let mut id = crate::odb::git_odb_write(
            odb.as_ref(),
            b"loose backend equivalence payload\n",
            crate::api::types::GitObjectType::BLOB,
        )
        .unwrap();
        let id_ref =
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::from_mut(&mut id).cast()) }.unwrap();
        let object = crate::odb::git_odb_read(&mut odb.as_mut(), id_ref).unwrap();
        let data = crate::odb::git_odb_object_data(object.as_ref())
            .unwrap()
            .elems()
            .collect();
        (
            id_ref.raw_bytes().elems().collect(),
            data,
            crate::odb::git_odb_object_size(object.as_ref()),
        )
    }

    #[test]
    fn io_equiv_loose_backend_options_write_and_read() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = TempDir::new("odb-loose-raw");
        let safe = TempDir::new("odb-loose-safe");
        let raw = unsafe { raw_loose(&raw.c_path()) };
        assert_eq!(raw, safe_loose(&safe.c_path()));
        assert_eq!(raw.1, b"loose backend equivalence payload\n");
    }
}
