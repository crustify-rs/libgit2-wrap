//! Safe wrappers for libgit2 config APIs.

use crate::ffi;

/// Wraps: git_configmap_t
/// The conversion applied when matching a textual configuration value.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitConfigmapType {
    /// Parse and match a false Boolean value.
    False = ffi::git_configmap_t_GIT_CONFIGMAP_FALSE,
    /// Parse and match a true Boolean value.
    True = ffi::git_configmap_t_GIT_CONFIGMAP_TRUE,
    /// Parse the value as a signed 32-bit integer.
    Int32 = ffi::git_configmap_t_GIT_CONFIGMAP_INT32,
    /// Compare the value to the mapping's string exactly.
    String = ffi::git_configmap_t_GIT_CONFIGMAP_STRING,
}

/// A raw configuration-map type not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitConfigmapType(ffi::git_configmap_t);

impl InvalidGitConfigmapType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_configmap_t {
        self.0
    }
}

impl From<GitConfigmapType> for ffi::git_configmap_t {
    fn from(value: GitConfigmapType) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_configmap_t> for GitConfigmapType {
    type Error = InvalidGitConfigmapType;

    fn try_from(value: ffi::git_configmap_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_configmap_t_GIT_CONFIGMAP_FALSE => Ok(Self::False),
            ffi::git_configmap_t_GIT_CONFIGMAP_TRUE => Ok(Self::True),
            ffi::git_configmap_t_GIT_CONFIGMAP_INT32 => Ok(Self::Int32),
            ffi::git_configmap_t_GIT_CONFIGMAP_STRING => Ok(Self::String),
            value => Err(InvalidGitConfigmapType(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn configmap_types_round_trip_through_the_c_type() {
        for kind in [
            GitConfigmapType::False,
            GitConfigmapType::True,
            GitConfigmapType::Int32,
            GitConfigmapType::String,
        ] {
            let raw = ffi::git_configmap_t::from(kind);
            assert_eq!(GitConfigmapType::try_from(raw), Ok(kind));
        }
    }

    #[test]
    fn configmap_type_rejects_unknown_values() {
        let raw = ffi::git_configmap_t_GIT_CONFIGMAP_STRING + 1;
        assert_eq!(GitConfigmapType::try_from(raw).unwrap_err().value(), raw);
    }

    #[test]
    fn configmap_type_matches_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitConfigmapType>(),
            size_of::<ffi::git_configmap_t>()
        );
        assert_eq!(
            align_of::<GitConfigmapType>(),
            align_of::<ffi::git_configmap_t>()
        );
    }
}
