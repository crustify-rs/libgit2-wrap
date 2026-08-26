//! Safe wrappers for libgit2 push APIs.

use crate::api::remote::GitPushOptionsMut;
use crate::ffi;

/// Wraps: git_push_options_init
/// Restores caller-owned push options to the published defaults.
///
/// The C entry point writes into storage its caller allocates and keeps, so
/// this wrapper borrows that storage exclusively rather than producing new
/// options; [`GitPushOptions::new`] builds the same defaults for a caller that
/// has no record yet.
///
/// `version` is a compatibility gate, not a selector.
/// `GIT_INIT_STRUCTURE_FROM_TEMPLATE` refuses anything outside
/// `1..=GIT_PUSH_OPTIONS_VERSION` with `GIT_ERROR_INVALID` and returns before
/// touching `options`, and every accepted version copies the same current
/// `GIT_PUSH_OPTIONS_INIT` template over the whole record.
///
/// That copy clears every `'data` pointer the record held — the shared
/// callback and proxy payloads, the proxy URL, the custom-header array and the
/// remote push options — and releases none of them, which is why it takes no
/// ownership from the caller: push options only ever borrow those.
///
/// [`GitPushOptions::new`]: crate::api::remote::GitPushOptions::new
pub fn git_push_options_init(
    options: &mut GitPushOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage, and the initializer only copies a static template into it,
    // retaining no pointer to it or to any field it overwrites.
    let status = unsafe { ffi::git_push_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

ffibox::define_ctype!(
    /// Wraps: git_push
    /// Opaque state for a push operation owned by its [`GitRemote`](crate::remote::GitRemote).
    ///
    /// `git_remote_upload` builds it into `remote->push` with `git_push_new`
    /// and the remote releases it with `git_push_free`, on reconnect and again
    /// in `git_remote_free`. Both operations are internal to
    /// `src/libgit2/push.h`; the public headers only forward-declare the type,
    /// and `git2/sys/transport.h` hands it to a transport's `push` callback as
    /// a borrowed handle. The wrapper therefore carries no ownership: it has
    /// borrowed handles and deliberately no `CDropped` or owning alias.
    GitPush,
    GitPushRef,
    GitPushMut,
    ffi::git_push
);

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::CCell;

    use super::*;
    use crate::api::errors::GitErrorClass;
    use crate::api::remote::{GitPushOptions, GitPushOptionsRef};
    use crate::proxy::ProxyType;
    use crate::strarray::GitStrArrayRef;
    use crate::util::errors::git_error_last;

    /// Asserts the complete `GIT_PUSH_OPTIONS_INIT` template, including the
    /// trailing fields its C initializer list leaves implicitly zeroed.
    fn assert_template_defaults(options: GitPushOptionsRef<'_, '_>) {
        assert_eq!(options.version(), ffi::GIT_PUSH_OPTIONS_VERSION);
        assert_eq!(options.packbuilder_parallelism(), 1);
        assert_eq!(options.follow_redirects(), Ok(None));
        assert_eq!(options.custom_headers().count(), 0);
        assert_eq!(options.remote_push_options().count(), 0);
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
    fn opaque_representation_and_handles_match_the_c_seam() {
        fn assert_cell<T: CCell>() {}

        assert_cell::<GitPush>();
        // No public header defines `git_push`, so bindgen emits an opaque
        // placeholder. The equalities below are only meaningful next to that
        // fact, and a body appearing here means the wrapper needs revisiting.
        assert_eq!(size_of::<ffi::git_push>(), 0);
        assert_eq!(size_of::<GitPush>(), size_of::<ffi::git_push>());
        assert_eq!(align_of::<GitPush>(), align_of::<ffi::git_push>());
        assert_eq!(
            size_of::<GitPushRef<'_>>(),
            size_of::<*const ffi::git_push>()
        );
        assert_eq!(size_of::<GitPushMut<'_>>(), size_of::<*mut ffi::git_push>());
    }

    /// The handles are pointer carriers: nothing here reads through `raw`,
    /// which is what lets an opaque, zero-sized `ffi::git_push` stand in for a
    /// push a caller cannot construct. `Box` of a zero-sized value yields the
    /// aligned, non-null, uniquely-owned address Rust guarantees for one.
    #[test]
    fn borrowed_handles_preserve_the_push_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_push>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_push>();

        {
            // SAFETY: `raw` is the live, aligned, non-null address of the
            // value `storage` owns, and the shared handle, which never
            // dereferences it, stays inside this scope.
            let shared = unsafe { GitPushRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the value remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitPushMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_push>>()) });
    }

    #[test]
    fn current_initializer_writes_published_push_defaults() {
        let mut options = GitPushOptions::new();
        assert_eq!(
            git_push_options_init(&mut options.as_mut(), ffi::GIT_PUSH_OPTIONS_VERSION),
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
        let option = c"--force";
        let mut entries = [option.as_ptr().cast_mut()];
        let mut header = ffi::git_strarray {
            strings: entries.as_mut_ptr(),
            count: entries.len(),
        };
        // SAFETY: `header` is an initialized string-array header over a live
        // pointer run whose single entry is a live NUL-terminated string, and
        // both outlive every handle borrowed from them below.
        let remote_push_options = unsafe { GitStrArrayRef::from_ptr(&raw mut header) }
            .expect("the address of a stack value is non-null");

        let mut options = GitPushOptions::new();
        {
            let mut view = options.as_mut();
            view.set_packbuilder_parallelism(4);
            view.set_remote_push_options(remote_push_options);
            let mut proxy = view.proxy_options_mut();
            proxy.set_proxy_type(ProxyType::Specified);
            proxy.set_url(Some(url));
        }
        assert_eq!(options.as_ref().remote_push_options().count(), 1);
        assert_eq!(options.as_ref().proxy_options().url(), Some(url));

        assert_eq!(
            git_push_options_init(&mut options.as_mut(), ffi::GIT_PUSH_OPTIONS_VERSION),
            Ok(())
        );
        assert_template_defaults(options.as_ref());
        // Both borrowed inputs are untouched: reinitialization dropped the
        // pointers to them rather than freeing anything.
        assert_eq!(url.to_bytes(), b"http://proxy.invalid/");
        assert_eq!(header.count, 1);
        assert_eq!(option.to_bytes(), b"--force");
    }

    /// The gate accepts every version at or below the published one, and each
    /// accepted version still yields the single current template.
    #[test]
    fn every_supported_version_selects_the_same_template() {
        let mut options = GitPushOptions::new();
        for version in 1..=ffi::GIT_PUSH_OPTIONS_VERSION {
            assert_eq!(
                git_push_options_init(&mut options.as_mut(), version),
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

        let mut options = GitPushOptions::new();
        options.as_mut().set_packbuilder_parallelism(4);

        for version in [0, ffi::GIT_PUSH_OPTIONS_VERSION + 1] {
            assert_eq!(
                git_push_options_init(&mut options.as_mut(), version),
                Err(-1)
            );
            let error = git_error_last();
            assert_eq!(error.klass, Ok(GitErrorClass::Invalid));
            let expected =
                std::ffi::CString::new(format!("invalid version {version} on git_push_options"))
                    .expect("a message without an interior NUL");
            assert_eq!(error.message.as_deref(), Some(expected.as_c_str()));
            assert_eq!(options.as_ref().packbuilder_parallelism(), 4);
        }

        drop(options);
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
