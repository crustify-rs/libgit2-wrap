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
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, TempDir};

    fn file_url(path: &std::path::Path) -> std::ffi::CString {
        std::ffi::CString::new(format!(
            "file://{}",
            path.to_str().expect("fixture path is UTF-8")
        ))
        .unwrap()
    }

    unsafe fn raw_head_and_refs(repository: *mut ffi::git_repository) -> (Vec<u8>, Vec<Vec<u8>>) {
        let mut head = core::ptr::null_mut();
        // SAFETY: output and repository are live.
        assert_eq!(
            unsafe { ffi::git_repository_head(&mut head, repository) },
            0
        );
        // SAFETY: the successful reference has a live target ID.
        let target = unsafe { ffi::git_reference_target(head) };
        assert!(!target.is_null());
        let id = unsafe {
            core::slice::from_raw_parts((*target).id.as_ptr(), crate::oid::RAW_DIGEST_LEN)
        }
        .to_vec();
        // SAFETY: lookup transferred this owner.
        unsafe { ffi::git_reference_free(head) };

        let mut names = ffi::git_strarray {
            strings: core::ptr::null_mut(),
            count: 0,
        };
        // SAFETY: output and repository are live.
        assert_eq!(
            unsafe { ffi::git_reference_list(&mut names, repository) },
            0
        );
        let mut refs = (0..names.count)
            .map(|index| {
                // SAFETY: successful list construction initialized every slot.
                let value = unsafe { *names.strings.add(index) };
                unsafe { core::ffi::CStr::from_ptr(value) }
                    .to_bytes()
                    .to_vec()
            })
            .collect::<Vec<_>>();
        // SAFETY: list construction initialized owned fields.
        unsafe { ffi::git_strarray_dispose(&mut names) };
        refs.sort();
        (id, refs)
    }

    fn safe_head_and_refs(
        repository: &mut crate::repository::GitRepositoryOwned,
    ) -> (Vec<u8>, Vec<Vec<u8>>) {
        let id = {
            let mut view = repository.as_mut();
            let head = crate::repository::git_repository_head(&mut view).unwrap();
            crate::refs::git_reference_target(head.as_ref())
                .unwrap()
                .raw_bytes()
                .elems()
                .collect()
        };
        let names = crate::refs::git_reference_list(&mut repository.as_mut()).unwrap();
        let strings = names.as_ref().strings().unwrap();
        let mut refs = (0..strings.len())
            .map(|index| strings.get(index).unwrap().to_bytes().to_vec())
            .collect::<Vec<_>>();
        refs.sort();
        (id, refs)
    }

    struct GitDaemon(std::process::Child);

    impl Drop for GitDaemon {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn git_daemon_url(source: &HistoryFixture) -> (GitDaemon, std::ffi::CString) {
        let probe = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let base = source.directory.path().parent().unwrap();
        let daemon = std::process::Command::new("git")
            .args([
                "daemon",
                "--reuseaddr",
                "--export-all",
                "--listen=127.0.0.1",
                &format!("--port={port}"),
                &format!("--base-path={}", base.to_str().unwrap()),
                base.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let name = source
            .directory
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        (
            GitDaemon(daemon),
            std::ffi::CString::new(format!("git://127.0.0.1:{port}/{name}")).unwrap(),
        )
    }

    #[test]
    fn io_equiv_local_transport_clone() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("clone-source");
        let raw_destination = TempDir::new("clone-raw");
        let safe_destination = TempDir::new("clone-safe");
        let url = file_url(source.directory.path());

        let mut raw_repository = core::ptr::null_mut();
        let raw_path = raw_destination.c_path();
        // A file URL deliberately takes the local transport rather than the
        // hardlink optimization, exercising fetch, pack and checkout I/O.
        assert_eq!(
            unsafe {
                ffi::git_clone(
                    &mut raw_repository,
                    url.as_ptr(),
                    raw_path.as_ptr(),
                    core::ptr::null(),
                )
            },
            0
        );
        assert!(!raw_repository.is_null());

        let safe_path = safe_destination.c_path();
        let mut safe_repository = git_clone(&url, &safe_path, None).unwrap();
        // SAFETY: raw clone transferred a live repository owner.
        let raw_observation = unsafe { raw_head_and_refs(raw_repository) };
        assert_eq!(raw_observation, safe_head_and_refs(&mut safe_repository));
        assert_eq!(
            std::fs::read(raw_destination.path().join("README.md")).unwrap(),
            std::fs::read(safe_destination.path().join("README.md")).unwrap()
        );
        assert_eq!(
            std::fs::read(raw_destination.path().join("src/alpha.c")).unwrap(),
            std::fs::read(safe_destination.path().join("src/alpha.c")).unwrap()
        );

        drop(safe_repository);
        // SAFETY: raw clone transferred this sole repository owner.
        unsafe { ffi::git_repository_free(raw_repository) };
    }

    #[test]
    fn io_equiv_git_protocol_clone_and_checkout() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("git-daemon-source");
        let (_daemon, url) = git_daemon_url(&source);
        let raw_destination = TempDir::new("git-daemon-clone-raw");
        let safe_destination = TempDir::new("git-daemon-clone-safe");

        let mut raw_repository = core::ptr::null_mut();
        let raw_path = raw_destination.c_path();
        assert_eq!(
            unsafe {
                ffi::git_clone(
                    &mut raw_repository,
                    url.as_ptr(),
                    raw_path.as_ptr(),
                    core::ptr::null(),
                )
            },
            0
        );
        let safe_path = safe_destination.c_path();
        let mut safe_repository = git_clone(&url, &safe_path, None).unwrap();
        let raw_observation = unsafe { raw_head_and_refs(raw_repository) };
        assert_eq!(raw_observation, safe_head_and_refs(&mut safe_repository));
        assert_eq!(
            std::fs::read(raw_destination.path().join("src/beta.c")).unwrap(),
            std::fs::read(safe_destination.path().join("src/beta.c")).unwrap()
        );
        drop(safe_repository);
        unsafe { ffi::git_repository_free(raw_repository) };
    }

    unsafe fn raw_clone_options(
        source: &CStr,
        destination: &CStr,
        bare: bool,
        local: ffi::git_clone_local_t,
        branch: Option<&CStr>,
    ) -> (bool, Vec<u8>, bool) {
        let mut options = unsafe { core::mem::zeroed::<ffi::git_clone_options>() };
        assert_eq!(
            unsafe { ffi::git_clone_options_init(&mut options, ffi::GIT_CLONE_OPTIONS_VERSION) },
            0
        );
        options.bare = i32::from(bare);
        options.local = local;
        options.checkout_branch = branch.map_or(core::ptr::null(), CStr::as_ptr);
        let mut repository = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_clone(
                    &mut repository,
                    source.as_ptr(),
                    destination.as_ptr(),
                    &options,
                )
            },
            0
        );
        let is_bare = unsafe { ffi::git_repository_is_bare(repository) } != 0;
        let mut head = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_head(&mut head, repository) },
            0
        );
        let head_name = unsafe { CStr::from_ptr(ffi::git_reference_name(head)) }
            .to_bytes()
            .to_vec();
        unsafe {
            ffi::git_reference_free(head);
            ffi::git_repository_free(repository);
        }
        (
            is_bare,
            head_name,
            std::path::Path::new(destination.to_str().unwrap())
                .join("README.md")
                .is_file(),
        )
    }

    fn safe_clone_options(
        source: &CStr,
        destination: &CStr,
        bare: bool,
        local: GitCloneLocal,
        branch: Option<&CStr>,
    ) -> (bool, Vec<u8>, bool) {
        let mut options = crate::api::clone::GitCloneOptions::new();
        git_clone_init_options(&mut options.as_mut(), ffi::GIT_CLONE_OPTIONS_VERSION).unwrap();
        {
            let mut view = options.as_mut();
            view.set_bare(bare);
            view.set_local(local);
            view.set_checkout_branch(branch);
        }
        let mut repository = git_clone(source, destination, Some(options.as_ref())).unwrap();
        let is_bare = crate::repository::git_repository_is_bare(repository.as_ref());
        let head_name = {
            let mut view = repository.as_mut();
            let head = crate::repository::git_repository_head(&mut view).unwrap();
            crate::refs::git_reference_name(head.as_ref())
                .to_bytes()
                .to_vec()
        };
        (
            is_bare,
            head_name,
            std::path::Path::new(destination.to_str().unwrap())
                .join("README.md")
                .is_file(),
        )
    }

    #[test]
    fn io_equiv_clone_local_policies_bare_and_checkout_branch() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("clone-options-source");
        let source_path = source.directory.c_path();
        for (label, bare, raw_local, safe_local, branch) in [
            (
                "nolinks-topic",
                false,
                ffi::git_clone_local_t_GIT_CLONE_LOCAL_NO_LINKS,
                GitCloneLocal::NoLinks,
                Some(c"topic"),
            ),
            (
                "local-bare",
                true,
                ffi::git_clone_local_t_GIT_CLONE_LOCAL,
                GitCloneLocal::Local,
                None,
            ),
        ] {
            let raw_destination = TempDir::new(&format!("clone-options-{label}-raw"));
            let safe_destination = TempDir::new(&format!("clone-options-{label}-safe"));
            let raw_path = raw_destination.c_path();
            let safe_path = safe_destination.c_path();
            let raw =
                unsafe { raw_clone_options(&source_path, &raw_path, bare, raw_local, branch) };
            let safe = safe_clone_options(&source_path, &safe_path, bare, safe_local, branch);
            assert_eq!(raw, safe);
            assert_eq!(raw.0, bare);
            assert_eq!(raw.2, !bare);
        }
    }
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
/// Restores caller-owned clone-options storage to the published defaults.
///
/// The record belongs to the caller throughout: C validates `version`, copies
/// the current `GIT_CLONE_OPTIONS_INIT` template over the storage it was
/// handed, and retains no pointer into it. This is the reinitializing
/// counterpart of [`GitCloneOptions::new`](crate::api::clone::GitCloneOptions::new),
/// which writes the same defaults into fresh storage, and the current spelling
/// of the deprecated [`git_clone_init_options`].
///
/// `version` is a compatibility gate, not a selector. C accepts only
/// `1..=GIT_CLONE_OPTIONS_VERSION`, and every accepted value copies the same
/// single template. Any other value is refused with `GIT_ERROR_INVALID`
/// before the record is touched, so a failed call leaves the caller's values
/// exactly as they were.
///
/// A successful call overwrites the whole record, so every `'data` borrow it
/// held stops being reachable through it: the checkout branch name, both
/// creation callbacks with their payloads, and the nested checkout and fetch
/// options with their own borrowed strings and callback tables. The record
/// owns none of them, so clearing them frees nothing and leaks nothing — the
/// caller's storage behind each borrow is untouched.
pub fn git_clone_options_init(
    options: &mut GitCloneOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies non-null writable
    // layout-compatible storage, and the initializer retains no pointer to
    // the options header or to any field it clears.
    let status = unsafe { ffi::git_clone_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod clone_options_init_tests {
    use std::ffi::CString;

    use super::*;
    use crate::api::clone::GitCloneOptions;
    use crate::api::remote::GitRemoteUpdateFlags;
    use crate::remote::{GitFetchPrune, GitRemoteAutotagOption};

    /// Writes a non-default value into every reachable slot, including a
    /// borrowed string the record must be able to forget without freeing.
    fn garble<'data>(options: &mut GitCloneOptionsMut<'_, 'data>, branch: &'data CStr) {
        options.set_version(0);
        options.set_bare(true);
        options.set_local(GitCloneLocal::NoLinks);
        options.set_checkout_branch(Some(branch));
        {
            let mut checkout = options.checkout_options_mut();
            checkout.set_version(0);
            checkout.set_target_directory(Some(branch));
        }
        let mut fetch = options.fetch_options_mut();
        fetch.set_version(0);
        fetch.set_prune(GitFetchPrune::Prune);
        fetch.set_download_tags(GitRemoteAutotagOption::All);
        fetch.set_update_flags(GitRemoteUpdateFlags::REPORT_UNCHANGED);
        fetch.callbacks_mut().set_version(0);
        fetch.proxy_options_mut().set_version(0);
    }

    /// Asserts the whole record equals `GIT_CLONE_OPTIONS_INIT`, including the
    /// nested `GIT_CHECKOUT_OPTIONS_INIT` and `GIT_FETCH_OPTIONS_INIT`.
    fn assert_published_defaults(options: GitCloneOptionsRef<'_, '_>) {
        assert_eq!(options.version(), ffi::GIT_CLONE_OPTIONS_VERSION);
        assert!(!options.bare());
        assert_eq!(options.local(), Ok(GitCloneLocal::Auto));
        assert_eq!(options.checkout_branch(), None);
        assert!(!options.has_repository_callback());
        assert!(!options.has_repository_callback_payload());
        assert!(!options.has_remote_callback());
        assert!(!options.has_remote_callback_payload());

        let checkout = options.checkout_options();
        assert_eq!(checkout.version(), ffi::GIT_CHECKOUT_OPTIONS_VERSION);
        assert_eq!(checkout.target_directory(), None);

        let fetch = options.fetch_options();
        assert_eq!(
            fetch.version(),
            ffi::GIT_FETCH_OPTIONS_VERSION as core::ffi::c_int
        );
        assert_eq!(fetch.prune(), Ok(GitFetchPrune::Unspecified));
        assert_eq!(
            fetch.download_tags(),
            Ok(GitRemoteAutotagOption::Unspecified)
        );
        assert_eq!(fetch.update_flags(), Ok(GitRemoteUpdateFlags::FETCH_HEAD));
        assert_eq!(
            fetch.callbacks().version(),
            ffi::GIT_REMOTE_CALLBACKS_VERSION
        );
        assert_eq!(
            fetch.proxy_options().version(),
            ffi::GIT_PROXY_OPTIONS_VERSION
        );
    }

    #[test]
    fn the_initializer_reinitializes_the_callers_own_record() {
        let _initialization = crate::libgit2::git_libgit2_init().unwrap();
        let branch = CString::new("refs/heads/scratch").expect("a branch name without NUL");
        let mut options = GitCloneOptions::new();
        garble(&mut options.as_mut(), &branch);

        git_clone_options_init(&mut options.as_mut(), ffi::GIT_CLONE_OPTIONS_VERSION)
            .expect("the published version reinitializes the record in place");

        assert_published_defaults(options.as_ref());
        // The borrowed branch name was cleared, never freed: the caller's own
        // string still owns its buffer and reads back unchanged.
        assert_eq!(branch.as_c_str(), c"refs/heads/scratch");
    }

    #[test]
    fn the_rust_constructor_agrees_with_the_c_template() {
        let _initialization = crate::libgit2::git_libgit2_init().unwrap();
        let mut from_c = GitCloneOptions::new();
        git_clone_options_init(&mut from_c.as_mut(), ffi::GIT_CLONE_OPTIONS_VERSION)
            .expect("the published version initializes");

        // `GitCloneOptions::new` hand-writes what the C template contains, so
        // the two must be indistinguishable through every accessor.
        assert_published_defaults(from_c.as_ref());
        let from_rust = GitCloneOptions::new();
        assert_published_defaults(from_rust.as_ref());
    }

    #[test]
    fn an_unsupported_version_is_refused_without_writing() {
        let _initialization = crate::libgit2::git_libgit2_init().unwrap();
        let branch = CString::new("refs/heads/scratch").expect("a branch name without NUL");
        let mut options = GitCloneOptions::new();
        garble(&mut options.as_mut(), &branch);

        for version in [0, ffi::GIT_CLONE_OPTIONS_VERSION + 1] {
            assert_eq!(
                git_clone_options_init(&mut options.as_mut(), version),
                Err(ffi::git_error_code_GIT_ERROR),
                "version {version} is outside the accepted compatibility range"
            );
        }

        // Refusal happens before the template is copied, so the caller keeps
        // every value it installed — including the borrowed branch name.
        let view = options.as_ref();
        assert_eq!(view.version(), 0);
        assert!(view.bare());
        assert_eq!(view.local(), Ok(GitCloneLocal::NoLinks));
        assert_eq!(view.checkout_branch(), Some(branch.as_c_str()));
        assert_eq!(view.checkout_options().version(), 0);
        assert_eq!(view.fetch_options().version(), 0);
    }

    #[test]
    fn the_initializer_takes_the_callers_storage_and_never_allocates_it() {
        // C's contract is "initialize the record I hand you". A wrapper that
        // returned fresh storage instead would make reinitialization of an
        // existing record unreachable from safe Rust.
        let wrapper: fn(&mut GitCloneOptionsMut<'_, '_>, core::ffi::c_uint) -> Result<(), i32> =
            git_clone_options_init;
        let _ = wrapper;
    }
}
