//! Safe wrappers for libgit2 attr APIs.

use crate::ffi;

/// Wraps: git_attr_value_t
/// A value category returned by libgit2's attribute APIs.
///
/// The string itself is returned separately when the category is [`Self::String`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub enum AttrValue {
    /// The attribute was not specified.
    Unspecified = ffi::git_attr_value_t_GIT_ATTR_VALUE_UNSPECIFIED,
    /// The attribute was set without an explicit value.
    True = ffi::git_attr_value_t_GIT_ATTR_VALUE_TRUE,
    /// The attribute was explicitly unset.
    False = ffi::git_attr_value_t_GIT_ATTR_VALUE_FALSE,
    /// The attribute has a string value.
    String = ffi::git_attr_value_t_GIT_ATTR_VALUE_STRING,
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
    fn public_classifier_handles_null_and_ordinary_strings() {
        assert_eq!(git_attr_value(None), AttrValue::Unspecified);
        assert_eq!(git_attr_value(Some(c"ordinary")), AttrValue::String);
    }

    #[test]
    fn classification_reads_pointer_identity_and_not_string_contents() {
        // The sentinel's own spelling, held in Rust storage, is not the
        // sentinel address, so libgit2 reports an ordinary string value.
        assert_eq!(
            git_attr_value(Some(c"[internal]__TRUE__")),
            AttrValue::String
        );
        assert_eq!(
            git_attr_value(Some(c"[internal]__UNSET__")),
            AttrValue::String
        );
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

/// A classified attribute lookup result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Attribute<'repo> {
    /// The attribute is not specified.
    Unspecified,
    /// The attribute is set without a value.
    True,
    /// The attribute is explicitly unset.
    False,
    /// The attribute carries a string borrowed from the repository cache.
    String(&'repo core::ffi::CStr),
}

/// Wraps: git_attr_get
/// Looks up one attribute and ties any returned string to the repository borrow.
pub fn git_attr_get<'repo>(
    mut repo: crate::repository::GitRepositoryMut<'repo>,
    flags: u32,
    path: &core::ffi::CStr,
    name: &core::ffi::CStr,
) -> Result<Attribute<'repo>, i32> {
    let mut value = core::ptr::null();
    // SAFETY: `repo` is live and exclusive, both strings are live, and
    // `value` is a writable output slot. The return type retains the repo
    // borrow that keeps the attribute cache alive and prevents safe flushing.
    let status = unsafe {
        ffi::git_attr_get(
            &mut value,
            repo.as_mut_ptr(),
            flags,
            path.as_ptr(),
            name.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }

    // SAFETY: `git_attr_value` accepts null and only classifies the pointer.
    let kind = unsafe { ffi::git_attr_value(value) };
    match AttrValue::try_from(kind).expect("git_attr_value returned an unknown category") {
        AttrValue::Unspecified => Ok(Attribute::Unspecified),
        AttrValue::True => Ok(Attribute::True),
        AttrValue::False => Ok(Attribute::False),
        AttrValue::String => {
            debug_assert!(!value.is_null());
            // SAFETY: a string-category success returns a non-null
            // NUL-terminated string retained by the repository cache.
            Ok(Attribute::String(unsafe {
                core::ffi::CStr::from_ptr(value)
            }))
        }
    }
}

/// Wraps: git_attr_value
/// Classifies an optional attribute pointer.
///
/// libgit2 classifies by pointer *identity*, not by contents: only the three
/// sentinel addresses `git_attr__unset`, `git_attr__true` and `git_attr__false`
/// -- which libgit2 hands out and never publishes -- map to
/// [`AttrValue::Unspecified`], [`AttrValue::True`] and [`AttrValue::False`].
/// A null pointer is also unspecified. Every other pointer, including any
/// string a Rust caller owns, is [`AttrValue::String`] whatever its bytes are.
/// Callers wanting the category of a lookup should use the classified
/// [`Attribute`] that [`git_attr_get`] already returns.
#[must_use]
pub fn git_attr_value(attr: Option<&core::ffi::CStr>) -> AttrValue {
    let attr = attr.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `attr` is null or a live C string; the function never
    // dereferences it and only compares it with libgit2's sentinel addresses.
    let raw = unsafe { ffi::git_attr_value(attr) };
    AttrValue::try_from(raw).expect("git_attr_value returned an unknown category")
}
