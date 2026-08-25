//! Safe wrappers for libgit2 settings APIs.

use core::ffi::{CStr, c_char, c_int, c_ulong};

use ffibox::CVal;

use crate::{
    api::{buffer::GitBuf, types::GitObjectType},
    config::GitConfigLevel,
    ffi,
    strarray::GitStrArray,
};

/// A typed invocation of one of libgit2's scalar or string global options.
///
/// Each variant fixes one option key together with the exact promoted vararg
/// types that key reads, which is what makes [`git_libgit2_opts`] safe.
///
/// Options involving allocator callbacks or implementation-specific X.509
/// pointers are deliberately absent until those dependency types have
/// ownership-compatible wrappers.
pub enum Libgit2Option<'a> {
    GetMwindowSize(&'a mut usize),
    SetMwindowSize(usize),
    GetMwindowMappedLimit(&'a mut usize),
    SetMwindowMappedLimit(usize),
    GetMwindowFileLimit(&'a mut usize),
    SetMwindowFileLimit(usize),
    SetSearchPath {
        level: GitConfigLevel,
        path: Option<&'a CStr>,
    },
    GetSearchPath {
        level: GitConfigLevel,
        out: &'a mut CVal<GitBuf>,
    },
    SetCacheObjectLimit {
        object_type: GitObjectType,
        size: usize,
    },
    SetCacheMaxSize(isize),
    EnableCaching(bool),
    GetCachedMemory {
        current: &'a mut isize,
        allowed: &'a mut isize,
    },
    GetTemplatePath(&'a mut CVal<GitBuf>),
    SetTemplatePath(&'a CStr),
    SetSslCertLocations {
        file: Option<&'a CStr>,
        path: Option<&'a CStr>,
    },
    SetUserAgent(Option<&'a CStr>),
    GetUserAgent(&'a mut CVal<GitBuf>),
    SetUserAgentProduct(Option<&'a CStr>),
    GetUserAgentProduct(&'a mut CVal<GitBuf>),
    SetSslCiphers(&'a CStr),
    EnableStrictObjectCreation(bool),
    EnableStrictSymbolicRefCreation(bool),
    EnableOfsDelta(bool),
    EnableFsyncGitdir(bool),
    GetWindowsSharemode(&'a mut c_ulong),
    SetWindowsSharemode(c_ulong),
    EnableStrictHashVerification(bool),
    EnableUnsavedIndexSafety(bool),
    GetPackMaxObjects(&'a mut usize),
    SetPackMaxObjects(usize),
    GetPackMaxObjectSize(&'a mut usize),
    SetPackMaxObjectSize(usize),
    DisablePackKeepFileChecks(bool),
    EnableHttpExpectContinue(bool),
    SetOdbPackedPriority(i32),
    SetOdbLoosePriority(i32),
    GetExtensions(&'a mut CVal<GitStrArray>),
    SetExtensions(&'a [&'a CStr]),
    GetOwnerValidation(&'a mut bool),
    SetOwnerValidation(bool),
    GetHomedir(&'a mut CVal<GitBuf>),
    SetHomedir(&'a CStr),
    GetServerConnectTimeout(&'a mut i32),
    SetServerConnectTimeout(i32),
    GetServerTimeout(&'a mut i32),
    SetServerTimeout(i32),
}

const fn c_bool(value: bool) -> c_int {
    if value { 1 } else { 0 }
}

/// Wraps: git_libgit2_opts
/// Applies a typed global option and preserves libgit2's status code.
pub fn git_libgit2_opts(option: Libgit2Option<'_>) -> Result<(), i32> {
    use Libgit2Option::*;
    // SAFETY: every match arm supplies exactly the promoted C vararg types
    // selected by its fixed option key. All output references are live and
    // writable for the call, and every string is NUL-terminated and transient.
    let status = unsafe {
        match option {
            GetMwindowSize(out) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_GET_MWINDOW_SIZE as c_int,
                out,
            ),
            SetMwindowSize(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_MWINDOW_SIZE as c_int,
                value,
            ),
            GetMwindowMappedLimit(out) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_GET_MWINDOW_MAPPED_LIMIT as c_int,
                out,
            ),
            SetMwindowMappedLimit(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_MWINDOW_MAPPED_LIMIT as c_int,
                value,
            ),
            GetMwindowFileLimit(out) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_GET_MWINDOW_FILE_LIMIT as c_int,
                out,
            ),
            SetMwindowFileLimit(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_MWINDOW_FILE_LIMIT as c_int,
                value,
            ),
            SetSearchPath { level, path } => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_SEARCH_PATH as c_int,
                level.as_raw() as c_int,
                path.map_or(core::ptr::null(), CStr::as_ptr),
            ),
            GetSearchPath { level, out } => {
                let mut out = out.as_mut();
                ffi::git_libgit2_opts(
                    ffi::git_libgit2_opt_t_GIT_OPT_GET_SEARCH_PATH as c_int,
                    level.as_raw() as c_int,
                    out.as_mut_ptr(),
                )
            }
            SetCacheObjectLimit { object_type, size } => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_CACHE_OBJECT_LIMIT as c_int,
                object_type.as_raw() as c_int,
                size,
            ),
            SetCacheMaxSize(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_CACHE_MAX_SIZE as c_int,
                value,
            ),
            EnableCaching(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_CACHING as c_int,
                c_bool(value),
            ),
            GetCachedMemory { current, allowed } => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_GET_CACHED_MEMORY as c_int,
                current,
                allowed,
            ),
            GetTemplatePath(out) => {
                let mut out = out.as_mut();
                ffi::git_libgit2_opts(
                    ffi::git_libgit2_opt_t_GIT_OPT_GET_TEMPLATE_PATH as c_int,
                    out.as_mut_ptr(),
                )
            }
            SetTemplatePath(path) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_TEMPLATE_PATH as c_int,
                path.as_ptr(),
            ),
            SetSslCertLocations { file, path } => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_SSL_CERT_LOCATIONS as c_int,
                file.map_or(core::ptr::null(), CStr::as_ptr),
                path.map_or(core::ptr::null(), CStr::as_ptr),
            ),
            SetUserAgent(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_USER_AGENT as c_int,
                value.map_or(core::ptr::null(), CStr::as_ptr),
            ),
            GetUserAgent(out) => {
                let mut out = out.as_mut();
                ffi::git_libgit2_opts(
                    ffi::git_libgit2_opt_t_GIT_OPT_GET_USER_AGENT as c_int,
                    out.as_mut_ptr(),
                )
            }
            SetUserAgentProduct(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_USER_AGENT_PRODUCT as c_int,
                value.map_or(core::ptr::null(), CStr::as_ptr),
            ),
            GetUserAgentProduct(out) => {
                let mut out = out.as_mut();
                ffi::git_libgit2_opts(
                    ffi::git_libgit2_opt_t_GIT_OPT_GET_USER_AGENT_PRODUCT as c_int,
                    out.as_mut_ptr(),
                )
            }
            SetSslCiphers(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_SSL_CIPHERS as c_int,
                value.as_ptr(),
            ),
            EnableStrictObjectCreation(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_STRICT_OBJECT_CREATION as c_int,
                c_bool(value),
            ),
            EnableStrictSymbolicRefCreation(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_STRICT_SYMBOLIC_REF_CREATION as c_int,
                c_bool(value),
            ),
            EnableOfsDelta(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_OFS_DELTA as c_int,
                c_bool(value),
            ),
            EnableFsyncGitdir(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_FSYNC_GITDIR as c_int,
                c_bool(value),
            ),
            GetWindowsSharemode(out) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_GET_WINDOWS_SHAREMODE as c_int,
                out,
            ),
            SetWindowsSharemode(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_WINDOWS_SHAREMODE as c_int,
                value,
            ),
            EnableStrictHashVerification(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_STRICT_HASH_VERIFICATION as c_int,
                c_bool(value),
            ),
            EnableUnsavedIndexSafety(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_UNSAVED_INDEX_SAFETY as c_int,
                c_bool(value),
            ),
            GetPackMaxObjects(out) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_GET_PACK_MAX_OBJECTS as c_int,
                out,
            ),
            SetPackMaxObjects(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_PACK_MAX_OBJECTS as c_int,
                value,
            ),
            GetPackMaxObjectSize(out) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_GET_PACK_MAX_OBJECT_SIZE as c_int,
                out,
            ),
            SetPackMaxObjectSize(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_PACK_MAX_OBJECT_SIZE as c_int,
                value,
            ),
            DisablePackKeepFileChecks(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_DISABLE_PACK_KEEP_FILE_CHECKS as c_int,
                c_bool(value),
            ),
            EnableHttpExpectContinue(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_ENABLE_HTTP_EXPECT_CONTINUE as c_int,
                c_bool(value),
            ),
            SetOdbPackedPriority(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_ODB_PACKED_PRIORITY as c_int,
                value as c_int,
            ),
            SetOdbLoosePriority(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_ODB_LOOSE_PRIORITY as c_int,
                value as c_int,
            ),
            GetExtensions(out) => {
                // Drop any prior owned array before C initializes a new owner.
                *out = GitStrArray::new();
                let mut out = out.as_mut();
                ffi::git_libgit2_opts(
                    ffi::git_libgit2_opt_t_GIT_OPT_GET_EXTENSIONS as c_int,
                    out.as_mut_ptr(),
                )
            }
            SetExtensions(extensions) => {
                let pointers: Vec<*const c_char> =
                    extensions.iter().map(|value| value.as_ptr()).collect();
                ffi::git_libgit2_opts(
                    ffi::git_libgit2_opt_t_GIT_OPT_SET_EXTENSIONS as c_int,
                    pointers.as_ptr(),
                    pointers.len(),
                )
            }
            GetOwnerValidation(out) => {
                let mut raw = 0;
                let status = ffi::git_libgit2_opts(
                    ffi::git_libgit2_opt_t_GIT_OPT_GET_OWNER_VALIDATION as c_int,
                    &mut raw,
                );
                if status == 0 {
                    *out = raw != 0;
                }
                status
            }
            SetOwnerValidation(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_OWNER_VALIDATION as c_int,
                c_bool(value),
            ),
            GetHomedir(out) => {
                let mut out = out.as_mut();
                ffi::git_libgit2_opts(
                    ffi::git_libgit2_opt_t_GIT_OPT_GET_HOMEDIR as c_int,
                    out.as_mut_ptr(),
                )
            }
            SetHomedir(path) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_HOMEDIR as c_int,
                path.as_ptr(),
            ),
            GetServerConnectTimeout(out) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_GET_SERVER_CONNECT_TIMEOUT as c_int,
                out,
            ),
            SetServerConnectTimeout(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_SERVER_CONNECT_TIMEOUT as c_int,
                value as c_int,
            ),
            GetServerTimeout(out) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_GET_SERVER_TIMEOUT as c_int,
                out,
            ),
            SetServerTimeout(value) => ffi::git_libgit2_opts(
                ffi::git_libgit2_opt_t_GIT_OPT_SET_SERVER_TIMEOUT as c_int,
                value as c_int,
            ),
        }
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn scalar_getters_use_their_typed_varargs() {
        let mut window = 0;
        git_libgit2_opts(Libgit2Option::GetMwindowSize(&mut window)).unwrap();
        assert!(window > 0);

        let mut validate = false;
        git_libgit2_opts(Libgit2Option::GetOwnerValidation(&mut validate)).unwrap();
        assert!(validate);
    }
}

/// Wraps: git_libgit2_buildinfo
/// Returns static build metadata when the requested value was compiled in.
#[must_use]
pub fn git_libgit2_buildinfo(info: crate::api::common::GitBuildInfo) -> Option<&'static CStr> {
    // SAFETY: the checked key is passed by value and C returns null or static
    // immutable NUL-terminated build metadata.
    let value = unsafe { ffi::git_libgit2_buildinfo(info.into()) };
    if value.is_null() {
        None
    } else {
        // SAFETY: non-null results are compile-time strings with static life.
        Some(unsafe { CStr::from_ptr(value) })
    }
}
