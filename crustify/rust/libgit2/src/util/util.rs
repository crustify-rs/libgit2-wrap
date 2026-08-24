//! Safe wrappers for libgit2 util APIs.

use core::ffi::c_void;
use core::ptr::NonNull;

use ffibox::CCell;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_refcount
    /// A layout-compatible refcount header embedded in libgit2 objects.
    ///
    /// Both fields are manipulated atomically by libgit2: `GIT_REFCOUNT_INC`
    /// and `GIT_REFCOUNT_DEC` drive the count, and `GIT_REFCOUNT_OWN` swaps
    /// the weak owner. The accessors below therefore route through the C
    /// atomic primitives rather than loading the fields in place, because a
    /// plain Rust read would race with a concurrent C update.
    ///
    /// The header carries no lifecycle of its own. `GIT_REFCOUNT_DEC` frees
    /// the *embedding* object through a free function only that object knows,
    /// so no owning pointer is bound to this type.
    GitRefcount,
    GitRefcountRef,
    GitRefcountMut,
    ffi::git_refcount
);

impl<'a> GitRefcountRef<'a> {
    /// Field: git_refcount.refcount
    /// Atomically reads the number of outstanding references to the
    /// embedding object.
    ///
    /// The value is a snapshot: a concurrent `GIT_REFCOUNT_INC` or
    /// `GIT_REFCOUNT_DEC` may change it before the caller observes it.
    #[must_use]
    pub fn count(&self) -> i32 {
        // SAFETY: this live shared handle addresses an initialized refcount
        // header, and the shim only performs an atomic load through it, so
        // passing the pointer as `*mut` writes nothing.
        unsafe { ffi::crustify_git_refcount_val(self.as_ptr().cast_mut()) }
    }

    /// Field: git_refcount.owner
    /// Reports whether a weak owner is currently recorded.
    ///
    /// The owner is a non-owning back-reference: it does not keep its
    /// referent alive, and a set owner suppresses the free that
    /// `GIT_REFCOUNT_DEC` would otherwise perform.
    #[must_use]
    pub fn has_owner(&self) -> bool {
        !self.owner_ptr().is_null()
    }

    /// Borrows the recorded weak owner as a `T` handle.
    ///
    /// Returns `None` when no owner is recorded.
    ///
    /// # Safety
    ///
    /// The stored pointer is type-erased, so the caller must supply the
    /// wrapper for the concrete type this header's embedder records — a
    /// repository for an object database or reference database, an attribute
    /// cache entry for an attribute file. The referent must also outlive
    /// `'a`, the region this header is borrowed for: the header records the
    /// pointer without holding a reference on it.
    pub unsafe fn owner<T>(&self) -> Option<T::Ref<'a>>
    where
        T: CCell + 'a,
    {
        // SAFETY: the caller asserts that a recorded owner addresses a live
        // `T` outlasting this borrow.
        NonNull::new(self.owner_ptr().cast::<T>()).map(|owner| unsafe { T::ref_from_raw(owner) })
    }

    /// Atomically loads the erased owner slot.
    fn owner_ptr(&self) -> *mut c_void {
        // SAFETY: as `count`, for the owner slot's atomic load.
        unsafe { ffi::crustify_git_refcount_owner(self.as_ptr().cast_mut()) }
    }
}

impl GitRefcountMut<'_> {
    /// Records a weak owner for the embedding object.
    ///
    /// This is `GIT_REFCOUNT_OWN`: the header stores the pointer without
    /// taking a reference on it, and never releases it.
    ///
    /// # Safety
    ///
    /// `owner` must address a live object that stays live until this slot is
    /// overwritten or cleared, because neither this header nor its embedder
    /// keeps the referent alive. Erase the pointer from the owner's own
    /// handle with `as_mut_void_ptr` so that reads through
    /// [`owner`](GitRefcountRef::owner) reconstitute the type it was erased
    /// from.
    ///
    /// While an owner is recorded, `GIT_REFCOUNT_DEC` stops freeing the
    /// embedding object, so the caller also takes on that object's release.
    pub unsafe fn set_owner(&mut self, owner: NonNull<c_void>) {
        // SAFETY: this exclusive handle addresses an initialized header and
        // the shim performs the atomic swap `GIT_REFCOUNT_OWN` expands to.
        unsafe { ffi::crustify_git_refcount_own(self.as_mut_ptr(), owner.as_ptr()) }
    }

    /// Clears the recorded weak owner, restoring `GIT_REFCOUNT_DEC`'s free.
    ///
    /// Storing a null owner cannot dangle, so no caller obligation remains.
    pub fn clear_owner(&mut self) {
        // SAFETY: as `set_owner`, and null is the C representation of an
        // unowned header.
        unsafe { ffi::crustify_git_refcount_own(self.as_mut_ptr(), core::ptr::null_mut()) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;
    use crate::oid::Oid;

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

    #[test]
    fn the_count_reads_the_atomic_field() {
        let mut raw = ffi::git_refcount {
            refcount: ffi::git_atomic32 { val: 3 },
            owner: core::ptr::null_mut::<c_void>(),
        };

        // SAFETY: as above, for an initialized stack header.
        let refcount = unsafe { GitRefcountRef::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(refcount.count(), 3);
    }

    #[test]
    fn the_weak_owner_round_trips_through_the_erased_slot() {
        let mut owner = ffi::git_oid {
            type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
            id: [0; 32],
        };
        let mut raw = ffi::git_refcount {
            refcount: ffi::git_atomic32 { val: 1 },
            owner: core::ptr::null_mut::<c_void>(),
        };

        // SAFETY: as above, for an initialized stack header.
        let mut refcount = unsafe { GitRefcountMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert!(!refcount.as_ref().has_owner());
        // SAFETY: no owner is recorded, so nothing is reconstituted yet.
        assert!(unsafe { refcount.as_ref().owner::<Oid>() }.is_none());

        // SAFETY: `owner` is a live stack value outliving every read below,
        // and the pointer is erased from the very type read back.
        unsafe {
            refcount.set_owner(
                NonNull::new((&raw mut owner).cast::<c_void>())
                    .expect("the address of a stack value is non-null"),
            );
        }
        assert!(refcount.as_ref().has_owner());
        // SAFETY: the recorded owner is the live `git_oid` erased above.
        let recorded =
            unsafe { refcount.as_ref().owner::<Oid>() }.expect("the recorded owner was just set");
        assert_eq!(recorded.as_ptr(), &raw const owner);

        refcount.clear_owner();
        assert!(!refcount.as_ref().has_owner());
    }
}
