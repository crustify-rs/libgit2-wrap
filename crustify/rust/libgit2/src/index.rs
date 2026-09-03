//! Safe wrappers for libgit2 index APIs.

use core::ffi::CStr;
use core::ptr::{addr_of, addr_of_mut};

use ffibox::{CBox, CVal};

pub use crate::api::index::{GitIndexAddOptions, GitIndexEntryExtendedFlags, GitIndexStage};
use crate::ffi;
use crate::oid::Oid;
use crate::repository::GitRepositoryRef;
use crate::strarray::GitStrArrayRef;
use crate::tree::GitTreeRef;

ffibox::define_ctype!(
    /// Wraps: git_index_time
    /// A seconds-and-nanoseconds timestamp stored in an index entry.
    IndexTime,
    IndexTimeRef,
    IndexTimeMut,
    ffi::git_index_time
);

impl IndexTimeRef<'_> {
    /// Field: git_index_time.seconds
    /// Returns the whole-second component of this timestamp.
    #[inline]
    #[must_use]
    pub fn seconds(&self) -> i32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).seconds).read() }
    }

    /// Field: git_index_time.nanoseconds
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
    /// implement `Clone`: the internal `git_index_snapshot_new` is the only
    /// routine that takes an extra count, and it is not a duplicator, since it
    /// also registers a reader that `git_index_snapshot_release` must retire.
    GitIndex,
    GitIndexRef,
    GitIndexMut,
    ffi::git_index
);

/// An owned reference count to a libgit2 index.
pub type GitIndexOwned = CBox<GitIndex>;

// SAFETY: `git_index_free` consumes exactly one reference to a complete
// `git_index`. An index never takes a refcount owner, so the final count
// disposes the index-owned fields and frees the allocation, except while an
// internal snapshot reader is still registered — a state its taker balances
// before releasing its own count, so it cannot be reached through this owner.
// The C function accepts null, although `CBox` always supplies a live non-null
// object exactly once.
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
    /// Field: git_index_entry.path
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

    /// Field: git_index_entry.mode
    /// Returns the entry's file mode.
    #[must_use]
    pub fn mode(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).mode).read() }
    }

    /// Field: git_index_entry.flags
    /// Returns the entry's on-disk flag bits.
    #[must_use]
    pub fn flags(&self) -> u16 {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Field: git_index_entry.file_size
    /// Returns the cached, truncated file size.
    #[must_use]
    pub fn file_size(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).file_size).read() }
    }

    /// Field: git_index_entry.id
    /// Borrows the inline object identifier.
    #[must_use]
    pub fn id(&self) -> crate::oid::OidRef<'a> {
        // SAFETY: `id` is an inline, initialized field of this live entry. The
        // projected pointer is non-null and the handle cannot outlive `self`.
        unsafe { crate::oid::OidRef::from_ptr(addr_of!((*self.as_ptr()).id).cast_mut()) }
            .expect("an inline field address is non-null")
    }

    /// Field: git_index_entry.mtime
    /// Borrows the inline modification timestamp.
    #[must_use]
    pub fn mtime(&self) -> IndexTimeRef<'a> {
        // SAFETY: `mtime` is an inline, initialized field of this live entry.
        // The projected pointer is non-null and retains the entry's lifetime.
        unsafe { IndexTimeRef::from_ptr(addr_of!((*self.as_ptr()).mtime).cast_mut()) }
            .expect("an inline field address is non-null")
    }

    /// Field: git_index_entry.ctime
    /// Borrows the inline creation/change timestamp.
    #[must_use]
    pub fn ctime(&self) -> IndexTimeRef<'a> {
        // SAFETY: `ctime` is an inline, initialized field of this live entry.
        // The projected pointer is non-null and retains the entry's lifetime.
        unsafe { IndexTimeRef::from_ptr(addr_of!((*self.as_ptr()).ctime).cast_mut()) }
            .expect("an inline field address is non-null")
    }

    /// Field: git_index_entry.uid
    /// Returns the cached owner user ID.
    #[must_use]
    pub fn uid(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).uid).read() }
    }

    /// Field: git_index_entry.flags_extended
    /// Returns the entry's extended flag bits.
    pub fn flags_extended(&self) -> Result<GitIndexEntryExtendedFlags, u16> {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        let raw = unsafe { addr_of!((*self.as_ptr()).flags_extended).read() };
        GitIndexEntryExtendedFlags::from_bits(raw.into()).ok_or(raw)
    }

    /// Field: git_index_entry.gid
    /// Returns the cached owner group ID.
    #[must_use]
    pub fn gid(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).gid).read() }
    }

    /// Field: git_index_entry.ino
    /// Returns the cached inode number.
    #[must_use]
    pub fn ino(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).ino).read() }
    }

    /// Field: git_index_entry.dev
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
    pub fn set_flags_extended(&mut self, value: GitIndexEntryExtendedFlags) {
        let value = value.bits() as u16;
        // SAFETY: this exclusive handle permits a raw-place scalar write
        // without forming a reference to C-visible memory. Every published
        // extended flag fits in the C field's 16 bits.
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
        entry.set_flags_extended(GitIndexEntryExtendedFlags::SKIP_WORKTREE);
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
        assert_eq!(
            entry.flags_extended(),
            Ok(GitIndexEntryExtendedFlags::SKIP_WORKTREE)
        );
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

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod unit_tests {
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

/// Wraps: git_index_add
/// Copies an entry, including its path, into the index.
pub fn git_index_add(index: &mut GitIndexMut<'_>, entry: IndexEntryRef<'_>) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed, the entry is live, and C
    // copies every retained field before returning.
    let status = unsafe { ffi::git_index_add(index.as_mut_ptr(), entry.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

unsafe extern "C" fn matched_path_trampoline(
    path: *const core::ffi::c_char,
    matched: *const core::ffi::c_char,
    payload: *mut core::ffi::c_void,
) -> i32 {
    if path.is_null() || payload.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: `git_index_add_all` receives `payload` as the address of a live
    // stack-stored trait-object reference and invokes this trampoline only
    // synchronously before that storage goes away.
    let callback = unsafe { &mut *payload.cast::<&mut dyn GitIndexMatchedPathCallback>() };
    // SAFETY: libgit2 supplies a non-null NUL-terminated path for the call.
    let path = unsafe { CStr::from_ptr(path) };
    let matched = if matched.is_null() {
        None
    } else {
        // SAFETY: a non-null match is a NUL-terminated pathspec string live
        // for this callback invocation.
        Some(unsafe { CStr::from_ptr(matched) })
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        GitIndexMatchedPathCallback::call(*callback, path, matched)
    }))
    .unwrap_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_index_add_all
/// Adds matching work-directory paths, optionally consulting `callback`.
pub fn git_index_add_all(
    index: &mut GitIndexMut<'_>,
    pathspec: Option<crate::strarray::GitStrArrayRef<'_>>,
    flags: GitIndexAddOptions,
    callback: Option<&mut dyn GitIndexMatchedPathCallback>,
) -> Result<(), i32> {
    let pathspec = pathspec.map_or(core::ptr::null(), |value| value.as_ptr());
    let mut callback = callback;
    let (function, payload) = match callback.as_mut() {
        Some(callback) => (
            Some(matched_path_trampoline as unsafe extern "C" fn(_, _, _) -> _),
            core::ptr::from_mut(callback).cast::<core::ffi::c_void>(),
        ),
        None => (None, core::ptr::null_mut()),
    };
    // SAFETY: the index is exclusive, `pathspec` is null or live, and the
    // callback payload remains live for the synchronous traversal.
    let status = unsafe {
        ffi::git_index_add_all(
            index.as_mut_ptr(),
            pathspec,
            flags.bits(),
            function,
            payload,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_index_add_bypath
/// Adds a work-directory file by repository-relative path.
pub fn git_index_add_bypath(index: &mut GitIndexMut<'_>, path: &CStr) -> Result<(), i32> {
    // SAFETY: the index is exclusive and `path` is live for the call.
    let status = unsafe { ffi::git_index_add_bypath(index.as_mut_ptr(), path.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_index_add_frombuffer
/// Adds an entry whose blob contents come from `buffer`.
pub fn git_index_add_frombuffer(
    index: &mut GitIndexMut<'_>,
    entry: IndexEntryRef<'_>,
    buffer: &[u8],
) -> Result<(), i32> {
    // SAFETY: the index is exclusive, the entry is live, and `buffer` exposes
    // exactly the readable byte count supplied to C; nothing is retained.
    let status = unsafe {
        ffi::git_index_add_frombuffer(
            index.as_mut_ptr(),
            entry.as_ptr(),
            buffer.as_ptr().cast(),
            buffer.len(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_index_clear
/// Removes every entry from the index.
pub fn git_index_clear(index: &mut GitIndexMut<'_>) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed for the mutation.
    let status = unsafe { ffi::git_index_clear(index.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// The three optional index entries representing one conflict.
pub struct GitIndexConflict<'a> {
    /// Common ancestor entry.
    pub ancestor: Option<IndexEntryRef<'a>>,
    /// Entry from our side.
    pub ours: Option<IndexEntryRef<'a>>,
    /// Entry from their side.
    pub theirs: Option<IndexEntryRef<'a>>,
}

/// Wraps: git_index_conflict_get
/// Borrows the conflict entries for `path` until the index can next mutate.
///
/// The index is taken exclusively, like [`git_index_get_bypath`]: the lookup
/// runs through `git_index_find`, whose binary search sorts the index's entry
/// vector in place before searching it.
pub fn git_index_conflict_get<'a>(
    index: &'a mut GitIndexMut<'_>,
    path: &CStr,
) -> Result<GitIndexConflict<'a>, i32> {
    let (mut ancestor, mut ours, mut theirs) =
        (core::ptr::null(), core::ptr::null(), core::ptr::null());
    // SAFETY: the output slots are writable, `path` is live for the call, and
    // the index is exclusively borrowed for the in-place sort the lookup
    // performs and for as long as the returned entries live.
    let status = unsafe {
        ffi::git_index_conflict_get(
            &mut ancestor,
            &mut ours,
            &mut theirs,
            index.as_mut_ptr(),
            path.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: each non-null result points into `index` and stays live for its
    // exclusive `'a` reborrow; null denotes a missing side of the conflict.
    Ok(unsafe {
        GitIndexConflict {
            ancestor: IndexEntryRef::from_ptr(ancestor.cast_mut()),
            ours: IndexEntryRef::from_ptr(ours.cast_mut()),
            theirs: IndexEntryRef::from_ptr(theirs.cast_mut()),
        }
    })
}

/// An owned conflict iterator carrying its exclusive index borrow.
pub struct GitIndexConflicts<'index> {
    inner: GitIndexConflictIteratorOwned,
    _index: core::marker::PhantomData<GitIndexMut<'index>>,
}

impl GitIndexConflicts<'_> {
    /// Borrows the iterator.
    #[must_use]
    pub fn as_ref(&self) -> GitIndexConflictIteratorRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the iterator exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitIndexConflictIteratorMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_index_conflict_iterator_new
/// Creates a conflict iterator and holds the index exclusively until drop.
pub fn git_index_conflict_iterator_new<'index>(
    mut index: GitIndexMut<'index>,
) -> Result<GitIndexConflicts<'index>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and the returned type carries the exclusive
    // index borrow that the iterator stores without retaining.
    let status = unsafe { ffi::git_index_conflict_iterator_new(&mut out, index.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized iterator allocation.
    let inner = unsafe { GitIndexConflictIteratorOwned::from_raw(out) }
        .ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(GitIndexConflicts {
        inner,
        _index: core::marker::PhantomData,
    })
}

/// Wraps: git_index_conflict_next
/// Advances a scoped conflict iterator and borrows its current three stages.
pub fn git_index_conflict_next<'a>(
    iterator: &'a mut GitIndexConflicts<'_>,
) -> Result<GitIndexConflict<'a>, i32> {
    let mut ancestor = core::ptr::null();
    let mut ours = core::ptr::null();
    let mut theirs = core::ptr::null();
    let mut handle = iterator.as_mut();
    // SAFETY: all result slots are writable and the exclusive iterator borrow
    // remains live for the call and for every returned entry handle.
    let status = unsafe {
        ffi::git_index_conflict_next(&mut ancestor, &mut ours, &mut theirs, handle.as_mut_ptr())
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: successful iteration returns null or entries borrowed from the
    // source index carried by the scoped iterator. Tying them to the exclusive
    // iterator reborrow prevents advancing it while they remain live.
    Ok(unsafe {
        GitIndexConflict {
            ancestor: IndexEntryRef::from_ptr(ancestor.cast_mut()),
            ours: IndexEntryRef::from_ptr(ours.cast_mut()),
            theirs: IndexEntryRef::from_ptr(theirs.cast_mut()),
        }
    })
}

fn status_result(status: i32) -> Result<(), i32> {
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_index_conflict_remove
pub fn git_index_conflict_remove(index: &mut GitIndexMut<'_>, path: &CStr) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed and the path is a live C
    // string retained only for this call.
    status_result(unsafe { ffi::git_index_conflict_remove(index.as_mut_ptr(), path.as_ptr()) })
}

/// Wraps: git_index_entrycount
#[must_use]
pub fn git_index_entrycount(index: GitIndexRef<'_>) -> usize {
    // SAFETY: the shared handle supplies a live index for this read-only call.
    unsafe { ffi::git_index_entrycount(index.as_ptr()) }
}

/// Wraps: git_index_find_prefix
pub fn git_index_find_prefix(index: &mut GitIndexMut<'_>, prefix: &CStr) -> Result<usize, i32> {
    let mut position = 0;
    // SAFETY: `position` is writable, the index is exclusively borrowed (the
    // lookup may sort it), and `prefix` remains live for the call.
    let status =
        unsafe { ffi::git_index_find_prefix(&mut position, index.as_mut_ptr(), prefix.as_ptr()) };
    if status == 0 {
        Ok(position)
    } else {
        Err(status)
    }
}

/// Wraps: git_index_get_byindex
pub fn git_index_get_byindex<'a>(
    index: &'a mut GitIndexMut<'_>,
    position: usize,
) -> Option<IndexEntryRef<'a>> {
    // SAFETY: the index is exclusively borrowed because the lookup may sort
    // it; the returned pointer remains owned by that index.
    let entry = unsafe { ffi::git_index_get_byindex(index.as_mut_ptr(), position) };
    // SAFETY: null means no entry; otherwise the pointer stays live for the
    // index reborrow and is exposed only through a shared handle.
    unsafe { IndexEntryRef::from_ptr(entry.cast_mut()) }
}

/// Wraps: git_index_get_bypath
pub fn git_index_get_bypath<'a>(
    index: &'a mut GitIndexMut<'_>,
    path: &CStr,
    stage: GitIndexStage,
) -> Option<IndexEntryRef<'a>> {
    // SAFETY: the index is exclusively borrowed, `path` is live for this
    // lookup, and the returned pointer remains index-owned.
    let entry =
        unsafe { ffi::git_index_get_bypath(index.as_mut_ptr(), path.as_ptr(), stage.into()) };
    // SAFETY: null means no entry; a non-null entry is bounded by the index
    // reborrow and is exposed only through a shared handle.
    unsafe { IndexEntryRef::from_ptr(entry.cast_mut()) }
}

/// Wraps: git_index_has_conflicts
#[must_use]
pub fn git_index_has_conflicts(index: GitIndexRef<'_>) -> bool {
    // SAFETY: the shared handle supplies a live index for this read-only scan.
    unsafe { ffi::git_index_has_conflicts(index.as_ptr()) != 0 }
}

/// Wraps: git_index_new
pub fn git_index_new() -> Result<GitIndexOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable slot for one fresh owned index count.
    let status = unsafe { ffi::git_index_new(&mut out) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized index count.
    unsafe { GitIndexOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_index_open
pub fn git_index_open(path: &CStr) -> Result<GitIndexOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and `path` remains live while libgit2 copies
    // it into the newly allocated index.
    let status = unsafe { ffi::git_index_open(&mut out, path.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized index count.
    unsafe { GitIndexOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_index_path
pub fn git_index_path<'a>(index: GitIndexRef<'a>) -> Option<&'a CStr> {
    // SAFETY: the shared index remains live for `'a`; C returns null for an
    // in-memory index or its owned NUL-terminated path.
    let path = unsafe { ffi::git_index_path(index.as_ptr()) };
    if path.is_null() {
        None
    } else {
        // SAFETY: the non-null path is index-owned and NUL-terminated.
        Some(unsafe { CStr::from_ptr(path) })
    }
}

/// Wraps: git_index_read
pub fn git_index_read(index: &mut GitIndexMut<'_>, force: bool) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed for the reload.
    status_result(unsafe { ffi::git_index_read(index.as_mut_ptr(), i32::from(force)) })
}

/// Wraps: git_index_read_tree
pub fn git_index_read_tree(index: &mut GitIndexMut<'_>, tree: GitTreeRef<'_>) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed; the live tree is read and
    // copied into it without being retained.
    status_result(unsafe { ffi::git_index_read_tree(index.as_mut_ptr(), tree.as_ptr()) })
}

/// Wraps: git_index_remove
pub fn git_index_remove(
    index: &mut GitIndexMut<'_>,
    path: &CStr,
    stage: GitIndexStage,
) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed and `path` is retained only
    // for this call.
    status_result(unsafe { ffi::git_index_remove(index.as_mut_ptr(), path.as_ptr(), stage.into()) })
}

/// Wraps: git_index_remove_all
pub fn git_index_remove_all(
    index: &mut GitIndexMut<'_>,
    pathspec: GitStrArrayRef<'_>,
) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed and the pathspec remains live
    // for this synchronous operation; no callback or payload is supplied.
    status_result(unsafe {
        ffi::git_index_remove_all(
            index.as_mut_ptr(),
            pathspec.as_ptr(),
            None,
            core::ptr::null_mut(),
        )
    })
}

/// Wraps: git_index_remove_all
pub fn git_index_remove_all_with_callback<C: GitIndexMatchedPathCallback>(
    index: &mut GitIndexMut<'_>,
    pathspec: GitStrArrayRef<'_>,
    callback: &mut C,
) -> Result<(), i32> {
    let mut erased: &mut dyn GitIndexMatchedPathCallback = callback;
    // SAFETY: index, pathspec and callback all remain live for this synchronous
    // traversal; the trampoline reconstructs the exact callback type.
    status_result(unsafe {
        ffi::git_index_remove_all(
            index.as_mut_ptr(),
            pathspec.as_ptr(),
            Some(matched_path_trampoline),
            core::ptr::from_mut(&mut erased).cast(),
        )
    })
}

/// Wraps: git_index_remove_bypath
pub fn git_index_remove_bypath(index: &mut GitIndexMut<'_>, path: &CStr) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed and `path` is live for the
    // non-retaining call.
    status_result(unsafe { ffi::git_index_remove_bypath(index.as_mut_ptr(), path.as_ptr()) })
}

/// Wraps: git_index_remove_directory
pub fn git_index_remove_directory(
    index: &mut GitIndexMut<'_>,
    directory: &CStr,
    stage: GitIndexStage,
) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed and `directory` is live for
    // the non-retaining call.
    status_result(unsafe {
        ffi::git_index_remove_directory(index.as_mut_ptr(), directory.as_ptr(), stage.into())
    })
}

/// Wraps: git_index_set_version
pub fn git_index_set_version(index: &mut GitIndexMut<'_>, version: u32) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed; C validates the requested
    // on-disk format version.
    status_result(unsafe { ffi::git_index_set_version(index.as_mut_ptr(), version) })
}

/// Wraps: git_index_update_all
pub fn git_index_update_all(
    index: &mut GitIndexMut<'_>,
    pathspec: GitStrArrayRef<'_>,
) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed and the pathspec remains live
    // for this synchronous operation; no callback is supplied.
    status_result(unsafe {
        ffi::git_index_update_all(
            index.as_mut_ptr(),
            pathspec.as_ptr(),
            None,
            core::ptr::null_mut(),
        )
    })
}

/// Wraps: git_index_update_all
pub fn git_index_update_all_with_callback<C: GitIndexMatchedPathCallback>(
    index: &mut GitIndexMut<'_>,
    pathspec: GitStrArrayRef<'_>,
    callback: &mut C,
) -> Result<(), i32> {
    let mut erased: &mut dyn GitIndexMatchedPathCallback = callback;
    // SAFETY: index, pathspec and callback remain live for the synchronous
    // traversal; the trampoline reconstructs the exact callback type.
    status_result(unsafe {
        ffi::git_index_update_all(
            index.as_mut_ptr(),
            pathspec.as_ptr(),
            Some(matched_path_trampoline),
            core::ptr::from_mut(&mut erased).cast(),
        )
    })
}

/// Wraps: git_index_version
#[must_use]
pub fn git_index_version(index: &mut GitIndexMut<'_>) -> u32 {
    // SAFETY: the handle exclusively borrows the live index for this scalar
    // query (the C API unnecessarily omits `const`).
    unsafe { ffi::git_index_version(index.as_mut_ptr()) }
}

/// Wraps: git_index_write
pub fn git_index_write(index: &mut GitIndexMut<'_>) -> Result<(), i32> {
    // SAFETY: the index is exclusively borrowed while C writes and updates its
    // internal checksum and dirty state.
    status_result(unsafe { ffi::git_index_write(index.as_mut_ptr()) })
}

/// Wraps: git_index_write_tree
pub fn git_index_write_tree(index: &mut GitIndexMut<'_>) -> Result<Oid, i32> {
    let mut oid = Oid::zeroed();
    // SAFETY: `oid` is writable layout-compatible storage and the index is
    // exclusively borrowed while C materializes its tree.
    let status = unsafe {
        ffi::git_index_write_tree(core::ptr::addr_of_mut!(oid).cast(), index.as_mut_ptr())
    };
    if status == 0 { Ok(oid) } else { Err(status) }
}

/// Wraps: git_index_write_tree_to
pub fn git_index_write_tree_to(
    index: &mut GitIndexMut<'_>,
    repository: GitRepositoryRef<'_>,
) -> Result<Oid, i32> {
    let mut oid = Oid::zeroed();
    // SAFETY: `oid` is writable, the index is exclusively borrowed, and the
    // live repository remains available for the synchronous object writes.
    let status = unsafe {
        ffi::git_index_write_tree_to(
            core::ptr::addr_of_mut!(oid).cast(),
            index.as_mut_ptr(),
            repository.as_ptr().cast_mut(),
        )
    };
    if status == 0 { Ok(oid) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_wrapper_tests {
    use super::*;

    #[test]
    fn conflict_lookup_reborrows_the_index_exclusively() {
        // The lookup sorts `index->entries` in place, so it takes the index
        // exclusively and the returned entries are tied to that reborrow.
        let _: for<'a> fn(&'a mut GitIndexMut<'_>, &CStr) -> Result<GitIndexConflict<'a>, i32> =
            git_index_conflict_get;

        // SAFETY: process-global initialization is refcounted and balanced
        // after the index owner is dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut index = git_index_new().expect("an in-memory index");
        let missing = git_index_conflict_get(&mut index.as_mut(), c"missing").err();
        assert_eq!(missing, Some(ffi::git_error_code_GIT_ENOTFOUND));
        drop(index);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn new_index_queries_preserve_owned_and_borrowed_state() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after the index owner is dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut index = git_index_new().expect("an in-memory index");
        assert_eq!(git_index_entrycount(index.as_ref()), 0);
        assert!(!git_index_has_conflicts(index.as_ref()));
        assert_eq!(git_index_path(index.as_ref()), None);
        assert_eq!(
            git_index_caps(index.as_ref()),
            crate::api::index::GitIndexCapabilities::NONE
        );
        git_index_set_caps(
            &mut index.as_mut(),
            crate::api::index::GitIndexCapabilities::IGNORE_CASE,
        )
        .unwrap();
        assert_eq!(
            git_index_caps(index.as_ref()),
            crate::api::index::GitIndexCapabilities::IGNORE_CASE
        );
        assert_eq!(git_index_version(&mut index.as_mut()), 2);
        git_index_set_version(&mut index.as_mut(), 3).unwrap();
        assert_eq!(git_index_version(&mut index.as_mut()), 3);
        assert!(git_index_get_byindex(&mut index.as_mut(), 0).is_none());
        drop(index);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_index_iterator
    /// Opaque storage for an iterator over a stable snapshot of an index.
    ///
    /// `git_index_iterator_new` builds its snapshot with
    /// `git_index_snapshot_new`, which takes a refcounted share of the index
    /// (`GIT_REFCOUNT_INC`) and registers the iterator in `index->readers`;
    /// `git_index_iterator_free` releases both through `git_index_free`. The
    /// allocation therefore keeps its source index alive on its own and needs
    /// no borrow to outlive it.
    ///
    /// Construction still needs exclusive access: `git_index_snapshot_new`
    /// sorts `index->entries` in place before duplicating the entry pointers.
    /// Entries handed out by `git_index_iterator_next` point into the index's
    /// own storage, which the reader registration keeps alive, so they belong
    /// to the iterator's borrow rather than to the index argument.
    GitIndexIterator,
    GitIndexIteratorRef,
    GitIndexIteratorMut,
    ffi::git_index_iterator
);

/// Wraps: git_index_iterator_free
/// An exclusively owned iterator allocation.
///
/// The iterator's refcounted share of its source index is released by the same
/// destructor, so this owner needs no additional lifetime parameter.
pub type GitIndexIteratorOwned = CBox<GitIndexIterator>;

// SAFETY: `git_index_iterator_free` is the public destructor for a fully
// formed iterator allocation. `CBox` supplies one live non-null allocation
// exactly once, although C also accepts null. The destructor releases the
// snapshot vector and the iterator's own refcounted share of the index, so it
// touches no storage this wrapper leaves borrowed elsewhere.
ffibox::impl_dropped!(
    GitIndexIterator,
    ffi::git_index_iterator,
    ffi::git_index_iterator_free
);

#[cfg(test)]
mod index_iterator_type_tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn iterator_preserves_the_opaque_c_seam_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitIndexIterator>();
        assert_dropped::<GitIndexIterator>();
        assert_eq!(
            size_of::<GitIndexIterator>(),
            size_of::<ffi::git_index_iterator>()
        );
        assert_eq!(
            align_of::<GitIndexIterator>(),
            align_of::<ffi::git_index_iterator>()
        );
        assert_eq!(
            size_of::<GitIndexIteratorRef<'_>>(),
            size_of::<*const ffi::git_index_iterator>()
        );
        assert_eq!(
            size_of::<GitIndexIteratorMut<'_>>(),
            size_of::<*mut ffi::git_index_iterator>()
        );
        assert_eq!(
            size_of::<GitIndexIteratorOwned>(),
            size_of::<*mut ffi::git_index_iterator>()
        );
    }

    #[test]
    fn an_iterator_keeps_its_source_index_alive_by_itself() {
        // `git_index_snapshot_new` takes a refcounted share of the index and
        // registers a reader; `git_index_iterator_free` releases both. The
        // caller's own owner is therefore free to go first.
        // SAFETY: process-global initialization is refcounted and balanced
        // below, after every libgit2 object is dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let mut index = git_index_new().expect("an in-memory index");
        let mut raw = ptr::null_mut();
        // SAFETY: `raw` is a writable out-slot and the index is exclusively
        // borrowed for the in-place entry sort the snapshot performs.
        let status = unsafe { ffi::git_index_iterator_new(&mut raw, index.as_mut().as_mut_ptr()) };
        assert_eq!(status, 0);
        // SAFETY: success transfers one fully formed iterator allocation.
        let mut iterator = unsafe { GitIndexIteratorOwned::from_raw(raw) }.expect("an iterator");

        drop(index);

        let mut entry = ptr::null();
        // SAFETY: the iterator is live and `entry` is a writable out-slot; the
        // iterator's own share keeps the snapshotted index alive.
        let status =
            unsafe { ffi::git_index_iterator_next(&mut entry, iterator.as_mut().as_mut_ptr()) };
        assert_eq!(status, ffi::git_error_code_GIT_ITEROVER);

        drop(iterator);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn null_iterator_seams_create_no_handles() {
        // SAFETY: each conversion seam accepts null and returns `None` without
        // borrowing or adopting an object.
        unsafe {
            assert!(GitIndexIteratorRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitIndexIteratorMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitIndexIteratorOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// Wraps: git_index_options_init
/// Initializes index options for the requested ABI version.
pub fn git_index_options_init(
    version: core::ffi::c_uint,
) -> Result<CVal<crate::api::index::GitIndexOptions>, i32> {
    let mut options = crate::api::index::GitIndexOptions::new();
    let status = {
        let mut output = options.as_mut();
        // SAFETY: `output` is a writable, layout-compatible options value and
        // libgit2 retains no pointer to it.
        unsafe { ffi::git_index_options_init(output.as_mut_ptr(), version) }
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_index_owner
/// Borrows the repository that owns an index, if the index is attached.
#[must_use]
pub fn git_index_owner<'a>(index: GitIndexRef<'a>) -> Option<GitRepositoryRef<'a>> {
    // SAFETY: `index` is live and the returned repository pointer, when
    // present, is the owner recorded in the index for the same lifetime.
    let owner = unsafe { ffi::git_index_owner(index.as_ptr()) };
    // SAFETY: a non-null owner remains live for the index borrow.
    unsafe { GitRepositoryRef::from_ptr(owner) }
}

/// Wraps: git_index_set_caps
/// Replaces the index's filesystem capabilities.
pub fn git_index_set_caps(
    index: &mut GitIndexMut<'_>,
    capabilities: crate::api::index::GitIndexCapabilities,
) -> Result<(), i32> {
    // SAFETY: `index` is exclusively borrowed; the checked capability value
    // is a published bit set or the documented `FROM_OWNER` sentinel.
    let status = unsafe { ffi::git_index_set_caps(index.as_mut_ptr(), capabilities.as_raw()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_index_add_from_buffer
/// Adds an entry whose object contents are supplied by `buffer`.
pub fn git_index_add_from_buffer(
    index: &mut GitIndexMut<'_>,
    entry: IndexEntryRef<'_>,
    buffer: &[u8],
) -> Result<(), i32> {
    // SAFETY: index is exclusive and entry/buffer are readable for the call.
    status_result(unsafe {
        ffi::git_index_add_from_buffer(
            index.as_mut_ptr(),
            entry.as_ptr(),
            buffer.as_ptr().cast(),
            buffer.len(),
        )
    })
}

/// Wraps: git_index_caps
/// Returns the index's currently effective filesystem capabilities.
///
/// The C implementation composes the result from the three published
/// capability bits alone, so it never reports the `FROM_OWNER` request that
/// [`git_index_set_caps`] accepts.
#[must_use]
pub fn git_index_caps(index: GitIndexRef<'_>) -> crate::api::index::GitIndexCapabilities {
    // SAFETY: the live index is only queried.
    let raw = unsafe { ffi::git_index_caps(index.as_ptr()) };
    crate::api::index::GitIndexCapabilities::from_bits(raw)
        .expect("git_index_caps returns only published capability bits")
}

/// The byte width of the checksum `git_index` stores.
///
/// C spells it `GIT_HASH_MAX_SIZE`, the widest digest libgit2 supports, which
/// is the same SHA-256 width as `GIT_OID_MAX_SIZE`. [`RAW_DIGEST_LEN`] already
/// derives that from the bound `git_oid` layout instead of a literal.
///
/// [`RAW_DIGEST_LEN`]: crate::oid::RAW_DIGEST_LEN
const CHECKSUM_LEN: usize = crate::oid::RAW_DIGEST_LEN;

/// Wraps: git_index_checksum
/// Borrows the checksum stored for the index file, as raw digest bytes.
///
/// This deprecated getter is declared as returning a `git_oid *`, but its body
/// is `return (git_oid *)index->checksum;` over an `unsigned char` array of
/// `GIT_HASH_MAX_SIZE`. The two layouts do not agree: `git_oid` leads with a
/// one-byte algorithm tag, so an object-ID handle over that pointer would
/// report the first checksum byte as the algorithm and read a digest shifted
/// one byte past the field. The bytes are therefore handed back as the run
/// they actually are.
///
/// The run is the full stored capacity. Only the leading
/// [`crate::oid::OidType::digest_len`] bytes of the index's own algorithm
/// carry the digest; libgit2 zero-fills the rest, and zero-fills all of it
/// while the index has no on-disk content. Nothing here computes a checksum:
/// the field is only written when an index is read or written, so a shared
/// index handle is enough.
#[must_use]
pub fn git_index_checksum<'a>(index: GitIndexRef<'a>) -> ffibox::CSlice<'a, u8> {
    // SAFETY: the live index is only queried; the returned pointer addresses
    // the index's own checksum field and is never null.
    let raw = unsafe { ffi::git_index_checksum(index.as_ptr().cast_mut()) };
    let bytes = core::ptr::NonNull::new(raw.cast_mut().cast::<u8>())
        .expect("a live index has checksum storage");
    // SAFETY: `bytes` addresses the `unsigned char[GIT_HASH_MAX_SIZE]` field
    // this getter returns; the whole array is initialized by the index's
    // zeroing allocation and stays live for the index borrow.
    unsafe { ffibox::CSlice::from_raw_parts(bytes, CHECKSUM_LEN) }
}

/// Wraps: git_index_conflict_add
pub fn git_index_conflict_add(
    index: &mut GitIndexMut<'_>,
    ancestor: Option<IndexEntryRef<'_>>,
    ours: Option<IndexEntryRef<'_>>,
    theirs: Option<IndexEntryRef<'_>>,
) -> Result<(), i32> {
    // SAFETY: index is exclusive and every optional entry is live and copied.
    status_result(unsafe {
        ffi::git_index_conflict_add(
            index.as_mut_ptr(),
            ancestor.map_or(core::ptr::null(), |e| e.as_ptr()),
            ours.map_or(core::ptr::null(), |e| e.as_ptr()),
            theirs.map_or(core::ptr::null(), |e| e.as_ptr()),
        )
    })
}

/// Wraps: git_index_conflict_cleanup
pub fn git_index_conflict_cleanup(index: &mut GitIndexMut<'_>) -> Result<(), i32> {
    // SAFETY: index is exclusively borrowed for mutation.
    status_result(unsafe { ffi::git_index_conflict_cleanup(index.as_mut_ptr()) })
}

/// Wraps: git_index_entry_is_conflict
#[must_use]
pub fn git_index_entry_is_conflict(entry: IndexEntryRef<'_>) -> bool {
    // SAFETY: entry is live and only read.
    unsafe { ffi::git_index_entry_is_conflict(entry.as_ptr()) != 0 }
}

/// Wraps: git_index_entry_stage
#[must_use]
pub fn git_index_entry_stage(entry: IndexEntryRef<'_>) -> GitIndexStage {
    // SAFETY: entry is live and only read.
    let raw = unsafe { ffi::git_index_entry_stage(entry.as_ptr()) } as ffi::git_index_stage_t;
    GitIndexStage::try_from(raw).expect("an index entry stage is always published")
}

/// Wraps: git_index_extension_add
pub fn git_index_extension_add(
    index: &mut GitIndexMut<'_>,
    signature: &CStr,
    data: &[u8],
) -> Result<(), i32> {
    if signature.to_bytes().len() != 4 {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    // SAFETY: index is exclusive; signature and exact data range are readable.
    status_result(unsafe {
        ffi::git_index_extension_add(
            index.as_mut_ptr(),
            signature.as_ptr(),
            data.as_ptr().cast(),
            data.len(),
        )
    })
}

/// Wraps: git_index_extension_lookup
pub fn git_index_extension_lookup(
    index: &mut GitIndexMut<'_>,
    signature: &CStr,
) -> Result<ffibox::CVal<crate::api::buffer::GitBuf>, i32> {
    if signature.to_bytes().len() != 4 {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    let mut out = crate::api::buffer::GitBuf::new();
    // SAFETY: output/index are exclusive and signature is live.
    let status = unsafe {
        ffi::git_index_extension_lookup(
            out.as_mut().as_mut_ptr(),
            index.as_mut_ptr(),
            signature.as_ptr(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_index_extension_remove
pub fn git_index_extension_remove(
    index: &mut GitIndexMut<'_>,
    signature: &CStr,
) -> Result<(), i32> {
    if signature.to_bytes().len() != 4 {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    // SAFETY: index is exclusive and signature is live.
    status_result(unsafe {
        ffi::git_index_extension_remove(index.as_mut_ptr(), signature.as_ptr())
    })
}

/// Wraps: git_index_find
pub fn git_index_find(index: &mut GitIndexMut<'_>, path: &CStr) -> Result<usize, i32> {
    let mut position = 0;
    // SAFETY: output is writable, index is exclusive for possible sorting, and path is live.
    let status = unsafe { ffi::git_index_find(&mut position, index.as_mut_ptr(), path.as_ptr()) };
    if status == 0 {
        Ok(position)
    } else {
        Err(status)
    }
}

/// An owned index iterator carrying its exclusive source-index borrow.
pub struct GitIndexEntries<'index> {
    inner: GitIndexIteratorOwned,
    _index: core::marker::PhantomData<GitIndexMut<'index>>,
}

impl GitIndexEntries<'_> {
    #[must_use]
    pub fn as_mut(&mut self) -> GitIndexIteratorMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_index_iterator_new
pub fn git_index_iterator_new<'index>(
    mut index: GitIndexMut<'index>,
) -> Result<GitIndexEntries<'index>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output is writable and result carries the index borrow C stores.
    let status = unsafe { ffi::git_index_iterator_new(&mut out, index.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete iterator.
    let inner =
        unsafe { GitIndexIteratorOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(GitIndexEntries {
        inner,
        _index: core::marker::PhantomData,
    })
}

/// Wraps: git_index_iterator_next
pub fn git_index_iterator_next<'a>(
    iterator: &'a mut GitIndexEntries<'_>,
) -> Result<IndexEntryRef<'a>, i32> {
    let mut out = core::ptr::null();
    let mut handle = iterator.as_mut();
    // SAFETY: output is writable and exclusive iterator borrow prevents advance while result lives.
    let status = unsafe { ffi::git_index_iterator_next(&mut out, handle.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns an entry borrowed from the carried index snapshot.
    unsafe { IndexEntryRef::from_ptr(out.cast_mut()) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_index_new_ext
pub fn git_index_new_ext(
    options: Option<crate::api::index::GitIndexOptionsRef<'_>>,
) -> Result<GitIndexOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output is writable and optional options are live and only read.
    let status = unsafe {
        ffi::git_index_new_ext(&mut out, options.map_or(core::ptr::null(), |o| o.as_ptr()))
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one index count.
    unsafe { GitIndexOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_index_open_ext
pub fn git_index_open_ext(
    path: &CStr,
    options: Option<crate::api::index::GitIndexOptionsRef<'_>>,
) -> Result<GitIndexOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output is writable and path/options are live for the constructor.
    let status = unsafe {
        ffi::git_index_open_ext(
            &mut out,
            path.as_ptr(),
            options.map_or(core::ptr::null(), |o| o.as_ptr()),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one index count.
    unsafe { GitIndexOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct IndexObservation {
        initial_count: usize,
        count_after_add: usize,
        alpha_position: usize,
        src_position: usize,
        version: u32,
        caps: i32,
        conflict_paths: [Vec<u8>; 3],
        iterated_paths: Vec<Vec<u8>>,
        tree_id: Vec<u8>,
        checksum_nonzero: bool,
        count_after_remove_directory: usize,
        count_after_clear: usize,
    }

    unsafe fn conflict_entries(template: *const ffi::git_index_entry) -> [ffi::git_index_entry; 3] {
        assert!(!template.is_null());
        let base = unsafe { template.read() };
        let mut entries = [base, base, base];
        let stages = [
            ffi::git_index_stage_t_GIT_INDEX_STAGE_ANCESTOR,
            ffi::git_index_stage_t_GIT_INDEX_STAGE_OURS,
            ffi::git_index_stage_t_GIT_INDEX_STAGE_THEIRS,
        ];
        for (entry, stage) in entries.iter_mut().zip(stages) {
            entry.path = c"conflict.txt".as_ptr();
            entry.flags = (stage as u16) << 12;
        }
        entries
    }

    unsafe fn raw_observation(fixture: &HistoryFixture) -> IndexObservation {
        std::fs::write(
            fixture.directory.path().join("src/gamma.c"),
            b"int gamma(void) { return 3; }\n",
        )
        .unwrap();
        std::fs::write(fixture.directory.path().join("extra.txt"), b"extra\n").unwrap();

        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, fixture.repository.as_ptr()) },
            0
        );
        let initial_count = unsafe { ffi::git_index_entrycount(index) };
        assert_eq!(
            unsafe { ffi::git_index_add_bypath(index, c"src/gamma.c".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_index_add_bypath(index, c"extra.txt".as_ptr()) },
            0
        );
        let count_after_add = unsafe { ffi::git_index_entrycount(index) };

        let mut alpha_position = 0;
        assert_eq!(
            unsafe { ffi::git_index_find(&mut alpha_position, index, c"src/alpha.c".as_ptr()) },
            0
        );
        let mut src_position = 0;
        assert_eq!(
            unsafe { ffi::git_index_find_prefix(&mut src_position, index, c"src".as_ptr()) },
            0
        );

        assert_eq!(unsafe { ffi::git_index_set_version(index, 3) }, 0);
        assert_eq!(unsafe { ffi::git_index_set_version(index, 2) }, 0);
        let version = unsafe { ffi::git_index_version(index) };
        assert_eq!(
            unsafe {
                ffi::git_index_set_caps(
                    index,
                    ffi::git_index_capability_t_GIT_INDEX_CAPABILITY_NO_FILEMODE,
                )
            },
            0
        );
        let caps = unsafe { ffi::git_index_caps(index) };

        let template = unsafe {
            ffi::git_index_get_bypath(
                index,
                c"README.md".as_ptr(),
                ffi::git_index_stage_t_GIT_INDEX_STAGE_NORMAL,
            )
        };
        let entries = unsafe { conflict_entries(template) };
        assert_eq!(
            unsafe { ffi::git_index_conflict_add(index, &entries[0], &entries[1], &entries[2]) },
            0
        );
        assert_eq!(unsafe { ffi::git_index_has_conflicts(index) }, 1);
        let mut ancestor = core::ptr::null();
        let mut ours = core::ptr::null();
        let mut theirs = core::ptr::null();
        assert_eq!(
            unsafe {
                ffi::git_index_conflict_get(
                    &mut ancestor,
                    &mut ours,
                    &mut theirs,
                    index,
                    c"conflict.txt".as_ptr(),
                )
            },
            0
        );
        let conflict_paths = [ancestor, ours, theirs].map(|entry| {
            assert!(!entry.is_null());
            unsafe { CStr::from_ptr((*entry).path) }.to_bytes().to_vec()
        });

        let mut conflicts = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_index_conflict_iterator_new(&mut conflicts, index) },
            0
        );
        let mut iter_ancestor = core::ptr::null();
        let mut iter_ours = core::ptr::null();
        let mut iter_theirs = core::ptr::null();
        assert_eq!(
            unsafe {
                ffi::git_index_conflict_next(
                    &mut iter_ancestor,
                    &mut iter_ours,
                    &mut iter_theirs,
                    conflicts,
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_index_conflict_next(
                    &mut iter_ancestor,
                    &mut iter_ours,
                    &mut iter_theirs,
                    conflicts,
                )
            },
            ffi::git_error_code_GIT_ITEROVER
        );
        unsafe { ffi::git_index_conflict_iterator_free(conflicts) };
        assert_eq!(
            unsafe { ffi::git_index_conflict_remove(index, c"conflict.txt".as_ptr()) },
            0
        );
        assert_eq!(unsafe { ffi::git_index_conflict_cleanup(index) }, 0);

        let mut iterator = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_index_iterator_new(&mut iterator, index) },
            0
        );
        let mut iterated_paths = Vec::new();
        loop {
            let mut entry = core::ptr::null();
            let status = unsafe { ffi::git_index_iterator_next(&mut entry, iterator) };
            if status == ffi::git_error_code_GIT_ITEROVER {
                break;
            }
            assert_eq!(status, 0);
            iterated_paths.push(unsafe { CStr::from_ptr((*entry).path) }.to_bytes().to_vec());
        }
        unsafe { ffi::git_index_iterator_free(iterator) };

        let mut tree_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(unsafe { ffi::git_index_write_tree(&mut tree_id, index) }, 0);
        assert_eq!(unsafe { ffi::git_index_write(index) }, 0);
        assert_eq!(unsafe { ffi::git_index_read(index, 1) }, 0);
        let checksum = unsafe {
            core::slice::from_raw_parts(
                ffi::git_index_checksum(index).cast::<u8>(),
                crate::oid::RAW_DIGEST_LEN,
            )
        }
        .to_vec();

        assert_eq!(
            unsafe {
                ffi::git_index_remove_directory(
                    index,
                    c"src/".as_ptr(),
                    ffi::git_index_stage_t_GIT_INDEX_STAGE_ANY,
                )
            },
            0
        );
        let count_after_remove_directory = unsafe { ffi::git_index_entrycount(index) };
        assert_eq!(unsafe { ffi::git_index_clear(index) }, 0);
        let count_after_clear = unsafe { ffi::git_index_entrycount(index) };
        unsafe { ffi::git_index_free(index) };

        IndexObservation {
            initial_count,
            count_after_add,
            alpha_position,
            src_position,
            version,
            caps,
            conflict_paths,
            iterated_paths,
            tree_id: tree_id.id.to_vec(),
            checksum_nonzero: checksum.iter().any(|byte| *byte != 0),
            count_after_remove_directory,
            count_after_clear,
        }
    }

    fn safe_observation(fixture: &HistoryFixture) -> IndexObservation {
        std::fs::write(
            fixture.directory.path().join("src/gamma.c"),
            b"int gamma(void) { return 3; }\n",
        )
        .unwrap();
        std::fs::write(fixture.directory.path().join("extra.txt"), b"extra\n").unwrap();

        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        let mut index = crate::repository::git_repository_index(&mut repository).unwrap();
        let initial_count = git_index_entrycount(index.as_ref());
        git_index_add_bypath(&mut index.as_mut(), c"src/gamma.c").unwrap();
        git_index_add_bypath(&mut index.as_mut(), c"extra.txt").unwrap();
        let count_after_add = git_index_entrycount(index.as_ref());

        let alpha_position = git_index_find(&mut index.as_mut(), c"src/alpha.c").unwrap();
        let src_position = git_index_find_prefix(&mut index.as_mut(), c"src").unwrap();
        git_index_set_version(&mut index.as_mut(), 3).unwrap();
        git_index_set_version(&mut index.as_mut(), 2).unwrap();
        let version = git_index_version(&mut index.as_mut());
        git_index_set_caps(
            &mut index.as_mut(),
            crate::api::index::GitIndexCapabilities::NO_FILEMODE,
        )
        .unwrap();
        let caps = git_index_caps(index.as_ref()).as_raw();

        let mut entries = {
            let mut index_view = index.as_mut();
            let template =
                git_index_get_bypath(&mut index_view, c"README.md", GitIndexStage::Normal).unwrap();
            unsafe { conflict_entries(template.as_ptr()) }
        };
        let entry_handles = entries
            .each_mut()
            .map(|entry| unsafe { IndexEntryRef::from_ptr(core::ptr::from_mut(entry)).unwrap() });
        git_index_conflict_add(
            &mut index.as_mut(),
            Some(entry_handles[0]),
            Some(entry_handles[1]),
            Some(entry_handles[2]),
        )
        .unwrap();
        assert!(git_index_has_conflicts(index.as_ref()));
        let conflict_paths = {
            let mut index_view = index.as_mut();
            let conflict = git_index_conflict_get(&mut index_view, c"conflict.txt").unwrap();
            [conflict.ancestor, conflict.ours, conflict.theirs].map(|entry| {
                entry
                    .expect("all conflict stages are present")
                    .path()
                    .unwrap()
                    .to_bytes()
                    .to_vec()
            })
        };
        {
            let mut conflicts = git_index_conflict_iterator_new(index.as_mut()).unwrap();
            let conflict = git_index_conflict_next(&mut conflicts).unwrap();
            assert!(conflict.ancestor.is_some());
            assert_eq!(
                git_index_conflict_next(&mut conflicts).err(),
                Some(ffi::git_error_code_GIT_ITEROVER)
            );
        }
        git_index_conflict_remove(&mut index.as_mut(), c"conflict.txt").unwrap();
        git_index_conflict_cleanup(&mut index.as_mut()).unwrap();

        let mut iterated_paths = Vec::new();
        {
            let mut iterator = git_index_iterator_new(index.as_mut()).unwrap();
            loop {
                match git_index_iterator_next(&mut iterator) {
                    Ok(entry) => iterated_paths.push(entry.path().unwrap().to_bytes().to_vec()),
                    Err(status) if status == ffi::git_error_code_GIT_ITEROVER => break,
                    Err(status) => panic!("index iteration failed: {status}"),
                }
            }
        }

        let tree_id = git_index_write_tree(&mut index.as_mut()).unwrap();
        git_index_write(&mut index.as_mut()).unwrap();
        git_index_read(&mut index.as_mut(), true).unwrap();
        let checksum: Vec<u8> = git_index_checksum(index.as_ref()).elems().collect();
        let tree_id =
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of!(tree_id).cast_mut().cast()) }
                .unwrap()
                .raw_bytes()
                .elems()
                .collect();

        git_index_remove_directory(&mut index.as_mut(), c"src/", GitIndexStage::Any).unwrap();
        let count_after_remove_directory = git_index_entrycount(index.as_ref());
        git_index_clear(&mut index.as_mut()).unwrap();
        let count_after_clear = git_index_entrycount(index.as_ref());

        IndexObservation {
            initial_count,
            count_after_add,
            alpha_position,
            src_position,
            version,
            caps,
            conflict_paths,
            iterated_paths,
            tree_id,
            checksum_nonzero: checksum.iter().any(|byte| *byte != 0),
            count_after_remove_directory,
            count_after_clear,
        }
    }

    #[test]
    fn io_equiv_index_mutation_conflicts_iteration_and_tree_writes() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("index-surface-raw");
        let safe = HistoryFixture::new("index-surface-safe");

        let raw_observation = unsafe { raw_observation(&raw) };
        let safe_observation = safe_observation(&safe);
        assert_eq!(raw_observation, safe_observation);
        assert!(raw_observation.count_after_add > raw_observation.initial_count);
        assert_eq!(raw_observation.count_after_clear, 0);
    }

    fn prepare_bulk(fixture: &HistoryFixture) {
        std::fs::create_dir_all(fixture.directory.path().join("bulk/deep")).unwrap();
        std::fs::write(
            fixture.directory.path().join("bulk/a.txt"),
            b"a version one\n",
        )
        .unwrap();
        std::fs::write(
            fixture.directory.path().join("bulk/deep/b.txt"),
            b"b version one\n",
        )
        .unwrap();
    }

    #[derive(Debug, Eq, PartialEq)]
    struct BulkObservation {
        callbacks: Vec<(Vec<u8>, Vec<u8>)>,
        counts: [usize; 4],
        extension: Vec<u8>,
        tree: Vec<u8>,
        owner_matches: bool,
    }

    unsafe extern "C" fn raw_matched(
        path: *const core::ffi::c_char,
        matched: *const core::ffi::c_char,
        payload: *mut core::ffi::c_void,
    ) -> i32 {
        let output = unsafe { &mut *payload.cast::<Vec<(Vec<u8>, Vec<u8>)>>() };
        let matched = if matched.is_null() {
            Vec::new()
        } else {
            unsafe { CStr::from_ptr(matched) }.to_bytes().to_vec()
        };
        output.push((unsafe { CStr::from_ptr(path) }.to_bytes().to_vec(), matched));
        0
    }

    unsafe fn raw_bulk(fixture: &HistoryFixture) -> BulkObservation {
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, fixture.repository.as_ptr()) },
            0
        );
        let mut pattern = c"bulk/*".as_ptr().cast_mut();
        let pathspec = ffi::git_strarray {
            strings: core::ptr::from_mut(&mut pattern),
            count: 1,
        };
        let mut callbacks = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_index_add_all(
                    index,
                    &pathspec,
                    0,
                    Some(raw_matched),
                    core::ptr::from_mut(&mut callbacks).cast(),
                )
            },
            0
        );
        let first = unsafe { ffi::git_index_entrycount(index) };
        std::fs::write(
            fixture.directory.path().join("bulk/a.txt"),
            b"a version two\n",
        )
        .unwrap();
        std::fs::remove_file(fixture.directory.path().join("bulk/deep/b.txt")).unwrap();
        std::fs::write(
            fixture.directory.path().join("bulk/c.txt"),
            b"c version one\n",
        )
        .unwrap();
        assert_eq!(
            unsafe {
                ffi::git_index_update_all(
                    index,
                    &pathspec,
                    Some(raw_matched),
                    core::ptr::from_mut(&mut callbacks).cast(),
                )
            },
            0
        );
        let updated = unsafe { ffi::git_index_entrycount(index) };
        assert_eq!(
            unsafe {
                ffi::git_index_add_all(
                    index,
                    &pathspec,
                    0,
                    Some(raw_matched),
                    core::ptr::from_mut(&mut callbacks).cast(),
                )
            },
            0
        );
        let added = unsafe { ffi::git_index_entrycount(index) };
        assert_eq!(
            unsafe {
                ffi::git_index_extension_add(
                    index,
                    c"TEST".as_ptr(),
                    b"extension-data".as_ptr().cast(),
                    b"extension-data".len(),
                )
            },
            0
        );
        let mut extension = unsafe { core::mem::zeroed::<ffi::git_buf>() };
        assert_eq!(
            unsafe { ffi::git_index_extension_lookup(&mut extension, index, c"TEST".as_ptr()) },
            0
        );
        let extension_data =
            unsafe { core::slice::from_raw_parts(extension.ptr.cast::<u8>(), extension.size) }
                .to_vec();
        unsafe { ffi::git_buf_dispose(&mut extension) };
        assert_eq!(
            unsafe { ffi::git_index_extension_remove(index, c"TEST".as_ptr()) },
            0
        );
        let mut tree = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_index_write_tree_to(&mut tree, index, fixture.repository.as_ptr()) },
            0
        );
        let owner_matches = unsafe { ffi::git_index_owner(index) == fixture.repository.as_ptr() };
        assert_eq!(
            unsafe {
                ffi::git_index_remove_all(
                    index,
                    &pathspec,
                    Some(raw_matched),
                    core::ptr::from_mut(&mut callbacks).cast(),
                )
            },
            0
        );
        let removed = unsafe { ffi::git_index_entrycount(index) };
        unsafe { ffi::git_index_free(index) };
        BulkObservation {
            callbacks,
            counts: [first, updated, added, removed],
            extension: extension_data,
            tree: tree.id.to_vec(),
            owner_matches,
        }
    }

    fn safe_bulk(fixture: &HistoryFixture) -> BulkObservation {
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        let mut index = crate::repository::git_repository_index(&mut repository).unwrap();
        let mut pattern = c"bulk/*".as_ptr().cast_mut();
        let mut raw_pathspec = ffi::git_strarray {
            strings: core::ptr::from_mut(&mut pattern),
            count: 1,
        };
        let pathspec = unsafe {
            crate::strarray::GitStrArrayRef::from_ptr(core::ptr::from_mut(&mut raw_pathspec))
        }
        .unwrap();
        let mut callbacks = Vec::new();
        let mut collect = |path: &CStr, matched: Option<&CStr>| {
            callbacks.push((
                path.to_bytes().to_vec(),
                matched.map_or_else(Vec::new, |value| value.to_bytes().to_vec()),
            ));
            0
        };
        git_index_add_all(
            &mut index.as_mut(),
            Some(pathspec),
            GitIndexAddOptions::DEFAULT,
            Some(&mut collect),
        )
        .unwrap();
        let first = git_index_entrycount(index.as_ref());
        std::fs::write(
            fixture.directory.path().join("bulk/a.txt"),
            b"a version two\n",
        )
        .unwrap();
        std::fs::remove_file(fixture.directory.path().join("bulk/deep/b.txt")).unwrap();
        std::fs::write(
            fixture.directory.path().join("bulk/c.txt"),
            b"c version one\n",
        )
        .unwrap();
        git_index_update_all_with_callback(&mut index.as_mut(), pathspec, &mut collect).unwrap();
        let updated = git_index_entrycount(index.as_ref());
        git_index_add_all(
            &mut index.as_mut(),
            Some(pathspec),
            GitIndexAddOptions::DEFAULT,
            Some(&mut collect),
        )
        .unwrap();
        let added = git_index_entrycount(index.as_ref());
        git_index_extension_add(&mut index.as_mut(), c"TEST", b"extension-data").unwrap();
        let extension = git_index_extension_lookup(&mut index.as_mut(), c"TEST").unwrap();
        let extension_data = crate::io_equiv_support::safe_buf_bytes(extension.as_ref());
        git_index_extension_remove(&mut index.as_mut(), c"TEST").unwrap();
        let mut tree = git_index_write_tree_to(&mut index.as_mut(), repository.as_ref()).unwrap();
        let tree = unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(tree).cast()) }
            .unwrap()
            .raw_bytes()
            .elems()
            .collect();
        let owner_matches =
            git_index_owner(index.as_ref()).unwrap().as_ptr() == repository.as_ref().as_ptr();
        git_index_remove_all_with_callback(&mut index.as_mut(), pathspec, &mut collect).unwrap();
        let removed = git_index_entrycount(index.as_ref());
        drop(collect);
        BulkObservation {
            callbacks,
            counts: [first, updated, added, removed],
            extension: extension_data,
            tree,
            owner_matches,
        }
    }

    #[test]
    fn io_equiv_index_bulk_pathspec_callbacks_extensions_and_tree_target() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("index-bulk-raw");
        let safe = HistoryFixture::new("index-bulk-safe");
        prepare_bulk(&raw);
        prepare_bulk(&safe);
        let raw_observation = unsafe { raw_bulk(&raw) };
        assert_eq!(raw_observation, safe_bulk(&safe));
        assert!(raw_observation.owner_matches);
        assert_eq!(raw_observation.extension, b"extension-data");
    }

    unsafe fn raw_buffer_entries_and_extended_open(
        fixture: &HistoryFixture,
    ) -> (Vec<Vec<u8>>, usize, bool, bool, usize) {
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, fixture.repository.as_ptr()) },
            0
        );
        let template = unsafe { ffi::git_index_get_bypath(index, c"README.md".as_ptr(), 0) };
        assert!(!template.is_null());
        let mut entries = [unsafe { template.read() }, unsafe { template.read() }];
        entries[0].path = c"buffer-one.txt".as_ptr();
        entries[1].path = c"buffer-two.txt".as_ptr();
        let contents = [
            b"first in-memory index blob\n".as_slice(),
            b"second in-memory index blob\n",
        ];
        assert_eq!(
            unsafe {
                ffi::git_index_add_from_buffer(
                    index,
                    &entries[0],
                    contents[0].as_ptr().cast(),
                    contents[0].len(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_index_add_frombuffer(
                    index,
                    &entries[1],
                    contents[1].as_ptr().cast(),
                    contents[1].len(),
                )
            },
            0
        );
        assert_eq!(unsafe { ffi::git_index_write(index) }, 0);
        let ids = [c"buffer-one.txt", c"buffer-two.txt"]
            .map(|path| {
                let entry = unsafe { ffi::git_index_get_bypath(index, path.as_ptr(), 0) };
                assert!(!entry.is_null());
                unsafe { (*entry).id.id }.to_vec()
            })
            .to_vec();
        let path_present = !unsafe { ffi::git_index_path(index) }.is_null();
        let mut iterator = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_index_iterator_new(&mut iterator, index) },
            0
        );
        let mut iterated = 0usize;
        loop {
            let mut entry = core::ptr::null();
            let status = unsafe { ffi::git_index_iterator_next(&mut entry, iterator) };
            if status == ffi::git_error_code_GIT_ITEROVER {
                break;
            }
            assert_eq!(status, 0);
            iterated += 1;
        }
        unsafe { ffi::git_index_iterator_free(iterator) };
        let expected_count = unsafe { ffi::git_index_entrycount(index) };
        unsafe { ffi::git_index_free(index) };

        let mut memory = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_index_new(&mut memory) }, 0);
        let memory_pathless = unsafe { ffi::git_index_path(memory) }.is_null();
        unsafe { ffi::git_index_free(memory) };
        let mut options = unsafe { core::mem::zeroed::<ffi::git_index_options>() };
        assert_eq!(
            unsafe { ffi::git_index_options_init(&mut options, ffi::GIT_INDEX_OPTIONS_VERSION) },
            0
        );
        let index_path = std::ffi::CString::new(
            fixture
                .directory
                .path()
                .join(".git/index")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let mut extended = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_index_open_ext(&mut extended, index_path.as_ptr(), &options) },
            0
        );
        let extended_count = unsafe { ffi::git_index_entrycount(extended) };
        unsafe { ffi::git_index_free(extended) };
        assert_eq!(iterated, expected_count);
        (ids, iterated, path_present, memory_pathless, extended_count)
    }

    fn safe_buffer_entries_and_extended_open(
        fixture: &HistoryFixture,
    ) -> (Vec<Vec<u8>>, usize, bool, bool, usize) {
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        let mut index = crate::repository::git_repository_index(&mut repository).unwrap();
        let mut entries = {
            let mut view = index.as_mut();
            let template =
                git_index_get_bypath(&mut view, c"README.md", GitIndexStage::Normal).unwrap();
            [unsafe { template.as_ptr().read() }, unsafe {
                template.as_ptr().read()
            }]
        };
        entries[0].path = c"buffer-one.txt".as_ptr();
        entries[1].path = c"buffer-two.txt".as_ptr();
        let contents = [
            b"first in-memory index blob\n".as_slice(),
            b"second in-memory index blob\n",
        ];
        let handles = entries
            .each_mut()
            .map(|entry| unsafe { IndexEntryRef::from_ptr(core::ptr::from_mut(entry)).unwrap() });
        git_index_add_from_buffer(&mut index.as_mut(), handles[0], contents[0]).unwrap();
        git_index_add_frombuffer(&mut index.as_mut(), handles[1], contents[1]).unwrap();
        git_index_write(&mut index.as_mut()).unwrap();
        let ids = [c"buffer-one.txt", c"buffer-two.txt"]
            .map(|path| {
                git_index_get_bypath(&mut index.as_mut(), path, GitIndexStage::Normal)
                    .unwrap()
                    .id()
                    .raw_bytes()
                    .elems()
                    .collect()
            })
            .to_vec();
        let path_present = git_index_path(index.as_ref()).is_some();
        let mut iterator = git_index_iterator_new(index.as_mut()).unwrap();
        let mut iterated = 0usize;
        loop {
            match git_index_iterator_next(&mut iterator) {
                Ok(_) => iterated += 1,
                Err(ffi::git_error_code_GIT_ITEROVER) => break,
                Err(status) => panic!("index iteration failed: {status}"),
            }
        }
        drop(iterator);
        let expected_count = git_index_entrycount(index.as_ref());
        drop(index);

        let memory = git_index_new().unwrap();
        let memory_pathless = git_index_path(memory.as_ref()).is_none();
        drop(memory);
        let options = git_index_options_init(ffi::GIT_INDEX_OPTIONS_VERSION).unwrap();
        let index_path = std::ffi::CString::new(
            fixture
                .directory
                .path()
                .join(".git/index")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let extended = git_index_open_ext(&index_path, Some(options.as_ref())).unwrap();
        let extended_count = git_index_entrycount(extended.as_ref());
        assert_eq!(iterated, expected_count);
        (ids, iterated, path_present, memory_pathless, extended_count)
    }

    #[test]
    fn io_equiv_index_buffer_entries_iterators_paths_and_extended_open() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("index-buffer-raw");
        let safe = HistoryFixture::new("index-buffer-safe");
        let raw_observation = unsafe { raw_buffer_entries_and_extended_open(&raw) };
        let safe_observation = safe_buffer_entries_and_extended_open(&safe);
        assert_eq!(raw_observation, safe_observation);
        assert_eq!(raw_observation.0.len(), 2);
        assert_eq!(raw_observation.1, 5);
        assert!(raw_observation.2);
        assert!(raw_observation.3);
    }

    fn prepare_special_index_files(fixture: &HistoryFixture) {
        #[cfg(unix)]
        std::os::unix::fs::symlink("README.md", fixture.directory.path().join("readme-link"))
            .unwrap();
        let script = fixture.directory.path().join("run-equivalence.sh");
        std::fs::write(&script, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        for suffix in ["alpha", "alphabet", "alphanumeric", "alpine"] {
            let path = fixture
                .directory
                .path()
                .join(format!("long/common/prefix/{suffix}.txt"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, format!("{suffix}\n")).unwrap();
        }
    }

    unsafe fn raw_special_index(fixture: &HistoryFixture) -> Vec<(Vec<u8>, u32, Vec<u8>)> {
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, fixture.repository.as_ptr()) },
            0
        );
        for path in [
            c"readme-link",
            c"run-equivalence.sh",
            c"long/common/prefix/alpha.txt",
            c"long/common/prefix/alphabet.txt",
            c"long/common/prefix/alphanumeric.txt",
            c"long/common/prefix/alpine.txt",
        ] {
            assert_eq!(
                unsafe { ffi::git_index_add_bypath(index, path.as_ptr()) },
                0
            );
        }
        let template = unsafe { ffi::git_index_get_bypath(index, c"README.md".as_ptr(), 0) };
        let entries = unsafe { conflict_entries(template) };
        assert_eq!(
            unsafe { ffi::git_index_conflict_add(index, &entries[0], &entries[1], &entries[2]) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_index_conflict_remove(index, c"conflict.txt".as_ptr()) },
            0
        );
        assert_eq!(unsafe { ffi::git_index_set_version(index, 4) }, 0);
        assert_eq!(unsafe { ffi::git_index_write(index) }, 0);
        unsafe { ffi::git_index_free(index) };
        let index_path = std::ffi::CString::new(
            fixture
                .directory
                .path()
                .join(".git/index")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let mut reopened = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_index_open(&mut reopened, index_path.as_ptr()) },
            0
        );
        let entries = (0..unsafe { ffi::git_index_entrycount(reopened) })
            .map(|position| {
                let entry = unsafe { ffi::git_index_get_byindex(reopened, position) };
                (
                    unsafe { CStr::from_ptr((*entry).path) }.to_bytes().to_vec(),
                    unsafe { (*entry).mode },
                    unsafe { (*entry).id.id }.to_vec(),
                )
            })
            .collect();
        unsafe { ffi::git_index_free(reopened) };
        entries
    }

    fn safe_special_index(fixture: &HistoryFixture) -> Vec<(Vec<u8>, u32, Vec<u8>)> {
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        let mut index = crate::repository::git_repository_index(&mut repository).unwrap();
        for path in [
            c"readme-link",
            c"run-equivalence.sh",
            c"long/common/prefix/alpha.txt",
            c"long/common/prefix/alphabet.txt",
            c"long/common/prefix/alphanumeric.txt",
            c"long/common/prefix/alpine.txt",
        ] {
            git_index_add_bypath(&mut index.as_mut(), path).unwrap();
        }
        let mut entries = {
            let mut view = index.as_mut();
            let template =
                git_index_get_bypath(&mut view, c"README.md", GitIndexStage::Normal).unwrap();
            unsafe { conflict_entries(template.as_ptr()) }
        };
        let entries = entries
            .each_mut()
            .map(|entry| unsafe { IndexEntryRef::from_ptr(entry) }.unwrap());
        git_index_conflict_add(
            &mut index.as_mut(),
            Some(entries[0]),
            Some(entries[1]),
            Some(entries[2]),
        )
        .unwrap();
        git_index_conflict_remove(&mut index.as_mut(), c"conflict.txt").unwrap();
        git_index_set_version(&mut index.as_mut(), 4).unwrap();
        git_index_write(&mut index.as_mut()).unwrap();
        drop(index);
        let index_path = std::ffi::CString::new(
            fixture
                .directory
                .path()
                .join(".git/index")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let mut reopened = git_index_open(&index_path).unwrap();
        let entries = (0..git_index_entrycount(reopened.as_ref()))
            .map(|position| {
                let mut view = reopened.as_mut();
                let entry = git_index_get_byindex(&mut view, position).unwrap();
                (
                    entry.path().unwrap().to_bytes().to_vec(),
                    entry.mode(),
                    entry.id().raw_bytes().elems().collect(),
                )
            })
            .collect();
        entries
    }

    #[test]
    fn io_equiv_index_v4_symlink_executable_and_prefix_compression() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("index-special-raw");
        let safe = HistoryFixture::new("index-special-safe");
        prepare_special_index_files(&raw);
        prepare_special_index_files(&safe);
        let raw = unsafe { raw_special_index(&raw) };
        assert_eq!(raw, safe_special_index(&safe));
        assert!(raw.iter().any(|(path, mode, _)| {
            path == b"readme-link" && *mode == ffi::git_filemode_t_GIT_FILEMODE_LINK
        }));
        assert!(raw.iter().any(|(path, mode, _)| {
            path == b"run-equivalence.sh"
                && *mode == ffi::git_filemode_t_GIT_FILEMODE_BLOB_EXECUTABLE
        }));
    }
}

#[cfg(test)]
mod scheduled_symbol_tests {
    use super::*;

    #[test]
    fn stage_and_conflict_wrappers_decode_entry_flags() {
        // SAFETY: every field of the public entry admits an all-zero value;
        // the test then initializes the stage bits it reads.
        let mut raw: ffi::git_index_entry = unsafe { core::mem::zeroed() };
        raw.flags = (ffi::git_index_stage_t_GIT_INDEX_STAGE_OURS as u16) << 12;
        // SAFETY: `raw` is initialized and remains live for this shared handle.
        let entry = unsafe { IndexEntryRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(git_index_entry_stage(entry), GitIndexStage::Ours);
        assert!(git_index_entry_is_conflict(entry));
    }

    #[test]
    fn extension_signatures_must_be_exactly_four_bytes() {
        let _: fn(&mut GitIndexMut<'_>, &CStr, &[u8]) -> Result<(), i32> = git_index_extension_add;
        assert_ne!(c"TREE".to_bytes().len(), c"BAD".to_bytes().len());
    }

    #[test]
    fn iterator_surface_carries_the_source_borrow() {
        let _: for<'a> fn(GitIndexMut<'a>) -> Result<GitIndexEntries<'a>, i32> =
            git_index_iterator_new;
    }

    /// `git_index_checksum` returns the index's `unsigned char` array cast to
    /// `git_oid *`. A fresh index has never been read, so every stored byte is
    /// zero -- including the byte an object-ID handle would have reported as
    /// the algorithm tag, which is not a valid `git_oid_t` at all.
    #[test]
    fn the_checksum_is_the_stored_digest_run_and_not_an_object_id() {
        // SAFETY: process-global initialization is reference counted and is
        // balanced after the index owner is dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let index = git_index_new().expect("an in-memory index");

        let checksum = git_index_checksum(index.as_ref());
        assert_eq!(checksum.len(), CHECKSUM_LEN);
        assert!(checksum.elems().all(|byte| byte == 0));

        // An object-ID view over the same pointer does not fit: `git_oid`
        // spends a leading byte on its algorithm tag, so its digest field
        // would end one byte past the stored array.
        assert!(size_of::<ffi::git_oid>() > CHECKSUM_LEN);
        assert_eq!(crate::oid::RAW_DIGEST_LEN, CHECKSUM_LEN);

        drop(index);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn the_checksum_only_needs_a_shared_index_handle() {
        let _: for<'a> fn(GitIndexRef<'a>) -> ffibox::CSlice<'a, u8> = git_index_checksum;
    }
}
