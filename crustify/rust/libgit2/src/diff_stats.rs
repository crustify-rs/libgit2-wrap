//! Safe wrappers for libgit2 diff_stats APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_diff_stats
    /// Opaque statistics accumulated for a diff.
    DiffStats,
    DiffStatsRef,
    DiffStatsMut,
    ffi::git_diff_stats
);

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
