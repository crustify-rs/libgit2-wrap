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

#[cfg(test)]
mod io_equiv {
    use super::*;
    use crate::io_equiv_support::Libgit2Init;

    #[test]
    fn io_equiv_git_ignore_path_is_ignored() {
        let _init = Libgit2Init::acquire();
        let root = std::env::temp_dir().join(format!(
            "crustify-io-equiv-ignore-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let raw_path = root.join("raw");
        let safe_path = root.join("safe");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let raw_path = std::ffi::CString::new(raw_path.to_str().unwrap()).unwrap();
        let safe_path = std::ffi::CString::new(safe_path.to_str().unwrap()).unwrap();

        let mut raw_repository = core::ptr::null_mut();
        // SAFETY: the output slot is writable and the path is a live C string.
        let raw_init_status =
            unsafe { ffi::git_repository_init(&mut raw_repository, raw_path.as_ptr(), 0) };
        assert_eq!(raw_init_status, 0);
        assert!(!raw_repository.is_null());
        let safe_repository = crate::repository::git_repository_init(&safe_path, false).unwrap();

        let rules = c"target/\n*.generated\n";
        // Use the same raw setup operation for both fixtures so this test
        // isolates the query wrapper rather than the rule-adder wrapper.
        // SAFETY: both repositories and the rule string remain live, and C
        // copies the rules into repository-local state.
        let raw_rule_status = unsafe { ffi::git_ignore_add_rule(raw_repository, rules.as_ptr()) };
        // SAFETY: as above, for the independent safe-leg repository fixture.
        let safe_rule_status = unsafe {
            ffi::git_ignore_add_rule(safe_repository.as_ref().as_ptr().cast_mut(), rules.as_ptr())
        };
        assert_eq!(raw_rule_status, 0);
        assert_eq!(safe_rule_status, 0);

        for path in [c"target/output.o", c"report.generated", c"src/main.c"] {
            let mut raw_ignored = -1;
            // SAFETY: the output is writable and both raw inputs remain live.
            let raw_status = unsafe {
                ffi::git_ignore_path_is_ignored(&mut raw_ignored, raw_repository, path.as_ptr())
            };
            let safe_status = git_ignore_path_is_ignored(safe_repository.as_ref(), path);
            assert_eq!(
                safe_status,
                if raw_status == 0 {
                    Ok(raw_ignored != 0)
                } else {
                    Err(raw_status)
                }
            );
        }

        drop(safe_repository);
        // SAFETY: releases the raw repository owner exactly once.
        unsafe { ffi::git_repository_free(raw_repository) };
        std::fs::remove_dir_all(root).unwrap();
    }
}
