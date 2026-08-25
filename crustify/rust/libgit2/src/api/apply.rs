//! Safe wrappers for libgit2 apply APIs.

use core::ops::{BitOr, BitOrAssign};

use crate::api::diff::DiffDeltaRef;
use crate::ffi;

/// Wraps: git_apply_flags_t
/// A checked set of flags controlling how libgit2 applies a patch.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitApplyFlags(ffi::git_apply_flags_t);

impl GitApplyFlags {
    /// Apply normally and write the resulting changes.
    pub const NONE: Self = Self(0);
    /// Check whether the patch applies without making changes.
    pub const CHECK: Self = Self(ffi::git_apply_flags_t_GIT_APPLY_CHECK);

    /// Converts raw bits when every flag is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_apply_flags_t) -> Option<Self> {
        if bits & !Self::CHECK.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_apply_flags_t {
        self.0
    }

    /// Returns whether no apply flags are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for GitApplyFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitApplyFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl From<GitApplyFlags> for ffi::git_apply_flags_t {
    fn from(value: GitApplyFlags) -> Self {
        value.bits()
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_apply_flags_form_valid_sets() {
        assert_eq!(GitApplyFlags::from_bits(0), Some(GitApplyFlags::NONE));
        assert_eq!(
            GitApplyFlags::from_bits(ffi::git_apply_flags_t_GIT_APPLY_CHECK),
            Some(GitApplyFlags::CHECK)
        );

        let mut flags = GitApplyFlags::NONE;
        assert!(flags.is_empty());
        flags |= GitApplyFlags::CHECK;
        assert!(flags.contains(GitApplyFlags::CHECK));
        assert_eq!(ffi::git_apply_flags_t::from(flags), flags.bits());
    }

    #[test]
    fn unknown_apply_flag_bits_are_rejected() {
        let unknown = ffi::git_apply_flags_t_GIT_APPLY_CHECK << 1;
        assert_eq!(GitApplyFlags::from_bits(unknown), None);
    }

    #[test]
    fn apply_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitApplyFlags>(),
            size_of::<ffi::git_apply_flags_t>()
        );
        assert_eq!(
            align_of::<GitApplyFlags>(),
            align_of::<ffi::git_apply_flags_t>()
        );
    }
}

/// Wraps: git_apply_delta_cb
/// Safe callable surface for deciding whether to apply one transient delta.
pub trait GitApplyDeltaCallback {
    /// Returns zero to apply, a positive value to skip, or a negative value
    /// to abort the operation.
    fn call(&mut self, delta: DiffDeltaRef<'_>) -> i32;
}

impl<F> GitApplyDeltaCallback for F
where
    F: for<'a> FnMut(DiffDeltaRef<'a>) -> i32,
{
    fn call(&mut self, delta: DiffDeltaRef<'_>) -> i32 {
        self(delta)
    }
}

#[cfg(test)]
mod callback_tests {
    use super::*;

    #[test]
    fn delta_callback_accepts_a_typed_transient_borrow() {
        let mut raw = crate::api::diff::DiffDelta::zeroed();
        // SAFETY: this initialized layout value remains live and immutable for
        // the callback invocation, and the handle is the only access path.
        let delta = unsafe {
            DiffDeltaRef::from_ptr(
                core::ptr::addr_of_mut!(raw).cast::<crate::ffi::git_diff_delta>(),
            )
        }
        .unwrap();
        let mut callback = |_: DiffDeltaRef<'_>| 7;
        assert_eq!(GitApplyDeltaCallback::call(&mut callback, delta), 7);
    }
}

/// Callback set sharing the single payload stored by [`GitApplyOptions`].
pub trait GitApplyOptionsCallbacks: GitApplyDeltaCallback {
    /// Returns zero to apply the hunk, a positive value to skip it, or a
    /// negative value to abort the operation.
    fn hunk(&mut self, hunk: crate::diff::DiffHunkRef<'_>) -> i32;
}

/// Wraps: git_apply_options
/// Layout-compatible apply options borrowing callback state for `'callback`.
#[repr(transparent)]
pub struct GitApplyOptions<'callback> {
    inner: ffibox::CType<ffi::git_apply_options>,
    _callback: core::marker::PhantomData<&'callback mut ()>,
}

/// Shared borrow of [`GitApplyOptions`].
#[repr(transparent)]
pub struct GitApplyOptionsRef<'object, 'callback>(
    ffibox::CPtr<'object, GitApplyOptions<'callback>>,
);

impl Clone for GitApplyOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitApplyOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitApplyOptions`].
#[repr(transparent)]
pub struct GitApplyOptionsMut<'object, 'callback>(GitApplyOptionsRef<'object, 'callback>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'callback> ffibox::CCell for GitApplyOptions<'callback> {
    type C = ffi::git_apply_options;
    type Ref<'object>
        = GitApplyOptionsRef<'object, 'callback>
    where
        Self: 'object;
    type Mut<'object>
        = GitApplyOptionsMut<'object, 'callback>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: core::ptr::NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitApplyOptionsRef(unsafe { ffibox::CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: core::ptr::NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitApplyOptionsMut(GitApplyOptionsRef(unsafe { ffibox::CPtr::new(ptr) }))
    }
}

// SAFETY: apply options borrow callback state and own no resource, so
// disposing their inline storage requires no action.
unsafe impl ffibox::CValued for GitApplyOptions<'_> {
    unsafe fn c_dispose(_this: core::ptr::NonNull<Self>) {}
}

impl<'callback> GitApplyOptions<'callback> {
    /// Constructs options equivalent to `GIT_APPLY_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> ffibox::CVal<Self> {
        // SAFETY: all fields of the C options struct admit the all-zero bit
        // pattern; the required ABI version is installed before return.
        let inner = unsafe { ffibox::CType::zeroed() };
        let mut options = ffibox::CVal::new(Self {
            inner,
            _callback: core::marker::PhantomData,
        });
        options.as_mut().set_version(ffi::GIT_APPLY_OPTIONS_VERSION);
        options
    }
}

impl<'object, 'callback> GitApplyOptionsRef<'object, 'callback> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify initialized options live for `'object`. Any
    /// callback payload must remain live and exclusively reserved for
    /// `'callback`, which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_apply_options) -> Option<Self> {
        core::ptr::NonNull::new(ptr.cast::<GitApplyOptions<'callback>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { ffibox::CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_apply_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_apply_options.flags
    /// Returns the checked set of configured apply flags.
    pub fn flags(&self) -> Result<GitApplyFlags, ffi::git_apply_flags_t> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let bits = unsafe { core::ptr::addr_of!((*self.as_ptr()).flags).read() };
        GitApplyFlags::from_bits(bits).ok_or(bits)
    }

    /// Field: git_apply_options.payload
    /// Reports whether callback state is installed.
    #[must_use]
    pub fn has_callback_payload(&self) -> bool {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        !unsafe { core::ptr::addr_of!((*self.as_ptr()).payload).read() }.is_null()
    }

    /// Field: git_apply_options.hunk_cb
    /// Reports whether a hunk callback is installed.
    #[must_use]
    pub fn has_hunk_callback(&self) -> bool {
        // SAFETY: this live shared handle permits the callback-slot read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).hunk_cb).read() }.is_some()
    }

    /// Field: git_apply_options.delta_cb
    /// Reports whether a delta callback is installed.
    #[must_use]
    pub fn has_delta_callback(&self) -> bool {
        // SAFETY: this live shared handle permits the callback-slot read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).delta_cb).read() }.is_some()
    }

    /// Field: git_apply_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).version).read() }
    }
}

impl<'object, 'callback> GitApplyOptionsMut<'object, 'callback> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path may use
    /// the options value for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_apply_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitApplyOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_apply_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitApplyOptionsRef<'_, 'callback> {
        GitApplyOptionsRef(self.0.0)
    }

    /// Replaces the apply behavior flags.
    pub fn set_flags(&mut self, flags: GitApplyFlags) {
        // SAFETY: this exclusive handle permits the scalar write, and the
        // wrapper contains only published apply-flag bits.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Installs typed delta and hunk callbacks sharing one payload.
    ///
    /// # Safety
    ///
    /// Callback methods must not unwind across the C boundary. The options
    /// must not be used by an operation that invokes the callbacks
    /// concurrently or after the `'callback` borrow ends.
    pub unsafe fn set_callbacks<C: GitApplyOptionsCallbacks>(
        &mut self,
        callbacks: &'callback mut C,
    ) {
        unsafe extern "C" fn delta<C: GitApplyOptionsCallbacks>(
            delta: *const ffi::git_diff_delta,
            payload: *mut core::ffi::c_void,
        ) -> core::ffi::c_int {
            // SAFETY: installation stores a live exclusive `C` in `payload`.
            let Some(callbacks) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return -1;
            };
            // SAFETY: libgit2 supplies a live initialized transient delta.
            let Some(delta) = (unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }) else {
                return -1;
            };
            GitApplyDeltaCallback::call(callbacks, delta)
        }

        unsafe extern "C" fn hunk<C: GitApplyOptionsCallbacks>(
            hunk: *const ffi::git_diff_hunk,
            payload: *mut core::ffi::c_void,
        ) -> core::ffi::c_int {
            // SAFETY: installation stores a live exclusive `C` in `payload`.
            let Some(callbacks) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return -1;
            };
            // SAFETY: libgit2 supplies a live initialized transient hunk.
            let Some(hunk) = (unsafe { crate::diff::DiffHunkRef::from_ptr(hunk.cast_mut()) })
            else {
                return -1;
            };
            callbacks.hunk(hunk)
        }

        let options = self.as_mut_ptr();
        // SAFETY: the caller guarantees payload validity and this exclusive
        // handle permits installing all three coupled callback fields.
        unsafe {
            core::ptr::addr_of_mut!((*options).delta_cb).write(Some(delta::<C>));
            core::ptr::addr_of_mut!((*options).hunk_cb).write(Some(hunk::<C>));
            core::ptr::addr_of_mut!((*options).payload)
                .write(core::ptr::from_mut(callbacks).cast());
        }
    }

    /// Clears both callbacks and their shared payload together.
    pub fn clear_callbacks(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits all three field writes.
        unsafe {
            core::ptr::addr_of_mut!((*options).delta_cb).write(None);
            core::ptr::addr_of_mut!((*options).hunk_cb).write(None);
            core::ptr::addr_of_mut!((*options).payload).write(core::ptr::null_mut());
        }
    }
}

#[cfg(test)]
mod apply_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    struct Callbacks {
        delta_calls: usize,
        hunk_calls: usize,
    }

    impl GitApplyDeltaCallback for Callbacks {
        fn call(&mut self, _delta: DiffDeltaRef<'_>) -> i32 {
            self.delta_calls += 1;
            0
        }
    }

    impl GitApplyOptionsCallbacks for Callbacks {
        fn hunk(&mut self, _hunk: crate::diff::DiffHunkRef<'_>) -> i32 {
            self.hunk_calls += 1;
            0
        }
    }

    #[test]
    fn options_preserve_layout_defaults_and_scalar_access() {
        assert_eq!(
            size_of::<GitApplyOptions<'static>>(),
            size_of::<ffi::git_apply_options>()
        );
        assert_eq!(
            align_of::<GitApplyOptions<'static>>(),
            align_of::<ffi::git_apply_options>()
        );
        assert_eq!(
            size_of::<GitApplyOptionsRef<'static, 'static>>(),
            size_of::<*mut ffi::git_apply_options>()
        );

        let mut options = GitApplyOptions::new();
        assert_eq!(options.as_ref().version(), ffi::GIT_APPLY_OPTIONS_VERSION);
        assert_eq!(options.as_ref().flags(), Ok(GitApplyFlags::NONE));
        assert!(!options.as_ref().has_delta_callback());
        assert!(!options.as_ref().has_hunk_callback());
        assert!(!options.as_ref().has_callback_payload());

        options.as_mut().set_flags(GitApplyFlags::CHECK);
        assert_eq!(options.as_ref().flags(), Ok(GitApplyFlags::CHECK));
    }

    #[test]
    fn callbacks_are_installed_and_cleared_as_one_payload() {
        let mut callbacks = Callbacks {
            delta_calls: 0,
            hunk_calls: 0,
        };
        let mut options = GitApplyOptions::new();
        // SAFETY: the callback does not unwind, remains live for the options
        // use, and no callback invocation overlaps in this test.
        unsafe { options.as_mut().set_callbacks(&mut callbacks) };
        assert!(options.as_ref().has_delta_callback());
        assert!(options.as_ref().has_hunk_callback());
        assert!(options.as_ref().has_callback_payload());

        let view = options.as_ref();
        let raw = view.as_ptr();
        // SAFETY: the three installed slots were written together above, and
        // these initialized transient records remain live for each call.
        unsafe {
            let payload = core::ptr::addr_of!((*raw).payload).read();
            let delta = core::ptr::addr_of!((*raw).delta_cb)
                .read()
                .expect("the delta callback is installed");
            let hunk = core::ptr::addr_of!((*raw).hunk_cb)
                .read()
                .expect("the hunk callback is installed");
            let raw_delta = crate::api::diff::DiffDelta::zeroed();
            let raw_hunk = crate::diff::DiffHunk::zeroed();
            assert_eq!(delta(core::ptr::addr_of!(raw_delta).cast(), payload), 0);
            assert_eq!(hunk(core::ptr::addr_of!(raw_hunk).cast(), payload), 0);
        }

        options.as_mut().clear_callbacks();
        assert!(!options.as_ref().has_delta_callback());
        assert!(!options.as_ref().has_hunk_callback());
        assert!(!options.as_ref().has_callback_payload());
        drop(options);
        assert_eq!(callbacks.delta_calls, 1);
        assert_eq!(callbacks.hunk_calls, 1);
    }
}
