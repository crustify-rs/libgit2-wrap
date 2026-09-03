//! Safe wrappers for libgit2 diff_stats APIs.

use ffibox::{CBox, CVal};

use crate::api::buffer::GitBuf;
use crate::diff::{DiffMut, DiffStatsFormat};
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
///
/// The diff is borrowed exclusively because the call mutates it: it bumps the
/// diff's reference count and expands every delta through
/// `git_patch_from_diff`, which takes a further count and records each delta's
/// cached binary-detection flag. The returned owner is independent of that
/// borrow, since `git_diff_stats_free` releases the count the call took.
pub fn git_diff_get_stats(diff: &mut DiffMut<'_>) -> Result<DiffStatsOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable owner slot and `diff` supplies the exclusive
    // access that the reference-count bump and delta expansion require. C
    // returns an independently owned stats object holding its own count.
    let status = unsafe { ffi::git_diff_get_stats(&mut out, diff.as_mut_ptr()) };
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

    /// Holds one libgit2 initialization count for the duration of a test.
    ///
    /// Every call below reaches libgit2's allocator and thread-local error
    /// state, neither of which exists before `git_libgit2_init`. Without this
    /// guard the test only survives when an unrelated test happens to hold a
    /// count concurrently.
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
    fn parsed_diff_stats_are_counted_and_formatted() {
        let _libgit2 = Libgit2Init::acquire();
        let patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n";
        let mut diff = crate::diff_parse::git_diff_from_buffer(patch).unwrap();
        let stats = git_diff_get_stats(&mut diff.as_mut()).unwrap();
        assert_eq!(git_diff_stats_files_changed(stats.as_ref()), 1);
        assert_eq!(git_diff_stats_insertions(stats.as_ref()), 1);
        assert_eq!(git_diff_stats_deletions(stats.as_ref()), 1);

        let mut out = GitBuf::new();
        git_diff_stats_to_buf(&mut out, stats.as_ref(), DiffStatsFormat::SHORT, 80).unwrap();
        assert!(out.as_ref().size() > 0);
    }
}

#[cfg(test)]
mod io_equiv {
    use super::*;
    use crate::io_equiv_support::{Libgit2Init, RawBuf, RawDiff, safe_buf_bytes};

    #[test]
    fn io_equiv_git_diff_stats_to_buf() {
        let _init = Libgit2Init::acquire();
        let patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,2 +1,2 @@\n-old\n same\n+new\n";

        let raw_diff = RawDiff::from_buffer(patch).unwrap();
        let mut raw_stats = core::ptr::null_mut();
        // SAFETY: the output slot is writable and the raw diff remains live.
        let raw_stats_status =
            unsafe { ffi::git_diff_get_stats(&mut raw_stats, raw_diff.as_ptr()) };
        assert_eq!(raw_stats_status, 0);
        assert!(!raw_stats.is_null());
        let mut raw_output = RawBuf::new();
        // SAFETY: both raw values remain live and the output header is empty
        // and writable for this formatting operation.
        let raw_status = unsafe {
            ffi::git_diff_stats_to_buf(
                &mut raw_output.0,
                raw_stats,
                ffi::git_diff_stats_format_t_GIT_DIFF_STATS_SHORT,
                80,
            )
        };

        let mut safe_diff = crate::diff_parse::git_diff_from_buffer(patch).unwrap();
        let safe_stats = git_diff_get_stats(&mut safe_diff.as_mut()).unwrap();
        let mut safe_output = GitBuf::new();
        let safe_status = git_diff_stats_to_buf(
            &mut safe_output,
            safe_stats.as_ref(),
            DiffStatsFormat::SHORT,
            80,
        );

        assert_eq!(
            safe_status,
            if raw_status == 0 {
                Ok(())
            } else {
                Err(raw_status)
            }
        );
        assert_eq!(safe_buf_bytes(safe_output.as_ref()), raw_output.bytes());
        drop(safe_stats);
        // SAFETY: releases the one stats owner returned above exactly once.
        unsafe { ffi::git_diff_stats_free(raw_stats) };
    }
}
