//! Safe wrappers for libgit2 status APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_status_show_t
/// Selects which comparisons libgit2 includes in a status scan.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum StatusShow {
    /// Compare both `HEAD` to the index and the index to the working directory.
    #[default]
    IndexAndWorkdir = ffi::git_status_show_t_GIT_STATUS_SHOW_INDEX_AND_WORKDIR,
    /// Compare `HEAD` to the index only.
    IndexOnly = ffi::git_status_show_t_GIT_STATUS_SHOW_INDEX_ONLY,
    /// Compare the index to the working directory only.
    WorkdirOnly = ffi::git_status_show_t_GIT_STATUS_SHOW_WORKDIR_ONLY,
}

/// An integer that is not a published [`StatusShow`] value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidStatusShow(ffi::git_status_show_t);

impl InvalidStatusShow {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_status_show_t {
        self.0
    }
}

impl From<StatusShow> for ffi::git_status_show_t {
    fn from(show: StatusShow) -> Self {
        show as Self
    }
}

impl TryFrom<ffi::git_status_show_t> for StatusShow {
    type Error = InvalidStatusShow;

    fn try_from(show: ffi::git_status_show_t) -> Result<Self, Self::Error> {
        match show {
            ffi::git_status_show_t_GIT_STATUS_SHOW_INDEX_AND_WORKDIR => Ok(Self::IndexAndWorkdir),
            ffi::git_status_show_t_GIT_STATUS_SHOW_INDEX_ONLY => Ok(Self::IndexOnly),
            ffi::git_status_show_t_GIT_STATUS_SHOW_WORKDIR_ONLY => Ok(Self::WorkdirOnly),
            value => Err(InvalidStatusShow(value)),
        }
    }
}

/// Wraps: git_status_t
/// A layout-compatible set of status flags for one path.
///
/// Unknown bits can be retained when carrying values from a newer libgit2,
/// while [`Self::from_bits`] validates values against the flags published by
/// the headers used to build this crate.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Status(ffi::git_status_t);

impl Status {
    /// The path is unchanged.
    pub const CURRENT: Self = Self(ffi::git_status_t_GIT_STATUS_CURRENT);
    /// The path is new in the index.
    pub const INDEX_NEW: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_NEW);
    /// The path is modified in the index.
    pub const INDEX_MODIFIED: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_MODIFIED);
    /// The path is deleted from the index.
    pub const INDEX_DELETED: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_DELETED);
    /// The path is renamed in the index.
    pub const INDEX_RENAMED: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_RENAMED);
    /// The path's type changed in the index.
    pub const INDEX_TYPECHANGE: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_TYPECHANGE);
    /// The path is new in the working directory.
    pub const WT_NEW: Self = Self(ffi::git_status_t_GIT_STATUS_WT_NEW);
    /// The path is modified in the working directory.
    pub const WT_MODIFIED: Self = Self(ffi::git_status_t_GIT_STATUS_WT_MODIFIED);
    /// The path is deleted from the working directory.
    pub const WT_DELETED: Self = Self(ffi::git_status_t_GIT_STATUS_WT_DELETED);
    /// The path's type changed in the working directory.
    pub const WT_TYPECHANGE: Self = Self(ffi::git_status_t_GIT_STATUS_WT_TYPECHANGE);
    /// The path is renamed in the working directory.
    pub const WT_RENAMED: Self = Self(ffi::git_status_t_GIT_STATUS_WT_RENAMED);
    /// The path cannot be read from the working directory.
    pub const WT_UNREADABLE: Self = Self(ffi::git_status_t_GIT_STATUS_WT_UNREADABLE);
    /// The path is ignored.
    pub const IGNORED: Self = Self(ffi::git_status_t_GIT_STATUS_IGNORED);
    /// The path is conflicted.
    pub const CONFLICTED: Self = Self(ffi::git_status_t_GIT_STATUS_CONFLICTED);
    /// Every status bit published by this version of libgit2.
    pub const ALL: Self = Self(
        Self::INDEX_NEW.0
            | Self::INDEX_MODIFIED.0
            | Self::INDEX_DELETED.0
            | Self::INDEX_RENAMED.0
            | Self::INDEX_TYPECHANGE.0
            | Self::WT_NEW.0
            | Self::WT_MODIFIED.0
            | Self::WT_DELETED.0
            | Self::WT_TYPECHANGE.0
            | Self::WT_RENAMED.0
            | Self::WT_UNREADABLE.0
            | Self::IGNORED.0
            | Self::CONFLICTED.0,
    );

    /// Converts raw bits when they contain only published status flags.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_status_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Retains all raw bits, including flags introduced by a newer libgit2.
    #[must_use]
    pub const fn from_bits_retain(bits: ffi::git_status_t) -> Self {
        Self(bits)
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_status_t {
        self.0
    }

    /// Returns whether the path has no changes.
    #[must_use]
    pub const fn is_current(self) -> bool {
        self.0 == Self::CURRENT.0
    }

    /// Returns whether every flag in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<ffi::git_status_t> for Status {
    fn from(bits: ffi::git_status_t) -> Self {
        Self::from_bits_retain(bits)
    }
}

impl From<Status> for ffi::git_status_t {
    fn from(status: Status) -> Self {
        status.bits()
    }
}

impl BitOr for Status {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Status {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Status {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Status {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for Status {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn status_show_values_round_trip_through_the_c_type() {
        for show in [
            StatusShow::IndexAndWorkdir,
            StatusShow::IndexOnly,
            StatusShow::WorkdirOnly,
        ] {
            let raw = ffi::git_status_show_t::from(show);
            assert_eq!(StatusShow::try_from(raw), Ok(show));
        }

        let invalid = ffi::git_status_show_t_GIT_STATUS_SHOW_WORKDIR_ONLY + 1;
        assert_eq!(StatusShow::try_from(invalid).unwrap_err().value(), invalid);
    }

    #[test]
    fn status_flags_combine_and_validate() {
        let status = Status::INDEX_MODIFIED | Status::WT_MODIFIED;
        assert!(status.contains(Status::INDEX_MODIFIED));
        assert!(status.intersects(Status::WT_MODIFIED));
        assert!(!status.intersects(Status::CONFLICTED));
        assert_eq!(Status::from_bits(status.bits()), Some(status));
        assert!(Status::CURRENT.is_current());
        assert!(Status::default().is_current());

        let unknown = Status::ALL.bits() + 1;
        assert_eq!(Status::from_bits(unknown), None);
        assert_eq!(Status::from_bits_retain(unknown).bits(), unknown);
    }

    #[test]
    fn status_wrappers_preserve_the_c_enum_layouts() {
        assert_eq!(size_of::<StatusShow>(), size_of::<ffi::git_status_show_t>());
        assert_eq!(
            align_of::<StatusShow>(),
            align_of::<ffi::git_status_show_t>()
        );
        assert_eq!(size_of::<Status>(), size_of::<ffi::git_status_t>());
        assert_eq!(align_of::<Status>(), align_of::<ffi::git_status_t>());
    }
}
