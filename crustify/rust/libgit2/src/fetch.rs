//! Safe wrappers for libgit2 fetch APIs.

use crate::api::remote::GitFetchOptionsMut;
use crate::ffi;

/// Wraps: git_fetch_options_init
/// Restores caller-owned fetch options to the published defaults.
///
/// The C entry point writes into storage its caller allocates and keeps, so
/// this wrapper borrows that storage exclusively rather than producing new
/// options; [`GitFetchOptions::new`] builds the same defaults for a caller
/// that has no record yet.
///
/// `version` is a compatibility gate, not a selector.
/// `GIT_INIT_STRUCTURE_FROM_TEMPLATE` refuses anything outside
/// `1..=GIT_FETCH_OPTIONS_VERSION` with `GIT_ERROR_INVALID` and returns before
/// touching `options`, and every accepted version copies the same current
/// `GIT_FETCH_OPTIONS_INIT` template over the whole record.
///
/// That copy clears every `'data` pointer the record held — the shared
/// callback and proxy payloads, the proxy URL and the custom-header array —
/// and releases none of them, which is why it takes no ownership from the
/// caller: fetch options only ever borrow those.
///
/// [`GitFetchOptions::new`]: crate::api::remote::GitFetchOptions::new
pub fn git_fetch_options_init(
    options: &mut GitFetchOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage, and the initializer only copies a static template into it,
    // retaining no pointer to it or to any field it overwrites.
    let status = unsafe { ffi::git_fetch_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::api::errors::GitErrorClass;
    use crate::api::remote::{
        GitFetchDepth, GitFetchOptions, GitFetchOptionsRef, GitRemoteUpdateFlags,
    };
    use crate::proxy::ProxyType;
    use crate::remote::{GitFetchPrune, GitRemoteAutotagOption};
    use crate::util::errors::git_error_last;

    /// Asserts the complete `GIT_FETCH_OPTIONS_INIT` template, including the
    /// trailing fields its C initializer list leaves implicitly zeroed.
    fn assert_template_defaults(options: GitFetchOptionsRef<'_, '_>) {
        assert_eq!(options.version(), ffi::GIT_FETCH_OPTIONS_VERSION as i32);
        assert_eq!(options.update_flags(), Ok(GitRemoteUpdateFlags::FETCH_HEAD));
        assert_eq!(options.prune(), Ok(GitFetchPrune::Unspecified));
        assert_eq!(
            options.download_tags(),
            Ok(GitRemoteAutotagOption::Unspecified)
        );
        assert_eq!(options.depth(), Ok(GitFetchDepth::FULL));
        assert_eq!(options.follow_redirects(), Ok(None));
        assert_eq!(options.custom_headers().count(), 0);
        assert_eq!(
            options.callbacks().version(),
            ffi::GIT_REMOTE_CALLBACKS_VERSION
        );
        assert_eq!(
            options.proxy_options().version(),
            ffi::GIT_PROXY_OPTIONS_VERSION
        );
        assert_eq!(options.proxy_options().url(), None);
    }

    #[test]
    fn current_initializer_writes_published_fetch_defaults() {
        let mut options = GitFetchOptions::new();
        assert_eq!(
            git_fetch_options_init(&mut options.as_mut(), ffi::GIT_FETCH_OPTIONS_VERSION),
            Ok(())
        );
        assert_template_defaults(options.as_ref());
    }

    /// The template is copied over the whole record, so caller state is
    /// discarded rather than merged — including borrowed nested data, which
    /// the options never owned and which the caller therefore still holds.
    #[test]
    fn reinitialization_discards_caller_state_without_releasing_borrowed_data() {
        let url = c"http://proxy.invalid/";
        let mut options = GitFetchOptions::new();
        {
            let mut view = options.as_mut();
            view.set_depth(GitFetchDepth::new(7).expect("a representable commit count"));
            view.set_download_tags(GitRemoteAutotagOption::All);
            view.set_update_flags(GitRemoteUpdateFlags::REPORT_UNCHANGED);
            view.set_prune(GitFetchPrune::Prune);
            let mut proxy = view.proxy_options_mut();
            proxy.set_proxy_type(ProxyType::Specified);
            proxy.set_url(Some(url));
        }
        assert_eq!(options.as_ref().proxy_options().url(), Some(url));

        assert_eq!(
            git_fetch_options_init(&mut options.as_mut(), ffi::GIT_FETCH_OPTIONS_VERSION),
            Ok(())
        );
        assert_template_defaults(options.as_ref());
        // The borrowed URL is untouched: reinitialization dropped the pointer
        // to it rather than freeing the string.
        assert_eq!(url.to_bytes(), b"http://proxy.invalid/");
    }

    /// The gate accepts every version at or below the published one, and each
    /// accepted version still yields the single current template.
    #[test]
    fn every_supported_version_selects_the_same_template() {
        let mut options = GitFetchOptions::new();
        for version in 1..=ffi::GIT_FETCH_OPTIONS_VERSION {
            assert_eq!(
                git_fetch_options_init(&mut options.as_mut(), version),
                Ok(())
            );
            assert_template_defaults(options.as_ref());
        }
    }

    /// The version check runs before the copy, so a refused version leaves the
    /// caller's record exactly as it was.
    #[test]
    fn an_unsupported_version_is_refused_before_the_record_is_touched() {
        // SAFETY: libgit2 initialization is process-global and refcounted. The
        // refusals below record a thread-local error, which needs the
        // threadstate, and the shutdown at the end balances this acquisition.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let depth = GitFetchDepth::new(7).expect("a representable commit count");
        let mut options = GitFetchOptions::new();
        options.as_mut().set_depth(depth);

        for version in [0, ffi::GIT_FETCH_OPTIONS_VERSION + 1] {
            assert_eq!(
                git_fetch_options_init(&mut options.as_mut(), version),
                Err(-1)
            );
            let error = git_error_last();
            assert_eq!(error.klass, Ok(GitErrorClass::Invalid));
            let expected =
                std::ffi::CString::new(format!("invalid version {version} on git_fetch_options"))
                    .expect("a message without an interior NUL");
            assert_eq!(error.message.as_deref(), Some(expected.as_c_str()));
            assert_eq!(options.as_ref().depth(), Ok(depth));
        }

        drop(options);
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
