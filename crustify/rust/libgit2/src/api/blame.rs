//! Safe wrappers for libgit2 blame APIs.

use core::ffi::CStr;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
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

ffibox::define_ctype!(
    /// Wraps: git_blame_line
    /// A layout-compatible borrowed view of one line in a blame result.
    ///
    /// Libgit2 owns both the line record and the length-delimited bytes. The
    /// handle lifetime must therefore remain bounded by the enclosing blame
    /// result that keeps its final blob alive.
    GitBlameLine,
    GitBlameLineRef,
    GitBlameLineMut,
    ffi::git_blame_line
);

impl<'a> GitBlameLineRef<'a> {
    /// Field: git_blame_line.len
    /// Returns the number of bytes in this line, excluding its newline.
    #[must_use]
    pub fn len(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).len).read() }
    }

    /// Returns whether this line contains no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Field: git_blame_line.ptr
    /// Borrows the line's length-delimited bytes from the enclosing blame.
    #[must_use]
    pub fn bytes(&self) -> ffibox::CSlice<'a, u8> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to the line record.
        let ptr = unsafe { addr_of!((*self.as_ptr()).ptr).read() };
        let ptr = core::ptr::NonNull::new(ptr.cast_mut().cast::<u8>())
            .expect("a valid git_blame_line has a non-null byte pointer");
        // SAFETY: a valid line points to `len` initialized bytes in the final
        // blob, which the handle's enclosing blame borrow keeps alive for `'a`.
        unsafe { ffibox::CSlice::from_raw_parts(ptr, self.len()) }
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

    #[test]
    fn line_wrapper_preserves_layout_and_handle_shape() {
        assert_eq!(size_of::<GitBlameLine>(), size_of::<ffi::git_blame_line>());
        assert_eq!(
            align_of::<GitBlameLine>(),
            align_of::<ffi::git_blame_line>()
        );
        assert_eq!(
            size_of::<GitBlameLineRef<'_>>(),
            size_of::<*mut ffi::git_blame_line>()
        );
        assert_eq!(
            size_of::<GitBlameLineMut<'_>>(),
            size_of::<*mut ffi::git_blame_line>()
        );
    }

    #[test]
    fn line_exposes_length_delimited_bytes_without_a_rust_slice() {
        let bytes = [0x61_u8, 0, 0xff];
        let mut raw = ffi::git_blame_line {
            ptr: bytes.as_ptr().cast(),
            len: bytes.len(),
        };

        // SAFETY: `raw` and `bytes` remain live and are not mutated while the
        // shared line handle and its dependent byte view are used.
        let line = unsafe { GitBlameLineRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(line.len(), 3);
        assert!(!line.is_empty());
        assert_eq!(line.bytes().elems().collect::<Vec<_>>(), bytes);
    }

    #[test]
    fn an_empty_line_keeps_its_start_pointer_and_yields_no_bytes() {
        // `index_blob_lines` records a zero-length line for a bare newline: it
        // stores the newline's own address as `ptr` and never a null one.
        let buffer = *b"\n";
        let mut raw = ffi::git_blame_line {
            ptr: buffer.as_ptr().cast(),
            len: 0,
        };

        // SAFETY: `raw` and `buffer` remain live and unmodified while the
        // shared line handle and its dependent byte view are used.
        let line = unsafe { GitBlameLineRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(line.len(), 0);
        assert!(line.is_empty());
        assert_eq!(line.bytes().elems().count(), 0);
    }
}

/// Wraps: git_blame_flag_t
/// A checked set of flags controlling libgit2 blame traversal.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitBlameFlags(ffi::git_blame_flag_t);

impl GitBlameFlags {
    /// Perform normal blame traversal without optional behavior.
    pub const NORMAL: Self = Self(ffi::git_blame_flag_t_GIT_BLAME_NORMAL);
    /// Track lines moved within the same file.
    pub const TRACK_COPIES_SAME_FILE: Self =
        Self(ffi::git_blame_flag_t_GIT_BLAME_TRACK_COPIES_SAME_FILE);
    /// Track lines moved between files in the same commit.
    pub const TRACK_COPIES_SAME_COMMIT_MOVES: Self =
        Self(ffi::git_blame_flag_t_GIT_BLAME_TRACK_COPIES_SAME_COMMIT_MOVES);
    /// Track lines copied between files in the same commit.
    pub const TRACK_COPIES_SAME_COMMIT_COPIES: Self =
        Self(ffi::git_blame_flag_t_GIT_BLAME_TRACK_COPIES_SAME_COMMIT_COPIES);
    /// Track lines copied from files in any commit.
    pub const TRACK_COPIES_ANY_COMMIT_COPIES: Self =
        Self(ffi::git_blame_flag_t_GIT_BLAME_TRACK_COPIES_ANY_COMMIT_COPIES);
    /// Follow only the first parent of each commit.
    pub const FIRST_PARENT: Self = Self(ffi::git_blame_flag_t_GIT_BLAME_FIRST_PARENT);
    /// Canonicalize authors and committers through the repository mailmap.
    pub const USE_MAILMAP: Self = Self(ffi::git_blame_flag_t_GIT_BLAME_USE_MAILMAP);
    /// Ignore whitespace-only differences.
    pub const IGNORE_WHITESPACE: Self = Self(ffi::git_blame_flag_t_GIT_BLAME_IGNORE_WHITESPACE);
    /// Every blame flag published by this version of libgit2.
    pub const ALL: Self = Self(
        Self::TRACK_COPIES_SAME_FILE.0
            | Self::TRACK_COPIES_SAME_COMMIT_MOVES.0
            | Self::TRACK_COPIES_SAME_COMMIT_COPIES.0
            | Self::TRACK_COPIES_ANY_COMMIT_COPIES.0
            | Self::FIRST_PARENT.0
            | Self::USE_MAILMAP.0
            | Self::IGNORE_WHITESPACE.0,
    );

    /// Converts raw bits when every bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_blame_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_blame_flag_t {
        self.0
    }

    /// Returns whether no optional behavior is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitBlameFlags> for ffi::git_blame_flag_t {
    fn from(flags: GitBlameFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_blame_flag_t> for GitBlameFlags {
    type Error = ffi::git_blame_flag_t;

    fn try_from(bits: ffi::git_blame_flag_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitBlameFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitBlameFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitBlameFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitBlameFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitBlameFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod blame_flag_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_blame_flags_combine_and_validate() {
        let flags = GitBlameFlags::FIRST_PARENT | GitBlameFlags::IGNORE_WHITESPACE;
        assert!(flags.contains(GitBlameFlags::FIRST_PARENT));
        assert!(flags.intersects(GitBlameFlags::IGNORE_WHITESPACE));
        assert!(!flags.is_empty());
        assert_eq!(GitBlameFlags::try_from(flags.bits()), Ok(flags));
        assert!(
            (!flags).contains(GitBlameFlags::USE_MAILMAP)
                && !(!flags).intersects(GitBlameFlags::FIRST_PARENT)
        );
    }

    #[test]
    fn unknown_blame_flag_bits_are_rejected() {
        let unknown = GitBlameFlags::ALL.bits() + 1;
        assert_eq!(GitBlameFlags::from_bits(unknown), None);
        assert_eq!(GitBlameFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn blame_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitBlameFlags>(),
            size_of::<ffi::git_blame_flag_t>()
        );
        assert_eq!(
            align_of::<GitBlameFlags>(),
            align_of::<ffi::git_blame_flag_t>()
        );
    }
}

/// Wraps: git_blame_file_from_buffer
/// Computes blame for in-memory contents and ties the result to `repo`.
pub fn git_blame_file_from_buffer<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    path: &core::ffi::CStr,
    contents: &[u8],
    options: Option<crate::blame::GitBlameOptionsRef<'_>>,
) -> Result<crate::blame::GitRepositoryBlame<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut());
    // SAFETY: output is writable and all borrowed inputs are live for this
    // synchronous call; the returned owner retains only the repository pointer.
    let status = unsafe {
        crate::ffi::git_blame_file_from_buffer(
            &mut out,
            repo.as_ptr().cast_mut(),
            path.as_ptr(),
            contents.as_ptr().cast(),
            contents.len(),
            options,
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized blame allocation.
    let inner =
        unsafe { ffibox::CBox::from_raw(out) }.ok_or(crate::ffi::git_error_code_GIT_ERROR)?;
    Ok(crate::blame::GitRepositoryBlame::from_owned(inner))
}
