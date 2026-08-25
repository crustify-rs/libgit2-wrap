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
    /// The iterator stores the index pointer supplied to its constructor
    /// without taking another ordinary owner. Safe construction must therefore
    /// carry an exclusive borrow of that index until the iterator is dropped.
    GitIndexIterator,
    GitIndexIteratorRef,
    GitIndexIteratorMut,
    ffi::git_index_iterator
);

/// Wraps: git_index_iterator_free
/// A raw owning iterator allocation.
///
/// This ownership building block does not encode the iterator's borrow of its
/// source index. A safe constructor must place it in a lifetime-carrying owner
/// before returning it.
pub type GitIndexIteratorOwned = CBox<GitIndexIterator>;

// SAFETY: `git_index_iterator_free` is the public destructor for a fully
// formed iterator allocation. `CBox` supplies one live non-null allocation
// exactly once. The destructor releases the iterator's snapshot through its
// borrowed index, whose lifetime must be carried by every safe constructor.
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
