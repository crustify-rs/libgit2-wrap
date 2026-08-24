//! Safe wrappers for libgit2 strarray APIs.

use core::ffi::{CStr, c_char};
use core::marker::PhantomData;
use core::ptr::NonNull;

use ffibox::CVal;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_strarray
    /// Layout-compatible header for a counted array of C strings.
    GitStrArray,
    GitStrArrayRef,
    GitStrArrayMut,
    ffi::git_strarray
);

// `git_strarray_dispose` releases owned fields and leaves the inline header
// allocated and reset, so this is a by-value lifecycle rather than `CBox`.
ffibox::impl_cvalued!(GitStrArray, ffi::git_strarray, ffi::git_strarray_dispose);

/// A borrowed view of the string-pointer run in a [`GitStrArray`].
///
/// Entries are optional because libgit2's copy routine explicitly accepts
/// null entries. The view copies pointer values out of the C-owned run rather
/// than forming a Rust slice over storage that C may retain and mutate.
#[derive(Clone, Copy)]
pub struct GitStrArrayStrings<'a> {
    ptr: NonNull<*mut c_char>,
    len: usize,
    _borrow: PhantomData<&'a CStr>,
}

impl GitStrArrayStrings<'_> {
    /// Number of pointer slots in the run.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the run contains no pointer slots.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<'a> GitStrArrayStrings<'a> {
    /// Borrow entry `index`, or return `None` for an out-of-range or null
    /// entry.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&'a CStr> {
        if index >= self.len {
            return None;
        }

        // SAFETY: `index < len`, and a valid `git_strarray` guarantees a live
        // initialized pointer slot for every index in the counted run. Reading
        // copies the pointer without forming a reference to C-visible storage.
        let string = unsafe { self.ptr.as_ptr().add(index).read() };
        if string.is_null() {
            return None;
        }

        // SAFETY: each non-null entry in a valid `git_strarray` points to a
        // NUL-terminated byte string that lives for the array borrow. Shared
        // access does not permit mutation through this view.
        Some(unsafe { CStr::from_ptr(string) })
    }
}

impl GitStrArray {
    /// Construct an empty inline owner. Its fields are disposed on drop.
    #[must_use]
    pub fn new() -> CVal<Self> {
        CVal::new(Self::zeroed())
    }
}

impl<'a> GitStrArrayRef<'a> {
    /// Field: git_strarray.strings
    /// Borrow the counted pointer run without exposing its raw pointer.
    ///
    /// `None` denotes a null outer pointer. This includes the canonical empty
    /// representation and also prevents malformed null/nonzero headers from
    /// producing a view.
    #[must_use]
    pub fn strings(&self) -> Option<GitStrArrayStrings<'a>> {
        let header = self.as_ptr();
        // SAFETY: both fields are initialized members of the live header and
        // are copied through raw-place projections.
        let (strings, count) = unsafe {
            (
                core::ptr::addr_of!((*header).strings).read(),
                core::ptr::addr_of!((*header).count).read(),
            )
        };

        Some(GitStrArrayStrings {
            ptr: NonNull::new(strings)?,
            len: count,
            _borrow: PhantomData,
        })
    }

    /// Field: git_strarray.count
    /// Number of string-pointer slots recorded by the header.
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        let header = self.as_ptr();
        // SAFETY: `header` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*header).count).read() }
    }

    /// Deep-copy this array into a new inline owner.
    ///
    /// Libgit2 compacts null source entries in the resulting array. The error
    /// value is the negative status returned when allocation fails.
    pub fn try_to_owned(&self) -> Result<CVal<GitStrArray>, i32> {
        let mut copy = GitStrArray::new();
        let status = {
            let mut target = copy.as_mut();
            // SAFETY: `target` is an empty, exclusively borrowed output header
            // and `self` is a live shared source. Libgit2 either initializes a
            // fully owned deep copy or resets the target to empty on failure.
            unsafe { ffi::git_strarray_copy(target.as_mut_ptr(), self.as_ptr()) }
        };

        if status == 0 { Ok(copy) } else { Err(status) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<GitStrArray>(), size_of::<ffi::git_strarray>());
        assert_eq!(align_of::<GitStrArray>(), align_of::<ffi::git_strarray>());
    }

    #[test]
    fn empty_owner_has_the_public_empty_state() {
        let array = GitStrArray::new();
        let view = array.as_ref();
        assert_eq!(view.count(), 0);
        assert!(view.strings().is_none());
    }

    #[test]
    fn borrowed_entries_can_be_viewed_and_deep_copied() {
        // SAFETY: libgit2 initialization is refcounted and the successful call
        // is balanced below after all configured allocations are dropped.
        let init_count = unsafe { ffi::git_libgit2_init() };
        assert!(init_count > 0);

        let mut entries = [
            c"alpha".as_ptr().cast_mut(),
            core::ptr::null_mut(),
            c"beta".as_ptr().cast_mut(),
        ];
        let mut raw = ffi::git_strarray {
            strings: entries.as_mut_ptr(),
            count: entries.len(),
        };
        // SAFETY: `raw` and its pointer run remain live for `borrowed`; each
        // non-null entry is a static NUL-terminated string. This is shared-only
        // despite the C declaration's historical mutable pointer spelling.
        let borrowed = unsafe { GitStrArrayRef::from_ptr(core::ptr::addr_of_mut!(raw)) }
            .expect("the stack header is non-null");

        assert_eq!(borrowed.count(), 3);
        let strings = borrowed.strings().expect("the pointer run is non-null");
        assert_eq!(strings.len(), 3);
        assert_eq!(strings.get(0), Some(c"alpha"));
        assert_eq!(strings.get(1), None);
        assert_eq!(strings.get(2), Some(c"beta"));
        assert_eq!(strings.get(3), None);

        let owned = borrowed
            .try_to_owned()
            .expect("libgit2 should allocate the deep copy");
        let owned_view = owned.as_ref();
        assert_eq!(owned_view.count(), 2);
        let owned_strings = owned_view.strings().unwrap();
        assert_eq!(owned_strings.get(0), Some(c"alpha"));
        assert_eq!(owned_strings.get(1), Some(c"beta"));
        drop(owned);

        // SAFETY: balances this test's successful initialization after the
        // copied strings and pointer array have been disposed.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
    }
}
