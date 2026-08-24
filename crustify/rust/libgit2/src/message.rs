//! Safe wrappers for libgit2 message APIs.

use core::ffi::CStr;
use core::ptr::{NonNull, addr_of};

use ffibox::{CSlice, CSliceMut, CVal};

use crate::api::buffer::GitBufMut;
use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_message_trailer
    /// A key/value pair borrowed from an owning message-trailer array.
    ///
    /// Both strings point into the array's private trailer block. A borrowed
    /// handle must therefore never outlive the array that produced it.
    GitMessageTrailer,
    GitMessageTrailerRef,
    GitMessageTrailerMut,
    ffi::git_message_trailer
);

impl<'a> GitMessageTrailerRef<'a> {
    /// Wraps: git_message_trailer.key
    /// Returns the trailer key borrowed from the containing trailer array.
    #[must_use]
    pub fn key(&self) -> &'a CStr {
        let trailer = self.as_ptr();
        // SAFETY: a live trailer produced by `git_message_trailers` has a
        // non-null NUL-terminated key in its array-owned trailer block. The
        // handle's lifetime is bounded by the owner that keeps that block
        // alive, and raw-place projection forms no reference to C storage.
        unsafe { CStr::from_ptr(core::ptr::addr_of!((*trailer).key).read()) }
    }

    /// Wraps: git_message_trailer.value
    /// Returns the trailer value borrowed from the containing trailer array.
    #[must_use]
    pub fn value(&self) -> &'a CStr {
        let trailer = self.as_ptr();
        // SAFETY: a live trailer produced by `git_message_trailers` has a
        // non-null NUL-terminated value in its array-owned trailer block. The
        // handle's lifetime is bounded by the owner that keeps that block
        // alive, and raw-place projection forms no reference to C storage.
        unsafe { CStr::from_ptr(core::ptr::addr_of!((*trailer).value).read()) }
    }
}

impl GitMessageTrailerMut<'_> {
    /// Stores a borrowed trailer key.
    ///
    /// # Safety
    ///
    /// `key` must remain alive and NUL-terminated for every subsequent use of
    /// the trailer, including uses after this mutable handle is released.
    pub unsafe fn set_key(&mut self, key: &CStr) {
        let trailer = self.as_mut_ptr();
        // SAFETY: this handle provides exclusive access to the pointer field,
        // raw-place projection forms no reference to C storage, and the caller
        // guarantees the stored string's required lifetime.
        unsafe { core::ptr::addr_of_mut!((*trailer).key).write(key.as_ptr()) }
    }

    /// Stores a borrowed trailer value.
    ///
    /// # Safety
    ///
    /// `value` must remain alive and NUL-terminated for every subsequent use
    /// of the trailer, including uses after this mutable handle is released.
    pub unsafe fn set_value(&mut self, value: &CStr) {
        let trailer = self.as_mut_ptr();
        // SAFETY: this handle provides exclusive access to the pointer field,
        // raw-place projection forms no reference to C storage, and the caller
        // guarantees the stored string's required lifetime.
        unsafe { core::ptr::addr_of_mut!((*trailer).value).write(value.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn trailer_wrapper_preserves_the_c_layout() {
        assert_eq!(
            size_of::<GitMessageTrailer>(),
            size_of::<ffi::git_message_trailer>()
        );
        assert_eq!(
            align_of::<GitMessageTrailer>(),
            align_of::<ffi::git_message_trailer>()
        );
        assert_eq!(
            size_of::<GitMessageTrailerRef<'_>>(),
            size_of::<*const ffi::git_message_trailer>()
        );
        assert_eq!(
            size_of::<GitMessageTrailerMut<'_>>(),
            size_of::<*mut ffi::git_message_trailer>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_replace_both_strings() {
        let first_key = c"Signed-off-by";
        let first_value = c"A. U. Thor";
        let second_key = c"Reviewed-by";
        let second_value = c"R. Viewer";
        let mut raw = ffi::git_message_trailer {
            key: first_key.as_ptr(),
            value: first_value.as_ptr(),
        };

        // SAFETY: `raw` and all four static C string literals remain alive for
        // the handles' complete use, and the mutable handle is exclusive.
        let mut trailer = unsafe { GitMessageTrailerMut::from_ptr(&raw mut raw) }
            .expect("address of a local trailer is non-null");
        assert_eq!(trailer.as_ref().key(), first_key);
        assert_eq!(trailer.as_ref().value(), first_value);
        // SAFETY: the replacement strings have static storage duration.
        unsafe {
            trailer.set_key(second_key);
            trailer.set_value(second_value);
        }
        assert_eq!(trailer.as_ref().key(), second_key);
        assert_eq!(trailer.as_ref().value(), second_value);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_message_trailer_array
    /// An inline header that owns a trailer descriptor array and its strings.
    GitMessageTrailerArray,
    GitMessageTrailerArrayRef,
    GitMessageTrailerArrayMut,
    ffi::git_message_trailer_array
);

// `git_message_trailer_array_free` releases both owned fields but retains the
// caller-provided inline header.
ffibox::impl_cvalued!(
    GitMessageTrailerArray,
    ffi::git_message_trailer_array,
    ffi::git_message_trailer_array_free
);

impl GitMessageTrailerArray {
    /// Constructs an empty owned array whose fields are disposed on drop.
    #[must_use]
    pub fn new() -> CVal<Self> {
        CVal::new(Self::zeroed())
    }
}

impl<'a> GitMessageTrailerArrayRef<'a> {
    /// Wraps: git_message_trailer_array.count
    /// Returns the number of initialized trailer descriptors.
    #[must_use]
    pub fn len(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).count).read() }
    }

    /// Returns whether the array contains no trailers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Wraps: git_message_trailer_array._trailer_block
    /// Reports whether the private backing string allocation is installed.
    ///
    /// Its bytes are intentionally not exposed: trailer keys and values are
    /// the public, lifetime-bound views into this allocation.
    #[must_use]
    pub fn has_trailer_block(&self) -> bool {
        // SAFETY: this live shared handle permits a raw-place pointer read
        // without forming a reference to C-visible memory.
        !unsafe { addr_of!((*self.as_ptr())._trailer_block).read() }.is_null()
    }

    /// Wraps: git_message_trailer_array.trailers
    /// Borrows the initialized trailer descriptors with their owner lifetime.
    ///
    /// `None` is the valid null representation used by an empty array.
    #[must_use]
    pub fn trailers(&self) -> Option<CSlice<'a, GitMessageTrailer>> {
        // SAFETY: both fields are initialized members of this live array and
        // raw-place projection forms no reference to C-visible memory.
        let (trailers, count) = unsafe {
            (
                addr_of!((*self.as_ptr()).trailers).read(),
                addr_of!((*self.as_ptr()).count).read(),
            )
        };
        let trailers = NonNull::new(trailers.cast::<GitMessageTrailer>())?;
        // SAFETY: a valid non-null array field owns `count` contiguous,
        // initialized descriptors. Their strings remain alive through the
        // same parent borrow because the parent owns `_trailer_block` too.
        Some(unsafe { CSlice::from_raw_parts(trailers, count) })
    }
}

impl GitMessageTrailerArrayMut<'_> {
    /// Exclusively borrows the initialized trailer descriptors.
    ///
    /// `None` is the valid null representation used by an empty array.
    #[must_use]
    pub fn trailers_mut(&mut self) -> Option<CSliceMut<'_, GitMessageTrailer>> {
        let array = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits raw-place reads of both
        // initialized fields without forming references to C-visible memory.
        let (trailers, count) = unsafe {
            (
                addr_of!((*array).trailers).read(),
                addr_of!((*array).count).read(),
            )
        };
        let trailers = NonNull::new(trailers.cast::<GitMessageTrailer>())?;
        // SAFETY: the valid field contains `count` contiguous initialized
        // descriptors, and the returned exclusive view is reborrowed from
        // `&mut self`, preventing competing handle access.
        Some(unsafe { CSliceMut::from_raw_parts(trailers, count) })
    }
}

#[cfg(test)]
mod trailer_array_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn trailer_array_wrapper_preserves_the_c_layout() {
        assert_eq!(
            size_of::<GitMessageTrailerArray>(),
            size_of::<ffi::git_message_trailer_array>()
        );
        assert_eq!(
            align_of::<GitMessageTrailerArray>(),
            align_of::<ffi::git_message_trailer_array>()
        );
        assert_eq!(
            size_of::<GitMessageTrailerArrayRef<'_>>(),
            size_of::<*const ffi::git_message_trailer_array>()
        );
        assert_eq!(
            size_of::<GitMessageTrailerArrayMut<'_>>(),
            size_of::<*mut ffi::git_message_trailer_array>()
        );
    }

    #[test]
    fn empty_owned_array_has_no_allocations() {
        let array = GitMessageTrailerArray::new();
        assert!(array.as_ref().is_empty());
        assert!(!array.as_ref().has_trailer_block());
        assert!(array.as_ref().trailers().is_none());
    }

    #[test]
    fn trailer_views_keep_strings_tied_to_the_array() {
        let mut trailers = [
            ffi::git_message_trailer {
                key: c"Signed-off-by".as_ptr(),
                value: c"A. U. Thor".as_ptr(),
            },
            ffi::git_message_trailer {
                key: c"Reviewed-by".as_ptr(),
                value: c"R. Viewer".as_ptr(),
            },
        ];
        let mut raw = ffi::git_message_trailer_array {
            trailers: trailers.as_mut_ptr(),
            count: trailers.len(),
            _trailer_block: c"private".as_ptr().cast_mut(),
        };

        // SAFETY: `raw`, its descriptor array, and all pointed-to static
        // strings remain alive for the complete exclusive handle use.
        let mut array = unsafe { GitMessageTrailerArrayMut::from_ptr(&raw mut raw) }
            .expect("the address of a local array is non-null");
        assert_eq!(array.as_ref().len(), 2);
        assert!(array.as_ref().has_trailer_block());
        let shared = array.as_ref().trailers().unwrap();
        assert_eq!(shared.get(0).unwrap().key(), c"Signed-off-by");
        assert_eq!(shared.get(1).unwrap().value(), c"R. Viewer");

        let mut exclusive = array.trailers_mut().unwrap();
        // SAFETY: the replacement string has static storage duration.
        unsafe {
            exclusive
                .get_mut(1)
                .expect("second descriptor exists")
                .set_value(c"Second Reviewer");
        }
        assert_eq!(exclusive.get(1).unwrap().value(), c"Second Reviewer");
    }
}

/// Wraps: git_message_prettify
/// Cleans whitespace and optionally removes comment lines.
pub fn git_message_prettify(
    output: &mut GitBufMut<'_>,
    message: &CStr,
    strip_comments: bool,
    comment_char: core::ffi::c_char,
) -> Result<(), i32> {
    // SAFETY: both handles address live values, `message` is NUL terminated,
    // and libgit2 retains none of these pointers.
    let status = unsafe {
        ffi::git_message_prettify(
            output.as_mut_ptr(),
            message.as_ptr(),
            i32::from(strip_comments),
            comment_char,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}
