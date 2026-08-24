//! Safe wrappers for libgit2 ignore APIs.

use core::ffi::CStr;

use crate::ffi;
use crate::repository::GitRepositoryRef;

/// Wraps: git_ignore_add_rule
/// Copies one or more newline-delimited rules into repository-local state.
pub fn git_ignore_add_rule(repository: GitRepositoryRef<'_>, rules: &CStr) -> Result<(), i32> {
    // SAFETY: both arguments are live for the call; libgit2 parses and copies
    // the rule text rather than retaining its pointer.
    let status =
        unsafe { ffi::git_ignore_add_rule(repository.as_ptr().cast_mut(), rules.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_ignore_clear_internal_rules
/// Restores the default in-memory ignore rules.
pub fn git_ignore_clear_internal_rules(repository: GitRepositoryRef<'_>) -> Result<(), i32> {
    // SAFETY: `repository` is live for the call and libgit2 owns the internal
    // ignore state it mutates.
    let status = unsafe { ffi::git_ignore_clear_internal_rules(repository.as_ptr().cast_mut()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_ignore_path_is_ignored
/// Tests a repository-relative, NUL-terminated path against ignore rules.
pub fn git_ignore_path_is_ignored(
    repository: GitRepositoryRef<'_>,
    pathname: &CStr,
) -> Result<bool, i32> {
    let mut ignored = 0;
    // SAFETY: `ignored` is writable, and both borrowed inputs remain live for
    // this non-retaining lookup.
    let status = unsafe {
        ffi::git_ignore_path_is_ignored(
            &mut ignored,
            repository.as_ptr().cast_mut(),
            pathname.as_ptr(),
        )
    };
    if status == 0 {
        Ok(ignored != 0)
    } else {
        Err(status)
    }
}
