//! Safe wrappers for libgit2 stash APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::types::GitSignatureRef;
use crate::ffi;
use crate::oid::OidRef;
use crate::stash::StashApplyProgress;
use crate::strarray::GitStrArrayRef;

/// Wraps: git_stash_apply_progress_cb
/// Safe callable surface for stash-application progress notifications.
pub trait GitStashApplyProgressCallback {
    /// Returns zero to continue, or a negative value to abort.
    fn call(&mut self, progress: StashApplyProgress) -> i32;
}

impl<F> GitStashApplyProgressCallback for F
where
    F: FnMut(StashApplyProgress) -> i32,
{
    fn call(&mut self, progress: StashApplyProgress) -> i32 {
        self(progress)
    }
}

/// Wraps: git_stash_cb
/// Safe callable surface for one transient stash-list entry.
pub trait GitStashCallback {
    /// Returns zero to continue iteration or nonzero to stop.
    ///
    /// `message` is the stash reflog entry's message. It is `None` for an
    /// entry whose reflog line carries no message part, which libgit2 stores
    /// as a null `git_reflog_entry::msg`.
    fn call(&mut self, index: usize, message: Option<&CStr>, stash_id: OidRef<'_>) -> i32;
}

impl<F> GitStashCallback for F
where
    F: FnMut(usize, Option<&CStr>, OidRef<'_>) -> i32,
{
    fn call(&mut self, index: usize, message: Option<&CStr>, stash_id: OidRef<'_>) -> i32 {
        self(index, message, stash_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::Oid;

    #[test]
    fn callback_surfaces_pass_checked_values() {
        let mut progress = |value| i32::from(value == StashApplyProgress::Done);
        assert_eq!(
            GitStashApplyProgressCallback::call(&mut progress, StashApplyProgress::Done,),
            1
        );

        let oid = Oid::zeroed();
        let raw = core::ptr::addr_of!(oid)
            .cast::<crate::ffi::git_oid>()
            .cast_mut();
        // SAFETY: the local layout-compatible OID remains live throughout the call.
        let oid = unsafe { OidRef::from_ptr(raw) }.unwrap();
        let mut entry = |index, message: Option<&CStr>, _: OidRef<'_>| {
            i32::from(index == 2 && message == Some(c"stash message"))
        };
        assert_eq!(
            GitStashCallback::call(&mut entry, 2, Some(c"stash message"), oid),
            1
        );
        assert_eq!(GitStashCallback::call(&mut entry, 2, None, oid), 0);
    }
}

/// Wraps: git_stash_save_options
/// Layout-compatible stash-save options whose configured pointers borrow
/// caller-owned values for the options lifetime.
///
/// `'data` is invariant. The setters store a `&'data` referent into the C
/// struct while the getters hand one back out, so a covariant `'data` would
/// let safe code shrink the parameter on the exclusive handle, install a
/// shorter-lived string or signature, and then read it back through a handle
/// still typed at the longer lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::stash::GitStashSaveOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitStashSaveOptionsMut<'object, 'static>,
/// ) -> GitStashSaveOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitStashSaveOptions<'data> {
    inner: CType<ffi::git_stash_save_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitStashSaveOptions`].
#[repr(transparent)]
pub struct GitStashSaveOptionsRef<'object, 'data>(CPtr<'object, GitStashSaveOptions<'data>>);

impl Clone for GitStashSaveOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitStashSaveOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitStashSaveOptions`].
#[repr(transparent)]
pub struct GitStashSaveOptionsMut<'object, 'data>(GitStashSaveOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and never form references to C-visible
// storage, and the shared handle exposes no writes.
unsafe impl<'data> CCell for GitStashSaveOptions<'data> {
    type C = ffi::git_stash_save_options;
    type Ref<'object>
        = GitStashSaveOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitStashSaveOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(p: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `p` is a live shared object.
        GitStashSaveOptionsRef(unsafe { CPtr::new(p) })
    }

    unsafe fn mut_from_raw<'object>(p: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitStashSaveOptionsMut(GitStashSaveOptionsRef(unsafe { CPtr::new(p) }))
    }
}

// SAFETY: this header only borrows its pointer fields and owns no resource;
// disposing inline storage therefore requires no action.
unsafe impl CValued for GitStashSaveOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitStashSaveOptions<'data> {
    /// Constructs options equivalent to `GIT_STASH_SAVE_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every field of the bindgen C struct admits an all-zero bit
        // pattern; the version is set before the value is returned.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        options
            .as_mut()
            .set_version(ffi::GIT_STASH_SAVE_OPTIONS_VERSION);
        options
    }
}

impl<'object, 'data> GitStashSaveOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify a valid initialized options value that lives for
    /// `'object`. Every non-null configured pointer must remain valid and
    /// immutable for `'data`, and `'data` must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_stash_save_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitStashSaveOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_stash_save_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_stash_save_options.message
    /// Borrows the optional stash description.
    #[must_use]
    pub fn message(&self) -> Option<&'object CStr> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let message = unsafe { addr_of!((*self.as_ptr()).message).read() };
        if message.is_null() {
            None
        } else {
            // SAFETY: safe construction only stores live NUL-terminated
            // strings, and the type's data borrow outlives this object borrow.
            Some(unsafe { CStr::from_ptr(message) })
        }
    }

    /// Field: git_stash_save_options.flags
    /// Returns the raw combination of published `GIT_STASH_*` bits.
    #[must_use]
    pub fn flags(&self) -> u32 {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Field: git_stash_save_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_stash_save_options.paths
    /// Borrows the inline path-array header.
    #[must_use]
    pub fn paths(&self) -> GitStrArrayRef<'object> {
        // SAFETY: this projects the initialized inline header without forming
        // a reference to C-visible storage.
        let paths = unsafe { addr_of!((*self.as_ptr()).paths).cast_mut() };
        // SAFETY: the projected header lives for this options borrow.
        unsafe { GitStrArrayRef::from_ptr(paths) }.expect("an inline field is non-null")
    }

    /// Field: git_stash_save_options.stasher
    /// Borrows the optional identity used to create the stash commits.
    #[must_use]
    pub fn stasher(&self) -> Option<GitSignatureRef<'object>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let stasher = unsafe { addr_of!((*self.as_ptr()).stasher).read() };
        // SAFETY: safe construction stores only a live signature whose data
        // borrow outlives this options borrow; null remains `None`.
        unsafe { GitSignatureRef::from_ptr(stasher.cast_mut()) }
    }
}

impl<'object, 'data> GitStashSaveOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path to the
    /// options value may be used for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_stash_save_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitStashSaveOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_stash_save_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitStashSaveOptionsRef<'_, 'data> {
        GitStashSaveOptionsRef(self.0.0)
    }

    /// Stores an optional borrowed stash description.
    pub fn set_message(&mut self, message: Option<&'data CStr>) {
        let message = message.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer-field write, and
        // the wrapper lifetime keeps any non-null string alive.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).message).write(message) }
    }

    /// Replaces the raw combination of published `GIT_STASH_*` bits.
    pub fn set_flags(&mut self, flags: u32) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Borrows a path-array header and copies its non-owning C view.
    pub fn set_paths(&mut self, paths: GitStrArrayRef<'data>) {
        // SAFETY: `paths` identifies a live initialized header. Copying the C
        // header transfers no ownership; the wrapper lifetime retains the
        // source allocation while these options may be used.
        let paths = unsafe { paths.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline header.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).paths).write(paths) }
    }

    /// Stores an optional borrowed stasher identity.
    pub fn set_stasher(&mut self, stasher: Option<GitSignatureRef<'data>>) {
        let stasher = stasher.map_or(core::ptr::null(), |value| value.as_ptr());
        // SAFETY: this exclusive handle permits the pointer-field write, and
        // the wrapper lifetime keeps any non-null signature alive.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).stasher).write(stasher) }
    }
}

#[cfg(test)]
mod save_options_tests {
    use core::mem::{align_of, size_of};

    use crate::api::types::{GitSignature, GitSignatureRef};
    use crate::strarray::GitStrArrayRef;

    use super::*;

    #[test]
    fn save_options_preserve_layout_and_borrowed_fields() {
        assert_eq!(
            size_of::<GitStashSaveOptions<'static>>(),
            size_of::<ffi::git_stash_save_options>()
        );
        assert_eq!(
            align_of::<GitStashSaveOptions<'static>>(),
            align_of::<ffi::git_stash_save_options>()
        );
        assert_eq!(
            size_of::<GitStashSaveOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_stash_save_options>()
        );

        let mut signature = GitSignature::zeroed();
        let signature_ptr = addr_of_mut!(signature).cast::<ffi::git_signature>();
        // SAFETY: the layout-compatible stack value remains live while the
        // options borrow it, and this test never inspects its null fields.
        let signature = unsafe { GitSignatureRef::from_ptr(signature_ptr) }.unwrap();

        let mut entries = [c"tracked.txt".as_ptr().cast_mut()];
        let mut paths = ffi::git_strarray {
            strings: entries.as_mut_ptr(),
            count: entries.len(),
        };
        // SAFETY: the stack header, pointer slot and static string all outlive
        // the options value and are accessed shared-only.
        let paths = unsafe { GitStrArrayRef::from_ptr(addr_of_mut!(paths)) }.unwrap();

        let mut options = GitStashSaveOptions::new();
        {
            let mut view = options.as_mut();
            view.set_message(Some(c"save this"));
            view.set_flags(5);
            view.set_paths(paths);
            view.set_stasher(Some(signature));
        }

        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_STASH_SAVE_OPTIONS_VERSION);
        assert_eq!(view.message(), Some(c"save this"));
        assert_eq!(view.flags(), 5);
        assert_eq!(view.paths().count(), 1);
        assert_eq!(view.paths().strings().unwrap().get(0), Some(c"tracked.txt"));
        assert_eq!(view.stasher().unwrap().as_ptr(), signature_ptr);
    }

    #[test]
    fn save_options_keep_a_scoped_data_borrow_across_shorter_object_borrows() {
        // The `compile_fail` doctest on `GitStashSaveOptions` covers the
        // direction that must be rejected. This covers the direction that must
        // keep working: `'data` is a local scope rather than `'static`, and
        // the value is reborrowed for several shorter `'object` lifetimes.
        let message = std::ffi::CString::new("scoped").unwrap();
        let mut options = GitStashSaveOptions::new();
        options.as_mut().set_message(Some(message.as_c_str()));
        assert_eq!(options.as_ref().message(), Some(message.as_c_str()));
        options.as_mut().set_flags(3);
        assert_eq!(options.as_ref().message(), Some(message.as_c_str()));
    }
}

/// Wraps: git_stash_apply_options
/// Layout-compatible stash-apply options borrowing callback state and nested
/// checkout data for `'data`.
#[repr(transparent)]
pub struct GitStashApplyOptions<'data> {
    inner: CType<ffi::git_stash_apply_options>,
    _data: PhantomData<&'data mut ()>,
}

/// Shared borrow of [`GitStashApplyOptions`].
#[repr(transparent)]
pub struct GitStashApplyOptionsRef<'object, 'data>(CPtr<'object, GitStashApplyOptions<'data>>);

impl Clone for GitStashApplyOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitStashApplyOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitStashApplyOptions`].
#[repr(transparent)]
pub struct GitStashApplyOptionsMut<'object, 'data>(GitStashApplyOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and never form references to C-visible
// storage, and the shared handle exposes no writes.
unsafe impl<'data> CCell for GitStashApplyOptions<'data> {
    type C = ffi::git_stash_apply_options;
    type Ref<'object>
        = GitStashApplyOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitStashApplyOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitStashApplyOptionsRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitStashApplyOptionsMut(GitStashApplyOptionsRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: this options header only borrows its callback payload and every
// pointer nested in its checkout options; disposing inline storage is a no-op.
unsafe impl CValued for GitStashApplyOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitStashApplyOptions<'data> {
    /// Constructs options equivalent to `GIT_STASH_APPLY_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every field of the bindgen struct admits the all-zero bit
        // pattern; both required ABI versions are installed before return.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_STASH_APPLY_OPTIONS_VERSION);
            view.checkout_options_mut()
                .set_version(ffi::GIT_CHECKOUT_OPTIONS_VERSION);
        }
        options
    }
}

impl<'object, 'data> GitStashApplyOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify initialized options live for `'object`. The
    /// callback payload and all data borrowed by the nested checkout options
    /// must remain valid for `'data`, which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_stash_apply_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitStashApplyOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_stash_apply_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_stash_apply_options.flags
    /// Returns the raw combination of published stash-apply bits.
    #[must_use]
    pub fn flags(&self) -> u32 {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Field: git_stash_apply_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_stash_apply_options.progress_cb
    /// Returns whether a progress callback is installed.
    #[must_use]
    pub fn has_progress_callback(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized callback slot.
        unsafe { addr_of!((*self.as_ptr()).progress_cb).read() }.is_some()
    }

    /// Field: git_stash_apply_options.progress_payload
    /// Returns whether progress-callback state is installed.
    #[must_use]
    pub fn has_progress_payload(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized opaque pointer.
        !unsafe { addr_of!((*self.as_ptr()).progress_payload).read() }.is_null()
    }

    /// Field: git_stash_apply_options.checkout_options
    /// Borrows the inline checkout options.
    #[must_use]
    pub fn checkout_options(&self) -> crate::api::checkout::GitCheckoutOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field
        // without forming a reference to C-visible storage.
        let checkout = unsafe { addr_of!((*self.as_ptr()).checkout_options).cast_mut() };
        // SAFETY: the projected options live for this outer options borrow and
        // inherit its nested-data lifetime contract.
        unsafe { crate::api::checkout::GitCheckoutOptionsRef::from_ptr(checkout) }
            .expect("an inline field is non-null")
    }
}

impl<'object, 'data> GitStashApplyOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path to the
    /// options value may be used for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_stash_apply_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitStashApplyOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_stash_apply_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitStashApplyOptionsRef<'_, 'data> {
        GitStashApplyOptionsRef(self.0.0)
    }

    /// Replaces the raw combination of published stash-apply bits.
    pub fn set_flags(&mut self, flags: u32) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Exclusively borrows the inline checkout options.
    #[must_use]
    pub fn checkout_options_mut(
        &mut self,
    ) -> crate::api::checkout::GitCheckoutOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection originates from this exclusive handle.
        let checkout = unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_options) };
        // SAFETY: the projected field is initialized, exclusively borrowed,
        // and inherits the outer options' nested-data lifetime contract.
        unsafe { crate::api::checkout::GitCheckoutOptionsMut::from_ptr(checkout) }
            .expect("an inline field is non-null")
    }

    /// Copies a non-owning checkout-options header into the inline field.
    pub fn set_checkout_options(
        &mut self,
        checkout: crate::api::checkout::GitCheckoutOptionsRef<'_, 'data>,
    ) {
        // SAFETY: the source identifies initialized layout-compatible options.
        // Copying the C header transfers no ownership, and `'data` retains all
        // nested borrows for the outer options' lifetime.
        let checkout = unsafe { checkout.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_options).write(checkout) }
    }

    /// Installs a typed Rust stash-application progress callback and payload.
    pub fn set_progress_callback<C: GitStashApplyProgressCallback>(
        &mut self,
        callback: &'data mut C,
    ) {
        unsafe extern "C" fn trampoline<C: GitStashApplyProgressCallback>(
            progress: ffi::git_stash_apply_progress_t,
            payload: *mut core::ffi::c_void,
        ) -> core::ffi::c_int {
            let (Ok(progress), Some(callback)) = (
                StashApplyProgress::try_from(progress),
                // SAFETY: installation keeps a live exclusive `C` in payload.
                unsafe { payload.cast::<C>().as_mut() },
            ) else {
                return -1;
            };
            callback.call(progress)
        }

        let options = self.as_mut_ptr();
        // SAFETY: both callback halves are updated coherently; the wrapper's
        // data lifetime reserves `callback` for every synchronous invocation.
        unsafe {
            addr_of_mut!((*options).progress_cb).write(Some(trampoline::<C>));
            addr_of_mut!((*options).progress_payload).write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears the progress callback and its payload together.
    pub fn clear_progress_callback(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both coherent field writes.
        unsafe {
            addr_of_mut!((*options).progress_cb).write(None);
            addr_of_mut!((*options).progress_payload).write(core::ptr::null_mut());
        }
    }
}

#[cfg(test)]
mod apply_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn apply_options_preserve_layout_defaults_and_nested_access() {
        assert_eq!(
            size_of::<GitStashApplyOptions<'static>>(),
            size_of::<ffi::git_stash_apply_options>()
        );
        assert_eq!(
            align_of::<GitStashApplyOptions<'static>>(),
            align_of::<ffi::git_stash_apply_options>()
        );
        assert_eq!(
            size_of::<GitStashApplyOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_stash_apply_options>()
        );

        let mut options = GitStashApplyOptions::new();
        {
            let mut view = options.as_mut();
            view.set_flags(1);
            view.checkout_options_mut().set_disable_filters(true);
        }
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_STASH_APPLY_OPTIONS_VERSION);
        assert_eq!(view.flags(), 1);
        assert_eq!(
            view.checkout_options().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
        assert!(view.checkout_options().disable_filters());
        assert!(!view.has_progress_callback());
        assert!(!view.has_progress_payload());
    }

    #[test]
    fn apply_progress_callback_is_typed_and_cleared_as_a_pair() {
        let mut seen = Vec::new();
        {
            let mut callback = |progress| {
                seen.push(progress);
                0
            };
            let mut options = GitStashApplyOptions::new();
            options.as_mut().set_progress_callback(&mut callback);
            let raw = options.as_ref().as_ptr();
            // SAFETY: the configured callback and payload remain live and the
            // published progress value is valid for the trampoline call.
            let status = unsafe {
                addr_of!((*raw).progress_cb)
                    .read()
                    .expect("callback installed")(
                    StashApplyProgress::Done.into(),
                    addr_of!((*raw).progress_payload).read(),
                )
            };
            assert_eq!(status, 0);
            let mut view = options.as_mut();
            view.clear_progress_callback();
            assert!(!view.as_ref().has_progress_callback());
            assert!(!view.as_ref().has_progress_payload());
        }
        assert_eq!(seen, [StashApplyProgress::Done]);
    }
}
