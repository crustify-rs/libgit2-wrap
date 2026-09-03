//! Safe wrappers for libgit2 oidarray APIs.

use core::ptr::NonNull;

use ffibox::{CLenDropped, CSlice, CSliceMut, CVal, CVec};

use crate::ffi;
use crate::oid::Oid;

ffibox::define_ctype!(
    /// Wraps: git_oidarray
    /// A layout-compatible inline header owning a counted object-ID run.
    OidArray,
    OidArrayRef,
    OidArrayMut,
    ffi::git_oidarray
);

// `git_oidarray_dispose` releases the owned ID run but retains its inline
// header, so this is a by-value lifecycle rather than a `CBox` destructor.
ffibox::impl_cvalued!(OidArray, ffi::git_oidarray, ffi::git_oidarray_dispose);

/// Deleter strategy for an ID run detached from an [`OidArray`].
pub struct OidArrayIdsFree;

// SAFETY: a detached ID run is uniquely owned and was allocated through
// libgit2's configured allocator. That allocator's `gfree` must remain
// compatible until this strategy drops the run.
unsafe impl CLenDropped for OidArrayIdsFree {
    unsafe fn c_drop_len(ptr: *mut u8, _byte_len: usize) {
        // SAFETY: the `CLenDropped` contract supplies the unique compatible
        // allocation detached from an oidarray.
        unsafe { ffi::crustify_git__free(ptr.cast()) }
    }
}

/// A uniquely owned object-ID run detached from an [`OidArray`].
pub type OidArrayIds = CVec<Oid, OidArrayIdsFree>;

impl OidArray {
    /// Constructs an empty inline owner whose ID run is disposed on drop.
    #[must_use]
    pub fn new() -> CVal<Self> {
        CVal::new(Self::zeroed())
    }
}

impl<'a> OidArrayRef<'a> {
    /// Field: git_oidarray.count
    /// Returns the number of object IDs in the array.
    #[must_use]
    pub fn count(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to C-visible storage.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).count).read() }
    }

    /// Field: git_oidarray.ids
    /// Borrows the counted object-ID run without exposing its owned pointer.
    ///
    /// `None` is the canonical empty state with a null pointer. A non-null
    /// zero-length allocation is represented by an empty view.
    #[must_use]
    pub fn ids(&self) -> Option<CSlice<'a, Oid>> {
        let array = self.as_ptr();
        // SAFETY: both fields are initialized members of this live header and
        // are copied through raw-place projections.
        let (ids, count) = unsafe {
            (
                core::ptr::addr_of!((*array).ids).read(),
                core::ptr::addr_of!((*array).count).read(),
            )
        };
        let ids = NonNull::new(ids.cast::<Oid>())?;
        // SAFETY: a valid non-null oidarray owns `count` contiguous initialized
        // `git_oid` values. The shared view is bounded by the header borrow.
        Some(unsafe { CSlice::from_raw_parts(ids, count) })
    }
}

impl OidArrayMut<'_> {
    /// Borrows the counted object-ID run exclusively.
    #[must_use]
    pub fn ids_mut(&mut self) -> Option<CSliceMut<'_, Oid>> {
        let array = self.as_mut_ptr();
        // SAFETY: both fields are initialized members of this exclusively
        // borrowed header and are copied through raw-place projections.
        let (ids, count) = unsafe {
            (
                core::ptr::addr_of!((*array).ids).read(),
                core::ptr::addr_of!((*array).count).read(),
            )
        };
        let ids = NonNull::new(ids.cast::<Oid>())?;
        // SAFETY: a valid non-null oidarray owns `count` contiguous initialized
        // IDs, and this view is tied to the exclusive handle reborrow.
        Some(unsafe { CSliceMut::from_raw_parts(ids, count) })
    }

    /// Moves the owned ID run out, leaving the header empty.
    #[must_use]
    pub fn take_ids(&mut self) -> Option<OidArrayIds> {
        self.replace_ids(None)
    }

    /// Replaces the owned ID run and returns the previous allocation.
    #[must_use]
    pub fn replace_ids(&mut self, ids: Option<OidArrayIds>) -> Option<OidArrayIds> {
        let (new_ids, new_count) = ids.map_or((core::ptr::null_mut(), 0), |ids| {
            let (ptr, count) = ids.into_raw_parts();
            (ptr.cast::<ffi::git_oid>(), count)
        });

        let array = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits copying and replacing the two
        // coupled ownership fields. Writing the new fields transfers their
        // allocation into the header and transfers the old allocation out.
        let (old_ids, old_count) = unsafe {
            let old_ids = core::ptr::addr_of!((*array).ids).read();
            let old_count = core::ptr::addr_of!((*array).count).read();
            core::ptr::addr_of_mut!((*array).ids).write(new_ids);
            core::ptr::addr_of_mut!((*array).count).write(new_count);
            (old_ids, old_count)
        };

        // SAFETY: the old non-null fields were a uniquely owned allocation
        // compatible with this strategy, and the header no longer owns them.
        unsafe { OidArrayIds::from_raw_parts(old_ids.cast::<Oid>(), old_count) }
    }

    /// Replaces the owned ID run, dropping the previous allocation.
    pub fn set_ids(&mut self, ids: Option<OidArrayIds>) {
        drop(self.replace_ids(ids));
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use crate::oid::OidType;

    use super::*;

    #[test]
    fn wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<OidArray>(), size_of::<ffi::git_oidarray>());
        assert_eq!(align_of::<OidArray>(), align_of::<ffi::git_oidarray>());
        assert_eq!(
            size_of::<OidArrayRef<'_>>(),
            size_of::<*const ffi::git_oidarray>()
        );
        assert_eq!(
            size_of::<OidArrayMut<'_>>(),
            size_of::<*mut ffi::git_oidarray>()
        );
    }

    #[test]
    fn empty_owner_has_the_public_empty_state() {
        let array = OidArray::new();
        assert_eq!(array.as_ref().count(), 0);
        assert!(array.as_ref().ids().is_none());
    }

    #[test]
    fn owned_ids_can_be_borrowed_mutated_and_detached() {
        // SAFETY: libgit2 initialization is refcounted and balanced below
        // after every configured allocation has been dropped.
        let init_count = unsafe { ffi::git_libgit2_init() };
        assert!(init_count > 0);

        let count = 2;
        // SAFETY: libgit2 is initialized, and a non-null result is a fresh
        // allocation compatible with `OidArrayIdsFree`.
        let raw = unsafe { ffi::crustify_git__malloc(count * size_of::<ffi::git_oid>()) }
            .cast::<ffi::git_oid>();
        assert!(!raw.is_null());
        for index in 0..count {
            // SAFETY: `raw` holds `count` writable `git_oid` slots; each is
            // initialized exactly once before the allocation is wrapped.
            unsafe {
                raw.add(index).write(ffi::git_oid {
                    type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
                    id: [index as u8; 32],
                });
            }
        }
        // SAFETY: the initialized run is uniquely owned and compatible with
        // the selected libgit2 allocator strategy.
        let ids = unsafe { OidArrayIds::from_raw_parts(raw.cast::<Oid>(), count) }
            .expect("the checked allocation is non-null");

        let mut array = OidArray::new();
        array.as_mut().set_ids(Some(ids));
        assert_eq!(array.as_ref().count(), 2);
        assert_eq!(
            array
                .as_ref()
                .ids()
                .and_then(|ids| ids.get(1))
                .unwrap()
                .oid_type(),
            Ok(OidType::Sha1)
        );

        {
            let mut array_mut = array.as_mut();
            let mut ids = array_mut.ids_mut().expect("the run is non-null");
            ids.get_mut(1).unwrap().set_oid_type(OidType::Sha256);
        }
        assert_eq!(
            array
                .as_ref()
                .ids()
                .and_then(|ids| ids.get(1))
                .unwrap()
                .oid_type(),
            Ok(OidType::Sha256)
        );

        let detached = array.as_mut().take_ids().expect("the run is owned");
        assert_eq!(detached.count(), 2);
        assert_eq!(array.as_ref().count(), 0);
        assert!(array.as_ref().ids().is_none());
        drop(detached);
        drop(array);

        // SAFETY: balances this test's successful initialization call.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
    }
}
