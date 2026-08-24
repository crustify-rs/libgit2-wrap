//! Safe wrappers for libgit2 blame APIs.

use core::ffi::CStr;
use core::ptr::addr_of;

use crate::api::types::GitSignatureRef;
use crate::ffi;
use crate::oid::OidRef;

ffibox::define_ctype!(
    /// Wraps: git_blame_hunk
    /// A layout-compatible, non-owning view of one hunk in a blame result.
    ///
    /// Libgit2 owns the hunk and every allocation reached through it. A hunk
    /// handle must not outlive the enclosing blame result.
    GitBlameHunk,
    GitBlameHunkRef,
    GitBlameHunkMut,
    ffi::git_blame_hunk
);

impl<'a> GitBlameHunkRef<'a> {
    /// Field: git_blame_hunk.boundary
    /// Returns whether history traversal stopped at a boundary commit.
    #[must_use]
    pub fn boundary(&self) -> bool {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).boundary).read() != 0 }
    }

    /// Field: git_blame_hunk.summary
    /// Borrows the commit summary, if this hunk is associated with a commit.
    #[must_use]
    pub fn summary(&self) -> Option<&'a CStr> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to the hunk.
        let summary = unsafe { addr_of!((*self.as_ptr()).summary).read() };
        if summary.is_null() {
            None
        } else {
            // SAFETY: a non-null summary in a valid hunk is NUL-terminated and
            // owned by the hunk, so it lives for this handle's `'a` borrow.
            Some(unsafe { CStr::from_ptr(summary) })
        }
    }

    /// Field: git_blame_hunk.orig_committer
    /// Borrows the originating commit's committer, when one is available.
    #[must_use]
    pub fn orig_committer(&self) -> Option<GitSignatureRef<'a>> {
        // SAFETY: raw-place projection reads the initialized optional pointer
        // without forming a reference to the hunk.
        let signature = unsafe { addr_of!((*self.as_ptr()).orig_committer).read() };
        // SAFETY: a non-null pointer denotes an initialized signature owned by
        // this hunk and therefore live for the handle's `'a` borrow.
        unsafe { GitSignatureRef::from_ptr(signature) }
    }

    /// Field: git_blame_hunk.orig_signature
    /// Borrows the originating commit's author, when one is available.
    #[must_use]
    pub fn orig_signature(&self) -> Option<GitSignatureRef<'a>> {
        // SAFETY: raw-place projection reads the initialized optional pointer
        // without forming a reference to the hunk.
        let signature = unsafe { addr_of!((*self.as_ptr()).orig_signature).read() };
        // SAFETY: a non-null pointer denotes an initialized signature owned by
        // this hunk and therefore live for the handle's `'a` borrow.
        unsafe { GitSignatureRef::from_ptr(signature) }
    }

    /// Field: git_blame_hunk.orig_start_line_number
    /// Returns the one-based start line in the originating file.
    #[must_use]
    pub fn orig_start_line_number(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).orig_start_line_number).read() }
    }

    /// Field: git_blame_hunk.orig_path
    /// Borrows the originating repository-relative path, when available.
    #[must_use]
    pub fn orig_path(&self) -> Option<&'a CStr> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to the hunk.
        let path = unsafe { addr_of!((*self.as_ptr()).orig_path).read() };
        if path.is_null() {
            None
        } else {
            // SAFETY: a non-null path in a valid hunk is NUL-terminated and
            // owned by the hunk, so it lives for this handle's `'a` borrow.
            Some(unsafe { CStr::from_ptr(path) })
        }
    }

    /// Field: git_blame_hunk.orig_commit_id
    /// Borrows the inline identifier of the originating commit.
    #[must_use]
    pub fn orig_commit_id(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection obtains the initialized inline field's
        // address without forming a reference to C-visible memory.
        let oid = unsafe { addr_of!((*self.as_ptr()).orig_commit_id) }.cast_mut();
        // SAFETY: the inline field is non-null and remains live for the hunk
        // handle's full `'a` borrow.
        unsafe { OidRef::from_ptr(oid) }.expect("an inline field is non-null")
    }

    /// Field: git_blame_hunk.final_committer
    /// Borrows the final commit's committer, when one is available.
    #[must_use]
    pub fn final_committer(&self) -> Option<GitSignatureRef<'a>> {
        // SAFETY: raw-place projection reads the initialized optional pointer
        // without forming a reference to the hunk.
        let signature = unsafe { addr_of!((*self.as_ptr()).final_committer).read() };
        // SAFETY: a non-null pointer denotes an initialized signature owned by
        // this hunk and therefore live for the handle's `'a` borrow.
        unsafe { GitSignatureRef::from_ptr(signature) }
    }

    /// Field: git_blame_hunk.final_signature
    /// Borrows the final commit's author, when one is available.
    #[must_use]
    pub fn final_signature(&self) -> Option<GitSignatureRef<'a>> {
        // SAFETY: raw-place projection reads the initialized optional pointer
        // without forming a reference to the hunk.
        let signature = unsafe { addr_of!((*self.as_ptr()).final_signature).read() };
        // SAFETY: a non-null pointer denotes an initialized signature owned by
        // this hunk and therefore live for the handle's `'a` borrow.
        unsafe { GitSignatureRef::from_ptr(signature) }
    }

    /// Field: git_blame_hunk.final_start_line_number
    /// Returns the one-based start line in the final file.
    #[must_use]
    pub fn final_start_line_number(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).final_start_line_number).read() }
    }

    /// Field: git_blame_hunk.final_commit_id
    /// Borrows the inline identifier of the commit that last changed the hunk.
    #[must_use]
    pub fn final_commit_id(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection obtains the initialized inline field's
        // address without forming a reference to C-visible memory.
        let oid = unsafe { addr_of!((*self.as_ptr()).final_commit_id) }.cast_mut();
        // SAFETY: the inline field is non-null and remains live for the hunk
        // handle's full `'a` borrow.
        unsafe { OidRef::from_ptr(oid) }.expect("an inline field is non-null")
    }

    /// Field: git_blame_hunk.lines_in_hunk
    /// Returns the number of lines attributed to this hunk.
    #[must_use]
    pub fn lines_in_hunk(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).lines_in_hunk).read() }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use crate::oid::OidType;

    use super::*;

    fn oid(kind: OidType, first: u8) -> ffi::git_oid {
        let mut id = [0; crate::oid::RAW_DIGEST_LEN];
        id[0] = first;
        ffi::git_oid {
            id,
            type_: ffi::git_oid_t::from(kind) as u8,
        }
    }

    #[test]
    fn hunk_wrapper_preserves_layout_and_handle_shape() {
        assert_eq!(size_of::<GitBlameHunk>(), size_of::<ffi::git_blame_hunk>());
        assert_eq!(
            align_of::<GitBlameHunk>(),
            align_of::<ffi::git_blame_hunk>()
        );
        assert_eq!(
            size_of::<GitBlameHunkRef<'_>>(),
            size_of::<*mut ffi::git_blame_hunk>()
        );
        assert_eq!(
            size_of::<GitBlameHunkMut<'_>>(),
            size_of::<*mut ffi::git_blame_hunk>()
        );
    }

    #[test]
    fn shared_hunk_reads_scalars_strings_and_inline_oids() {
        let summary = c"summary";
        let path = c"src/file.rs";
        let mut raw = ffi::git_blame_hunk {
            lines_in_hunk: 4,
            final_commit_id: oid(OidType::Sha256, 0x22),
            final_start_line_number: 9,
            final_signature: ptr::null_mut(),
            final_committer: ptr::null_mut(),
            orig_commit_id: oid(OidType::Sha1, 0x11),
            orig_path: path.as_ptr(),
            orig_start_line_number: 3,
            orig_signature: ptr::null_mut(),
            orig_committer: ptr::null_mut(),
            summary: summary.as_ptr(),
            boundary: 1,
        };

        // SAFETY: `raw` is initialized, remains live, and is not mutated while
        // this shared handle and its dependent views are used.
        let hunk = unsafe { GitBlameHunkRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(hunk.lines_in_hunk(), 4);
        assert_eq!(hunk.final_start_line_number(), 9);
        assert_eq!(hunk.orig_start_line_number(), 3);
        assert!(hunk.boundary());
        assert_eq!(hunk.summary(), Some(summary));
        assert_eq!(hunk.orig_path(), Some(path));
        assert_eq!(hunk.final_commit_id().oid_type(), Ok(OidType::Sha256));
        assert_eq!(hunk.final_commit_id().raw_bytes().elem(0), Some(0x22));
        assert_eq!(hunk.orig_commit_id().oid_type(), Ok(OidType::Sha1));
        assert_eq!(hunk.orig_commit_id().raw_bytes().elem(0), Some(0x11));
    }

    #[test]
    fn synthetic_hunk_preserves_nullable_owned_fields() {
        let mut raw = ffi::git_blame_hunk {
            lines_in_hunk: 1,
            final_commit_id: oid(OidType::Sha1, 0),
            final_start_line_number: 1,
            final_signature: ptr::null_mut(),
            final_committer: ptr::null_mut(),
            orig_commit_id: oid(OidType::Sha1, 0),
            orig_path: ptr::null(),
            orig_start_line_number: 0,
            orig_signature: ptr::null_mut(),
            orig_committer: ptr::null_mut(),
            summary: ptr::null(),
            boundary: 0,
        };

        // SAFETY: `raw` is initialized and exclusively borrowed by this handle.
        let hunk = unsafe { GitBlameHunkMut::from_ptr(&raw mut raw) }.unwrap();
        let hunk = hunk.as_ref();
        assert!(hunk.summary().is_none());
        assert!(hunk.orig_path().is_none());
        assert!(hunk.final_signature().is_none());
        assert!(hunk.final_committer().is_none());
        assert!(hunk.orig_signature().is_none());
        assert!(hunk.orig_committer().is_none());
        assert!(!hunk.boundary());
    }
}
