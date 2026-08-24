//! Safe wrappers for libgit2 index APIs.

use core::ffi::CStr;
use core::ptr::{addr_of, addr_of_mut};

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_index_time
    /// A seconds-and-nanoseconds timestamp stored in an index entry.
    IndexTime,
    IndexTimeRef,
    IndexTimeMut,
    ffi::git_index_time
);

impl IndexTimeRef<'_> {
    /// Wraps: git_index_time.seconds
    /// Returns the whole-second component of this timestamp.
    #[inline]
    #[must_use]
    pub fn seconds(&self) -> i32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).seconds).read() }
    }

    /// Wraps: git_index_time.nanoseconds
    /// Returns the subsecond nanosecond component of this timestamp.
    #[inline]
    #[must_use]
    pub fn nanoseconds(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).nanoseconds).read() }
    }
}

impl IndexTimeMut<'_> {
    /// Sets the whole-second component of this timestamp.
    #[inline]
    pub fn set_seconds(&mut self, value: i32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).seconds).write(value) }
    }

    /// Sets the subsecond nanosecond component of this timestamp.
    #[inline]
    pub fn set_nanoseconds(&mut self, value: u32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).nanoseconds).write(value) }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_index_conflict_iterator
    /// Opaque storage for an iterator over conflicts in a libgit2 index.
    ///
    /// The iterator borrows the index supplied to its constructor. Safe
    /// constructor and iteration wrappers must therefore keep that exclusive
    /// index borrow alive for as long as the owner exists.
    GitIndexConflictIterator,
    GitIndexConflictIteratorRef,
    GitIndexConflictIteratorMut,
    ffi::git_index_conflict_iterator
);

/// Wraps: git_index_conflict_iterator_free
/// An owning conflict-iterator allocation.
///
/// This raw ownership building block does not by itself carry the iterator's
/// borrow of its source index. Safe constructors must wrap it in a
/// lifetime-carrying handle before returning it.
pub type GitIndexConflictIteratorOwned = CBox<GitIndexConflictIterator>;

// SAFETY: `git_index_conflict_iterator_free` is the public destructor for a
// fully formed conflict-iterator allocation and accepts null, although `CBox`
// always supplies a live non-null allocation exactly once. It does not access
// the borrowed index stored by the iterator.
ffibox::impl_dropped!(
    GitIndexConflictIterator,
    ffi::git_index_conflict_iterator,
    ffi::git_index_conflict_iterator_free
);

/// Wraps: git_index_matched_path_cb
/// Safe callable surface for pathspec matches during index updates.
///
/// `matched_pathspec` is `None` when the caller supplied no pathspec: every
/// index callsite forwards the pointer `git_pathspec__match` filled in, and
/// that routine reports a match with a null pattern for an empty pathspec
/// list, as `git_index_add_all(index, NULL, ...)` produces.
pub trait GitIndexMatchedPathCallback {
    /// Returns zero to apply, positive to skip, or negative to abort.
    fn call(&mut self, path: &core::ffi::CStr, matched_pathspec: Option<&core::ffi::CStr>) -> i32;
}

impl<F> GitIndexMatchedPathCallback for F
where
    F: FnMut(&core::ffi::CStr, Option<&core::ffi::CStr>) -> i32,
{
    fn call(&mut self, path: &core::ffi::CStr, matched_pathspec: Option<&core::ffi::CStr>) -> i32 {
        self(path, matched_pathspec)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_index_preserves_the_c_seam_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitIndex>();
        assert_dropped::<GitIndex>();
        assert_eq!(size_of::<GitIndex>(), size_of::<ffi::git_index>());
        assert_eq!(align_of::<GitIndex>(), align_of::<ffi::git_index>());
        assert_eq!(
            size_of::<GitIndexRef<'_>>(),
            size_of::<*const ffi::git_index>()
        );
        assert_eq!(
            size_of::<GitIndexMut<'_>>(),
            size_of::<*mut ffi::git_index>()
        );
        assert_eq!(size_of::<GitIndexOwned>(), size_of::<*mut ffi::git_index>());
    }

    #[test]
    fn null_index_seams_create_no_handles() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitIndexRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitIndexMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitIndexOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn index_time_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<IndexTime>(), size_of::<ffi::git_index_time>());
        assert_eq!(align_of::<IndexTime>(), align_of::<ffi::git_index_time>());
        assert_eq!(
            size_of::<IndexTimeRef<'_>>(),
            size_of::<*const ffi::git_index_time>()
        );
        assert_eq!(
            size_of::<IndexTimeMut<'_>>(),
            size_of::<*mut ffi::git_index_time>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_both_components() {
        let mut raw = ffi::git_index_time {
            seconds: -2,
            nanoseconds: 3,
        };

        // SAFETY: `raw` is initialized, non-null, exclusively borrowed for the
        // handle's lifetime, and remains live until the handle is last used.
        let mut time = unsafe { IndexTimeMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(time.as_ref().seconds(), -2);
        assert_eq!(time.as_ref().nanoseconds(), 3);

        time.set_seconds(4);
        time.set_nanoseconds(5);
        assert_eq!(time.as_ref().seconds(), 4);
        assert_eq!(time.as_ref().nanoseconds(), 5);
    }

    #[test]
    fn conflict_iterator_preserves_the_ffi_layout_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitIndexConflictIterator>();
        assert_dropped::<GitIndexConflictIterator>();
        assert_eq!(
            size_of::<GitIndexConflictIterator>(),
            size_of::<ffi::git_index_conflict_iterator>()
        );
        assert_eq!(
            align_of::<GitIndexConflictIterator>(),
            align_of::<ffi::git_index_conflict_iterator>()
        );
        assert_eq!(
            size_of::<GitIndexConflictIteratorRef<'_>>(),
            size_of::<*const ffi::git_index_conflict_iterator>()
        );
        assert_eq!(
            size_of::<GitIndexConflictIteratorMut<'_>>(),
            size_of::<*mut ffi::git_index_conflict_iterator>()
        );
        assert_eq!(
            size_of::<GitIndexConflictIteratorOwned>(),
            size_of::<*mut ffi::git_index_conflict_iterator>()
        );
    }

    #[test]
    fn null_conflict_iterator_seams_create_no_handle() {
        // SAFETY: each conversion accepts null and returns `None` without
        // borrowing or adopting an object.
        unsafe {
            assert!(GitIndexConflictIteratorRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitIndexConflictIteratorMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitIndexConflictIteratorOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

    /// Holds one libgit2 initialization count for the duration of a test.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and
            // refcounted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard, after every libgit2 owner has already been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    #[test]
    fn conflict_iterator_owner_releases_its_allocation() {
        let _libgit2 = Libgit2Init::acquire();

        let mut raw_index = ptr::null_mut();
        // SAFETY: `raw_index` is a writable out-slot; on success libgit2
        // transfers one owned in-memory index reference through it.
        let status = unsafe { ffi::git_index_new(&raw mut raw_index) };
        assert_eq!(status, 0);
        // SAFETY: the successful constructor produced one non-null owned
        // index count that has not been adopted elsewhere.
        let mut index = unsafe { GitIndexOwned::from_raw(raw_index) }.expect("a new index");

        let mut raw_iterator = ptr::null_mut();
        // SAFETY: `raw_iterator` is a writable out-slot and the index stays
        // exclusively borrowed for as long as the iterator below lives. The
        // constructor stores the index pointer without retaining a count.
        let status = unsafe {
            ffi::git_index_conflict_iterator_new(&raw mut raw_iterator, index.as_mut().as_mut_ptr())
        };
        assert_eq!(status, 0);
        // SAFETY: the successful constructor produced one non-null owned
        // iterator allocation that has not been adopted elsewhere.
        let mut iterator =
            unsafe { GitIndexConflictIteratorOwned::from_raw(raw_iterator) }.expect("an iterator");
        assert_eq!(iterator.as_ref().as_ptr(), raw_iterator.cast_const());
        assert_eq!(iterator.as_mut().as_mut_ptr(), raw_iterator);

        // `git_index_conflict_iterator_free` runs here and must not touch the
        // borrowed index, which outlives it.
        drop(iterator);
        drop(index);
    }
}

#[cfg(test)]
mod callback_tests {
    use super::*;

    #[test]
    fn matched_path_callback_preserves_control_result() {
        let mut callback = |path: &core::ffi::CStr, spec: Option<&core::ffi::CStr>| {
            assert_eq!(path, c"src/lib.rs");
            assert_eq!(spec, Some(c"src/*"));
            1
        };
        assert_eq!(
            GitIndexMatchedPathCallback::call(&mut callback, c"src/lib.rs", Some(c"src/*")),
            1
        );
    }

    #[test]
    fn matched_path_callback_accepts_an_absent_pathspec() {
        let mut seen = None;
        let mut callback = |path: &core::ffi::CStr, spec: Option<&core::ffi::CStr>| {
            seen = Some(spec.is_none());
            assert_eq!(path, c"src/lib.rs");
            0
        };
        assert_eq!(
            GitIndexMatchedPathCallback::call(&mut callback, c"src/lib.rs", None),
            0
        );
        drop(callback);
        assert_eq!(seen, Some(true));
    }
}

ffibox::define_ctype!(
    /// Wraps: git_index
    /// An opaque, reference-counted libgit2 index.
    ///
    /// Each [`GitIndexOwned`] represents one reference count and releases it
    /// with `git_index_free`. Libgit2 does not publish an operation for
    /// acquiring another count, so owning handles intentionally do not
    /// implement `Clone`.
    GitIndex,
    GitIndexRef,
    GitIndexMut,
    ffi::git_index
);

/// An owned reference count to a libgit2 index.
pub type GitIndexOwned = CBox<GitIndex>;

// SAFETY: `git_index_free` consumes exactly one reference to a complete
// `git_index`. On the final count it disposes the index-owned fields and frees
// the allocation; it accepts null, although `CBox` always supplies a live
// non-null object exactly once.
ffibox::impl_dropped!(GitIndex, ffi::git_index, ffi::git_index_free);

ffibox::define_ctype!(
    /// Wraps: git_index_entry
    /// The public, layout-compatible representation of one index entry.
    ///
    /// The struct never owns anything, in either of its two roles. As
    /// caller-built input it is a by-value record whose `path` the caller
    /// keeps alive: `git_index_add` copies the string before returning. As an
    /// entry read back out of an index, it is the header of an
    /// `entry_internal` whose `path` points into that allocation's trailing
    /// storage, so the entry and its string live and die with the index.
    /// Either way the borrow outlives no handle taken over it.
    IndexEntry,
    IndexEntryRef,
    IndexEntryMut,
    ffi::git_index_entry
);

impl<'a> IndexEntryRef<'a> {
    /// Wraps: git_index_entry.path
    /// Returns the optional NUL-terminated path borrowed by this entry.
    #[must_use]
    pub fn path(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the C-visible entry.
        let path = unsafe { addr_of!((*self.as_ptr()).path).read() };
        if path.is_null() {
            None
        } else {
            // SAFETY: a usable entry requires its non-null path to remain a
            // NUL-terminated string for the entry's lifetime. The returned
            // reference is bounded by this handle's borrow.
            Some(unsafe { CStr::from_ptr(path) })
        }
    }

    /// Wraps: git_index_entry.mode
    /// Returns the entry's file mode.
    #[must_use]
    pub fn mode(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).mode).read() }
    }

    /// Wraps: git_index_entry.flags
    /// Returns the entry's on-disk flag bits.
    #[must_use]
    pub fn flags(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Wraps: git_index_entry.file_size
    /// Returns the cached, truncated file size.
    #[must_use]
    pub fn file_size(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).file_size).read() }
    }

    /// Wraps: git_index_entry.id
    /// Borrows the inline object identifier.
    #[must_use]
    pub fn id(&self) -> crate::oid::OidRef<'a> {
        // SAFETY: `id` is an inline, initialized field of this live entry. The
        // projected pointer is non-null and the handle cannot outlive `self`.
        unsafe { crate::oid::OidRef::from_ptr(addr_of!((*self.as_ptr()).id).cast_mut()) }
            .expect("an inline field address is non-null")
    }

    /// Wraps: git_index_entry.mtime
    /// Borrows the inline modification timestamp.
    #[must_use]
    pub fn mtime(&self) -> IndexTimeRef<'a> {
        // SAFETY: `mtime` is an inline, initialized field of this live entry.
        // The projected pointer is non-null and retains the entry's lifetime.
        unsafe { IndexTimeRef::from_ptr(addr_of!((*self.as_ptr()).mtime).cast_mut()) }
            .expect("an inline field address is non-null")
    }

    /// Wraps: git_index_entry.ctime
    /// Borrows the inline creation/change timestamp.
    #[must_use]
    pub fn ctime(&self) -> IndexTimeRef<'a> {
        // SAFETY: `ctime` is an inline, initialized field of this live entry.
        // The projected pointer is non-null and retains the entry's lifetime.
        unsafe { IndexTimeRef::from_ptr(addr_of!((*self.as_ptr()).ctime).cast_mut()) }
            .expect("an inline field address is non-null")
    }

    /// Wraps: git_index_entry.uid
    /// Returns the cached owner user ID.
    #[must_use]
    pub fn uid(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).uid).read() }
    }

    /// Wraps: git_index_entry.flags_extended
    /// Returns the entry's extended flag bits.
    #[must_use]
    pub fn flags_extended(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).flags_extended).read() }
    }

    /// Wraps: git_index_entry.gid
    /// Returns the cached owner group ID.
    #[must_use]
    pub fn gid(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).gid).read() }
    }

    /// Wraps: git_index_entry.ino
    /// Returns the cached inode number.
    #[must_use]
    pub fn ino(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).ino).read() }
    }

    /// Wraps: git_index_entry.dev
    /// Returns the cached device number.
    #[must_use]
    pub fn dev(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).dev).read() }
    }
}

impl IndexEntryMut<'_> {
    /// Stores a borrowed path pointer in this entry.
    ///
    /// # Safety
    ///
    /// A non-null `path` must remain alive and NUL-terminated for every later
    /// use of the entry, including uses after this handle is released.
    pub unsafe fn set_borrowed_path(&mut self, path: Option<&CStr>) {
        let path = path.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits a raw-place field write, and
        // the caller upholds the stored pointer's lifetime contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).path).write(path) }
    }

    /// Clears the optional borrowed path.
    pub fn clear_path(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime obligation.
        unsafe { self.set_borrowed_path(None) }
    }

    /// Sets the entry's file mode.
    pub fn set_mode(&mut self, value: u32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write
        // without forming a reference to C-visible memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).mode).write(value) }
    }

    /// Replaces the entry's on-disk flag bits.
    pub fn set_flags(&mut self, value: u16) {
        // SAFETY: this exclusive handle permits a raw-place scalar write
        // without forming a reference to C-visible memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(value) }
    }

    /// Sets the cached, truncated file size.
    pub fn set_file_size(&mut self, value: u32) {
        // SAFETY: as `set_mode`, for this scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).file_size).write(value) }
    }

    /// Borrows the inline object identifier exclusively.
    #[must_use]
    pub fn id_mut(&mut self) -> crate::oid::OidMut<'_> {
        // SAFETY: this exclusive reborrow projects the live inline field. Its
        // address is non-null and the returned handle is bounded by `&mut self`.
        unsafe { crate::oid::OidMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).id)) }
            .expect("an inline field address is non-null")
    }

    /// Borrows the inline modification timestamp exclusively.
    #[must_use]
    pub fn mtime_mut(&mut self) -> IndexTimeMut<'_> {
        // SAFETY: this exclusive reborrow projects the live inline field. Its
        // address is non-null and the returned handle is bounded by `&mut self`.
        unsafe { IndexTimeMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).mtime)) }
            .expect("an inline field address is non-null")
    }

    /// Borrows the inline creation/change timestamp exclusively.
    #[must_use]
    pub fn ctime_mut(&mut self) -> IndexTimeMut<'_> {
        // SAFETY: this exclusive reborrow projects the live inline field. Its
        // address is non-null and the returned handle is bounded by `&mut self`.
        unsafe { IndexTimeMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).ctime)) }
            .expect("an inline field address is non-null")
    }

    /// Sets the cached owner user ID.
    pub fn set_uid(&mut self, value: u32) {
        // SAFETY: as `set_mode`, for this scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).uid).write(value) }
    }

    /// Replaces the entry's extended flag bits.
    pub fn set_flags_extended(&mut self, value: u16) {
        // SAFETY: this exclusive handle permits a raw-place scalar write
        // without forming a reference to C-visible memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags_extended).write(value) }
    }

    /// Sets the cached owner group ID.
    pub fn set_gid(&mut self, value: u32) {
        // SAFETY: as `set_mode`, for this scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).gid).write(value) }
    }

    /// Sets the cached inode number.
    pub fn set_ino(&mut self, value: u32) {
        // SAFETY: as `set_mode`, for this scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).ino).write(value) }
    }

    /// Sets the cached device number.
    pub fn set_dev(&mut self, value: u32) {
        // SAFETY: as `set_mode`, for this scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).dev).write(value) }
    }
}

#[cfg(test)]
mod entry_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn index_entry_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<IndexEntry>(), size_of::<ffi::git_index_entry>());
        assert_eq!(align_of::<IndexEntry>(), align_of::<ffi::git_index_entry>());
        assert_eq!(
            size_of::<IndexEntryRef<'_>>(),
            size_of::<*const ffi::git_index_entry>()
        );
        assert_eq!(
            size_of::<IndexEntryMut<'_>>(),
            size_of::<*mut ffi::git_index_entry>()
        );
    }

    #[test]
    fn borrowed_handles_access_every_entry_field() {
        let mut raw = IndexEntry::zeroed();
        // SAFETY: `raw` is a live, initialized, layout-compatible wrapper and
        // is exclusively borrowed until the handle's last use.
        let mut entry = unsafe { IndexEntryMut::from_ptr(addr_of_mut!(raw).cast()) }
            .expect("the address of a stack value is non-null");

        // SAFETY: the static C string outlives every use of `raw`.
        unsafe { entry.set_borrowed_path(Some(c"src/lib.rs")) };
        entry.set_mode(0o100644);
        entry.set_flags(0x1234);
        entry.set_file_size(42);
        entry.set_uid(1000);
        entry.set_flags_extended(0x4000);
        entry.set_gid(1001);
        entry.set_ino(7);
        entry.set_dev(8);
        entry.mtime_mut().set_seconds(9);
        entry.mtime_mut().set_nanoseconds(10);
        entry.ctime_mut().set_seconds(11);
        entry.ctime_mut().set_nanoseconds(12);
        entry.id_mut().set_oid_type(crate::oid::OidType::Sha256);

        let entry = entry.as_ref();
        assert_eq!(entry.path(), Some(c"src/lib.rs"));
        assert_eq!(entry.mode(), 0o100644);
        assert_eq!(entry.flags(), 0x1234);
        assert_eq!(entry.file_size(), 42);
        assert_eq!(entry.uid(), 1000);
        assert_eq!(entry.flags_extended(), 0x4000);
        assert_eq!(entry.gid(), 1001);
        assert_eq!(entry.ino(), 7);
        assert_eq!(entry.dev(), 8);
        assert_eq!(entry.mtime().seconds(), 9);
        assert_eq!(entry.mtime().nanoseconds(), 10);
        assert_eq!(entry.ctime().seconds(), 11);
        assert_eq!(entry.ctime().nanoseconds(), 12);
        assert_eq!(entry.id().oid_type(), Ok(crate::oid::OidType::Sha256));
    }

    #[test]
    fn zeroed_entry_has_no_path_and_can_be_cleared() {
        let mut raw = IndexEntry::zeroed();
        // SAFETY: `raw` is live, initialized, layout-compatible, and
        // exclusively borrowed by the handle until its last use.
        let mut entry = unsafe { IndexEntryMut::from_ptr(addr_of_mut!(raw).cast()) }.unwrap();
        assert_eq!(entry.as_ref().path(), None);
        // SAFETY: the static string outlives the entry.
        unsafe { entry.set_borrowed_path(Some(c"temporary")) };
        entry.clear_path();
        assert_eq!(entry.as_ref().path(), None);
    }
}
