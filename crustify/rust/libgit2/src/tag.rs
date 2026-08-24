//! Safe wrappers for libgit2 tag APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CCloned, define_ctype, impl_dropped};

use crate::ffi;

define_ctype!(
    /// Wraps: git_tag
    /// An opaque, reference-counted annotated Git tag.
    ///
    /// Owned handles release one cache reference with `git_tag_free`, while
    /// cloning acquires another reference with `git_tag_dup`. Repository-backed
    /// tags borrow their repository, which must remain alive while they are used.
    GitTag,
    GitTagRef,
    GitTagMut,
    ffi::git_tag
);

/// An owned reference to an annotated Git tag.
pub type GitTagOwned = CBox<GitTag>;

// SAFETY: `git_tag_free` consumes one reference to a complete tag and releases
// the allocation only when its underlying object cache refcount reaches zero.
// `GitTag` is transparent over the corresponding bindgen type.
impl_dropped!(GitTag, ffi::git_tag, ffi::git_tag_free);

// SAFETY: `git_tag_dup` increments the live tag's underlying object refcount
// and writes the same pointer to its non-null output slot. The new reference is
// independently released by the `CDropped` implementation above.
unsafe impl CCloned for GitTag {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live tag; `duplicate` is a
        // valid output slot, and the wrapper is layout-compatible with
        // `ffi::git_tag`.
        let result = unsafe {
            ffi::git_tag_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_tag>(),
            )
        };
        debug_assert_eq!(result, 0);
        NonNull::new(duplicate.cast::<Self>())
    }
}

/// Wraps: git_tag_name_is_valid
/// Checks whether `name` is a valid tag shorthand.
///
/// `None` mirrors libgit2's accepted null input and is reported as invalid.
pub fn git_tag_name_is_valid(name: Option<&core::ffi::CStr>) -> Result<bool, i32> {
    let mut valid = 0;
    let name = name.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `valid` is writable and `name` is null or a live C string.
    let status = unsafe { ffi::git_tag_name_is_valid(&mut valid, name) };
    if status == 0 {
        Ok(valid != 0)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_tag_preserves_layout_and_refcount_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_refcounted<T: CDropped + CCloned>() {}

        assert_cell::<GitTag>();
        assert_refcounted::<GitTag>();
        assert_eq!(size_of::<GitTag>(), size_of::<ffi::git_tag>());
        assert_eq!(align_of::<GitTag>(), align_of::<ffi::git_tag>());
        assert_eq!(size_of::<GitTagRef<'_>>(), size_of::<*const ffi::git_tag>());
        assert_eq!(size_of::<GitTagMut<'_>>(), size_of::<*mut ffi::git_tag>());
        assert_eq!(size_of::<GitTagOwned>(), size_of::<*mut ffi::git_tag>());
    }

    #[test]
    fn null_tag_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitTagRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitTagMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitTagOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn tag_names_are_checked_through_the_safe_surface() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(git_tag_name_is_valid(Some(c"v1.0")), Ok(true));
        assert_eq!(git_tag_name_is_valid(None), Ok(false));
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
