//! Safe wrappers for libgit2 notes APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::addr_of_mut;

use ffibox::{CBox, CVal};

use crate::api::buffer::GitBuf;
use crate::api::notes::GitNoteForeachCallback;
use crate::api::types::GitSignatureRef;
use crate::commit::GitCommitRef;
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
#[allow(clippy::items_after_test_module)]
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

/// Wraps: git_note_author
/// Borrows the note author's signature.
pub fn git_note_author<'a>(note: GitNoteRef<'a>) -> GitSignatureRef<'a> {
    // SAFETY: a live note owns a complete non-null author signature for its
    // lifetime.
    let signature = unsafe { ffi::git_note_author(note.as_ptr()) };
    // SAFETY: the returned signature is note-owned and bounded by `'a`.
    unsafe { GitSignatureRef::from_ptr(signature.cast_mut()) }.expect("a live note has an author")
}

/// Wraps: git_note_committer
/// Borrows the note committer's signature.
pub fn git_note_committer<'a>(note: GitNoteRef<'a>) -> GitSignatureRef<'a> {
    // SAFETY: a live note owns a complete non-null committer signature for its
    // lifetime.
    let signature = unsafe { ffi::git_note_committer(note.as_ptr()) };
    // SAFETY: the returned signature is note-owned and bounded by `'a`.
    unsafe { GitSignatureRef::from_ptr(signature.cast_mut()) }.expect("a live note has a committer")
}

/// Wraps: git_note_create
/// Creates or replaces a note and returns the new notes commit ID.
pub fn git_note_create(
    repo: GitRepositoryRef<'_>,
    notes_ref: Option<&CStr>,
    author: GitSignatureRef<'_>,
    committer: GitSignatureRef<'_>,
    oid: OidRef<'_>,
    note: &CStr,
    force: bool,
) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable and all optional/non-null borrowed arguments
    // remain live for this synchronous operation; C copies the note text.
    let status = unsafe {
        ffi::git_note_create(
            addr_of_mut!(out).cast(),
            repo.as_ptr().cast_mut(),
            notes_ref.map_or(core::ptr::null(), CStr::as_ptr),
            author.as_ptr(),
            committer.as_ptr(),
            oid.as_ptr(),
            note.as_ptr(),
            i32::from(force),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_note_remove
/// Removes the note attached to an object.
pub fn git_note_remove(
    repo: GitRepositoryRef<'_>,
    notes_ref: Option<&CStr>,
    author: GitSignatureRef<'_>,
    committer: GitSignatureRef<'_>,
    oid: OidRef<'_>,
) -> Result<(), i32> {
    // SAFETY: all handles and the optional C string stay live throughout the
    // synchronous commit update and C retains none of their pointers.
    let status = unsafe {
        ffi::git_note_remove(
            repo.as_ptr().cast_mut(),
            notes_ref.map_or(core::ptr::null(), CStr::as_ptr),
            author.as_ptr(),
            committer.as_ptr(),
            oid.as_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_note_commit_create
/// Creates a notes commit directly from an optional parent commit.
pub fn git_note_commit_create(
    repo: GitRepositoryRef<'_>,
    parent: Option<GitCommitRef<'_>>,
    author: GitSignatureRef<'_>,
    committer: GitSignatureRef<'_>,
    oid: OidRef<'_>,
    note: &CStr,
    allow_overwrite: bool,
) -> Result<(Oid, Oid), i32> {
    let mut commit_id = Oid::zeroed();
    let mut blob_id = Oid::zeroed();
    // SAFETY: both outputs are writable; every handle and string remains live
    // for the synchronous call, and C copies the note text.
    let status = unsafe {
        ffi::git_note_commit_create(
            addr_of_mut!(commit_id).cast(),
            addr_of_mut!(blob_id).cast(),
            repo.as_ptr().cast_mut(),
            parent.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut()),
            author.as_ptr(),
            committer.as_ptr(),
            oid.as_ptr(),
            note.as_ptr(),
            i32::from(allow_overwrite),
        )
    };
    if status == 0 {
        Ok((commit_id, blob_id))
    } else {
        Err(status)
    }
}

/// Wraps: git_note_commit_iterator_new
/// Creates a notes iterator whose validity is tied to the source commit.
pub fn git_note_commit_iterator_new<'commit>(
    notes_commit: GitCommitRef<'commit>,
) -> Result<GitNoteIterator<'commit>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: `raw` is writable and `notes_commit` remains live for the call;
    // success transfers one complete iterator owner.
    let status = unsafe {
        ffi::git_note_commit_iterator_new(addr_of_mut!(raw), notes_commit.as_ptr().cast_mut())
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one complete owned iterator.
    let inner = unsafe { CBox::<GitIterator>::from_raw(raw) }
        .expect("successful note iterator construction returns non-null");
    Ok(GitNoteIterator {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_note_commit_read
/// Reads an independently owned note from a notes commit.
pub fn git_note_commit_read(
    repo: GitRepositoryRef<'_>,
    notes_commit: GitCommitRef<'_>,
    oid: OidRef<'_>,
) -> Result<GitNoteOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output is writable and all input handles remain live for
    // the call; success transfers one complete note allocation.
    let status = unsafe {
        ffi::git_note_commit_read(
            addr_of_mut!(raw),
            repo.as_ptr().cast_mut(),
            notes_commit.as_ptr().cast_mut(),
            oid.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one complete owned note.
    unsafe { GitNoteOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_note_commit_remove
/// Removes a note and returns the newly created notes commit ID.
pub fn git_note_commit_remove(
    repo: GitRepositoryRef<'_>,
    notes_commit: GitCommitRef<'_>,
    author: GitSignatureRef<'_>,
    committer: GitSignatureRef<'_>,
    oid: OidRef<'_>,
) -> Result<Oid, i32> {
    let mut commit_id = Oid::zeroed();
    // SAFETY: the output is writable and every borrowed handle remains live
    // for the synchronous commit rewrite.
    let status = unsafe {
        ffi::git_note_commit_remove(
            addr_of_mut!(commit_id).cast(),
            repo.as_ptr().cast_mut(),
            notes_commit.as_ptr().cast_mut(),
            author.as_ptr(),
            committer.as_ptr(),
            oid.as_ptr(),
        )
    };
    if status == 0 {
        Ok(commit_id)
    } else {
        Err(status)
    }
}

unsafe extern "C" fn note_foreach_trampoline<C: GitNoteForeachCallback>(
    blob_id: *const ffi::git_oid,
    annotated_id: *const ffi::git_oid,
    payload: *mut core::ffi::c_void,
) -> i32 {
    if blob_id.is_null() || annotated_id.is_null() || payload.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: the wrapper installs the exact callback object for this
    // synchronous traversal and C does not retain the payload.
    let callback = unsafe { &mut *payload.cast::<C>() };
    // SAFETY: libgit2 supplies complete non-null transient IDs.
    let blob = unsafe { OidRef::from_ptr(blob_id.cast_mut()) }.expect("checked non-null");
    // SAFETY: as above, for the annotated-object ID.
    let annotated = unsafe { OidRef::from_ptr(annotated_id.cast_mut()) }.expect("checked non-null");
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        callback.call(blob, annotated)
    }))
    .unwrap_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_note_foreach
/// Visits every note in the selected namespace.
pub fn git_note_foreach<C: GitNoteForeachCallback>(
    repo: GitRepositoryRef<'_>,
    notes_ref: Option<&CStr>,
    callback: &mut C,
) -> Result<(), i32> {
    // SAFETY: all inputs remain live for the synchronous traversal; the
    // trampoline and payload have matching monomorphized types.
    let status = unsafe {
        ffi::git_note_foreach(
            repo.as_ptr().cast_mut(),
            notes_ref.map_or(core::ptr::null(), CStr::as_ptr),
            Some(note_foreach_trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    unsafe fn raw_notes(repository: *mut ffi::git_repository) -> (Vec<u8>, Vec<u8>, usize) {
        let mut target = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut target, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut signature = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_signature_new(
                    &mut signature,
                    c"Crustify".as_ptr(),
                    c"crustify@example.com".as_ptr(),
                    1_700_000_300,
                    0,
                )
            },
            0
        );
        let mut commit = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_note_create(
                    &mut commit,
                    repository,
                    core::ptr::null(),
                    signature,
                    signature,
                    &target,
                    c"reviewed by equivalence test".as_ptr(),
                    0,
                )
            },
            0
        );
        let mut note = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_note_read(&mut note, repository, core::ptr::null(), &target) },
            0
        );
        let message = unsafe { CStr::from_ptr(ffi::git_note_message(note)) }
            .to_bytes()
            .to_vec();
        unsafe extern "C" fn count(
            _blob: *const ffi::git_oid,
            _target: *const ffi::git_oid,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            unsafe { *payload.cast::<usize>() += 1 };
            0
        }
        let mut notes = 0usize;
        assert_eq!(
            unsafe {
                ffi::git_note_foreach(
                    repository,
                    core::ptr::null(),
                    Some(count),
                    core::ptr::from_mut(&mut notes).cast(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_note_remove(repository, core::ptr::null(), signature, signature, &target)
            },
            0
        );
        unsafe {
            ffi::git_note_free(note);
            ffi::git_signature_free(signature);
        }
        (commit.id.to_vec(), message, notes)
    }

    fn safe_notes(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
    ) -> (Vec<u8>, Vec<u8>, usize) {
        let mut target = crate::refs::git_reference_name_to_id(repository, c"HEAD").unwrap();
        let target = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(target).cast()) }.unwrap();
        let signature = crate::signature::git_signature_new(
            c"Crustify",
            c"crustify@example.com",
            1_700_000_300,
            0,
        )
        .unwrap();
        let mut commit = git_note_create(
            repository.as_ref(),
            None,
            signature.as_ref(),
            signature.as_ref(),
            target,
            c"reviewed by equivalence test",
            false,
        )
        .unwrap();
        let note = git_note_read(repository.as_ref(), None, target).unwrap();
        let message = git_note_message(note.as_ref()).to_bytes().to_vec();
        let mut notes = 0usize;
        git_note_foreach(
            repository.as_ref(),
            None,
            &mut |_blob: OidRef<'_>, _target: OidRef<'_>| {
                notes += 1;
                0
            },
        )
        .unwrap();
        git_note_remove(
            repository.as_ref(),
            None,
            signature.as_ref(),
            signature.as_ref(),
            target,
        )
        .unwrap();
        let commit = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(commit).cast()) }.unwrap();
        (commit.raw_bytes().elems().collect(), message, notes)
    }

    #[test]
    fn io_equiv_note_create_read_foreach_and_remove() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("notes-raw");
        let safe = HistoryFixture::new("notes-safe");
        let raw_observation = unsafe { raw_notes(raw.repository.as_ptr()) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(raw_observation, safe_notes(&mut safe_repository));
        assert_eq!(raw_observation.1, b"reviewed by equivalence test");
        assert_eq!(raw_observation.2, 1);
    }
}
