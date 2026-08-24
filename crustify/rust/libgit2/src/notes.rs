//! Safe wrappers for libgit2 notes APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::addr_of_mut;

use ffibox::{CBox, CVal};

use crate::api::buffer::GitBuf;
use crate::ffi;
use crate::iterator::{GitIterator, GitIteratorMut};
use crate::oid::{Oid, OidRef};
use crate::repository::GitRepositoryRef;

ffibox::define_ctype!(
    /// Wraps: git_note
    /// An opaque note allocation owned by libgit2.
    ///
    /// Owned pointers use [`GitNoteOwned`]. Dropping an owner releases the
    /// note's duplicated signatures and message along with the allocation.
    GitNote,
    GitNoteRef,
    GitNoteMut,
    ffi::git_note
);

/// An owning handle to a fully formed libgit2 note.
pub type GitNoteOwned = CBox<GitNote>;

/// A notes iterator whose repository is kept live by its Rust lifetime.
pub struct GitNoteIterator<'repo> {
    inner: CBox<GitIterator>,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl GitNoteIterator<'_> {
    /// Borrows the iterator exclusively.
    pub fn as_mut(&mut self) -> GitIteratorMut<'_> {
        self.inner.as_mut()
    }
}

// SAFETY: `git_note_free` is the public destructor for a fully formed
// `git_note`. It accepts null, although `CBox` supplies one live non-null
// allocation exactly once, and releases all owned fields before the header.
ffibox::impl_dropped!(GitNote, ffi::git_note, ffi::git_note_free);

/// Wraps: git_note_default_ref
/// Resolves the configured default notes reference into an owned buffer.
pub fn git_note_default_ref(repo: GitRepositoryRef<'_>) -> Result<CVal<GitBuf>, i32> {
    let mut out = GitBuf::new();
    let status = {
        let mut output = out.as_mut();
        // SAFETY: `output` is a valid writable buffer header, `repo` is live,
        // and libgit2 retains neither pointer.
        unsafe { ffi::git_note_default_ref(output.as_mut_ptr(), repo.as_ptr().cast_mut()) }
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_note_free
/// Consumes an owned note.
pub fn git_note_free(note: GitNoteOwned) {
    drop(note);
}

/// Wraps: git_note_id
/// Borrows the note object's ID.
pub fn git_note_id<'a>(note: GitNoteRef<'a>) -> OidRef<'a> {
    // SAFETY: `note` is live and libgit2 returns its non-null inline ID.
    let id = unsafe { ffi::git_note_id(note.as_ptr()) };
    // SAFETY: the returned inline field remains live for the note borrow.
    unsafe { OidRef::from_ptr(id.cast_mut()) }.expect("a live note has an ID")
}

/// Wraps: git_note_iterator_free
/// Consumes a scoped note iterator.
pub fn git_note_iterator_free(iterator: GitNoteIterator<'_>) {
    drop(iterator);
}

/// Wraps: git_note_iterator_new
/// Creates an iterator tied to the repository it consults.
pub fn git_note_iterator_new<'repo>(
    repo: GitRepositoryRef<'repo>,
    notes_ref: Option<&CStr>,
) -> Result<GitNoteIterator<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: `raw` is writable, optional reference is null or a live C
    // string, and the returned iterator is tied to `repo` below.
    let status = unsafe {
        ffi::git_note_iterator_new(
            addr_of_mut!(raw),
            repo.as_ptr().cast_mut(),
            notes_ref.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes one newly owned complete iterator.
    let inner = unsafe { CBox::<GitIterator>::from_raw(raw) }
        .expect("successful note iterator construction returns non-null");
    Ok(GitNoteIterator {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_note_message
/// Borrows the note's NUL-terminated message.
pub fn git_note_message<'a>(note: GitNoteRef<'a>) -> &'a CStr {
    // SAFETY: a live note owns a non-null NUL-terminated message for its
    // lifetime.
    unsafe { CStr::from_ptr(ffi::git_note_message(note.as_ptr())) }
}

/// Wraps: git_note_next
/// Returns the current note and annotated-object IDs, then advances.
pub fn git_note_next(iterator: &mut GitNoteIterator<'_>) -> Result<(Oid, Oid), i32> {
    let mut note = Oid::zeroed();
    let mut annotated = Oid::zeroed();
    let mut it = iterator.as_mut();
    // SAFETY: both outputs are writable and `it` is exclusively borrowed for
    // the operation, which may advance its state.
    let status = unsafe {
        ffi::git_note_next(
            addr_of_mut!(note).cast(),
            addr_of_mut!(annotated).cast(),
            it.as_mut_ptr(),
        )
    };
    if status == 0 {
        Ok((note, annotated))
    } else {
        Err(status)
    }
}

/// Wraps: git_note_read
/// Reads an independently owned note for `oid`.
pub fn git_note_read(
    repo: GitRepositoryRef<'_>,
    notes_ref: Option<&CStr>,
    oid: OidRef<'_>,
) -> Result<GitNoteOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: `raw` is writable and all borrowed arguments remain live for
    // the call; libgit2 returns an independent note allocation.
    let status = unsafe {
        ffi::git_note_read(
            addr_of_mut!(raw),
            repo.as_ptr().cast_mut(),
            notes_ref.map_or(core::ptr::null(), CStr::as_ptr),
            oid.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success yields one fully formed owned note.
    unsafe { GitNoteOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn note_wrapper_preserves_the_opaque_seam_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitNote>();
        assert_dropped::<GitNote>();
        assert_eq!(size_of::<GitNote>(), size_of::<ffi::git_note>());
        assert_eq!(align_of::<GitNote>(), align_of::<ffi::git_note>());
        assert_eq!(
            size_of::<GitNoteRef<'_>>(),
            size_of::<*const ffi::git_note>()
        );
        assert_eq!(size_of::<GitNoteMut<'_>>(), size_of::<*mut ffi::git_note>());
        assert_eq!(size_of::<GitNoteOwned>(), size_of::<*mut ffi::git_note>());
    }

    #[test]
    fn null_note_seams_create_no_handle() {
        // SAFETY: all conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitNoteRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitNoteMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitNoteOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
