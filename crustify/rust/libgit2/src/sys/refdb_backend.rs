//! Safe wrappers for libgit2 refdb_backend APIs.

use core::ffi::CStr;
use core::ptr::{NonNull, addr_of, null_mut};

use ffibox::{CBox, CDropped, define_ctype};

use crate::ffi;
use crate::refdb::GitRefdbRef;
use crate::refs::{GitReference, GitReferenceOwned};

define_ctype!(
    /// Wraps: git_reference_iterator
    /// A stateful iterator supplied by a reference database backend.
    ///
    /// Owned iterators use [`GitReferenceIteratorOwned`]. Each iterator owns
    /// one reference to its database and advances only through an exclusive
    /// borrowed handle.
    GitReferenceIterator,
    GitReferenceIteratorRef,
    GitReferenceIteratorMut,
    ffi::git_reference_iterator
);

/// An owned reference iterator.
pub type GitReferenceIteratorOwned = CBox<GitReferenceIterator>;

/// Wraps: git_reference_iterator.free
// SAFETY: `git_reference_iterator_free` is the public destructor for a fully
// constructed iterator. It releases the iterator's database reference and
// invokes the backend's destructor exactly once. It accepts null, although
// `CBox` supplies a live non-null allocation.
unsafe impl CDropped for GitReferenceIterator {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the trait contract supplies one live, fully constructed
        // iterator owned by this handle, and the transparent wrapper has the
        // corresponding C layout.
        unsafe { ffi::git_reference_iterator_free(obj.as_ptr().cast()) }
    }
}

impl<'a> GitReferenceIteratorRef<'a> {
    /// Wraps: git_reference_iterator.db
    /// Borrows the reference database retained by this iterator.
    #[must_use]
    pub fn db(&self) -> GitRefdbRef<'a> {
        // SAFETY: raw-place projection reads the initialized, non-null
        // database pointer from a fully constructed iterator without forming
        // a reference over C-visible storage. The iterator owns a database
        // count for at least this handle's `'a` lifetime.
        let db = unsafe { addr_of!((*self.as_ptr()).db).read() };
        // SAFETY: the construction and destruction contract above keeps `db`
        // live and non-null for this iterator's lifetime.
        unsafe { GitRefdbRef::from_ptr(db) }.expect("a complete iterator retains a refdb")
    }
}

impl GitReferenceIteratorMut<'_> {
    /// Wraps: git_reference_iterator.next
    /// Advances the iterator and returns the next independently owned
    /// reference.
    pub fn next_reference(&mut self) -> Result<GitReferenceOwned, i32> {
        let mut reference = null_mut();
        // SAFETY: `reference` is a writable output slot and the exclusive
        // handle supplies a live iterator. The public helper invokes the
        // backend callback and attaches an owned database count to its result.
        let status = unsafe { ffi::git_reference_next(&mut reference, self.as_mut_ptr()) };
        if status != 0 {
            return Err(status);
        }

        // SAFETY: success from `git_reference_next` initializes `reference`
        // with a fresh, fully constructed reference owned by the caller.
        unsafe { CBox::<GitReference>::from_raw(reference) }.ok_or(-1)
    }

    /// Wraps: git_reference_iterator.next_name
    /// Advances the iterator and borrows the next reference name.
    ///
    /// The returned name keeps this mutable handle borrowed, so the iterator
    /// cannot advance again while backend-owned scratch storage is observed.
    pub fn next_name(&mut self) -> Result<&CStr, i32> {
        let mut name = core::ptr::null();
        // SAFETY: `name` is a writable output slot and this exclusive handle
        // supplies a live iterator. The callback retains the returned string.
        let status = unsafe { ffi::git_reference_next_name(&mut name, self.as_mut_ptr()) };
        if status != 0 {
            return Err(status);
        }
        if name.is_null() {
            return Err(-1);
        }

        // SAFETY: on success the callback returns a NUL-terminated string that
        // stays live until at least the next iterator advance. The result is
        // tied to this exclusive reborrow, preventing such an advance.
        Ok(unsafe { CStr::from_ptr(name) })
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    unsafe extern "C" fn no_next(
        _reference: *mut *mut ffi::git_reference,
        _iterator: *mut ffi::git_reference_iterator,
    ) -> i32 {
        ffi::git_error_code_GIT_ITEROVER
    }

    unsafe extern "C" fn static_next_name(
        name: *mut *const core::ffi::c_char,
        _iterator: *mut ffi::git_reference_iterator,
    ) -> i32 {
        if name.is_null() {
            return -1;
        }
        // SAFETY: the caller supplies the required writable output slot, and
        // the static C string remains valid indefinitely.
        unsafe { name.write(c"refs/heads/main".as_ptr()) };
        0
    }

    unsafe extern "C" fn no_free(_iterator: *mut ffi::git_reference_iterator) {}

    #[test]
    fn iterator_matches_the_c_layout_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitReferenceIterator>();
        assert_dropped::<GitReferenceIterator>();
        assert_eq!(
            size_of::<GitReferenceIterator>(),
            size_of::<ffi::git_reference_iterator>()
        );
        assert_eq!(
            align_of::<GitReferenceIterator>(),
            align_of::<ffi::git_reference_iterator>()
        );
        assert_eq!(
            size_of::<GitReferenceIteratorRef<'_>>(),
            size_of::<*const ffi::git_reference_iterator>()
        );
        assert_eq!(
            size_of::<GitReferenceIteratorMut<'_>>(),
            size_of::<*mut ffi::git_reference_iterator>()
        );
        assert_eq!(
            size_of::<Option<GitReferenceIteratorOwned>>(),
            size_of::<*mut ffi::git_reference_iterator>()
        );
    }

    #[test]
    fn null_iterator_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitReferenceIteratorRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitReferenceIteratorMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitReferenceIteratorOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn database_getter_returns_a_lifetime_bound_handle() {
        let mut db = MaybeUninit::<ffi::git_refdb>::zeroed();
        let mut raw = ffi::git_reference_iterator {
            db: db.as_mut_ptr(),
            next: Some(no_next),
            next_name: Some(static_next_name),
            free: Some(no_free),
        };

        // SAFETY: `raw` and `db` remain live for this scope and only a shared
        // handle observes the fully initialized iterator record.
        let iterator = unsafe { GitReferenceIteratorRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(iterator.db().as_ptr(), db.as_ptr());
    }

    #[test]
    fn iterator_operations_dispatch_callbacks_without_exposing_raw_slots() {
        let mut db = MaybeUninit::<ffi::git_refdb>::zeroed();
        let mut raw = ffi::git_reference_iterator {
            db: db.as_mut_ptr(),
            next: Some(no_next),
            next_name: Some(static_next_name),
            free: Some(no_free),
        };

        // SAFETY: `raw`, its database storage, and its static callbacks remain
        // live and are exclusively borrowed by this handle.
        let mut iterator = unsafe { GitReferenceIteratorMut::from_ptr(&raw mut raw) }.unwrap();
        assert!(matches!(
            iterator.next_reference(),
            Err(ffi::git_error_code_GIT_ITEROVER)
        ));
        assert_eq!(iterator.next_name(), Ok(c"refs/heads/main"));
    }
}
