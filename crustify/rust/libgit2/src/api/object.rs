//! Safe wrappers for libgit2 object APIs.

use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::types::GitObjectType;
use crate::ffi;
use crate::filter::GitFilterListRef;
use crate::oid::{InvalidOidType, OidType};

/// Wraps: git_object_id_options
/// Layout-compatible object-ID options whose optional filter list is borrowed
/// for `'data`.
///
/// The data lifetime is invariant: a mutable handle can install a filter list
/// and a later shared handle can borrow it back. Invariance prevents safe code
/// from shrinking `'data` while setting the field and then observing a stale
/// pointer through a longer-lived handle.
#[repr(transparent)]
pub struct GitObjectIdOptions<'data> {
    inner: CType<ffi::git_object_id_options>,
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitObjectIdOptions`].
#[repr(transparent)]
pub struct GitObjectIdOptionsRef<'object, 'data>(CPtr<'object, GitObjectIdOptions<'data>>);

impl Clone for GitObjectIdOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitObjectIdOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitObjectIdOptions`].
#[repr(transparent)]
pub struct GitObjectIdOptionsMut<'object, 'data>(GitObjectIdOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitObjectIdOptions<'data> {
    type C = ffi::git_object_id_options;
    type Ref<'object>
        = GitObjectIdOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitObjectIdOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitObjectIdOptionsRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitObjectIdOptionsMut(GitObjectIdOptionsRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: this options header only borrows its filter list and owns no
// resource, so disposing inline storage requires no action.
unsafe impl CValued for GitObjectIdOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitObjectIdOptions<'data> {
    /// Constructs options equivalent to `GIT_OBJECT_ID_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every bindgen field admits the all-zero bit pattern. The
        // required ABI version is installed before the value is returned.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        options
            .as_mut()
            .set_version(ffi::GIT_OBJECT_ID_OPTIONS_VERSION);
        options
    }
}

impl<'object, 'data> GitObjectIdOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify initialized options live for `'object`. A non-null
    /// filter list must remain live and shared-only for `'data`, which must
    /// outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_object_id_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitObjectIdOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_object_id_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_object_id_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_object_id_options.oid_type
    /// Returns the selected object-ID format, or `None` for libgit2's default.
    pub fn oid_type(&self) -> Result<Option<OidType>, InvalidOidType> {
        // SAFETY: raw-place projection copies the initialized scalar field.
        let raw = unsafe { addr_of!((*self.as_ptr()).oid_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            OidType::try_from(raw).map(Some)
        }
    }

    /// Field: git_object_id_options.filters
    /// Borrows the optional caller-owned filter list.
    #[must_use]
    pub fn filters(&self) -> Option<GitFilterListRef<'object>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let filters = unsafe { addr_of!((*self.as_ptr()).filters).read() };
        // SAFETY: the wrapper contract keeps a non-null filter list live and
        // shared-only for at least this options borrow.
        unsafe { GitFilterListRef::from_ptr(filters) }
    }

    /// Field: git_object_id_options.object_type
    /// Returns the configured object kind, or `None` for the default blob.
    pub fn object_type(&self) -> Result<Option<GitObjectType>, ffi::git_object_t> {
        // SAFETY: raw-place projection copies the initialized scalar field.
        let raw = unsafe { addr_of!((*self.as_ptr()).object_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            GitObjectType::from_raw(raw).map(Some).ok_or(raw)
        }
    }
}

impl<'object, 'data> GitObjectIdOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path to the
    /// options value may be used for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_object_id_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitObjectIdOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_object_id_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitObjectIdOptionsRef<'_, 'data> {
        GitObjectIdOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects an object-ID format, or the libgit2 default with `None`.
    pub fn set_oid_type(&mut self, oid_type: Option<OidType>) {
        let raw = oid_type.map_or(0, Into::into);
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).oid_type).write(raw) }
    }

    /// Stores an optional borrowed filter list.
    pub fn set_filters(&mut self, filters: Option<GitFilterListRef<'data>>) {
        let filters = filters.map_or(core::ptr::null(), |filters| filters.as_ptr());
        // SAFETY: this exclusive handle permits the pointer write, and the
        // invariant wrapper lifetime keeps a non-null list alive and shared.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).filters).write(filters.cast_mut()) }
    }

    /// Selects an object kind, or the default blob with `None`.
    pub fn set_object_type(&mut self, object_type: Option<GitObjectType>) {
        let raw = object_type.map_or(0, Into::into);
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).object_type).write(raw) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn options_preserve_layout_and_defaults() {
        assert_eq!(
            size_of::<GitObjectIdOptions<'static>>(),
            size_of::<ffi::git_object_id_options>()
        );
        assert_eq!(
            align_of::<GitObjectIdOptions<'static>>(),
            align_of::<ffi::git_object_id_options>()
        );
        assert_eq!(
            size_of::<GitObjectIdOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_object_id_options>()
        );

        let mut options = GitObjectIdOptions::new();
        {
            let mut view = options.as_mut();
            assert_eq!(view.as_ref().version(), ffi::GIT_OBJECT_ID_OPTIONS_VERSION);
            assert_eq!(view.as_ref().oid_type(), Ok(None));
            assert_eq!(view.as_ref().object_type(), Ok(None));
            assert!(view.as_ref().filters().is_none());

            view.set_oid_type(Some(OidType::Sha256));
            view.set_object_type(Some(GitObjectType::BLOB));
            view.set_filters(None);

            assert_eq!(view.as_ref().oid_type(), Ok(Some(OidType::Sha256)));
            assert_eq!(view.as_ref().object_type(), Ok(Some(GitObjectType::BLOB)));
            assert!(view.as_ref().filters().is_none());
        }
    }
}
