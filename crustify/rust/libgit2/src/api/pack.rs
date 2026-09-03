//! Safe wrappers for libgit2 pack APIs.

use crate::ffi;

/// Wraps: git_packbuilder_stage_t
/// A checked stage reported while libgit2 builds a packfile.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitPackbuilderStage {
    /// Objects are being enumerated and added to the pack.
    AddingObjects = ffi::git_packbuilder_stage_t_GIT_PACKBUILDER_ADDING_OBJECTS,
    /// Candidate deltas are being selected and generated.
    Deltafication = ffi::git_packbuilder_stage_t_GIT_PACKBUILDER_DELTAFICATION,
}

/// A raw packbuilder stage not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitPackbuilderStage(ffi::git_packbuilder_stage_t);

impl InvalidGitPackbuilderStage {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_packbuilder_stage_t {
        self.0
    }
}

impl From<GitPackbuilderStage> for ffi::git_packbuilder_stage_t {
    fn from(stage: GitPackbuilderStage) -> Self {
        stage as Self
    }
}

impl TryFrom<ffi::git_packbuilder_stage_t> for GitPackbuilderStage {
    type Error = InvalidGitPackbuilderStage;

    fn try_from(stage: ffi::git_packbuilder_stage_t) -> Result<Self, Self::Error> {
        match stage {
            ffi::git_packbuilder_stage_t_GIT_PACKBUILDER_ADDING_OBJECTS => Ok(Self::AddingObjects),
            ffi::git_packbuilder_stage_t_GIT_PACKBUILDER_DELTAFICATION => Ok(Self::Deltafication),
            value => Err(InvalidGitPackbuilderStage(value)),
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn packbuilder_stages_round_trip() {
        for stage in [
            GitPackbuilderStage::AddingObjects,
            GitPackbuilderStage::Deltafication,
        ] {
            let raw = ffi::git_packbuilder_stage_t::from(stage);
            assert_eq!(GitPackbuilderStage::try_from(raw), Ok(stage));
        }
    }

    #[test]
    fn packbuilder_stage_rejects_unknown_values() {
        let raw = ffi::git_packbuilder_stage_t_GIT_PACKBUILDER_DELTAFICATION + 1;
        assert_eq!(GitPackbuilderStage::try_from(raw).unwrap_err().value(), raw);
    }

    #[test]
    fn packbuilder_stage_matches_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitPackbuilderStage>(),
            size_of::<ffi::git_packbuilder_stage_t>()
        );
        assert_eq!(
            align_of::<GitPackbuilderStage>(),
            align_of::<ffi::git_packbuilder_stage_t>()
        );
    }
}
