//! Safe wrappers for libgit2 patch APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CDropped};

use crate::diff::DiffLineRef;
use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_patch
    /// An opaque, reference-counted patch managed by libgit2.
    ///
    /// Owned patch references use [`GitPatchOwned`]. Dropping an owner
    /// decrements the patch's reference count and releases its concrete patch
    /// representation when the final reference is gone.
    GitPatch,
    GitPatchRef,
    GitPatchMut,
    ffi::git_patch
);

/// An owning reference to a fully formed libgit2 patch.
pub type GitPatchOwned = CBox<GitPatch>;

/// Wraps: git_patch_free
// SAFETY: `git_patch_free` consumes one owning reference to a fully formed
// `git_patch`. It decrements the embedded reference count and dispatches to the
// concrete patch destructor exactly when the final reference is released.
unsafe impl CDropped for GitPatch {
    unsafe fn c_drop(object: NonNull<Self>) {
        // SAFETY: the trait contract supplies one live owned patch count and
        // the wrapper is transparent over `ffi::git_patch`.
        unsafe { ffi::git_patch_free(object.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn patch_wrapper_preserves_the_opaque_seam_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitPatch>();
        assert_dropped::<GitPatch>();
        assert_eq!(size_of::<GitPatch>(), size_of::<ffi::git_patch>());
        assert_eq!(align_of::<GitPatch>(), align_of::<ffi::git_patch>());
        assert_eq!(
            size_of::<GitPatchRef<'_>>(),
            size_of::<*const ffi::git_patch>()
        );
        assert_eq!(
            size_of::<GitPatchMut<'_>>(),
            size_of::<*mut ffi::git_patch>()
        );
        assert_eq!(size_of::<GitPatchOwned>(), size_of::<*mut ffi::git_patch>());
    }

    #[test]
    fn null_patch_seams_create_no_handle() {
        // SAFETY: all conversions explicitly accept null and return `None`
        // without borrowing or adopting a patch.
        unsafe {
            assert!(GitPatchRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPatchMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPatchOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// Wraps: git_patch_get_line_in_hunk
/// Borrows a line stored in `patch`, returning the libgit2 error code when an
/// index is out of range.
pub fn git_patch_get_line_in_hunk<'a>(
    patch: GitPatchRef<'a>,
    hunk_index: usize,
    line_index: usize,
) -> Result<DiffLineRef<'a>, i32> {
    let mut line = core::ptr::null();
    // SAFETY: `line` is a writable output slot and `patch` is a live borrowed
    // patch. Libgit2 retains no new pointer and returns an internal line whose
    // storage remains valid for the patch borrow.
    let status = unsafe {
        ffi::git_patch_get_line_in_hunk(
            core::ptr::addr_of_mut!(line),
            patch.as_ptr().cast_mut(),
            hunk_index,
            line_index,
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success initializes `line` to a live line embedded in `patch`;
    // the returned handle inherits the patch's `'a` lifetime.
    unsafe { DiffLineRef::from_ptr(line.cast_mut()) }.ok_or(status)
}

/// Counts each kind of line in a patch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PatchLineStats {
    /// Unchanged context lines.
    pub context: usize,
    /// Added lines.
    pub additions: usize,
    /// Deleted lines.
    pub deletions: usize,
}

/// Wraps: git_patch_line_stats
/// Returns all three line counters for `patch`.
pub fn git_patch_line_stats(patch: GitPatchRef<'_>) -> Result<PatchLineStats, i32> {
    let mut stats = PatchLineStats {
        context: 0,
        additions: 0,
        deletions: 0,
    };
    // SAFETY: each counter is a distinct writable scalar and `patch` is a
    // live shared input retained only for the call.
    let status = unsafe {
        ffi::git_patch_line_stats(
            core::ptr::addr_of_mut!(stats.context),
            core::ptr::addr_of_mut!(stats.additions),
            core::ptr::addr_of_mut!(stats.deletions),
            patch.as_ptr(),
        )
    };
    if status == 0 { Ok(stats) } else { Err(status) }
}

/// Wraps: git_patch_num_hunks
/// Returns the number of hunks in `patch`.
#[must_use]
pub fn git_patch_num_hunks(patch: GitPatchRef<'_>) -> usize {
    // SAFETY: `patch` is a live shared input and the call retains no pointer.
    unsafe { ffi::git_patch_num_hunks(patch.as_ptr()) }
}

/// Wraps: git_patch_num_lines_in_hunk
/// Returns the line count, or the negative libgit2 error for an invalid hunk.
pub fn git_patch_num_lines_in_hunk(
    patch: GitPatchRef<'_>,
    hunk_index: usize,
) -> Result<usize, i32> {
    // SAFETY: `patch` is a live shared input and the call retains no pointer.
    let count = unsafe { ffi::git_patch_num_lines_in_hunk(patch.as_ptr(), hunk_index) };
    usize::try_from(count).map_err(|_| count)
}

/// Wraps: git_patch_size
/// Computes the selected serialized size. Libgit2 may lazily populate patch
/// header state, so this operation requires exclusive access.
#[must_use]
pub fn git_patch_size(
    patch: &mut GitPatchMut<'_>,
    include_context: bool,
    include_hunk_headers: bool,
    include_file_headers: bool,
) -> usize {
    // SAFETY: the mutable handle provides exclusive access for any lazy state
    // updates and each boolean is converted to the C zero/nonzero convention.
    unsafe {
        ffi::git_patch_size(
            patch.as_mut_ptr(),
            i32::from(include_context),
            i32::from(include_hunk_headers),
            i32::from(include_file_headers),
        )
    }
}
