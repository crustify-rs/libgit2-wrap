//! Safe wrappers for libgit2 diff_stats APIs.

use ffibox::{CBox, CVal};

use crate::api::buffer::GitBuf;
use crate::diff::{DiffRef, DiffStatsFormat};
use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_diff_stats
    /// Opaque statistics accumulated for a diff.
    DiffStats,
    DiffStatsRef,
    DiffStatsMut,
    ffi::git_diff_stats
);

/// Wraps: git_diff_stats_free
/// An owned diff-statistics result.
pub type DiffStatsOwned = CBox<DiffStats>;

// SAFETY: `git_diff_stats_free` is the public destructor for a fully formed
// `git_diff_stats` allocation and accepts null, although `CBox` supplies a
// live non-null allocation exactly once.
ffibox::impl_dropped!(DiffStats, ffi::git_diff_stats, ffi::git_diff_stats_free);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_stats_preserve_the_c_layout_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<DiffStats>();
        assert_dropped::<DiffStats>();
        assert_eq!(size_of::<DiffStats>(), size_of::<ffi::git_diff_stats>());
        assert_eq!(align_of::<DiffStats>(), align_of::<ffi::git_diff_stats>());
        assert_eq!(
            size_of::<DiffStatsRef<'_>>(),
            size_of::<*const ffi::git_diff_stats>()
        );
        assert_eq!(
            size_of::<DiffStatsMut<'_>>(),
            size_of::<*mut ffi::git_diff_stats>()
        );
        assert_eq!(
            size_of::<DiffStatsOwned>(),
            size_of::<*mut ffi::git_diff_stats>()
        );
    }

    #[test]
    fn null_stats_seams_create_no_handle() {
        // SAFETY: each conversion accepts null and returns `None` without
        // borrowing or adopting an object.
        unsafe {
            assert!(DiffStatsRef::from_ptr(ptr::null_mut()).is_none());
            assert!(DiffStatsMut::from_ptr(ptr::null_mut()).is_none());
            assert!(DiffStatsOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// Wraps: git_diff_get_stats
/// Accumulates statistics and retains its own reference count to `diff`.
pub fn git_diff_get_stats(diff: DiffRef<'_>) -> Result<DiffStatsOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and `diff` is live. C increments the diff's
    // reference count before returning the independently owned stats object.
    let status = unsafe { ffi::git_diff_get_stats(&mut out, diff.as_ptr().cast_mut()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns a complete owned stats allocation.
    unsafe { DiffStatsOwned::from_raw(out) }.ok_or(-1)
}

/// Wraps: git_diff_stats_deletions
#[must_use]
pub fn git_diff_stats_deletions(stats: DiffStatsRef<'_>) -> usize {
    // SAFETY: `stats` is a live shared handle and C reads one scalar field.
    unsafe { ffi::git_diff_stats_deletions(stats.as_ptr()) }
}

/// Wraps: git_diff_stats_files_changed
#[must_use]
pub fn git_diff_stats_files_changed(stats: DiffStatsRef<'_>) -> usize {
    // SAFETY: `stats` is a live shared handle and C reads one scalar field.
    unsafe { ffi::git_diff_stats_files_changed(stats.as_ptr()) }
}

/// Wraps: git_diff_stats_insertions
#[must_use]
pub fn git_diff_stats_insertions(stats: DiffStatsRef<'_>) -> usize {
    // SAFETY: `stats` is a live shared handle and C reads one scalar field.
    unsafe { ffi::git_diff_stats_insertions(stats.as_ptr()) }
}

/// Wraps: git_diff_stats_to_buf
/// Formats statistics into an owned libgit2 buffer header.
pub fn git_diff_stats_to_buf(
    out: &mut CVal<GitBuf>,
    stats: DiffStatsRef<'_>,
    format: DiffStatsFormat,
    width: usize,
) -> Result<(), i32> {
    // SAFETY: the buffer is exclusively borrowed, `stats` is live, and the
    // validated format contains only bits published by this libgit2 version.
    let status = unsafe {
        ffi::git_diff_stats_to_buf(
            out.as_mut().as_mut_ptr(),
            stats.as_ptr(),
            format.bits(),
            width,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod wrapper_tests {
    use super::*;

    #[test]
    fn parsed_diff_stats_are_counted_and_formatted() {
        let patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n";
        let diff = crate::diff_parse::git_diff_from_buffer(patch).unwrap();
        let stats = git_diff_get_stats(diff.as_ref()).unwrap();
        assert_eq!(git_diff_stats_files_changed(stats.as_ref()), 1);
        assert_eq!(git_diff_stats_insertions(stats.as_ref()), 1);
        assert_eq!(git_diff_stats_deletions(stats.as_ref()), 1);

        let mut out = GitBuf::new();
        git_diff_stats_to_buf(&mut out, stats.as_ref(), DiffStatsFormat::SHORT, 80).unwrap();
        assert!(out.as_ref().size() > 0);
    }
}
