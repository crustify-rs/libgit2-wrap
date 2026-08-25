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
