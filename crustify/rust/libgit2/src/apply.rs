//! Safe wrappers for libgit2 apply APIs.

use crate::api::apply::{GitApplyOptionsMut, GitApplyOptionsRef};
use crate::diff::DiffRef;
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
pub fn git_apply(
    repository: &mut GitRepositoryMut<'_>,
    diff: DiffRef<'_>,
    location: ApplyLocation,
    options: Option<GitApplyOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository is exclusively borrowed, while the diff and
    // optional options remain live and read-only for the synchronous call.
    let status = unsafe {
        ffi::git_apply(
            repository.as_mut_ptr(),
            diff.as_ptr().cast_mut(),
            location.into(),
            options,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_apply_options_init
/// Initializes apply options for `version`.
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
pub fn git_apply_to_tree(
    repository: &mut GitRepositoryMut<'_>,
    preimage: GitTreeRef<'_>,
    diff: DiffRef<'_>,
    options: Option<GitApplyOptionsRef<'_, '_>>,
) -> Result<GitIndexOwned, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: `out` is writable, the repository is exclusive, and all other
    // typed inputs remain live and read-only for the call.
    let status = unsafe {
        ffi::git_apply_to_tree(
            &mut out,
            repository.as_mut_ptr(),
            preimage.as_ptr().cast_mut(),
            diff.as_ptr().cast_mut(),
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

    #[test]
    fn initializer_writes_the_requested_apply_version() {
        let mut options = GitApplyOptions::new();
        git_apply_options_init(&mut options.as_mut(), ffi::GIT_APPLY_OPTIONS_VERSION).unwrap();
        assert_eq!(options.as_ref().version(), ffi::GIT_APPLY_OPTIONS_VERSION);
    }
}
