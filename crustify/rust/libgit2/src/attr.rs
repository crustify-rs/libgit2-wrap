//! Safe wrappers for libgit2 attr APIs.

use crate::ffi;

/// A value category returned by libgit2's attribute APIs.
///
/// The string itself is returned separately when the category is [`Self::String`].
/// Wraps: git_attr_value_t
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(C)]
pub enum AttrValue {
    /// The attribute was not specified.
    Unspecified = ffi::git_attr_value_t_GIT_ATTR_VALUE_UNSPECIFIED as isize,
    /// The attribute was set without an explicit value.
    True = ffi::git_attr_value_t_GIT_ATTR_VALUE_TRUE as isize,
    /// The attribute was explicitly unset.
    False = ffi::git_attr_value_t_GIT_ATTR_VALUE_FALSE as isize,
    /// The attribute has a string value.
    String = ffi::git_attr_value_t_GIT_ATTR_VALUE_STRING as isize,
}

/// A raw attribute category that is not defined by the linked libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidAttrValue(pub ffi::git_attr_value_t);

impl TryFrom<ffi::git_attr_value_t> for AttrValue {
    type Error = InvalidAttrValue;

    fn try_from(raw: ffi::git_attr_value_t) -> Result<Self, Self::Error> {
        match raw {
            ffi::git_attr_value_t_GIT_ATTR_VALUE_UNSPECIFIED => Ok(Self::Unspecified),
            ffi::git_attr_value_t_GIT_ATTR_VALUE_TRUE => Ok(Self::True),
            ffi::git_attr_value_t_GIT_ATTR_VALUE_FALSE => Ok(Self::False),
            ffi::git_attr_value_t_GIT_ATTR_VALUE_STRING => Ok(Self::String),
            raw => Err(InvalidAttrValue(raw)),
        }
    }
}

impl From<AttrValue> for ffi::git_attr_value_t {
    fn from(value: AttrValue) -> Self {
        value as Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn attr_values_round_trip_through_the_ffi_representation() {
        let values = [
            AttrValue::Unspecified,
            AttrValue::True,
            AttrValue::False,
            AttrValue::String,
        ];

        for value in values {
            let raw = ffi::git_attr_value_t::from(value);
            assert_eq!(AttrValue::try_from(raw), Ok(value));
        }
    }

    #[test]
    fn unknown_attr_values_are_rejected() {
        let raw = ffi::git_attr_value_t_GIT_ATTR_VALUE_STRING + 1;
        assert_eq!(AttrValue::try_from(raw), Err(InvalidAttrValue(raw)));
    }

    #[test]
    fn attr_value_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<AttrValue>(), size_of::<ffi::git_attr_value_t>());
        assert_eq!(align_of::<AttrValue>(), align_of::<ffi::git_attr_value_t>());
    }
}
