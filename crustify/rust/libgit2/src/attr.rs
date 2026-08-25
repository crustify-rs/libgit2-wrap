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

/// Wraps: git_attr_add_macro
/// Adds a macro definition to the repository's attribute cache.
pub fn git_attr_add_macro(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    name: &core::ffi::CStr,
    values: &core::ffi::CStr,
) -> Result<(), i32> {
    // SAFETY: the repository is exclusive and both live strings are copied
    // into cache-owned storage before this call returns.
    let status =
        unsafe { ffi::git_attr_add_macro(repo.as_mut_ptr(), name.as_ptr(), values.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

unsafe extern "C" fn attr_foreach_trampoline<C>(
    name: *const core::ffi::c_char,
    value: *const core::ffi::c_char,
    payload: *mut core::ffi::c_void,
) -> i32
where
    C: crate::api::attr::GitAttrForeachCallback,
{
    if name.is_null() || payload.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: the guard above rejects a null payload, and the synchronous
    // wrapper installs the address of a live `C` that outlives the traversal.
    let callback = unsafe { &mut *payload.cast::<C>() };
    // SAFETY: `name` is a required callback-scoped string.
    let name = unsafe { core::ffi::CStr::from_ptr(name) };
    // SAFETY: classification only compares the possibly-null pointer with the
    // three immutable libgit2 sentinels.
    let value = match unsafe { ffi::git_attr_value(value) } {
        ffi::git_attr_value_t_GIT_ATTR_VALUE_UNSPECIFIED => Attribute::Unspecified,
        ffi::git_attr_value_t_GIT_ATTR_VALUE_TRUE => Attribute::True,
        ffi::git_attr_value_t_GIT_ATTR_VALUE_FALSE => Attribute::False,
        ffi::git_attr_value_t_GIT_ATTR_VALUE_STRING if !value.is_null() => {
            // SAFETY: a string-category value is a callback-scoped C string.
            Attribute::String(unsafe { core::ffi::CStr::from_ptr(value) })
        }
        _ => return ffi::git_error_code_GIT_ERROR,
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback.call(name, value)))
        .unwrap_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_attr_foreach
/// Visits the effective attributes for `path` synchronously.
pub fn git_attr_foreach<C>(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    flags: u32,
    path: &core::ffi::CStr,
    callback: &mut C,
) -> Result<(), i32>
where
    C: crate::api::attr::GitAttrForeachCallback,
{
    // SAFETY: all inputs and the erased callback payload remain live for the
    // complete synchronous traversal; the trampoline catches Rust panics.
    let status = unsafe {
        ffi::git_attr_foreach(
            repo.as_mut_ptr(),
            flags,
            path.as_ptr(),
            Some(attr_foreach_trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_attr_get_many
/// Looks up several attributes and ties returned strings to `repo`.
pub fn git_attr_get_many<'repo>(
    mut repo: crate::repository::GitRepositoryMut<'repo>,
    flags: u32,
    path: &core::ffi::CStr,
    names: &[&core::ffi::CStr],
) -> Result<Vec<Attribute<'repo>>, i32> {
    let mut values = vec![core::ptr::null(); names.len()];
    let mut raw_names: Vec<_> = names.iter().map(|name| name.as_ptr()).collect();
    // SAFETY: outputs and name pointers cover `names.len()` elements; the
    // repository and path are live, and returned cache strings are lifetime-
    // bound by consuming the repository handle into this result.
    let status = unsafe {
        ffi::git_attr_get_many(
            values.as_mut_ptr(),
            repo.as_mut_ptr(),
            flags,
            path.as_ptr(),
            names.len(),
            raw_names.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    values
        .into_iter()
        .map(|value| {
            // SAFETY: classification only compares pointer identity.
            match unsafe { ffi::git_attr_value(value) } {
                ffi::git_attr_value_t_GIT_ATTR_VALUE_UNSPECIFIED => Ok(Attribute::Unspecified),
                ffi::git_attr_value_t_GIT_ATTR_VALUE_TRUE => Ok(Attribute::True),
                ffi::git_attr_value_t_GIT_ATTR_VALUE_FALSE => Ok(Attribute::False),
                ffi::git_attr_value_t_GIT_ATTR_VALUE_STRING if !value.is_null() => {
                    // SAFETY: string-category values are cache-owned C strings.
                    Ok(Attribute::String(unsafe {
                        core::ffi::CStr::from_ptr(value)
                    }))
                }
                raw => Err(InvalidAttrValue(raw).0 as i32),
            }
        })
        .collect()
}

#[cfg(test)]
mod scheduled_callback_tests {
    use super::*;
    #[test]
    fn attr_trampoline_delivers_optional_values() {
        unsafe fn invoke<C>(callback: &mut C) -> i32
        where
            C: crate::api::attr::GitAttrForeachCallback,
        {
            // SAFETY: the literals and exact callback payload remain live for
            // this direct trampoline invocation.
            unsafe {
                attr_foreach_trampoline::<C>(
                    c"kind".as_ptr(),
                    c"value".as_ptr(),
                    core::ptr::from_mut(callback).cast(),
                )
            }
        }

        let mut seen = false;
        let mut callback = |name: &core::ffi::CStr, value: Attribute<'_>| {
            seen = name == c"kind" && value == Attribute::String(c"value");
            0
        };
        // SAFETY: `invoke` keeps the callback live for the direct call.
        let status = unsafe { invoke(&mut callback) };
        assert_eq!(status, 0);
        assert!(seen);
    }
}

/// Wraps: git_attr_foreach_ext
/// Visits effective attributes using the supplied extended options.
pub fn git_attr_foreach_ext<C>(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    options: crate::api::attr::GitAttrOptionsRef<'_>,
    path: &core::ffi::CStr,
    callback: &mut C,
) -> Result<(), i32>
where
    C: crate::api::attr::GitAttrForeachCallback,
{
    // SAFETY: every typed input is live for this synchronous traversal; the
    // callback payload has its exact concrete type and the trampoline catches panics.
    let status = unsafe {
        ffi::git_attr_foreach_ext(
            repo.as_mut_ptr(),
            options.as_ptr().cast_mut(),
            path.as_ptr(),
            Some(attr_foreach_trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    status_result(status)
}

/// Wraps: git_attr_get_ext
/// Looks up one attribute using extended options.
pub fn git_attr_get_ext<'repo>(
    mut repo: crate::repository::GitRepositoryMut<'repo>,
    options: crate::api::attr::GitAttrOptionsRef<'_>,
    path: &core::ffi::CStr,
    name: &core::ffi::CStr,
) -> Result<Attribute<'repo>, i32> {
    let mut value = core::ptr::null();
    // SAFETY: the output slot, repository, options and strings are live; any
    // returned string is cache-owned and tied to the consumed repository borrow.
    let status = unsafe {
        ffi::git_attr_get_ext(
            &mut value,
            repo.as_mut_ptr(),
            options.as_ptr().cast_mut(),
            path.as_ptr(),
            name.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    classify_attribute(value)
}

/// Wraps: git_attr_get_many_ext
/// Looks up several attributes using extended options.
pub fn git_attr_get_many_ext<'repo>(
    mut repo: crate::repository::GitRepositoryMut<'repo>,
    options: crate::api::attr::GitAttrOptionsRef<'_>,
    path: &core::ffi::CStr,
    names: &[&core::ffi::CStr],
) -> Result<Vec<Attribute<'repo>>, i32> {
    let mut values = vec![core::ptr::null(); names.len()];
    let mut raw_names: Vec<_> = names.iter().map(|name| name.as_ptr()).collect();
    // SAFETY: both arrays cover `names.len()` elements and all other inputs
    // are live for the call; result strings remain repository-cache owned.
    let status = unsafe {
        ffi::git_attr_get_many_ext(
            values.as_mut_ptr(),
            repo.as_mut_ptr(),
            options.as_ptr().cast_mut(),
            path.as_ptr(),
            names.len(),
            raw_names.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    values.into_iter().map(classify_attribute).collect()
}

fn classify_attribute<'a>(value: *const core::ffi::c_char) -> Result<Attribute<'a>, i32> {
    // SAFETY: classification compares pointer identity and accepts null.
    match unsafe { ffi::git_attr_value(value) } {
        ffi::git_attr_value_t_GIT_ATTR_VALUE_UNSPECIFIED => Ok(Attribute::Unspecified),
        ffi::git_attr_value_t_GIT_ATTR_VALUE_TRUE => Ok(Attribute::True),
        ffi::git_attr_value_t_GIT_ATTR_VALUE_FALSE => Ok(Attribute::False),
        ffi::git_attr_value_t_GIT_ATTR_VALUE_STRING if !value.is_null() => {
            // SAFETY: string-category values are NUL-terminated and cache-owned.
            Ok(Attribute::String(unsafe {
                core::ffi::CStr::from_ptr(value)
            }))
        }
        _ => Err(ffi::git_error_code_GIT_ERROR),
    }
}

fn status_result(status: i32) -> Result<(), i32> {
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_attr_options_init
/// Creates extended attribute options initialized for this ABI.
pub fn git_attr_options_init() -> Result<ffibox::CVal<crate::api::attr::GitAttrOptions>, i32> {
    let mut options = ffibox::CVal::new(crate::api::attr::GitAttrOptions::zeroed());
    // SAFETY: the inline options storage is exclusively writable.
    let status = unsafe {
        ffi::git_attr_options_init(options.as_mut().as_mut_ptr(), ffi::GIT_ATTR_OPTIONS_VERSION)
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}
