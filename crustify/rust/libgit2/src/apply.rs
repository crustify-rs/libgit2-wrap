//! Safe wrappers for libgit2 apply APIs.

use crate::ffi;

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
