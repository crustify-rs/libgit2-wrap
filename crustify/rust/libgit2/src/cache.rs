//! Safe wrappers for libgit2 cache APIs.

use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CBox, CCloned, CDropped};

use crate::api::types::GitObjectType;
use crate::ffi;
use crate::oid::{OidMut, OidRef};

ffibox::define_ctype!(
    /// Wraps: git_cached_obj
    /// The layout-compatible, reference-counted prefix shared by cached
    /// parsed objects and cached object-database values.
    GitCachedObj,
    GitCachedObjRef,
    GitCachedObjMut,
    ffi::git_cached_obj
);

/// An independently owned reference to cached object storage.
///
/// Cloning atomically acquires another reference. Dropping dispatches the
/// final destruction according to the object's checked cache-storage kind.
pub type GitCachedObjOwned = CBox<GitCachedObj>;

/// The concrete representation held in a cache entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum CacheStoreKind {
    /// Not yet classified for the cache.
    ///
    /// This is the state a freshly allocated object carries until
    /// `git_cache_store_raw` or `git_cache_store_parsed` claims it; the
    /// cache also uses it as the wildcard when looking an entry up. A final
    /// reference released in this state is handed to the plain allocator
    /// free rather than to either concrete destructor.
    Any = 0,
    /// A cached object-database value.
    Raw = 1,
    /// A cached parsed [`crate::object::GitObject`].
    Parsed = 2,
}

/// An unrecognized cache-storage discriminator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidCacheStoreKind(u16);

impl InvalidCacheStoreKind {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for CacheStoreKind {
    type Error = InvalidCacheStoreKind;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Any),
            1 => Ok(Self::Raw),
            2 => Ok(Self::Parsed),
            value => Err(InvalidCacheStoreKind(value)),
        }
    }
}

/// An object-kind value that cannot be represented by a live cache object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidCachedObjectType(i16);

impl InvalidCachedObjectType {
    /// Returns the unrecognized C field value.
    #[must_use]
    pub const fn value(self) -> i16 {
        self.0
    }
}

impl<'a> GitCachedObjRef<'a> {
    /// Field: git_cached_obj.size
    /// Returns the memory size charged to the cache for this object.
    #[must_use]
    pub fn size(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).size).read() }
    }

    /// Field: git_cached_obj.oid
    /// Borrows the object identifier embedded in this cache prefix.
    #[must_use]
    pub fn oid(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection points at the initialized embedded OID,
        // which remains live for this cache object's `'a` borrow.
        unsafe { OidRef::from_ptr(addr_of!((*self.as_ptr()).oid).cast_mut()) }
            .expect("an embedded field has a non-null address")
    }

    /// Field: git_cached_obj.refcount
    /// Atomically reads the number of outstanding references to this cached
    /// object.
    ///
    /// The value is a snapshot: another thread holding the cache lock may
    /// acquire or release a reference before the caller observes it. Owned
    /// references are counted by [`GitCachedObjOwned`], which is the safe
    /// way to hold one.
    #[must_use]
    pub fn refcount(&self) -> i32 {
        // SAFETY: this live shared handle addresses an initialized cached
        // object, and the shim only performs an atomic load through it, so
        // passing the pointer as `*mut` writes nothing.
        unsafe { ffi::crustify_git_cached_obj_refcount(self.as_ptr().cast_mut()) }
    }

    /// Field: git_cached_obj.flags
    /// Returns the checked representation stored in the cache.
    pub fn store_kind(&self) -> Result<CacheStoreKind, InvalidCacheStoreKind> {
        // SAFETY: as `size`, for the initialized discriminator field.
        let raw = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        CacheStoreKind::try_from(raw)
    }

    /// Field: git_cached_obj.type
    /// Returns the checked Git object kind stored in this cache prefix.
    pub fn object_type(&self) -> Result<GitObjectType, InvalidCachedObjectType> {
        // SAFETY: as `size`, for the initialized object-kind field.
        let raw = unsafe { addr_of!((*self.as_ptr()).type_).read() };
        GitObjectType::from_raw(ffi::git_object_t::from(raw)).ok_or(InvalidCachedObjectType(raw))
    }
}

impl GitCachedObjMut<'_> {
    /// Sets the memory size charged to the cache for this object.
    ///
    /// libgit2 charges `size` to the owning cache when the object is stored
    /// and refunds the same field when it is evicted, so changing it while
    /// the object is published leaves the cache's memory accounting adrift.
    pub fn set_size(&mut self, size: usize) {
        // SAFETY: this exclusive handle permits a raw-place scalar write
        // without forming a reference to C-visible memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).size).write(size) }
    }

    /// Borrows the embedded object identifier exclusively.
    #[must_use]
    pub fn oid_mut(&mut self) -> OidMut<'_> {
        // SAFETY: the projected field is live and the returned handle is tied
        // to this exclusive reborrow.
        unsafe { OidMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).oid)) }
            .expect("an embedded field has a non-null address")
    }

    /// Copies an object identifier into the embedded field.
    ///
    /// A published cache entry is keyed by a pointer to this very field, so
    /// overwriting it while the object is in a cache makes the entry
    /// unreachable under both its old and its new identifier.
    pub fn set_oid(&mut self, oid: OidRef<'_>) {
        // SAFETY: both OIDs are initialized and layout-compatible; `copy`
        // also permits the degenerate overlapping case without references.
        unsafe { core::ptr::copy(oid.as_ptr(), addr_of_mut!((*self.as_mut_ptr()).oid), 1) }
    }

    /// Selects the concrete representation stored in the cache.
    ///
    /// # Safety
    ///
    /// `kind` must describe the allocation that begins with this prefix. It
    /// controls which concrete destructor runs when the final reference drops.
    pub unsafe fn set_store_kind(&mut self, kind: CacheStoreKind) {
        // SAFETY: this exclusive handle permits a raw-place scalar write, and
        // every enum discriminant is a valid cache-object representation.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(kind as u16) }
    }

    /// Sets a concrete object kind.
    ///
    /// Wildcard and invalid sentinels are rejected because they cannot index
    /// libgit2's cache-size policy table.
    ///
    /// # Safety
    ///
    /// For a parsed object, `object_type` must match its concrete allocation;
    /// the value selects the concrete destructor for that allocation.
    pub unsafe fn set_object_type(
        &mut self,
        object_type: GitObjectType,
    ) -> Result<(), InvalidCachedObjectType> {
        let raw = object_type.as_raw();
        if !matches!(
            object_type,
            GitObjectType::COMMIT | GitObjectType::TREE | GitObjectType::BLOB | GitObjectType::TAG
        ) {
            return Err(InvalidCachedObjectType(raw as i16));
        }

        // SAFETY: this exclusive handle permits a raw-place scalar write, and
        // the accepted published object kinds all fit the C `int16_t` field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).type_).write(raw as i16) }
        Ok(())
    }
}

// SAFETY: the shim consumes one reference to a fully initialized cached-object
// prefix and dispatches the final destructor using its representation flag.
unsafe impl CDropped for GitCachedObj {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the `CDropped` caller supplies one live owned reference, and
        // `GitCachedObj` is transparent over `ffi::git_cached_obj`.
        unsafe { ffi::crustify_git_cached_obj_decref(obj.as_ptr().cast()) }
    }
}

// SAFETY: the shim atomically increments the live object's reference count and
// therefore creates one independent reference released by `CDropped` above.
unsafe impl CCloned for GitCachedObj {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        // SAFETY: the `CCloned` caller supplies a live cached-object prefix.
        unsafe { ffi::crustify_git_cached_obj_incref(obj.as_ptr().cast()) }
        Some(obj)
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CCloned, CDropped};

    use super::*;
    use crate::oid::OidType;

    #[test]
    fn cached_object_preserves_layout_and_refcount_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_refcounted<T: CDropped + CCloned>() {}

        assert_cell::<GitCachedObj>();
        assert_refcounted::<GitCachedObj>();
        assert_eq!(size_of::<GitCachedObj>(), size_of::<ffi::git_cached_obj>());
        assert_eq!(
            align_of::<GitCachedObj>(),
            align_of::<ffi::git_cached_obj>()
        );
        assert_eq!(
            size_of::<GitCachedObjRef<'_>>(),
            size_of::<*const ffi::git_cached_obj>()
        );
        assert_eq!(
            size_of::<GitCachedObjMut<'_>>(),
            size_of::<*mut ffi::git_cached_obj>()
        );
        assert_eq!(
            size_of::<Option<GitCachedObjOwned>>(),
            size_of::<*mut ffi::git_cached_obj>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_checked_fields() {
        let mut raw = ffi::git_cached_obj {
            oid: ffi::git_oid {
                type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
                id: [0; 32],
            },
            type_: GitObjectType::BLOB.as_raw() as i16,
            flags: CacheStoreKind::Raw as u16,
            size: 12,
            refcount: ffi::git_atomic32 { val: 1 },
        };

        // SAFETY: `raw` is a live initialized value and remains exclusively
        // borrowed until the handle's final use.
        let mut cached = unsafe { GitCachedObjMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(cached.as_ref().size(), 12);
        assert_eq!(cached.as_ref().refcount(), 1);
        assert_eq!(cached.as_ref().store_kind(), Ok(CacheStoreKind::Raw));
        assert_eq!(cached.as_ref().object_type(), Ok(GitObjectType::BLOB));
        assert_eq!(cached.as_ref().oid().oid_type(), Ok(OidType::Sha1));

        cached.set_size(99);
        // SAFETY: this stack value is only a cache-prefix test fixture; it is
        // never dispatched to either concrete destructor.
        unsafe { cached.set_store_kind(CacheStoreKind::Parsed) };
        // SAFETY: as above, no concrete parsed allocation is claimed or freed.
        unsafe { cached.set_object_type(GitObjectType::COMMIT) }.unwrap();
        cached.oid_mut().set_oid_type(OidType::Sha256);

        assert_eq!(cached.as_ref().size(), 99);
        assert_eq!(cached.as_ref().store_kind(), Ok(CacheStoreKind::Parsed));
        assert_eq!(cached.as_ref().object_type(), Ok(GitObjectType::COMMIT));
        assert_eq!(cached.as_ref().oid().oid_type(), Ok(OidType::Sha256));
        assert_eq!(
            // SAFETY: the invalid sentinel is rejected before any write.
            unsafe { cached.set_object_type(GitObjectType::ANY) },
            Err(InvalidCachedObjectType(GitObjectType::ANY.as_raw() as i16))
        );
    }

    #[test]
    fn discriminators_map_every_published_c_value() {
        assert_eq!(CacheStoreKind::try_from(0), Ok(CacheStoreKind::Any));
        assert_eq!(CacheStoreKind::try_from(1), Ok(CacheStoreKind::Raw));
        assert_eq!(CacheStoreKind::try_from(2), Ok(CacheStoreKind::Parsed));
        assert_eq!(CacheStoreKind::try_from(3), Err(InvalidCacheStoreKind(3)));
        assert_eq!(InvalidCacheStoreKind(7).value(), 7);
        assert_eq!(InvalidCachedObjectType(-7).value(), -7);
    }

    #[test]
    fn owned_references_clone_and_release_through_the_cache_shims() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        // SAFETY: the allocator is initialized and the requested size is the
        // exact complete cached-object layout.
        let raw = unsafe { ffi::crustify_git__malloc(size_of::<ffi::git_cached_obj>()) }
            .cast::<ffi::git_cached_obj>();
        assert!(!raw.is_null());
        // SAFETY: `raw` owns suitably aligned writable storage of the exact
        // type and no initialized value currently occupies it.
        unsafe {
            raw.write(ffi::git_cached_obj {
                oid: ffi::git_oid {
                    type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
                    id: [0; 32],
                },
                type_: GitObjectType::BLOB.as_raw() as i16,
                // The generic finalizer path releases this test allocation.
                flags: CacheStoreKind::Any as u16,
                size: 0,
                refcount: ffi::git_atomic32 { val: 1 },
            });
        }

        // SAFETY: `raw` now holds one fully initialized owned cache reference.
        let owner = unsafe { GitCachedObjOwned::from_raw(raw) }.unwrap();
        assert_eq!(owner.as_ref().store_kind(), Ok(CacheStoreKind::Any));
        assert_eq!(owner.as_ref().refcount(), 1);
        let duplicate = owner.clone();
        assert_eq!(owner.as_ref().as_ptr(), duplicate.as_ref().as_ptr());
        assert_eq!(owner.as_ref().refcount(), 2);
        // SAFETY: both owners are live, no refcount operation is concurrent,
        // and this quiescent test-only observation does not race with C.
        assert_eq!(unsafe { addr_of!((*raw).refcount.val).read() }, 2);
        drop(duplicate);
        // SAFETY: the original owner still keeps the allocation alive and no
        // refcount operation is concurrent with this observation.
        assert_eq!(unsafe { addr_of!((*raw).refcount.val).read() }, 1);
        assert_eq!(owner.as_ref().refcount(), 1);
        drop(owner);

        // SAFETY: balances this test's successful initialization after the
        // cache allocation and both references have been released.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
