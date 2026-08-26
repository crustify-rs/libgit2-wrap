//! Safe wrappers for libgit2 config APIs.

use core::ffi::CStr;
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
///
/// The finalizer releases the caller's claim on the entry rather than
/// necessarily freeing its storage: every in-tree backend installs
/// `git_config_list_entry_free`, which drops one reference on the
/// configuration list the entry is embedded in.
pub type GitConfigBackendEntryOwned = CBox<GitConfigBackendEntry>;

/// Field: git_config_backend_entry.free
// SAFETY: adopting an owned backend entry requires a fully initialized
// concrete entry whose mandatory callback releases that entry exactly once.
// `CBox` invokes it once and never accesses the entry afterward.
unsafe impl CDropped for GitConfigBackendEntry {
    unsafe fn c_drop(entry: NonNull<Self>) {
        let entry = entry.as_ptr().cast::<ffi::git_config_backend_entry>();
        // SAFETY: the lifecycle contract supplies a live fully initialized
        // entry; raw-place projection reads its callback without forming a
        // reference to the C-visible allocation.
        let free = unsafe { addr_of!((*entry).free).read() }
            .expect("a valid backend entry has a free callback");
        // SAFETY: this is the concrete finalizer installed in the uniquely
        // claimed entry, and the `CDropped` contract grants its final call.
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
    /// Borrows the originating backend when a custom iterator records one.
    #[must_use]
    pub fn backend(&self) -> Option<GitConfigBackendRef<'_>> {
        // SAFETY: this live shared handle permits a raw-place pointer read.
        let backend = unsafe { addr_of!((*self.as_ptr()).backend).read() };
        // SAFETY: a non-null extension slot points to the backend that owns or
        // otherwise outlives this iterator, and the result is tied to the
        // iterator borrow.
        unsafe { GitConfigBackendRef::from_ptr(backend) }
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
        assert!(iterator.as_ref().backend().is_none());
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

ffibox::define_ctype!(
    /// Wraps: git_config_backend
    /// A layout-compatible polymorphic configuration backend.
    ///
    /// Safe methods dispatch the concrete backend's callback table without
    /// exposing its raw pointer contracts. An owned handle represents a
    /// backend not yet transferred to a [`crate::config::GitConfig`].
    GitConfigBackend,
    GitConfigBackendRef,
    GitConfigBackendMut,
    ffi::git_config_backend
);

/// A uniquely owned configuration backend allocation.
pub type GitConfigBackendOwned = CBox<GitConfigBackend>;

/// A backend operation that a concrete implementation did not install.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitConfigBackendOperation {
    /// Open and parse the backing store.
    Open,
    /// Look up one entry.
    Get,
    /// Replace one entry.
    Set,
    /// Replace all matching values.
    SetMultivar,
    /// Delete one entry.
    Delete,
    /// Delete all matching values.
    DeleteMultivar,
    /// Create an iterator.
    Iterator,
    /// Create a read-only snapshot.
    Snapshot,
    /// Lock the backing store.
    Lock,
    /// Commit or roll back and unlock the backing store.
    Unlock,
}

/// Failure while dispatching a configuration-backend callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitConfigBackendError {
    /// The concrete backend omitted the requested callback.
    Unsupported(GitConfigBackendOperation),
    /// The callback returned a libgit2 status code.
    Libgit2(i32),
    /// A successful callback did not initialize its required output.
    MissingOutput(GitConfigBackendOperation),
}

/// An owned iterator tethered to the backend recorded in its extension slot.
pub struct GitConfigBackendIteratorOwned<'backend> {
    inner: GitConfigIteratorOwned,
    _backend: core::marker::PhantomData<GitConfigBackendRef<'backend>>,
}

impl GitConfigBackendIteratorOwned<'_> {
    /// Borrows the iterator without advancing it.
    #[must_use]
    pub fn as_ref(&self) -> GitConfigIteratorRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the iterator exclusively for advancing it.
    #[must_use]
    pub fn as_mut(&mut self) -> GitConfigIteratorMut<'_> {
        self.inner.as_mut()
    }
}

/// An owned snapshot backend tethered to the source backend it reads.
///
/// `git_config_backend_snapshot` stores the source backend pointer in the
/// snapshot without taking ownership of it, and `config_snapshot_open`
/// dereferences that pointer to copy every entry across. The snapshot is
/// therefore a borrower of its source until it has been opened, and outliving
/// that source is rejected:
///
/// ```compile_fail
/// use libgit2::sys::config::{GitConfigBackendOwned, GitConfigBackendSnapshotOwned};
///
/// fn escape(mut backend: GitConfigBackendOwned) -> GitConfigBackendSnapshotOwned<'static> {
///     backend.as_mut().snapshot().unwrap()
/// }
/// ```
pub struct GitConfigBackendSnapshotOwned<'source> {
    inner: GitConfigBackendOwned,
    _source: core::marker::PhantomData<GitConfigBackendRef<'source>>,
}

impl GitConfigBackendSnapshotOwned<'_> {
    /// Borrows the snapshot backend.
    #[must_use]
    pub fn as_ref(&self) -> GitConfigBackendRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the snapshot backend exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitConfigBackendMut<'_> {
        self.inner.as_mut()
    }

    /// Releases the tether to the source backend.
    ///
    /// # Safety
    /// Only the snapshot's `open` callback reads the stored source pointer,
    /// and it copies out every entry there, so a snapshot that has already
    /// been opened is independent of its source. The caller must guarantee
    /// that `open` has completed and that no path opens this backend again --
    /// including the `open` that `git_config_add_backend` performs when the
    /// snapshot is handed to a configuration.
    pub unsafe fn into_owned(self) -> GitConfigBackendOwned {
        self.inner
    }
}

/// Field: git_config_backend.free
// SAFETY: every fully constructed backend installs a concrete finalizer that
// accepts the embedded base pointer and releases the complete allocation once.
unsafe impl CDropped for GitConfigBackend {
    unsafe fn c_drop(backend: NonNull<Self>) {
        let backend = backend.as_ptr().cast::<ffi::git_config_backend>();
        // SAFETY: the lifecycle contract supplies a live initialized backend;
        // raw-place projection reads its callback without forming a reference.
        let free = unsafe { addr_of!((*backend).free).read() }
            .expect("a complete configuration backend has a free callback");
        // SAFETY: the callback is the finalizer paired with this uniquely
        // owned concrete backend allocation.
        unsafe { free(backend) }
    }
}

impl<'a> GitConfigBackendRef<'a> {
    /// Field: git_config_backend.version
    /// Returns the public backend ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_config_backend.cfg
    /// Borrows the configuration this backend was last attached to.
    ///
    /// # Safety
    /// libgit2 writes this back-reference exactly once, in
    /// `git_config_add_backend`, and never clears or rewrites it. It does not
    /// track the backend: `git_config_open_level` publishes the same
    /// reference-counted backend instance through a second `git_config`
    /// without updating `cfg`, so freeing the original configuration leaves a
    /// live backend naming destroyed storage. The caller must therefore
    /// establish out of band that the recorded configuration is still live and
    /// stays live for `'a`.
    #[must_use]
    pub unsafe fn config(&self) -> Option<crate::config::GitConfigRef<'a>> {
        // SAFETY: raw-place projection copies the nullable back-reference.
        let config = unsafe { addr_of!((*self.as_ptr()).cfg).read() };
        // SAFETY: the caller of this unsafe getter guarantees that a recorded
        // configuration is live for `'a`.
        unsafe { crate::config::GitConfigRef::from_ptr(config) }
    }

    /// Field: git_config_backend.readonly
    /// Returns whether the backend rejects writes as a snapshot.
    #[must_use]
    pub fn is_readonly(&self) -> bool {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).readonly).read() != 0 }
    }
}

impl GitConfigBackendMut<'_> {
    fn callback<T: Copy>(
        &self,
        field: *const Option<T>,
        operation: GitConfigBackendOperation,
    ) -> Result<T, GitConfigBackendError> {
        // SAFETY: every caller projects `field` from this live backend and
        // names the exact initialized callback slot.
        unsafe { field.read() }.ok_or(GitConfigBackendError::Unsupported(operation))
    }

    /// Replaces the public backend ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this live exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Replaces the read-only marker.
    pub fn set_readonly(&mut self, readonly: bool) {
        // SAFETY: this live exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).readonly).write(i32::from(readonly)) }
    }

    /// Field: git_config_backend.open
    /// Opens the backend at `level`, optionally relative to a repository.
    ///
    /// # Safety
    /// A concrete backend may retain `repository` for later refreshes (the
    /// built-in file backend does). When present, it must therefore remain
    /// live until this backend is destroyed or opened again with another
    /// repository.
    ///
    /// Opening is also the one callback that reads a snapshot backend's stored
    /// source pointer, so a backend produced by [`Self::snapshot`] may only be
    /// opened while its source is still live.
    pub unsafe fn open(
        &mut self,
        level: crate::config::GitConfigLevel,
        repository: Option<crate::repository::GitRepositoryRef<'_>>,
    ) -> Result<(), GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).open) },
            GitConfigBackendOperation::Open,
        )?;
        let repository = repository.map_or(core::ptr::null(), |repo| repo.as_ptr());
        // SAFETY: all arguments are live for this synchronous call and the
        // exclusive handle grants backend mutation.
        let status = unsafe { callback(backend, level.as_raw(), repository) };
        Self::status(status)
    }

    /// Field: git_config_backend.get
    /// Looks up one entry and takes the caller's independently releasable claim.
    pub fn get(&mut self, key: &CStr) -> Result<GitConfigBackendEntryOwned, GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).get) },
            GitConfigBackendOperation::Get,
        )?;
        let mut entry = core::ptr::null_mut();
        // SAFETY: `entry` is writable, `key` is a live C string, and this
        // exclusive handle supplies the live backend.
        let status = unsafe { callback(backend, key.as_ptr(), &mut entry) };
        if status != 0 {
            return Err(GitConfigBackendError::Libgit2(status));
        }
        // SAFETY: success transfers one releasable entry claim to the caller.
        unsafe { GitConfigBackendEntryOwned::from_raw(entry) }.ok_or(
            GitConfigBackendError::MissingOutput(GitConfigBackendOperation::Get),
        )
    }

    /// Field: git_config_backend.set
    /// Replaces one string value.
    pub fn set(&mut self, key: &CStr, value: &CStr) -> Result<(), GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).set) },
            GitConfigBackendOperation::Set,
        )?;
        // SAFETY: both strings and the exclusively borrowed backend remain
        // live for this synchronous callback.
        Self::status(unsafe { callback(backend, key.as_ptr(), value.as_ptr()) })
    }

    /// Field: git_config_backend.set_multivar
    /// Replaces values matching `regexp`.
    pub fn set_multivar(
        &mut self,
        name: &CStr,
        regexp: &CStr,
        value: &CStr,
    ) -> Result<(), GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).set_multivar) },
            GitConfigBackendOperation::SetMultivar,
        )?;
        // SAFETY: all strings and the backend remain live synchronously.
        Self::status(unsafe { callback(backend, name.as_ptr(), regexp.as_ptr(), value.as_ptr()) })
    }

    /// Field: git_config_backend.del
    /// Deletes one entry.
    pub fn delete(&mut self, key: &CStr) -> Result<(), GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).del) },
            GitConfigBackendOperation::Delete,
        )?;
        // SAFETY: `key` and the backend remain live synchronously.
        Self::status(unsafe { callback(backend, key.as_ptr()) })
    }

    /// Field: git_config_backend.del_multivar
    /// Deletes values matching `regexp`.
    pub fn delete_multivar(
        &mut self,
        key: &CStr,
        regexp: &CStr,
    ) -> Result<(), GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).del_multivar) },
            GitConfigBackendOperation::DeleteMultivar,
        )?;
        // SAFETY: both strings and the backend remain live synchronously.
        Self::status(unsafe { callback(backend, key.as_ptr(), regexp.as_ptr()) })
    }

    /// Field: git_config_backend.iterator
    /// Creates an iterator tied to this backend's lifetime.
    pub fn iterator(&mut self) -> Result<GitConfigBackendIteratorOwned<'_>, GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).iterator) },
            GitConfigBackendOperation::Iterator,
        )?;
        let mut iterator = core::ptr::null_mut();
        // SAFETY: `iterator` is writable and the backend remains exclusively
        // borrowed for the returned tether's lifetime.
        let status = unsafe { callback(&mut iterator, backend) };
        if status != 0 {
            return Err(GitConfigBackendError::Libgit2(status));
        }
        // SAFETY: success transfers one complete iterator allocation.
        let inner = unsafe { GitConfigIteratorOwned::from_raw(iterator) }.ok_or(
            GitConfigBackendError::MissingOutput(GitConfigBackendOperation::Iterator),
        )?;
        Ok(GitConfigBackendIteratorOwned {
            inner,
            _backend: core::marker::PhantomData,
        })
    }

    /// Field: git_config_backend.snapshot
    /// Creates a read-only backend snapshot that borrows this backend.
    ///
    /// The snapshot is not yet independent: `git_config_backend_snapshot`
    /// records this backend's address in it and its `open` callback calls back
    /// into `self` to copy the entries out. The result therefore keeps this
    /// backend exclusively borrowed until
    /// [`GitConfigBackendSnapshotOwned::into_owned`] discharges that
    /// obligation.
    pub fn snapshot(
        &mut self,
    ) -> Result<GitConfigBackendSnapshotOwned<'_>, GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).snapshot) },
            GitConfigBackendOperation::Snapshot,
        )?;
        let mut snapshot = core::ptr::null_mut();
        // SAFETY: `snapshot` is writable and this handle supplies the live
        // source backend for the synchronous copy.
        let status = unsafe { callback(&mut snapshot, backend) };
        if status != 0 {
            return Err(GitConfigBackendError::Libgit2(status));
        }
        // SAFETY: success transfers one fully constructed backend allocation.
        let inner = unsafe { GitConfigBackendOwned::from_raw(snapshot) }.ok_or(
            GitConfigBackendError::MissingOutput(GitConfigBackendOperation::Snapshot),
        )?;
        Ok(GitConfigBackendSnapshotOwned {
            inner,
            _source: core::marker::PhantomData,
        })
    }

    /// Field: git_config_backend.lock
    /// Locks the backend's backing store against concurrent writers.
    pub fn lock(&mut self) -> Result<(), GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).lock) },
            GitConfigBackendOperation::Lock,
        )?;
        // SAFETY: this exclusive handle supplies the live backend.
        Self::status(unsafe { callback(backend) })
    }

    /// Field: git_config_backend.unlock
    /// Unlocks the backend, committing when `success` is true and rolling back
    /// otherwise.
    pub fn unlock(&mut self, success: bool) -> Result<(), GitConfigBackendError> {
        let backend = self.as_mut_ptr();
        let callback = self.callback(
            // SAFETY: the raw-place projection is derived from this handle.
            unsafe { addr_of!((*backend).unlock) },
            GitConfigBackendOperation::Unlock,
        )?;
        // SAFETY: this exclusive handle supplies the live backend.
        Self::status(unsafe { callback(backend, i32::from(success)) })
    }

    fn status(status: i32) -> Result<(), GitConfigBackendError> {
        if status == 0 {
            Ok(())
        } else {
            Err(GitConfigBackendError::Libgit2(status))
        }
    }
}

#[cfg(test)]
mod backend_tests {
    use core::mem::{align_of, size_of};
    use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

    use ffibox::{CCell, CDropped};

    use super::*;

    static BACKEND_FREES: AtomicUsize = AtomicUsize::new(0);
    static SNAPSHOT_FREES: AtomicUsize = AtomicUsize::new(0);
    static SNAPSHOT_SOURCE: AtomicPtr<ffi::git_config_backend> =
        AtomicPtr::new(core::ptr::null_mut());

    /// Mirrors `config_snapshot_backend`: the snapshot's own header followed
    /// by the borrowed source pointer that `git_config_backend_snapshot`
    /// records.
    #[repr(C)]
    struct TestSnapshot {
        parent: ffi::git_config_backend,
        source: *mut ffi::git_config_backend,
    }

    unsafe extern "C" fn free_snapshot(backend: *mut ffi::git_config_backend) {
        SNAPSHOT_FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: `snapshot_backend` is the only producer of this callback's
        // backends, and the parent header is its allocation's first member.
        drop(unsafe { Box::from_raw(backend.cast::<TestSnapshot>()) });
    }

    unsafe extern "C" fn snapshot_backend(
        out: *mut *mut ffi::git_config_backend,
        source: *mut ffi::git_config_backend,
    ) -> i32 {
        let mut parent = raw_backend();
        parent.readonly = 1;
        parent.free = Some(free_snapshot);
        let snapshot = Box::into_raw(Box::new(TestSnapshot { parent, source }));
        SNAPSHOT_SOURCE.store(source, Ordering::SeqCst);
        // SAFETY: the callback contract supplies a writable output slot, and
        // the parent header is the first member of the fresh allocation.
        unsafe { out.write(addr_of_mut!((*snapshot).parent)) };
        0
    }

    unsafe extern "C" fn free_backend(backend: *mut ffi::git_config_backend) {
        BACKEND_FREES.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the test transfers exactly this fresh Box allocation to its
        // one backend owner.
        drop(unsafe { Box::from_raw(backend) });
    }

    unsafe extern "C" fn lock_backend(backend: *mut ffi::git_config_backend) -> i32 {
        // SAFETY: the callback contract supplies the live backend exclusively.
        unsafe { addr_of_mut!((*backend).readonly).write(1) };
        0
    }

    unsafe extern "C" fn unlock_backend(
        backend: *mut ffi::git_config_backend,
        success: i32,
    ) -> i32 {
        // SAFETY: the callback contract supplies the live backend exclusively.
        unsafe { addr_of_mut!((*backend).readonly).write(0) };
        if success != 0 { 0 } else { -7 }
    }

    fn raw_backend() -> ffi::git_config_backend {
        ffi::git_config_backend {
            version: ffi::GIT_CONFIG_BACKEND_VERSION,
            readonly: 0,
            cfg: core::ptr::null_mut(),
            open: None,
            get: None,
            set: None,
            set_multivar: None,
            del: None,
            del_multivar: None,
            iterator: None,
            snapshot: None,
            lock: Some(lock_backend),
            unlock: Some(unlock_backend),
            free: Some(free_backend),
        }
    }

    #[test]
    fn backend_matches_the_c_layout_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitConfigBackend>();
        assert_dropped::<GitConfigBackend>();
        assert_eq!(
            size_of::<GitConfigBackend>(),
            size_of::<ffi::git_config_backend>()
        );
        assert_eq!(
            align_of::<GitConfigBackend>(),
            align_of::<ffi::git_config_backend>()
        );
        assert_eq!(
            size_of::<GitConfigBackendRef<'_>>(),
            size_of::<*const ffi::git_config_backend>()
        );
        assert_eq!(
            size_of::<GitConfigBackendOwned>(),
            size_of::<*mut ffi::git_config_backend>()
        );
    }

    #[test]
    fn scalar_fields_and_missing_callbacks_are_safe() {
        let mut raw = raw_backend();
        // SAFETY: `raw` remains live and is exclusively accessed by this handle.
        let mut backend = unsafe { GitConfigBackendMut::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(backend.as_ref().version(), ffi::GIT_CONFIG_BACKEND_VERSION);
        assert!(!backend.as_ref().is_readonly());
        // SAFETY: this backend was never attached, so its back-reference is
        // null and no configuration liveness is claimed.
        assert!(unsafe { backend.as_ref().config() }.is_none());
        backend.set_readonly(true);
        assert!(backend.as_ref().is_readonly());
        backend.lock().unwrap();
        assert!(backend.as_ref().is_readonly());
        backend.unlock(true).unwrap();
        assert!(!backend.as_ref().is_readonly());
        assert_eq!(
            backend.delete(c"missing"),
            Err(GitConfigBackendError::Unsupported(
                GitConfigBackendOperation::Delete
            ))
        );
    }

    #[test]
    fn a_snapshot_borrows_the_source_it_records_until_it_is_detached() {
        SNAPSHOT_FREES.store(0, Ordering::SeqCst);
        SNAPSHOT_SOURCE.store(core::ptr::null_mut(), Ordering::SeqCst);

        let mut raw = raw_backend();
        raw.snapshot = Some(snapshot_backend);
        let source = &raw mut raw;
        // SAFETY: `raw` remains live and is exclusively accessed by this handle.
        let mut backend = unsafe { GitConfigBackendMut::from_ptr(source) }.unwrap();

        let mut snapshot = backend.snapshot().unwrap();
        assert_eq!(SNAPSHOT_SOURCE.load(Ordering::SeqCst), source);
        assert!(snapshot.as_ref().is_readonly());
        snapshot.as_mut().set_version(ffi::GIT_CONFIG_BACKEND_VERSION);
        drop(snapshot);
        assert_eq!(SNAPSHOT_FREES.load(Ordering::SeqCst), 1);

        // The tether ended with the snapshot, so the source is usable again.
        backend.lock().unwrap();
        backend.unlock(true).unwrap();

        let snapshot = backend.snapshot().unwrap();
        // SAFETY: this test snapshot installs no `open` callback, so nothing
        // ever reads the source pointer it recorded.
        let detached = unsafe { snapshot.into_owned() };
        drop(detached);
        assert_eq!(SNAPSHOT_FREES.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn owned_backend_dispatches_its_concrete_finalizer_once() {
        BACKEND_FREES.store(0, Ordering::SeqCst);
        let raw = Box::into_raw(Box::new(raw_backend()));
        // SAFETY: `raw` is a fresh, fully initialized allocation whose
        // callback reclaims it exactly once.
        let backend = unsafe { GitConfigBackendOwned::from_raw(raw) }.unwrap();
        drop(backend);
        assert_eq!(BACKEND_FREES.load(Ordering::SeqCst), 1);
    }
}
