//! Safe wrappers for libgit2 filter APIs.

use ffibox::{define_ctype, impl_dropped};

use crate::ffi;

define_ctype!(
    /// Wraps: git_filter_list
    /// An ordered list of filters prepared by libgit2.
    ///
    /// The public C API keeps the layout opaque. An owning
    /// [`ffibox::CBox<GitFilterList>`] releases the list, its entry buffer and
    /// every filter payload with `git_filter_list_free`.
    GitFilterList,
    GitFilterListRef,
    GitFilterListMut,
    ffi::git_filter_list
);

// SAFETY: `git_filter_list_free` is the public destructor for a complete list
// allocation. It accepts null, while `CDropped` supplies a live non-null value,
// runs each entry's cleanup callback, releases the entry buffer, and finally
// frees the list allocation. `GitFilterList` is transparent over the matching
// bindgen type.
impl_dropped!(
    GitFilterList,
    ffi::git_filter_list,
    ffi::git_filter_list_free
);

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CBox, CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_filter_list_has_c_layout_and_lifecycle_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitFilterList>();
        assert_dropped::<GitFilterList>();
        assert_eq!(
            size_of::<GitFilterList>(),
            size_of::<ffi::git_filter_list>()
        );
        assert_eq!(
            align_of::<GitFilterList>(),
            align_of::<ffi::git_filter_list>()
        );
        assert_eq!(
            size_of::<GitFilterListRef<'_>>(),
            size_of::<*mut ffi::git_filter_list>()
        );
        assert_eq!(
            size_of::<GitFilterListMut<'_>>(),
            size_of::<*mut ffi::git_filter_list>()
        );
    }

    #[test]
    fn null_filter_list_seams_create_no_handle() {
        // SAFETY: each conversion explicitly accepts null and returns `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitFilterListRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitFilterListMut::from_ptr(ptr::null_mut()).is_none());
            assert!(CBox::<GitFilterList>::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// Wraps: git_filter_list_apply_to_buffer
/// Applies an optional filter list to length-delimited bytes.
pub fn git_filter_list_apply_to_buffer(
    filters: Option<&mut GitFilterListMut<'_>>,
    input: &[u8],
) -> Result<ffibox::CVal<crate::api::buffer::GitBuf>, i32> {
    let mut out = crate::api::buffer::GitBuf::new();
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: output is writable, input covers its exact length, and optional
    // filters remain live for this non-retaining operation.
    let status = unsafe {
        ffi::git_filter_list_apply_to_buffer(
            out.as_mut().as_mut_ptr(),
            filters,
            input.as_ptr().cast(),
            input.len(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_filter_list_apply_to_file
/// Applies an optional filter list to a file.
pub fn git_filter_list_apply_to_file(
    filters: Option<&mut GitFilterListMut<'_>>,
    repo: crate::repository::GitRepositoryRef<'_>,
    path: &core::ffi::CStr,
) -> Result<ffibox::CVal<crate::api::buffer::GitBuf>, i32> {
    let mut out = crate::api::buffer::GitBuf::new();
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: all handles and path are live and output is exclusively writable.
    let status = unsafe {
        ffi::git_filter_list_apply_to_file(
            out.as_mut().as_mut_ptr(),
            filters,
            repo.as_ptr().cast_mut(),
            path.as_ptr(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_filter_list_contains
/// Reports whether an optional filter list contains `name`.
#[must_use]
pub fn git_filter_list_contains(
    filters: Option<GitFilterListRef<'_>>,
    name: &core::ffi::CStr,
) -> bool {
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut());
    // SAFETY: the optional handle and required name are live for the query.
    unsafe { ffi::git_filter_list_contains(filters, name.as_ptr()) != 0 }
}

/// Wraps: git_filter_list_free
/// An owning filter list released automatically by its registered destructor.
pub type GitFilterListOwned = ffibox::CBox<GitFilterList>;

/// Wraps: git_filter_list_stream_buffer
/// Streams filtered bytes into `target`.
pub fn git_filter_list_stream_buffer(
    filters: Option<&mut GitFilterListMut<'_>>,
    input: &[u8],
    target: &mut crate::api::types::GitWriteStreamMut<'_>,
) -> Result<(), i32> {
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: all borrowed storage is live and target is exclusive for the
    // complete synchronous streaming operation.
    let status = unsafe {
        ffi::git_filter_list_stream_buffer(
            filters,
            input.as_ptr().cast(),
            input.len(),
            target.as_mut_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_filter_list_stream_file
/// Streams a filtered file into `target`.
pub fn git_filter_list_stream_file(
    filters: Option<&mut GitFilterListMut<'_>>,
    repo: crate::repository::GitRepositoryRef<'_>,
    path: &core::ffi::CStr,
    target: &mut crate::api::types::GitWriteStreamMut<'_>,
) -> Result<(), i32> {
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: every input is live and target is exclusive for the synchronous call.
    let status = unsafe {
        ffi::git_filter_list_stream_file(
            filters,
            repo.as_ptr().cast_mut(),
            path.as_ptr(),
            target.as_mut_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_filter_tests {
    use super::*;
    #[test]
    fn null_filter_list_contains_nothing() {
        assert!(!git_filter_list_contains(None, c"ident"));
    }
}
