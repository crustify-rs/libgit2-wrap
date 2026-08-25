//! Safe wrappers for libgit2 fetch APIs.

use crate::api::remote::GitFetchOptions;
use crate::ffi;

/// Wraps: git_fetch_options_init
/// Creates fetch options initialized for `version`.
pub fn git_fetch_options_init<'data>(
    version: core::ffi::c_uint,
) -> Result<ffibox::CVal<GitFetchOptions<'data>>, i32> {
    let mut options = GitFetchOptions::<'data>::new();
    // SAFETY: the inline options storage is exclusively writable and the C
    // initializer retains no pointer to it or to any of its cleared fields.
    let status = unsafe { ffi::git_fetch_options_init(options.as_mut().as_mut_ptr(), version) };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::remote::GitRemoteUpdateFlags;

    #[test]
    fn current_initializer_writes_published_fetch_defaults() {
        let options = git_fetch_options_init(ffi::GIT_FETCH_OPTIONS_VERSION)
            .expect("the published fetch-options version initializes");
        let options = options.as_ref();
        assert_eq!(options.version(), ffi::GIT_FETCH_OPTIONS_VERSION as i32);
        assert_eq!(options.update_flags(), Ok(GitRemoteUpdateFlags::FETCH_HEAD));
        assert_eq!(
            options.callbacks().version(),
            ffi::GIT_REMOTE_CALLBACKS_VERSION
        );
        assert_eq!(
            options.proxy_options().version(),
            ffi::GIT_PROXY_OPTIONS_VERSION
        );
    }
}
