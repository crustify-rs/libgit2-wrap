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
