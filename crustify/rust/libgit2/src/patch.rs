//! Safe wrappers for libgit2 patch APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CDropped};

use crate::api::diff::DiffDeltaRef;
use crate::diff::{DiffHunkRef, DiffLineRef, DiffMut};
use crate::ffi;
use crate::repository::GitRepositoryRef;

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

    /// The C side proves this: `patch_generated_init_common` calls
    /// `git_diff_addref`, and `git_patch_parsed_from_diff` hands back a
    /// counted reference to a patch that owns copies of its delta, paths and
    /// line contents. Neither borrows the `git_diff` header, so the wrapper
    /// must not tie the patch to the diff's Rust lifetime.
    #[test]
    fn a_patch_taken_from_a_diff_outlives_that_diff() {
        // SAFETY: libgit2 initialization is refcounted and balanced below,
        // after every allocation this test made has been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let source = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n";
        let mut diff = crate::diff_parse::git_diff_from_buffer(source).expect("valid patch");
        let patch = git_patch_from_diff(&mut diff.as_mut(), 0)
            .expect("delta 0 exists")
            .expect("a parsed delta is never skipped");

        drop(diff);

        assert_eq!(git_patch_num_hunks(patch.as_ref()), 1);
        let (hunk, lines) = git_patch_get_hunk(patch.as_ref(), 0).expect("hunk 0 exists");
        assert_eq!(lines, 2);
        assert_eq!(hunk.new_lines(), 1);
        assert_eq!(
            git_patch_line_stats(patch.as_ref()).expect("line stats are available"),
            PatchLineStats {
                context: 0,
                additions: 1,
                deletions: 1,
            }
        );
        // A parsed patch leaves the cached size counters at zero, so only the
        // formatted file header contributes. Repeating the call shows the
        // shared borrow observes no lazily populated state.
        assert_eq!(git_patch_size(patch.as_ref(), true, true, false), 0);
        let with_header = git_patch_size(patch.as_ref(), true, true, true);
        assert!(with_header > 0);
        assert_eq!(
            git_patch_size(patch.as_ref(), true, true, true),
            with_header
        );
        assert!(git_patch_get_line_in_hunk(patch.as_ref(), 0, 0).is_ok());
        assert!(git_patch_get_line_in_hunk(patch.as_ref(), 0, 99).is_err());
        drop(patch);

        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
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
///
/// The C declaration is non-const, but the body only indexes the patch's
/// cached hunk and line arrays, so a shared borrow states the real contract.
pub fn git_patch_get_line_in_hunk<'a>(
    patch: GitPatchRef<'a>,
    hunk_index: usize,
    line_index: usize,
) -> Result<DiffLineRef<'a>, i32> {
    let mut line = core::ptr::null();
    // SAFETY: `line` is a writable output slot and `patch` is a live borrowed
    // patch that libgit2 neither writes through nor retains. It returns an
    // internal line whose storage remains valid for the patch borrow.
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
    Ok(unsafe { DiffLineRef::from_ptr(line.cast_mut()) }
        .expect("a successful line lookup returns non-null"))
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
/// Computes the selected serialized size.
///
/// The C declaration takes a non-const patch, but the body only reads the
/// cached `content_size`, `context_size` and `header_size` counters and
/// formats a header from a `const git_diff_delta` into its own scratch
/// buffer. It populates nothing lazily, so a shared borrow states the real
/// contract.
#[must_use]
pub fn git_patch_size(
    patch: GitPatchRef<'_>,
    include_context: bool,
    include_hunk_headers: bool,
    include_file_headers: bool,
) -> usize {
    // SAFETY: `patch` is a live shared input. `git_patch_size` writes nothing
    // through the pointer and retains none of it, so restoring mutability at
    // the seam only satisfies the C declaration. Each boolean is converted to
    // the C zero/nonzero convention.
    unsafe {
        ffi::git_patch_size(
            patch.as_ptr().cast_mut(),
            i32::from(include_context),
            i32::from(include_hunk_headers),
            i32::from(include_file_headers),
        )
    }
}

/// Wraps: git_patch_from_diff
/// Creates an independently owned patch for one delta of `diff`.
///
/// The result outlives `diff`: a generated patch takes its own `git_diff`
/// reference in `patch_generated_init_common`, and a parsed patch owns copies
/// of its delta, paths and line contents while holding a count on its parse
/// context. Only the transient call needs `diff`, which it borrows exclusively
/// because the generated path bumps the diff's reference count.
///
/// Returns `Ok(None)` for a delta libgit2 declines to expand. C documents that
/// case — an unchanged or binary file — as a successful call whose output slot
/// is left null.
pub fn git_patch_from_diff(
    diff: &mut DiffMut<'_>,
    index: usize,
) -> Result<Option<GitPatchOwned>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: `raw` is a writable owner slot and `diff` supplies the exclusive
    // access the call needs to acquire its own diff reference.
    let status =
        unsafe { ffi::git_patch_from_diff(core::ptr::addr_of_mut!(raw), diff.as_mut_ptr(), index) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: on success `raw` is either null, meaning the delta was skipped,
    // or one complete owned patch reference this wrapper adopts.
    Ok(unsafe { GitPatchOwned::from_raw(raw) })
}

/// Wraps: git_patch_get_hunk
/// Borrows one hunk and returns its line count.
///
/// As with [`git_patch_get_line_in_hunk`], the C declaration is non-const but
/// the body only indexes the patch's cached hunk array, so a shared borrow is
/// enough and it carries the returned hunk's lifetime.
pub fn git_patch_get_hunk<'a>(
    patch: GitPatchRef<'a>,
    index: usize,
) -> Result<(DiffHunkRef<'a>, usize), i32> {
    let mut raw = core::ptr::null();
    let mut lines = 0;
    // SAFETY: both outputs are writable and `patch` is a live shared input
    // that libgit2 neither writes through nor retains; the returned hunk lives
    // inside it for `'a`.
    let status = unsafe {
        ffi::git_patch_get_hunk(
            core::ptr::addr_of_mut!(raw),
            core::ptr::addr_of_mut!(lines),
            patch.as_ptr().cast_mut(),
            index,
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns a non-null hunk inside the live patch.
    let hunk = unsafe { DiffHunkRef::from_ptr(raw.cast_mut()) }
        .expect("a successful hunk lookup returns non-null");
    Ok((hunk, lines))
}

/// Wraps: git_patch_get_delta
/// Borrows the delta associated with a patch.
#[must_use]
pub fn git_patch_get_delta<'a>(patch: GitPatchRef<'a>) -> DiffDeltaRef<'a> {
    // SAFETY: the live shared patch owns its delta and the accessor only reads
    // the stored pointer.
    let delta = unsafe { ffi::git_patch_get_delta(patch.as_ptr()) };
    // SAFETY: every complete patch has a non-null delta that remains live for
    // the patch borrow carried into this result.
    unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }
        .expect("a complete patch always carries a delta")
}

#[cfg(test)]
mod delta_accessor_tests {
    use super::*;

    #[test]
    fn delta_borrow_is_tied_to_the_patch() {
        let _: for<'a> fn(GitPatchRef<'a>) -> DiffDeltaRef<'a> = git_patch_get_delta;
    }
}

/// Wraps: git_patch_owner
/// Borrows the repository associated with a patch, if it has one.
#[must_use]
pub fn git_patch_owner<'a>(patch: GitPatchRef<'a>) -> Option<GitRepositoryRef<'a>> {
    // SAFETY: `patch` is live and the returned repository pointer is borrowed
    // from it. The getter retains nothing and may return null.
    let owner = unsafe { ffi::git_patch_owner(patch.as_ptr()) };
    // SAFETY: a non-null owner remains live for the patch borrow.
    unsafe { GitRepositoryRef::from_ptr(owner) }
}
