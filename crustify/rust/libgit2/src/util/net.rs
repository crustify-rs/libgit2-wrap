//! Safe wrappers for libgit2 net APIs.

use crate::ffi;

/// Wraps: git_direction
/// The direction of a network operation.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Direction {
    /// Download objects and references from a remote.
    Fetch = ffi::git_direction_GIT_DIRECTION_FETCH,
    /// Upload objects and references to a remote.
    Push = ffi::git_direction_GIT_DIRECTION_PUSH,
}

/// A raw value that is not a valid [`Direction`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDirection(ffi::git_direction);

impl InvalidDirection {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_direction {
        self.0
    }
}

impl From<Direction> for ffi::git_direction {
    fn from(direction: Direction) -> Self {
        direction as Self
    }
}

impl TryFrom<ffi::git_direction> for Direction {
    type Error = InvalidDirection;

    fn try_from(direction: ffi::git_direction) -> Result<Self, Self::Error> {
        match direction {
            ffi::git_direction_GIT_DIRECTION_FETCH => Ok(Self::Fetch),
            ffi::git_direction_GIT_DIRECTION_PUSH => Ok(Self::Push),
            value => Err(InvalidDirection(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn directions_round_trip_through_the_c_type() {
        for direction in [Direction::Fetch, Direction::Push] {
            let raw = ffi::git_direction::from(direction);
            assert_eq!(Direction::try_from(raw), Ok(direction));
        }
    }

    #[test]
    fn invalid_directions_are_rejected_without_constructing_an_enum() {
        let invalid = ffi::git_direction_GIT_DIRECTION_PUSH + 1;
        let error = Direction::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn direction_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<Direction>(), size_of::<ffi::git_direction>());
        assert_eq!(align_of::<Direction>(), align_of::<ffi::git_direction>());
    }
}
