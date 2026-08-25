//! Safe wrappers for libgit2 apply APIs.

use crate::api::apply::{GitApplyOptionsMut, GitApplyOptionsRef};
use crate::diff::DiffMut;
use crate::ffi;
use crate::index::GitIndexOwned;
use crate::repository::GitRepositoryMut;
use crate::tree::GitTreeRef;

/// Wraps: git_apply_location_t
/// Selects where libgit2 applies a patch.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ApplyLocation {
    /// Apply to the working directory only.
    Workdir = ffi::git_apply_location_t_GIT_APPLY_LOCATION_WORKDIR,
    /// Apply to the index only.
    Index = ffi::git_apply_location_t_GIT_APPLY_LOCATION_INDEX,
    /// Apply to both the working directory and the index.
    Both = ffi::git_apply_location_t_GIT_APPLY_LOCATION_BOTH,
}

/// An integer that is not a valid [`ApplyLocation`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidApplyLocation(ffi::git_apply_location_t);

impl InvalidApplyLocation {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_apply_location_t {
        self.0
    }
}

impl From<ApplyLocation> for ffi::git_apply_location_t {
    fn from(location: ApplyLocation) -> Self {
        location as Self
    }
}

impl TryFrom<ffi::git_apply_location_t> for ApplyLocation {
    type Error = InvalidApplyLocation;

    fn try_from(location: ffi::git_apply_location_t) -> Result<Self, Self::Error> {
        match location {
            ffi::git_apply_location_t_GIT_APPLY_LOCATION_WORKDIR => Ok(Self::Workdir),
            ffi::git_apply_location_t_GIT_APPLY_LOCATION_INDEX => Ok(Self::Index),
            ffi::git_apply_location_t_GIT_APPLY_LOCATION_BOTH => Ok(Self::Both),
            value => Err(InvalidApplyLocation(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn apply_locations_round_trip_through_the_c_type() {
        for location in [
            ApplyLocation::Workdir,
            ApplyLocation::Index,
            ApplyLocation::Both,
        ] {
            let raw = ffi::git_apply_location_t::from(location);
            assert_eq!(ApplyLocation::try_from(raw), Ok(location));
        }
    }

    #[test]
    fn invalid_apply_location_is_rejected_without_constructing_an_enum() {
        let invalid = ffi::git_apply_location_t_GIT_APPLY_LOCATION_BOTH + 1;
        let error = ApplyLocation::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn apply_location_matches_the_c_abi_scalar() {
        assert_eq!(
            size_of::<ApplyLocation>(),
            size_of::<ffi::git_apply_location_t>()
        );
        assert_eq!(
            align_of::<ApplyLocation>(),
            align_of::<ffi::git_apply_location_t>()
        );
    }
}

/// Wraps: git_apply
/// Applies `diff` to the selected repository location.
///
/// The diff is borrowed exclusively. Applying a delta generates its patch
/// through `git_patch_from_diff`, and patch generation writes into the diff:
/// `git_diff_file_content__init_from_diff` runs the attribute lookup against
/// `diff->attrsession`, whose first use sets `init_setup` and caches the
/// preloaded attribute files, and loading a file's content stamps binary
/// flags, sizes and object IDs into `delta->old_file` and `delta->new_file`
/// inside `diff->deltas`. That is the same storage
/// [`git_patch_from_diff`](crate::patch::git_patch_from_diff) and
/// [`git_diff_print`](crate::diff_print::git_diff_print) already take
/// exclusively.
pub fn git_apply(
    repository: &mut GitRepositoryMut<'_>,
    diff: &mut DiffMut<'_>,
    location: ApplyLocation,
    options: Option<GitApplyOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository and the diff are both exclusively borrowed for
    // the writes the apply performs through them, and the optional options
    // remain live and read-only for the synchronous call.
    let status = unsafe {
        ffi::git_apply(
            repository.as_mut_ptr(),
            diff.as_mut_ptr(),
            location.into(),
            options,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_apply_options_init
/// Fills `options` with the published apply defaults.
///
/// `version` is a compatibility bound, not the value written. C checks it
/// first and returns `Err(-1)` without touching the storage when it is zero or
/// above the compiled maximum; otherwise the whole template is copied over the
/// storage, so the version left behind is `GIT_APPLY_OPTIONS_VERSION`. Every
/// other `*_options_init` and `*_init_options` entry point in libgit2 forwards
/// to the same `GIT_INIT_STRUCTURE_FROM_TEMPLATE` macro and behaves this way.
pub fn git_apply_options_init(
    options: &mut GitApplyOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle exposes writable layout-compatible storage;
    // initialization retains no pointer into it.
    let status = unsafe { ffi::git_apply_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_apply_to_tree
/// Applies `diff` to `preimage` and returns the resulting in-memory index.
///
/// The diff is exclusive for the same reason it is in [`git_apply`]: this
/// entry point reaches `apply_deltas` and therefore `git_patch_from_diff`.
/// The tree stays shared, because `git_reader_for_tree` only records it in a
/// reader that is freed before the call returns and reads it through
/// `git_tree_entry_bypath`.
pub fn git_apply_to_tree(
    repository: &mut GitRepositoryMut<'_>,
    preimage: GitTreeRef<'_>,
    diff: &mut DiffMut<'_>,
    options: Option<GitApplyOptionsRef<'_, '_>>,
) -> Result<GitIndexOwned, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: `out` is writable, the repository and the diff are exclusively
    // borrowed for the writes the apply performs, and the tree and optional
    // options remain live and read-only for the call.
    let status = unsafe {
        ffi::git_apply_to_tree(
            &mut out,
            repository.as_mut_ptr(),
            preimage.as_ptr().cast_mut(),
            diff.as_mut_ptr(),
            options,
        )
    };
    if status == 0 {
        // SAFETY: success transfers one complete non-null index allocation.
        unsafe { GitIndexOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod scheduled_wrapper_tests {
    use super::*;
    use crate::api::apply::GitApplyOptions;
    use crate::diff_generate::git_diff_tree_to_tree;
    use crate::repository::git_repository_open_bare;

    #[test]
    fn initializer_writes_the_published_apply_version() {
        let mut options = GitApplyOptions::new();
        git_apply_options_init(&mut options.as_mut(), ffi::GIT_APPLY_OPTIONS_VERSION).unwrap();
        assert_eq!(options.as_ref().version(), ffi::GIT_APPLY_OPTIONS_VERSION);
    }

    /// The smallest on-disk bare repository `git_repository_open_bare`
    /// accepts, so an empty generated diff exists without a working tree.
    struct BareRepo(std::path::PathBuf);

    impl BareRepo {
        fn create(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-apply-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("objects")).expect("a private temporary directory");
            std::fs::create_dir_all(path.join("refs/heads")).expect("a refs directory");
            std::fs::write(path.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                path.join("config"),
                b"[core]\n\trepositoryformatversion = 0\n\tbare = true\n",
            )
            .expect("a config file");
            Self(path)
        }

        fn c_path(&self) -> std::ffi::CString {
            std::ffi::CString::new(self.0.to_str().expect("a UTF-8 temporary path"))
                .expect("a path without interior NUL")
        }
    }

    impl Drop for BareRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn applying_a_diff_takes_the_repository_and_the_diff_exclusively() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after every owner created here has been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let directory = BareRepo::create("index");
        let mut owner = git_repository_open_bare(&directory.c_path())
            .expect("the hand-built directory is a bare repository");
        {
            let mut repository = owner.as_mut();
            // Both handles have to be exclusive: applying writes the index
            // through the repository, and generating each delta's patch
            // writes the attribute session and file records inside the diff.
            let mut diff = git_diff_tree_to_tree(&mut repository, None, None, None)
                .expect("two absent trees still diff");
            git_apply(
                &mut repository,
                &mut diff.as_mut(),
                ApplyLocation::Index,
                None,
            )
            .expect("an empty diff applies to the index");
        }
        drop(owner);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
