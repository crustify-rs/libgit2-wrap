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
    // C does not always write this slot. Its early refusals — an unsupported
    // options version, a non-empty destination (`GIT_EEXISTS`), a failing
    // repository-creation callback — all return before the single assignment
    // at the end of `clone_repo`, so the caller's initial value is what
    // survives them. The null below is therefore load-bearing, not defensive.
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
    // SAFETY: `output` is either the null it was initialized to, the null C
    // stores after releasing the partial clone, or one complete repository
    // allocation transferred on success.
    let repository = unsafe { GitRepositoryOwned::from_raw(output) };
    clone_result(status, repository)
}

/// Wraps: git_clone_init_options
/// Initializes deprecated clone-options storage for `version`.
///
/// C accepts only a `version` in `1..=GIT_CLONE_OPTIONS_VERSION`. Any other
/// value is refused before the template is copied, so a failed call leaves the
/// storage exactly as the caller left it rather than resetting it.
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
    use std::ffi::CString;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::api::clone::GitCloneOptions;
    use crate::api::repository::GitRepositoryInitFlags;
    use crate::repository::{
        GitRepositoryOwned, git_repository_init_ext, git_repository_init_init_options,
        git_repository_is_bare, git_repository_path,
    };

    /// A balanced hold on the process-global libgit2 initialization, which
    /// every allocation below the FFI seam requires.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and refcounted;
            // `Drop` below balances this successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard; every owner taken under it is dropped first.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A temporary directory tree removed when the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-clone-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }

        /// Writes the minimum on-disk bare repository libgit2 will open: an
        /// object store, a ref namespace, a symbolic `HEAD` and a config. It
        /// holds no objects, which is exactly the empty-clone case.
        fn into_empty_bare_repository(self) -> Self {
            std::fs::create_dir_all(self.0.join("objects/info")).expect("an object directory");
            std::fs::create_dir_all(self.0.join("objects/pack")).expect("a pack directory");
            std::fs::create_dir_all(self.0.join("refs/heads")).expect("a branch namespace");
            std::fs::create_dir_all(self.0.join("refs/tags")).expect("a tag namespace");
            std::fs::write(self.0.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                self.0.join("config"),
                b"[core]\n\trepositoryformatversion = 0\n\tbare = true\n",
            )
            .expect("a config file");
            self
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn c_path(&self) -> CString {
            CString::new(self.0.to_str().expect("a UTF-8 temporary path"))
                .expect("a path without interior NUL")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn assert_bare_repository_at(repository: &GitRepositoryOwned, expected: &Path) {
        assert!(git_repository_is_bare(repository.as_ref()));
        let path = git_repository_path(repository.as_ref()).expect("an on-disk repository path");
        let path = path.to_str().expect("a UTF-8 repository path");
        assert_eq!(
            Path::new(path.trim_end_matches('/')),
            expected,
            "the clone landed at the requested destination"
        );
    }

    #[test]
    fn cloning_a_local_bare_source_transfers_one_owned_repository() {
        let _init = Libgit2Init::acquire();
        let source = Scratch::new("local-source").into_empty_bare_repository();
        let destination = Scratch::new("local-destination");

        let mut options = GitCloneOptions::new();
        options.as_mut().set_bare(true);
        let repository = git_clone(
            &source.c_path(),
            &destination.c_path(),
            Some(options.as_ref()),
        )
        .expect("an empty local source clones");

        assert_bare_repository_at(&repository, destination.path());
    }

    #[test]
    fn a_refused_clone_reports_the_error_without_a_repository() {
        let _init = Libgit2Init::acquire();
        let source = Scratch::new("eexists-source").into_empty_bare_repository();
        let destination = Scratch::new("eexists-destination");
        std::fs::create_dir_all(destination.path()).expect("a destination directory");
        std::fs::write(destination.path().join("occupant"), b"in the way")
            .expect("a file that makes the destination non-empty");

        // C refuses this before it assigns the output slot, so the wrapper
        // observes only the null it installed itself. A wrapper that trusted
        // C to null the slot would hand back uninitialized storage here.
        let refusal = git_clone(&source.c_path(), &destination.c_path(), None);
        assert_eq!(
            refusal.err(),
            Some(ffi::git_error_code_GIT_EEXISTS),
            "a non-empty destination is refused and yields no repository"
        );
    }

    #[test]
    fn clone_creates_its_repository_through_a_typed_callback() {
        let _init = Libgit2Init::acquire();
        let source = Scratch::new("callback-source").into_empty_bare_repository();
        let destination = Scratch::new("callback-destination");

        let mut requested: Vec<(CString, bool)> = Vec::new();
        let mut create = |path: &CStr, bare: bool| {
            requested.push((path.to_owned(), bare));
            let mut init = git_repository_init_init_options(1)?;
            init.as_mut()
                .set_flags(GitRepositoryInitFlags::MKPATH | GitRepositoryInitFlags::BARE);
            git_repository_init_ext(path, &mut init.as_mut())
        };

        let repository = {
            let mut options = GitCloneOptions::new();
            {
                let mut view = options.as_mut();
                view.set_bare(true);
                view.set_repository_callback(&mut create);
            }
            assert!(options.as_ref().has_repository_callback());
            git_clone(
                &source.c_path(),
                &destination.c_path(),
                Some(options.as_ref()),
            )
            .expect("the callback-created repository clones")
        };

        assert_bare_repository_at(&repository, destination.path());
        assert_eq!(
            requested,
            vec![(destination.c_path(), true)],
            "clone asked the callback exactly once, forwarding the bare flag"
        );
    }

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
    fn deprecated_initializer_rejects_an_unsupported_version() {
        let mut options = crate::api::clone::GitCloneOptions::new();
        options.as_mut().set_bare(true);
        options.as_mut().set_local(GitCloneLocal::NoLinks);

        for version in [0, ffi::GIT_CLONE_OPTIONS_VERSION + 1] {
            assert_eq!(
                git_clone_init_options(&mut options.as_mut(), version),
                Err(ffi::git_error_code_GIT_ERROR)
            );
        }

        // The refusal happens before the template is copied, so the caller's
        // own values survive it.
        let view = options.as_ref();
        assert!(view.bare());
        assert_eq!(view.local(), Ok(GitCloneLocal::NoLinks));
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

/// Wraps: git_clone_options_init
/// Creates clone options initialized for `version`.
pub fn git_clone_options_init<'data>(
    version: core::ffi::c_uint,
) -> Result<ffibox::CVal<crate::api::clone::GitCloneOptions<'data>>, i32> {
    let mut options = crate::api::clone::GitCloneOptions::<'data>::new();
    // SAFETY: the inline options storage is exclusively writable and the C
    // initializer retains no pointer to it or to any of its cleared fields.
    let status = unsafe { ffi::git_clone_options_init(options.as_mut().as_mut_ptr(), version) };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod clone_options_init_tests {
    use super::*;

    #[test]
    fn current_initializer_writes_published_clone_defaults() {
        let _initialization = crate::libgit2::git_libgit2_init().unwrap();
        let options = git_clone_options_init(ffi::GIT_CLONE_OPTIONS_VERSION)
            .expect("the published clone-options version initializes");
        let options = options.as_ref();
        assert_eq!(options.version(), ffi::GIT_CLONE_OPTIONS_VERSION);
        assert_eq!(options.local(), Ok(GitCloneLocal::Auto));
        assert!(!options.bare());
        assert_eq!(
            options.checkout_options().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
        assert_eq!(
            options.fetch_options().version(),
            ffi::GIT_FETCH_OPTIONS_VERSION as core::ffi::c_int
        );
    }

    #[test]
    fn initializer_rejects_unknown_versions() {
        let _initialization = crate::libgit2::git_libgit2_init().unwrap();
        assert_eq!(
            git_clone_options_init::<'static>(0).err(),
            Some(ffi::git_error_code_GIT_ERROR)
        );
        assert_eq!(
            git_clone_options_init::<'static>(ffi::GIT_CLONE_OPTIONS_VERSION + 1).err(),
            Some(ffi::git_error_code_GIT_ERROR)
        );
    }
}
