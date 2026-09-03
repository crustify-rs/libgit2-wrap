//! Libgit2 allocation lifecycle strategies.

use core::ptr::NonNull;

use ffibox::{CCloned, CDropped, CLenDropped};

use crate::ffi;

/// Wraps: git__free
/// Deleter strategy for allocations made by libgit2's configured allocator.
///
/// Libgit2 looks up its process-global `gfree` callback when the owner is
/// dropped, rather than recording the callback that allocated the block. Code
/// that replaces the allocator must therefore keep its new `gfree` compatible
/// with every outstanding allocation.
pub struct GitMallocFree;

// SAFETY: `c_drop` dispatches through libgit2's currently installed `gfree`.
// `CVoidBox::from_raw` requires callers to supply a compatible allocation; for
// this strategy that obligation includes keeping the installed callback
// compatible until the owner is dropped.
unsafe impl CDropped for GitMallocFree {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the `CDropped` contract guarantees that `obj` is uniquely
        // owned and compatible with libgit2's currently installed `gfree`.
        unsafe { ffi::crustify_git__free(obj.as_ptr().cast()) }
    }
}

// SAFETY: the same configured allocator frees counted byte buffers without
// needing their length. `CVec::from_raw_parts` supplies unique ownership and
// passes the original byte count, which this allocator deliberately ignores.
unsafe impl CLenDropped for GitMallocFree {
    unsafe fn c_drop_len(ptr: *mut u8, _byte_len: usize) {
        // SAFETY: the trait contract guarantees a unique compatible
        // allocation; libgit2's configured free routine accepts its address.
        unsafe { ffi::crustify_git__free(ptr.cast()) }
    }
}

/// Wraps: git__free
/// Deleter and cloner strategy for NUL-terminated strings allocated through
/// libgit2's configured allocator.
///
/// Cloning uses the process-global allocator installed at clone time, while
/// dropping uses the process-global `gfree` installed at drop time. Code that
/// replaces the allocator must therefore keep its new `gfree` compatible with
/// every outstanding string, including clones made under an earlier allocator.
pub struct GitStrdupFree;

// SAFETY: `c_drop` dispatches through libgit2's currently installed `gfree`.
// `CrustifyStr::from_raw` requires a compatible uniquely owned C string and
// this strategy requires the installed callback to remain compatible.
unsafe impl CDropped for GitStrdupFree {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the `CDropped` contract guarantees that `obj` is uniquely
        // owned and compatible with libgit2's currently installed `gfree`.
        unsafe { ffi::crustify_git__free(obj.as_ptr().cast()) }
    }
}

/// Wraps: git__strdup
// SAFETY: `c_clone` delegates to `git__strdup`, which copies the complete
// NUL-terminated input into a fresh allocation made by libgit2's configured
// allocator. Under the strategy's documented allocator-compatibility
// invariant, a successful result therefore owes one matching `c_drop`.
unsafe impl CCloned for GitStrdupFree {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        // SAFETY: the `CCloned` contract guarantees that `obj` addresses a live
        // NUL-terminated string. `git__strdup` reads it without invalidating it.
        NonNull::new(unsafe { ffi::crustify_git__strdup(obj.as_ptr().cast()) }.cast())
    }
}

#[cfg(test)]
mod unit_tests {
    use ffibox::{CVoidBox, CrustifyStr};

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
    #[test]
    fn libgit2_string_clone_is_independently_owned() {
        // SAFETY: libgit2's process-global initialization is refcounted and
        // this test balances the successful call with shutdown below.
        let init_count = unsafe { ffi::git_libgit2_init() };
        assert!(init_count > 0);

        // SAFETY: the source is a live NUL-terminated string. Libgit2 is
        // initialized, so a non-null result is a fresh configured allocation.
        let raw = unsafe { ffi::crustify_git__strdup(c"crustify".as_ptr()) };
        // SAFETY: `raw` is null or a fresh, uniquely owned NUL-terminated
        // string allocated by libgit2 and matched by `GitStrdupFree`.
        let original = unsafe { CrustifyStr::<GitStrdupFree>::from_raw(raw) }
            .expect("libgit2 should duplicate the test string");

        let cloned = original
            .try_clone()
            .expect("libgit2 should clone the owned string");
        assert_eq!(original.as_bytes(), b"crustify");
        assert_eq!(cloned.as_bytes(), original.as_bytes());
        assert_ne!(cloned.as_ptr(), original.as_ptr());

        drop(cloned);
        drop(original);

        // SAFETY: balances this test's successful `git_libgit2_init` call.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
    }
}
