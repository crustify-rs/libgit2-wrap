//! Safe wrappers for libgit2 cherrypick APIs.

use crate::api::checkout::{GitCheckoutOptionsMut, GitCheckoutOptionsRef};
use crate::api::merge::{GitMergeOptionsMut, GitMergeOptionsRef};
use crate::ffi;

/// Wraps: git_cherrypick_options
/// Layout-compatible cherry-pick options whose embedded option headers borrow
/// caller-owned data for `'data`.
///
/// `'data` is invariant. The embedded merge and checkout headers reached
/// through [`GitCherrypickOptionsMut::merge_options_mut`] and
/// [`GitCherrypickOptionsMut::checkout_options_mut`] store `&'data` referents
/// and hand them back out. A covariant `'data` would let safe code shrink the
/// parameter on the exclusive handle, configure a shorter-lived string, tree
/// or index through those headers, and then read it back through a handle
/// still typed at the longer lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::cherrypick::GitCherrypickOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitCherrypickOptionsMut<'object, 'static>,
/// ) -> GitCherrypickOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitCherrypickOptions<'data> {
    inner: ffibox::CType<ffi::git_cherrypick_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: core::marker::PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitCherrypickOptions`].
#[repr(transparent)]
pub struct GitCherrypickOptionsRef<'object, 'data>(
    ffibox::CPtr<'object, GitCherrypickOptions<'data>>,
);

impl Clone for GitCherrypickOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitCherrypickOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitCherrypickOptions`].
#[repr(transparent)]
pub struct GitCherrypickOptionsMut<'object, 'data>(GitCherrypickOptionsRef<'object, 'data>);

// SAFETY: the layout is transparent over the matching bindgen struct; the
// pointer-sized handles use raw-place access and the shared one cannot write.
unsafe impl<'data> ffibox::CCell for GitCherrypickOptions<'data> {
    type C = ffi::git_cherrypick_options;
    type Ref<'object>
        = GitCherrypickOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitCherrypickOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: core::ptr::NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitCherrypickOptionsRef(unsafe { ffibox::CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: core::ptr::NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitCherrypickOptionsMut(GitCherrypickOptionsRef(unsafe { ffibox::CPtr::new(ptr) }))
    }
}

// SAFETY: the embedded option headers only borrow configured data and own no
// resource, so disposing this inline aggregate requires no action.
unsafe impl ffibox::CValued for GitCherrypickOptions<'_> {
    unsafe fn c_dispose(_this: core::ptr::NonNull<Self>) {}
}

impl<'data> GitCherrypickOptions<'data> {
    /// Constructs options equivalent to `GIT_CHERRYPICK_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> ffibox::CVal<Self> {
        // SAFETY: every field admits the all-zero bit pattern; all published
        // nonzero defaults are installed before return.
        let inner = unsafe { ffibox::CType::zeroed() };
        let mut options = ffibox::CVal::new(Self {
            inner,
            _data: core::marker::PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_CHERRYPICK_OPTIONS_VERSION);
            {
                let mut merge = view.merge_options_mut();
                merge.set_version(ffi::GIT_MERGE_OPTIONS_VERSION);
                merge.set_flags(crate::api::merge::GitMergeFlags::FIND_RENAMES);
            }
            view.checkout_options_mut()
                .set_version(ffi::GIT_CHECKOUT_OPTIONS_VERSION);
        }
        options
    }
}

impl<'object, 'data> GitCherrypickOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify initialized options live for `'object`. Every value
    /// borrowed by the embedded headers remains valid for `'data`, and
    /// `'data` outlives `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_cherrypick_options) -> Option<Self> {
        core::ptr::NonNull::new(ptr.cast::<GitCherrypickOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { ffibox::CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_cherrypick_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_cherrypick_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_cherrypick_options.checkout_opts
    /// Borrows the embedded checkout options header.
    #[must_use]
    pub fn checkout_options(&self) -> GitCheckoutOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { core::ptr::addr_of!((*self.as_ptr()).checkout_opts).cast_mut() };
        // SAFETY: the projected inline field lives for this aggregate borrow.
        unsafe { GitCheckoutOptionsRef::from_ptr(field) }.expect("inline field is non-null")
    }

    /// Field: git_cherrypick_options.merge_opts
    /// Borrows the embedded merge options header.
    #[must_use]
    pub fn merge_options(&self) -> GitMergeOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { core::ptr::addr_of!((*self.as_ptr()).merge_opts).cast_mut() };
        // SAFETY: the projected inline field lives for this aggregate borrow.
        unsafe { GitMergeOptionsRef::from_ptr(field) }.expect("inline field is non-null")
    }

    /// Field: git_cherrypick_options.mainline
    /// Returns the selected mainline parent, or zero for a non-merge commit.
    #[must_use]
    pub fn mainline(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).mainline).read() }
    }
}

impl<'object, 'data> GitCherrypickOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path may use
    /// the options for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_cherrypick_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitCherrypickOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_cherrypick_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitCherrypickOptionsRef<'_, 'data> {
        GitCherrypickOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Exclusively borrows the embedded checkout options header.
    #[must_use]
    pub fn checkout_options_mut(&mut self) -> GitCheckoutOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field
        // through this exclusive aggregate handle.
        let field = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).checkout_opts) };
        // SAFETY: the field is live and exclusively borrowed with `self`.
        unsafe { GitCheckoutOptionsMut::from_ptr(field) }.expect("inline field is non-null")
    }

    /// Exclusively borrows the embedded merge options header.
    #[must_use]
    pub fn merge_options_mut(&mut self) -> GitMergeOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field
        // through this exclusive aggregate handle.
        let field = unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).merge_opts) };
        // SAFETY: the field is live and exclusively borrowed with `self`.
        unsafe { GitMergeOptionsMut::from_ptr(field) }.expect("inline field is non-null")
    }

    /// Selects the mainline parent, or zero for a non-merge commit.
    pub fn set_mainline(&mut self, mainline: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).mainline).write(mainline) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn options_preserve_layout_and_published_defaults() {
        assert_eq!(
            size_of::<GitCherrypickOptions<'static>>(),
            size_of::<ffi::git_cherrypick_options>()
        );
        assert_eq!(
            align_of::<GitCherrypickOptions<'static>>(),
            align_of::<ffi::git_cherrypick_options>()
        );
        assert_eq!(
            size_of::<GitCherrypickOptionsRef<'static, 'static>>(),
            size_of::<*mut ffi::git_cherrypick_options>()
        );

        let options = GitCherrypickOptions::new();
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_CHERRYPICK_OPTIONS_VERSION
        );
        assert_eq!(options.as_ref().mainline(), 0);
        assert_eq!(
            options.as_ref().merge_options().version(),
            ffi::GIT_MERGE_OPTIONS_VERSION
        );
        assert_eq!(
            options.as_ref().merge_options().flags(),
            Ok(crate::api::merge::GitMergeFlags::FIND_RENAMES)
        );
        assert_eq!(
            options.as_ref().checkout_options().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
    }

    #[test]
    fn scalar_and_embedded_mutation_use_exclusive_handles() {
        let mut options = GitCherrypickOptions::new();
        {
            let mut view = options.as_mut();
            view.set_mainline(2);
            view.merge_options_mut().set_recursion_limit(7);
            view.checkout_options_mut().set_file_mode(0o100644);
        }
        assert_eq!(options.as_ref().mainline(), 2);
        assert_eq!(options.as_ref().merge_options().recursion_limit(), 7);
        assert_eq!(options.as_ref().checkout_options().file_mode(), 0o100644);
    }
}
