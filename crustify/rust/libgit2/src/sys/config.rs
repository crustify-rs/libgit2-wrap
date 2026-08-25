//! Safe wrappers for libgit2 config APIs.

use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CBox, CDropped};

use crate::config::{GitConfigEntryMut, GitConfigEntryRef};
use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_config_backend_entry
    /// Layout-compatible entry supplied by a custom configuration backend.
    GitConfigBackendEntry,
    GitConfigBackendEntryRef,
    GitConfigBackendEntryMut,
    ffi::git_config_backend_entry
);

/// An owned backend entry whose concrete finalizer is stored in the entry.
pub type GitConfigBackendEntryOwned = CBox<GitConfigBackendEntry>;

/// Field: git_config_backend_entry.free
// SAFETY: adopting an owned backend entry requires a fully initialized
// concrete entry whose mandatory callback finalizes that allocation exactly
// once. `CBox` invokes it once and never accesses the entry afterward.
unsafe impl CDropped for GitConfigBackendEntry {
    unsafe fn c_drop(entry: NonNull<Self>) {
        let entry = entry.as_ptr().cast::<ffi::git_config_backend_entry>();
        // SAFETY: the lifecycle contract supplies a live fully initialized
        // entry; raw-place projection reads its callback without forming a
        // reference to the C-visible allocation.
        let free = unsafe { addr_of!((*entry).free).read() }
            .expect("a valid backend entry has a free callback");
        // SAFETY: this is the concrete finalizer installed in the uniquely
        // owned entry, and the `CDropped` contract grants its final call.
        unsafe { free(entry) }
    }
}

impl<'a> GitConfigBackendEntryRef<'a> {
    /// Field: git_config_backend_entry.entry
    /// Borrows the embedded public configuration entry.
    #[must_use]
    pub fn entry(&self) -> GitConfigEntryRef<'a> {
        // SAFETY: raw-place projection reaches the initialized first member
        // without forming a reference over C-visible storage.
        let entry = unsafe { addr_of!((*self.as_ptr()).entry) }.cast_mut();
        // SAFETY: an embedded member is non-null and remains live for the
        // backend entry handle's complete `'a` borrow.
        unsafe { GitConfigEntryRef::from_ptr(entry) }
            .expect("an embedded configuration entry is non-null")
    }
}

impl GitConfigBackendEntryMut<'_> {
    /// Borrows the embedded public configuration entry exclusively.
    #[must_use]
    pub fn entry_mut(&mut self) -> GitConfigEntryMut<'_> {
        // SAFETY: the projected member is initialized, and its exclusive
        // handle is tied to this handle's exclusive reborrow.
        unsafe { GitConfigEntryMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).entry)) }
            .expect("an embedded configuration entry is non-null")
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use ffibox::CCell;

    use super::*;

    static FREES: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn free_test_entry(entry: *mut ffi::git_config_backend_entry) {
        FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the test transfers exactly one `Box` allocation to the
        // owner, whose one final callback returns that allocation here.
        drop(unsafe { Box::from_raw(entry) });
    }

    fn raw_entry() -> ffi::git_config_backend_entry {
        ffi::git_config_backend_entry {
            entry: ffi::git_config_entry {
                name: c"core.bare".as_ptr(),
                value: c"true".as_ptr(),
                backend_type: c"test".as_ptr(),
                origin_path: core::ptr::null(),
                include_depth: 0,
                level: crate::config::GitConfigLevel::LOCAL.as_raw(),
            },
            free: Some(free_test_entry),
        }
    }

    #[test]
    fn backend_entry_matches_the_c_layout_and_owner_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitConfigBackendEntry>();
        assert_dropped::<GitConfigBackendEntry>();
        assert_eq!(
            size_of::<GitConfigBackendEntry>(),
            size_of::<ffi::git_config_backend_entry>()
        );
        assert_eq!(
            align_of::<GitConfigBackendEntry>(),
            align_of::<ffi::git_config_backend_entry>()
        );
        assert_eq!(
            size_of::<GitConfigBackendEntryRef<'_>>(),
            size_of::<*const ffi::git_config_backend_entry>()
        );
        assert_eq!(
            size_of::<GitConfigBackendEntryOwned>(),
            size_of::<*mut ffi::git_config_backend_entry>()
        );
    }

    #[test]
    fn backend_entry_projects_the_public_entry() {
        let mut raw = raw_entry();
        // SAFETY: `raw` and its static strings remain live, and this is the
        // only handle addressing it during the test.
        let mut backend = unsafe { GitConfigBackendEntryMut::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(backend.as_ref().entry().name(), c"core.bare");
        assert_eq!(backend.entry_mut().as_ref().value(), Some(c"true"));
    }

    #[test]
    fn owned_backend_entry_dispatches_its_concrete_finalizer_once() {
        FREES.store(0, Ordering::SeqCst);
        let raw = Box::into_raw(Box::new(raw_entry()));
        // SAFETY: `raw` is a fresh fully initialized allocation whose callback
        // reclaims exactly this `Box` when the owner drops.
        let owned = unsafe { GitConfigBackendEntryOwned::from_raw(raw) }.unwrap();
        assert_eq!(owned.as_ref().entry().backend_type(), c"test");
        drop(owned);
        assert_eq!(FREES.load(Ordering::SeqCst), 1);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_config_iterator
    /// A layout-compatible polymorphic configuration iterator.
    ///
    /// Advancing requires an exclusive borrowed handle because each concrete
    /// backend may invalidate its previously returned entry.
    GitConfigIterator,
    GitConfigIteratorRef,
    GitConfigIteratorMut,
    ffi::git_config_iterator
);

/// An owned configuration iterator whose callback releases its concrete
/// allocation.
pub type GitConfigIteratorOwned = CBox<GitConfigIterator>;

/// Field: git_config_iterator.free
// SAFETY: every fully constructed iterator installs a concrete finalizer that
// accepts its embedded base pointer and releases the complete allocation once.
unsafe impl CDropped for GitConfigIterator {
    unsafe fn c_drop(iterator: NonNull<Self>) {
        let iterator = iterator.as_ptr().cast::<ffi::git_config_iterator>();
        // SAFETY: the lifecycle contract supplies a live, fully initialized
        // iterator, and raw-place projection reads its callback without
        // forming a reference over the C-visible allocation.
        let free = unsafe { addr_of!((*iterator).free).read() }
            .expect("a complete configuration iterator has a free callback");
        // SAFETY: the callback is the concrete finalizer paired with this
        // unique iterator allocation and this is its one terminal invocation.
        unsafe { free(iterator) }
    }
}

impl GitConfigIteratorRef<'_> {
    /// Field: git_config_iterator.flags
    /// Returns the backend-specific iterator flags.
    #[must_use]
    pub fn flags(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Field: git_config_iterator.backend
    /// Returns whether a custom iterator records its originating backend.
    ///
    /// In-tree iterators leave this optional extension slot empty. Its
    /// concrete type will gain a borrowed handle when `git_config_backend` is
    /// wrapped; no raw pointer escapes in the meantime.
    #[must_use]
    pub fn has_backend(&self) -> bool {
        // SAFETY: this live shared handle permits a raw-place pointer read.
        !unsafe { addr_of!((*self.as_ptr()).backend).read() }.is_null()
    }
}

impl GitConfigIteratorMut<'_> {
    /// Field: git_config_iterator.next
    /// Advances the iterator and borrows its current backend entry.
    ///
    /// The returned handle keeps this iterator exclusively borrowed, so safe
    /// code cannot advance again while backend-managed entry storage is used.
    pub fn next_backend_entry(&mut self) -> Result<GitConfigBackendEntryRef<'_>, i32> {
        let mut entry = core::ptr::null_mut();
        // SAFETY: this live exclusive handle permits a raw-place callback read.
        let next = unsafe { addr_of!((*self.as_mut_ptr()).next).read() }
            .expect("a complete configuration iterator has a next callback");
        // SAFETY: `entry` is a writable output slot, this exclusive handle
        // supplies the live iterator, and the callback is invoked synchronously.
        let status = unsafe { next(&mut entry, self.as_mut_ptr()) };
        if status != 0 {
            return Err(status);
        }

        // SAFETY: callback success publishes a live backend-managed entry that
        // remains valid until the next advance or iterator destruction. The
        // returned handle is tied to this exclusive reborrow, preventing both.
        unsafe { GitConfigBackendEntryRef::from_ptr(entry) }.ok_or(ffi::git_error_code_GIT_ERROR)
    }

    /// Sets the backend-specific iterator flags.
    pub fn set_flags(&mut self, flags: core::ffi::c_uint) {
        // SAFETY: this live exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }
}

#[cfg(test)]
mod iterator_tests {
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use ffibox::CCell;

    use super::*;

    static ITERATOR_FREES: AtomicUsize = AtomicUsize::new(0);

    #[repr(C)]
    struct TestIterator {
        parent: ffi::git_config_iterator,
        entry: ffi::git_config_backend_entry,
    }

    unsafe extern "C" fn next_test_entry(
        out: *mut *mut ffi::git_config_backend_entry,
        iterator: *mut ffi::git_config_iterator,
    ) -> core::ffi::c_int {
        if out.is_null() || iterator.is_null() {
            return ffi::git_error_code_GIT_ERROR;
        }
        let concrete = iterator.cast::<TestIterator>();
        // SAFETY: the callback is installed only in a `TestIterator`, whose
        // parent is its first member, and `out` is the caller's writable slot.
        unsafe { out.write(addr_of_mut!((*concrete).entry)) };
        0
    }

    unsafe extern "C" fn free_test_iterator(iterator: *mut ffi::git_config_iterator) {
        ITERATOR_FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the ownership test transfers exactly one boxed
        // `TestIterator`, whose parent is the first member, to `CBox`.
        drop(unsafe { Box::from_raw(iterator.cast::<TestIterator>()) });
    }

    unsafe extern "C" fn no_free_entry(_entry: *mut ffi::git_config_backend_entry) {}

    fn iterator_entry() -> ffi::git_config_backend_entry {
        ffi::git_config_backend_entry {
            entry: ffi::git_config_entry {
                name: c"core.bare".as_ptr(),
                value: c"true".as_ptr(),
                backend_type: c"test".as_ptr(),
                origin_path: core::ptr::null(),
                include_depth: 0,
                level: crate::config::GitConfigLevel::LOCAL.as_raw(),
            },
            free: Some(no_free_entry),
        }
    }

    fn raw_iterator() -> TestIterator {
        TestIterator {
            parent: ffi::git_config_iterator {
                backend: core::ptr::null_mut(),
                flags: 0,
                next: Some(next_test_entry),
                free: Some(free_test_iterator),
            },
            entry: iterator_entry(),
        }
    }

    #[test]
    fn iterator_matches_the_c_layout_and_owner_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitConfigIterator>();
        assert_dropped::<GitConfigIterator>();
        assert_eq!(
            size_of::<GitConfigIterator>(),
            size_of::<ffi::git_config_iterator>()
        );
        assert_eq!(
            align_of::<GitConfigIterator>(),
            align_of::<ffi::git_config_iterator>()
        );
        assert_eq!(
            size_of::<GitConfigIteratorRef<'_>>(),
            size_of::<*const ffi::git_config_iterator>()
        );
        assert_eq!(
            size_of::<Option<GitConfigIteratorOwned>>(),
            size_of::<*mut ffi::git_config_iterator>()
        );
    }

    #[test]
    fn iterator_fields_and_advance_preserve_borrows() {
        let mut raw = raw_iterator();
        // SAFETY: `raw` remains live and this is its only handle while used.
        let mut iterator =
            unsafe { GitConfigIteratorMut::from_ptr(addr_of_mut!(raw.parent)) }.unwrap();
        assert_eq!(iterator.as_ref().flags(), 0);
        assert!(!iterator.as_ref().has_backend());
        iterator.set_flags(7);
        assert_eq!(iterator.as_ref().flags(), 7);
        assert_eq!(
            iterator.next_backend_entry().unwrap().entry().name(),
            c"core.bare"
        );
    }

    #[test]
    fn owned_iterator_dispatches_its_concrete_finalizer_once() {
        ITERATOR_FREES.store(0, Ordering::SeqCst);
        let raw = Box::into_raw(Box::new(raw_iterator()));
        // SAFETY: the parent is the first member of this fresh, fully
        // initialized allocation and its callback reclaims that allocation.
        let owned =
            unsafe { GitConfigIteratorOwned::from_raw(addr_of_mut!((*raw).parent)) }.unwrap();
        drop(owned);
        assert_eq!(ITERATOR_FREES.load(Ordering::SeqCst), 1);
    }
}
