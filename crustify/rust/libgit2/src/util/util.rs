//! Safe wrappers for libgit2 util APIs.

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_refcount
    /// A layout-compatible refcount header embedded in libgit2 objects.
    ///
    /// The count and optional weak owner are manipulated atomically by
    /// libgit2. They are intentionally not exposed as ordinary Rust fields:
    /// unsynchronized loads or stores would violate the C concurrency
    /// contract, and the erased owner does not keep its pointee alive.
    GitRefcount,
    GitRefcountRef,
    GitRefcountMut,
    ffi::git_refcount
);

#[cfg(test)]
mod tests {
    use core::ffi::c_void;
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn refcount_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<GitRefcount>(), size_of::<ffi::git_refcount>());
        assert_eq!(align_of::<GitRefcount>(), align_of::<ffi::git_refcount>());
        assert_eq!(
            size_of::<GitRefcountRef<'_>>(),
            size_of::<*const ffi::git_refcount>()
        );
        assert_eq!(
            size_of::<GitRefcountMut<'_>>(),
            size_of::<*mut ffi::git_refcount>()
        );
    }

    #[test]
    fn shared_and_exclusive_handles_preserve_the_raw_address() {
        let mut raw = ffi::git_refcount {
            refcount: ffi::git_atomic32 { val: 1 },
            owner: core::ptr::null_mut::<c_void>(),
        };
        let raw_ptr = &raw mut raw;

        // SAFETY: `raw_ptr` addresses an initialized stack value that remains
        // live and exclusively borrowed until the handle's last use.
        let mut refcount = unsafe { GitRefcountMut::from_ptr(raw_ptr) }
            .expect("the address of a stack value is non-null");
        assert_eq!(refcount.as_ref().as_ptr(), raw_ptr.cast_const());
        assert_eq!(refcount.as_mut_ptr(), raw_ptr);
    }
}
