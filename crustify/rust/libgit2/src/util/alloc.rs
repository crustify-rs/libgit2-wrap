//! Libgit2 allocation lifecycle strategies.

use core::ptr::NonNull;

use ffibox::CDropped;

use crate::ffi;

/// Wraps: git__free
/// Deleter strategy for allocations made by libgit2's configured allocator.
pub struct GitMallocFree;

// SAFETY: `c_drop` dispatches through the same configurable allocator used by
// libgit2 allocation routines. `CVoidBox::from_raw` requires callers to supply
// a compatible allocation.
unsafe impl CDropped for GitMallocFree {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the `CDropped` contract guarantees that `obj` is a uniquely
        // owned allocation made by libgit2's currently configured allocator.
        unsafe { ffi::crustify_git__free(obj.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use ffibox::CVoidBox;

    use super::*;

    #[test]
    fn libgit2_allocation_is_released_on_drop() {
        // SAFETY: libgit2's process-global initialization is refcounted and
        // this test balances the successful call with shutdown below.
        let init_count = unsafe { ffi::git_libgit2_init() };
        assert!(init_count > 0);

        // SAFETY: libgit2 is initialized, so its allocator is installed; the
        // returned opaque allocation is checked below.
        let raw = unsafe { ffi::crustify_git__malloc(32) };
        // SAFETY: `raw` is null or a fresh, uniquely owned allocation from
        // libgit2's configured allocator, whose matching strategy is
        // `GitMallocFree`.
        let owned = unsafe { CVoidBox::<GitMallocFree>::from_raw(raw) }
            .expect("libgit2 should allocate the test block");

        drop(owned);

        // SAFETY: balances this test's successful `git_libgit2_init` call.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
    }
}
