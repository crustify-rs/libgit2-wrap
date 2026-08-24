//! Safe wrappers for libgit2 reflog APIs.

use core::ffi::CStr;

use ffibox::CBox;

use crate::ffi;
use crate::oid::OidRef;
use crate::repository::GitRepositoryMut;

ffibox::define_ctype!(
    /// Wraps: git_reflog
    /// An opaque in-memory reference log owned by libgit2.
    ///
    /// Owned pointers use [`GitReflogOwned`]. Dropping an owner releases its
    /// entries, reference name, retained reference-database count, and header.
    GitReflog,
    GitReflogRef,
    GitReflogMut,
    ffi::git_reflog
);

/// Wraps: git_reflog_free
/// An owning handle that frees a fully formed libgit2 reflog on drop.
pub type GitReflogOwned = CBox<GitReflog>;

// SAFETY: `git_reflog_free` is the public destructor for a fully formed,
// ordinary libgit2 reflog. It accepts null, although `CBox` supplies one live
// non-null allocation exactly once, and releases all fields before the header.
ffibox::impl_dropped!(GitReflog, ffi::git_reflog, ffi::git_reflog_free);

ffibox::define_ctype!(
    /// Wraps: git_reflog_entry
    /// An opaque reflog entry borrowed from its containing [`GitReflog`].
    ///
    /// Libgit2's public API does not transfer ownership of individual entries;
    /// callers receive a shared handle whose lifetime remains tied to the log.
    GitReflogEntry,
    GitReflogEntryRef,
    GitReflogEntryMut,
    ffi::git_reflog_entry
);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn wrappers_preserve_the_opaque_ffi_seams() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}
        fn assert_copy<T: Copy>() {}

        assert_cell::<GitReflog>();
        assert_cell::<GitReflogEntry>();
        assert_dropped::<GitReflog>();
        assert_copy::<GitReflogRef<'_>>();
        assert_copy::<GitReflogEntryRef<'_>>();

        assert_eq!(size_of::<GitReflog>(), size_of::<ffi::git_reflog>());
        assert_eq!(align_of::<GitReflog>(), align_of::<ffi::git_reflog>());
        assert_eq!(
            size_of::<GitReflogRef<'_>>(),
            size_of::<*const ffi::git_reflog>()
        );
        assert_eq!(
            size_of::<GitReflogMut<'_>>(),
            size_of::<*mut ffi::git_reflog>()
        );
        assert_eq!(
            size_of::<GitReflogOwned>(),
            size_of::<*mut ffi::git_reflog>()
        );

        assert_eq!(
            size_of::<GitReflogEntry>(),
            size_of::<ffi::git_reflog_entry>()
        );
        assert_eq!(
            align_of::<GitReflogEntry>(),
            align_of::<ffi::git_reflog_entry>()
        );
        assert_eq!(
            size_of::<GitReflogEntryRef<'_>>(),
            size_of::<*const ffi::git_reflog_entry>()
        );
        assert_eq!(
            size_of::<GitReflogEntryMut<'_>>(),
            size_of::<*mut ffi::git_reflog_entry>()
        );
    }

    #[test]
    fn null_reflog_seams_create_no_handle() {
        // SAFETY: all conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitReflogRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitReflogMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitReflogOwned::from_raw(ptr::null_mut()).is_none());
            assert!(GitReflogEntryRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitReflogEntryMut::from_ptr(ptr::null_mut()).is_none());
        }
    }
}

/// Wraps: git_reflog_delete
/// Deletes the persisted reflog named by `name`.
pub fn git_reflog_delete(repo: &mut GitRepositoryMut<'_>, name: &CStr) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusively borrowed and `name` is
    // a live NUL-terminated string retained only for this call.
    let status = unsafe { ffi::git_reflog_delete(repo.as_mut_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_reflog_drop
/// Removes an in-memory entry, optionally repairing the previous entry.
pub fn git_reflog_drop(
    reflog: &mut GitReflogMut<'_>,
    index: usize,
    rewrite_previous_entry: bool,
) -> Result<(), i32> {
    // SAFETY: `reflog` is live and exclusively borrowed; the scalar arguments
    // carry no additional validity requirements.
    let status = unsafe {
        ffi::git_reflog_drop(
            reflog.as_mut_ptr(),
            index,
            i32::from(rewrite_previous_entry),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_reflog_entry_byindex
/// Borrows the entry at reverse-chronological `index`.
#[must_use]
pub fn git_reflog_entry_byindex<'a>(
    reflog: GitReflogRef<'a>,
    index: usize,
) -> Option<GitReflogEntryRef<'a>> {
    // SAFETY: `reflog` is live and shared; a non-null result remains owned by
    // that reflog for the returned handle's lifetime.
    let entry = unsafe { ffi::git_reflog_entry_byindex(reflog.as_ptr(), index) }.cast_mut();
    // SAFETY: the non-null case is the borrowed entry described above.
    unsafe { GitReflogEntryRef::from_ptr(entry) }
}

/// Wraps: git_reflog_entry_id_new
/// Borrows the entry's new object ID.
#[must_use]
pub fn git_reflog_entry_id_new<'a>(entry: GitReflogEntryRef<'a>) -> OidRef<'a> {
    // SAFETY: `entry` is live and the getter returns its non-null inline OID.
    let oid = unsafe { ffi::git_reflog_entry_id_new(entry.as_ptr()) }.cast_mut();
    // SAFETY: a valid entry always contains this inline OID.
    unsafe { OidRef::from_ptr(oid) }.expect("a reflog entry has an inline new OID")
}

/// Wraps: git_reflog_entry_id_old
/// Borrows the entry's previous object ID.
#[must_use]
pub fn git_reflog_entry_id_old<'a>(entry: GitReflogEntryRef<'a>) -> OidRef<'a> {
    // SAFETY: `entry` is live and the getter returns its non-null inline OID.
    let oid = unsafe { ffi::git_reflog_entry_id_old(entry.as_ptr()) }.cast_mut();
    // SAFETY: a valid entry always contains this inline OID.
    unsafe { OidRef::from_ptr(oid) }.expect("a reflog entry has an inline old OID")
}

/// Wraps: git_reflog_entry_message
/// Borrows the optional message owned by an entry.
#[must_use]
pub fn git_reflog_entry_message<'a>(entry: GitReflogEntryRef<'a>) -> Option<&'a CStr> {
    // SAFETY: the getter returns null or an entry-owned NUL-terminated string
    // that remains live for the entry borrow.
    let message = unsafe { ffi::git_reflog_entry_message(entry.as_ptr()) };
    if message.is_null() {
        None
    } else {
        // SAFETY: non-null is the live NUL-terminated message described above.
        Some(unsafe { CStr::from_ptr(message) })
    }
}

/// Wraps: git_reflog_entrycount
/// Returns the number of entries currently held by the reflog.
#[must_use]
pub fn git_reflog_entrycount(reflog: GitReflogRef<'_>) -> usize {
    // SAFETY: source inspection shows this legacy mutable parameter is only
    // read, so a shared handle is sufficient and no pointer is retained.
    unsafe { ffi::git_reflog_entrycount(reflog.as_ptr().cast_mut()) }
}

/// Wraps: git_reflog_read
/// Loads an independently owned reflog from a repository.
pub fn git_reflog_read(
    repo: &mut GitRepositoryMut<'_>,
    name: &CStr,
) -> Result<GitReflogOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: `raw` is writable, the repository is live and exclusive, and
    // `name` is a live string. On success the result owns its refdb count.
    let status = unsafe { ffi::git_reflog_read(&mut raw, repo.as_mut_ptr(), name.as_ptr()) };
    if status == 0 {
        // SAFETY: success transfers a complete reflog allocation.
        Ok(unsafe { GitReflogOwned::from_raw(raw) }
            .expect("libgit2 succeeded without returning a reflog"))
    } else {
        if !raw.is_null() {
            // SAFETY: a populated error output remains caller-owned.
            drop(unsafe { GitReflogOwned::from_raw(raw) });
        }
        Err(status)
    }
}

/// Wraps: git_reflog_rename
/// Renames a persisted reflog.
pub fn git_reflog_rename(
    repo: &mut GitRepositoryMut<'_>,
    old_name: &CStr,
    new_name: &CStr,
) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusive and both strings remain
    // live for the call; none of these pointers is retained.
    let status =
        unsafe { ffi::git_reflog_rename(repo.as_mut_ptr(), old_name.as_ptr(), new_name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_reflog_write
/// Atomically writes the in-memory reflog back to its backend.
pub fn git_reflog_write(reflog: &mut GitReflogMut<'_>) -> Result<(), i32> {
    // SAFETY: `reflog` is live and exclusively borrowed for backend mutation.
    let status = unsafe { ffi::git_reflog_write(reflog.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}
