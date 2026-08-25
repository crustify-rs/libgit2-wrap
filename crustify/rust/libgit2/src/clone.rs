//! Safe wrappers for libgit2 clone APIs.

use core::ffi::CStr;

use crate::api::clone::{GitCloneOptionsMut, GitCloneOptionsRef};
use crate::ffi;
use crate::repository::GitRepositoryOwned;

/// Wraps: git_clone_local_t
/// Controls whether cloning from a local source bypasses the Git transport.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum GitCloneLocal {
    /// Auto-detect local paths, but use the Git transport for `file://` URLs.
    #[default]
    Auto = ffi::git_clone_local_t_GIT_CLONE_LOCAL_AUTO,
    /// Always bypass the Git transport for local sources.
    Local = ffi::git_clone_local_t_GIT_CLONE_LOCAL,
    /// Never bypass the Git transport.
    NoLocal = ffi::git_clone_local_t_GIT_CLONE_NO_LOCAL,
    /// Bypass the Git transport without creating hardlinks.
    NoLinks = ffi::git_clone_local_t_GIT_CLONE_LOCAL_NO_LINKS,
}

impl From<GitCloneLocal> for ffi::git_clone_local_t {
    fn from(value: GitCloneLocal) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_clone_local_t> for GitCloneLocal {
    type Error = ffi::git_clone_local_t;

    fn try_from(value: ffi::git_clone_local_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_clone_local_t_GIT_CLONE_LOCAL_AUTO => Ok(Self::Auto),
            ffi::git_clone_local_t_GIT_CLONE_LOCAL => Ok(Self::Local),
            ffi::git_clone_local_t_GIT_CLONE_NO_LOCAL => Ok(Self::NoLocal),
            ffi::git_clone_local_t_GIT_CLONE_LOCAL_NO_LINKS => Ok(Self::NoLinks),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn clone_local_has_the_c_layout_and_values() {
        assert_eq!(
            size_of::<GitCloneLocal>(),
            size_of::<ffi::git_clone_local_t>()
        );
        assert_eq!(
            align_of::<GitCloneLocal>(),
            align_of::<ffi::git_clone_local_t>()
        );
        assert_eq!(ffi::git_clone_local_t::from(GitCloneLocal::Auto), 0);
        assert_eq!(ffi::git_clone_local_t::from(GitCloneLocal::NoLinks), 3);
    }

    #[test]
    fn clone_local_validates_raw_values() {
        assert_eq!(GitCloneLocal::try_from(2), Ok(GitCloneLocal::NoLocal));
        assert_eq!(GitCloneLocal::try_from(4), Err(4));
    }
}

fn clone_result(
    status: i32,
    repository: Option<GitRepositoryOwned>,
) -> Result<GitRepositoryOwned, i32> {
    if status == 0 {
        repository.ok_or(ffi::git_error_code_GIT_ERROR)
    } else {
        drop(repository);
        Err(status)
    }
}

/// Wraps: git_clone
/// Clones `url` into `local_path` and returns the new owned repository.
pub fn git_clone(
    url: &CStr,
    local_path: &CStr,
    options: Option<GitCloneOptionsRef<'_, '_>>,
) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the output slot is writable, both strings are live and
    // NUL-terminated, and the options header plus every nested borrow remain
    // live for this synchronous clone and its callbacks.
    let status = unsafe {
        ffi::git_clone(
            core::ptr::addr_of_mut!(output),
            url.as_ptr(),
            local_path.as_ptr(),
            options,
        )
    };
    // SAFETY: the C contract leaves `output` null on failure or transfers one
    // complete repository allocation on success.
    let repository = unsafe { GitRepositoryOwned::from_raw(output) };
    clone_result(status, repository)
}

/// Wraps: git_clone_init_options
/// Initializes deprecated clone-options storage for `version`.
pub fn git_clone_init_options(
    options: &mut GitCloneOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage, and the initializer retains no pointer to the options header.
    let status = unsafe { ffi::git_clone_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_clone_symbol_tests {
    use super::*;

    #[test]
    fn deprecated_initializer_writes_current_clone_defaults() {
        let mut options = crate::api::clone::GitCloneOptions::new();
        git_clone_init_options(&mut options.as_mut(), ffi::GIT_CLONE_OPTIONS_VERSION).unwrap();
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_CLONE_OPTIONS_VERSION);
        assert!(!view.bare());
        assert_eq!(view.local(), Ok(GitCloneLocal::Auto));
    }

    #[test]
    fn clone_result_preserves_errors_and_rejects_missing_success_output() {
        assert!(matches!(clone_result(-123, None), Err(-123)));
        assert_eq!(
            clone_result(0, None).unwrap_err(),
            ffi::git_error_code_GIT_ERROR
        );
    }

    #[test]
    fn clone_surface_contains_no_raw_pointer_obligations() {
        let wrapper: fn(
            &CStr,
            &CStr,
            Option<GitCloneOptionsRef<'static, 'static>>,
        ) -> Result<GitRepositoryOwned, i32> = git_clone;
        let _ = wrapper;
    }
}
