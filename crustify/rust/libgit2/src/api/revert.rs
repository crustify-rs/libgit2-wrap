//! Safe wrappers for libgit2 revert APIs.

use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::checkout::{GitCheckoutOptions, GitCheckoutOptionsMut, GitCheckoutOptionsRef};
use crate::api::merge::{GitMergeOptions, GitMergeOptionsMut, GitMergeOptionsRef};
use crate::ffi;

/// Wraps: git_revert_options
/// Layout-compatible revert options whose embedded merge and checkout options
/// borrow their configured data for `'data`.
///
/// `'data` is invariant. [`GitRevertOptionsMut::set_merge_options`] copies a
/// `&'data`-borrowing header into the C struct and the embedded headers hand
/// their referents back out. A covariant `'data` would let safe code shrink
/// the parameter on the exclusive handle, install a shorter-lived similarity
/// metric, merge driver name or checkout string, and then read it back
/// through a handle still typed at the longer lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::revert::GitRevertOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitRevertOptionsMut<'object, 'static>,
/// ) -> GitRevertOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitRevertOptions<'data> {
    inner: CType<ffi::git_revert_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitRevertOptions`].
#[repr(transparent)]
pub struct GitRevertOptionsRef<'object, 'data>(CPtr<'object, GitRevertOptions<'data>>);

impl Clone for GitRevertOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitRevertOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitRevertOptions`].
#[repr(transparent)]
pub struct GitRevertOptionsMut<'object, 'data>(GitRevertOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitRevertOptions<'data> {
    type C = ffi::git_revert_options;
    type Ref<'object>
        = GitRevertOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitRevertOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(p: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `p` is a live shared object.
        GitRevertOptionsRef(unsafe { CPtr::new(p) })
    }

    unsafe fn mut_from_raw<'object>(p: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitRevertOptionsMut(GitRevertOptionsRef(unsafe { CPtr::new(p) }))
    }
}

// SAFETY: the outer header owns no allocation; its embedded option wrappers
// also only borrow their configured pointers and need no inline disposal.
unsafe impl CValued for GitRevertOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitRevertOptions<'data> {
    /// Constructs options equivalent to `GIT_REVERT_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every raw field admits zero. Required versions and nested
        // defaults are installed before returning the initialized wrapper.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        let merge: CVal<GitMergeOptions<'data>> = GitMergeOptions::new();
        let checkout: CVal<GitCheckoutOptions<'data>> = GitCheckoutOptions::new();
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_REVERT_OPTIONS_VERSION);
            view.set_merge_options(merge.as_ref());
            // SAFETY: the default checkout options contain no callbacks or
            // other mutable payload, so this inline copy cannot alias state.
            unsafe { view.set_checkout_options(checkout.as_ref()) };
        }
        options
    }
}

impl<'object, 'data> GitRevertOptionsRef<'object, 'data> {
    /// Borrows a raw options pointer, returning `None` for null.
    ///
    /// # Safety
    /// `ptr` must identify an initialized options value live for `'object`.
    /// Every pointer and callback state reachable through its embedded options
    /// must remain valid for `'data`, which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_revert_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitRevertOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_revert_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_revert_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_revert_options.checkout_opts
    /// Borrows the embedded checkout options.
    #[must_use]
    pub fn checkout_options(&self) -> GitCheckoutOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field
        // without forming a reference over C-visible storage.
        let options = unsafe { addr_of!((*self.as_ptr()).checkout_opts).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { GitCheckoutOptionsRef::from_ptr(options) }.expect("an inline field is non-null")
    }

    /// Field: git_revert_options.merge_opts
    /// Borrows the embedded merge options.
    #[must_use]
    pub fn merge_options(&self) -> GitMergeOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field
        // without forming a reference over C-visible storage.
        let options = unsafe { addr_of!((*self.as_ptr()).merge_opts).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { GitMergeOptionsRef::from_ptr(options) }.expect("an inline field is non-null")
    }

    /// Field: git_revert_options.mainline
    /// Returns the selected merge parent, or zero for a non-merge commit.
    #[must_use]
    pub fn mainline(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar.
        unsafe { addr_of!((*self.as_ptr()).mainline).read() }
    }
}

impl<'object, 'data> GitRevertOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw options pointer, returning `None` for null.
    ///
    /// # Safety
    /// The shared-handle requirements apply, and no other access path may use
    /// the value for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_revert_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitRevertOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_revert_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitRevertOptionsRef<'_, 'data> {
        GitRevertOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Replaces the merge-parent selection.
    pub fn set_mainline(&mut self, mainline: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).mainline).write(mainline) }
    }

    /// Copies a non-owning merge-options view into the inline field.
    pub fn set_merge_options(&mut self, options: GitMergeOptionsRef<'_, 'data>) {
        // SAFETY: `options` identifies an initialized header and copying it
        // transfers no ownership; the outer lifetime retains its borrows.
        let options = unsafe { options.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).merge_opts).write(options) }
    }

    /// Copies a non-owning checkout-options view into the inline field.
    ///
    /// # Safety
    /// If `options` contains callbacks, no invocation may overlap between the
    /// source header, this copy, or any further C copies. All referenced
    /// callback state must remain exclusively reserved for `'data`.
    pub unsafe fn set_checkout_options(&mut self, options: GitCheckoutOptionsRef<'_, 'data>) {
        // SAFETY: `options` identifies an initialized header and copying it
        // transfers no ownership; the outer lifetime retains its borrows.
        let options = unsafe { options.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_opts).write(options) }
    }

    /// Exclusively borrows the embedded merge options.
    #[must_use]
    pub fn merge_options_mut(&mut self) -> GitMergeOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let options = unsafe { addr_of_mut!((*self.as_mut_ptr()).merge_opts) };
        // SAFETY: the field is non-null and exclusively borrowed for the
        // returned handle's lifetime.
        unsafe { GitMergeOptionsMut::from_ptr(options) }.expect("an inline field is non-null")
    }

    /// Exclusively borrows the embedded checkout options.
    #[must_use]
    pub fn checkout_options_mut(&mut self) -> GitCheckoutOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let options = unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_opts) };
        // SAFETY: the field is non-null and exclusively borrowed for the
        // returned handle's lifetime.
        unsafe { GitCheckoutOptionsMut::from_ptr(options) }.expect("an inline field is non-null")
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn revert_options_preserve_layout_and_published_defaults() {
        assert_eq!(
            size_of::<GitRevertOptions<'static>>(),
            size_of::<ffi::git_revert_options>()
        );
        assert_eq!(
            align_of::<GitRevertOptions<'static>>(),
            align_of::<ffi::git_revert_options>()
        );
        assert_eq!(
            size_of::<GitRevertOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_revert_options>()
        );
        let options = GitRevertOptions::new();
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_REVERT_OPTIONS_VERSION);
        assert_eq!(view.mainline(), 0);
        assert_eq!(
            view.merge_options().version(),
            ffi::GIT_MERGE_OPTIONS_VERSION
        );
        assert_eq!(
            view.checkout_options().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
    }

    #[test]
    fn scalar_and_embedded_options_are_exclusively_mutable() {
        let mut options = GitRevertOptions::new();
        {
            let mut view = options.as_mut();
            view.set_mainline(2);
            view.merge_options_mut().set_rename_threshold(73);
            view.checkout_options_mut().set_disable_filters(true);
        }
        let view = options.as_ref();
        assert_eq!(view.mainline(), 2);
        assert_eq!(view.merge_options().rename_threshold(), 73);
        assert!(view.checkout_options().disable_filters());
    }
}
