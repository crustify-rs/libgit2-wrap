//! Safe wrappers for libgit2 submodule APIs.

use core::ffi::CStr;

use crate::submodule::GitSubmoduleMut;

/// Wraps: git_submodule_cb
/// Safe callable surface for one transient submodule visit.
pub trait GitSubmoduleCallback {
    /// Returns zero to continue iteration or a nonzero status to stop.
    fn call(&mut self, submodule: GitSubmoduleMut<'_>, name: &CStr) -> i32;
}

impl<F> GitSubmoduleCallback for F
where
    F: FnMut(GitSubmoduleMut<'_>, &CStr) -> i32,
{
    fn call(&mut self, submodule: GitSubmoduleMut<'_>, name: &CStr) -> i32 {
        self(submodule, name)
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::MaybeUninit;

    use super::*;

    #[test]
    fn callback_receives_an_exclusive_typed_handle() {
        let storage = Box::new(MaybeUninit::<crate::ffi::git_submodule>::zeroed());
        let raw = Box::into_raw(storage).cast::<crate::ffi::git_submodule>();
        // SAFETY: `raw` addresses live opaque storage and this scope has
        // exclusive access for the callback invocation.
        let submodule = unsafe { GitSubmoduleMut::from_ptr(raw) }.unwrap();
        let mut callback = |value: GitSubmoduleMut<'_>, name: &CStr| {
            i32::from(value.as_ref().as_ptr() == raw.cast_const() && name == c"child")
        };
        assert_eq!(
            GitSubmoduleCallback::call(&mut callback, submodule, c"child"),
            1
        );
        // SAFETY: the callback handle was consumed and dropped; this recovers
        // the exact allocation returned by `Box::into_raw`.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<crate::ffi::git_submodule>>()) });
    }
}

use core::marker::PhantomData;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::checkout::{GitCheckoutOptionsMut, GitCheckoutOptionsRef};
use crate::api::remote::{GitFetchOptionsMut, GitFetchOptionsRef};
use crate::ffi;

/// Wraps: git_submodule_update_options
/// Layout-compatible submodule-update options borrowing all nested strings,
/// objects and callback state for `'data`.
///
/// `'data` is invariant. The header itself owns no borrowed slot, but the
/// embedded checkout and fetch options reached through
/// [`GitSubmoduleUpdateOptionsMut::checkout_options_mut`] and
/// [`GitSubmoduleUpdateOptionsMut::fetch_options_mut`] inherit `'data`, store
/// `&'data` strings and callback receivers, and hand them back out through the
/// matching shared getters. Those nested wrappers pin their own parameter, so
/// a covariant `'data` here would reach them again: safe code could shrink the
/// parameter on this exclusive handle, install a shorter-lived proxy URL,
/// header array or callback receiver through `fetch_options_mut()`, and then
/// read the released storage back through a handle still typed at the longer
/// lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::submodule::GitSubmoduleUpdateOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitSubmoduleUpdateOptionsMut<'object, 'static>,
/// ) -> GitSubmoduleUpdateOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitSubmoduleUpdateOptions<'data> {
    inner: CType<ffi::git_submodule_update_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitSubmoduleUpdateOptions`].
#[repr(transparent)]
pub struct GitSubmoduleUpdateOptionsRef<'object, 'data>(
    CPtr<'object, GitSubmoduleUpdateOptions<'data>>,
);

impl Clone for GitSubmoduleUpdateOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitSubmoduleUpdateOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitSubmoduleUpdateOptions`].
#[repr(transparent)]
pub struct GitSubmoduleUpdateOptionsMut<'object, 'data>(
    GitSubmoduleUpdateOptionsRef<'object, 'data>,
);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitSubmoduleUpdateOptions<'data> {
    type C = ffi::git_submodule_update_options;
    type Ref<'object>
        = GitSubmoduleUpdateOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitSubmoduleUpdateOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitSubmoduleUpdateOptionsRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitSubmoduleUpdateOptionsMut(GitSubmoduleUpdateOptionsRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: submodule-update options own no resource. Every pointer in their
// inline option records is borrowed for the wrapper's data lifetime.
unsafe impl CValued for GitSubmoduleUpdateOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitSubmoduleUpdateOptions<'data> {
    /// Constructs options equivalent to `GIT_SUBMODULE_UPDATE_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every raw field admits zero. Required versions and nonzero
        // defaults are installed before the initialized value escapes.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_SUBMODULE_UPDATE_OPTIONS_VERSION);
            view.set_allow_fetch(true);
            view.checkout_options_mut()
                .set_version(ffi::GIT_CHECKOUT_OPTIONS_VERSION);
            let mut fetch = view.fetch_options_mut();
            fetch.set_version(ffi::GIT_FETCH_OPTIONS_VERSION as core::ffi::c_int);
            fetch.set_update_flags(crate::api::remote::GitRemoteUpdateFlags::FETCH_HEAD);
            fetch
                .callbacks_mut()
                .set_version(ffi::GIT_REMOTE_CALLBACKS_VERSION);
            fetch
                .proxy_options_mut()
                .set_version(ffi::GIT_PROXY_OPTIONS_VERSION);
        }
        options
    }
}

impl<'object, 'data> GitSubmoduleUpdateOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    /// `ptr` must identify initialized options live for `'object`. Every
    /// nested borrow and callback payload must remain valid for `'data`, which
    /// must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_submodule_update_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitSubmoduleUpdateOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_submodule_update_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_submodule_update_options.version
    /// Returns the submodule-update ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_submodule_update_options.checkout_opts
    /// Borrows the embedded checkout options.
    #[must_use]
    pub fn checkout_options(&self) -> GitCheckoutOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { addr_of!((*self.as_ptr()).checkout_opts).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { GitCheckoutOptionsRef::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Field: git_submodule_update_options.fetch_opts
    /// Borrows the embedded fetch options.
    #[must_use]
    pub fn fetch_options(&self) -> GitFetchOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { addr_of!((*self.as_ptr()).fetch_opts).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { GitFetchOptionsRef::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Field: git_submodule_update_options.allow_fetch
    /// Returns whether a missing target commit may be fetched.
    #[must_use]
    pub fn allow_fetch(&self) -> bool {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).allow_fetch).read() != 0 }
    }
}

impl<'object, 'data> GitSubmoduleUpdateOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    /// The shared-handle requirements apply, and no other access path may use
    /// the options for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_submodule_update_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitSubmoduleUpdateOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_submodule_update_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitSubmoduleUpdateOptionsRef<'_, 'data> {
        GitSubmoduleUpdateOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects whether a missing target commit may be fetched.
    pub fn set_allow_fetch(&mut self, allow: bool) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe {
            addr_of_mut!((*self.as_mut_ptr()).allow_fetch).write(core::ffi::c_int::from(allow))
        }
    }

    /// Exclusively borrows the embedded checkout options.
    #[must_use]
    pub fn checkout_options_mut(&mut self) -> GitCheckoutOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_opts) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime and retains the outer data lifetime.
        unsafe { GitCheckoutOptionsMut::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Exclusively borrows the embedded fetch options.
    #[must_use]
    pub fn fetch_options_mut(&mut self) -> GitFetchOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { addr_of_mut!((*self.as_mut_ptr()).fetch_opts) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime and retains the outer data lifetime.
        unsafe { GitFetchOptionsMut::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Copies checkout options into the inline field.
    ///
    /// # Safety
    /// Callback invocations through the source, this copy, and any further C
    /// copies must not overlap. All nested callback state remains exclusively
    /// reserved for `'data`.
    pub unsafe fn set_checkout_options(&mut self, options: GitCheckoutOptionsRef<'_, 'data>) {
        // SAFETY: the source identifies an initialized borrowed header.
        let options = unsafe { options.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_opts).write(options) }
    }

    /// Copies fetch options into the inline field.
    ///
    /// # Safety
    /// Callback invocations through the source, this copy, and any further C
    /// copies must not overlap. All nested callback state remains exclusively
    /// reserved for `'data`.
    pub unsafe fn set_fetch_options(&mut self, options: GitFetchOptionsRef<'_, 'data>) {
        // SAFETY: the source identifies an initialized borrowed header.
        let options = unsafe { options.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).fetch_opts).write(options) }
    }
}

#[cfg(test)]
mod update_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn update_options_preserve_layout_and_defaults() {
        assert_eq!(
            size_of::<GitSubmoduleUpdateOptions<'static>>(),
            size_of::<ffi::git_submodule_update_options>()
        );
        assert_eq!(
            align_of::<GitSubmoduleUpdateOptions<'static>>(),
            align_of::<ffi::git_submodule_update_options>()
        );
        let options = GitSubmoduleUpdateOptions::new();
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_SUBMODULE_UPDATE_OPTIONS_VERSION);
        assert!(view.allow_fetch());
        assert_eq!(
            view.checkout_options().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
        assert_eq!(
            view.fetch_options().version(),
            ffi::GIT_FETCH_OPTIONS_VERSION as core::ffi::c_int
        );
    }

    #[test]
    fn nested_and_scalar_fields_are_mutable() {
        let mut options = GitSubmoduleUpdateOptions::new();
        {
            let mut view = options.as_mut();
            view.set_allow_fetch(false);
            view.checkout_options_mut().set_disable_filters(true);
            view.fetch_options_mut()
                .set_depth(crate::api::remote::GitFetchDepth::new(2).unwrap());
        }
        let view = options.as_ref();
        assert!(!view.allow_fetch());
        assert!(view.checkout_options().disable_filters());
        assert_eq!(
            view.fetch_options().depth(),
            Ok(crate::api::remote::GitFetchDepth::new(2).unwrap())
        );
    }

    /// The pinned `'data` still accepts a referent that merely outlives the
    /// options, so invariance does not force `'static` on callers.
    #[test]
    fn pinned_options_accept_a_scoped_referent() {
        let url = std::ffi::CString::new("http://scoped.invalid/").unwrap();
        let mut options = GitSubmoduleUpdateOptions::new();
        options
            .as_mut()
            .fetch_options_mut()
            .proxy_options_mut()
            .set_url(Some(&url));
        assert_eq!(
            options.as_ref().fetch_options().proxy_options().url(),
            Some(url.as_c_str())
        );
    }
}

/// Wraps: git_submodule_status_t
/// A checked set of submodule location and modification status bits.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitSubmoduleStatusFlags(ffi::git_submodule_status_t);

impl GitSubmoduleStatusFlags {
    /// No location or modification status is present.
    pub const NONE: Self = Self(0);
    /// The superproject's `HEAD` contains the submodule.
    pub const IN_HEAD: Self = Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_IN_HEAD);
    /// The superproject index contains the submodule.
    pub const IN_INDEX: Self = Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_IN_INDEX);
    /// The superproject configuration contains the submodule.
    pub const IN_CONFIG: Self = Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_IN_CONFIG);
    /// The superproject working directory contains the submodule.
    pub const IN_WORKDIR: Self = Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_IN_WD);
    /// The index contains a submodule absent from `HEAD`.
    pub const INDEX_ADDED: Self =
        Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_INDEX_ADDED);
    /// `HEAD` contains a submodule absent from the index.
    pub const INDEX_DELETED: Self =
        Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_INDEX_DELETED);
    /// The index and `HEAD` record different submodule commits.
    pub const INDEX_MODIFIED: Self =
        Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_INDEX_MODIFIED);
    /// The working-directory submodule is not initialized.
    pub const WORKDIR_UNINITIALIZED: Self =
        Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_WD_UNINITIALIZED);
    /// The working directory contains a submodule absent from the index.
    pub const WORKDIR_ADDED: Self = Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_WD_ADDED);
    /// The index contains a submodule absent from the working directory.
    pub const WORKDIR_DELETED: Self =
        Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_WD_DELETED);
    /// The working directory and index record different submodule commits.
    pub const WORKDIR_MODIFIED: Self =
        Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_WD_MODIFIED);
    /// The submodule's own index contains changes.
    pub const WORKDIR_INDEX_MODIFIED: Self =
        Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_WD_INDEX_MODIFIED);
    /// The submodule's own working directory contains modified files.
    pub const WORKDIR_FILES_MODIFIED: Self =
        Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_WD_WD_MODIFIED);
    /// The submodule's own working directory contains untracked files.
    pub const WORKDIR_UNTRACKED: Self =
        Self(ffi::git_submodule_status_t_GIT_SUBMODULE_STATUS_WD_UNTRACKED);
    /// Every status bit published by this version of libgit2.
    pub const ALL: Self = Self(
        Self::IN_HEAD.0
            | Self::IN_INDEX.0
            | Self::IN_CONFIG.0
            | Self::IN_WORKDIR.0
            | Self::INDEX_ADDED.0
            | Self::INDEX_DELETED.0
            | Self::INDEX_MODIFIED.0
            | Self::WORKDIR_UNINITIALIZED.0
            | Self::WORKDIR_ADDED.0
            | Self::WORKDIR_DELETED.0
            | Self::WORKDIR_MODIFIED.0
            | Self::WORKDIR_INDEX_MODIFIED.0
            | Self::WORKDIR_FILES_MODIFIED.0
            | Self::WORKDIR_UNTRACKED.0,
    );

    /// Converts raw bits when every bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_submodule_status_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_submodule_status_t {
        self.0
    }

    /// Returns whether no status bits are present.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every status in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any status in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl TryFrom<ffi::git_submodule_status_t> for GitSubmoduleStatusFlags {
    type Error = ffi::git_submodule_status_t;

    fn try_from(bits: ffi::git_submodule_status_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl From<GitSubmoduleStatusFlags> for ffi::git_submodule_status_t {
    fn from(status: GitSubmoduleStatusFlags) -> Self {
        status.bits()
    }
}

impl BitOr for GitSubmoduleStatusFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitSubmoduleStatusFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitSubmoduleStatusFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitSubmoduleStatusFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitSubmoduleStatusFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod status_flag_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn status_bits_validate_and_form_sets() {
        let status =
            GitSubmoduleStatusFlags::IN_INDEX | GitSubmoduleStatusFlags::WORKDIR_INDEX_MODIFIED;
        assert!(status.contains(GitSubmoduleStatusFlags::IN_INDEX));
        assert!(status.intersects(GitSubmoduleStatusFlags::WORKDIR_INDEX_MODIFIED));
        assert_eq!(GitSubmoduleStatusFlags::try_from(status.bits()), Ok(status));
        assert_eq!(
            GitSubmoduleStatusFlags::from_bits(GitSubmoduleStatusFlags::ALL.bits() << 1),
            None
        );
        assert_eq!(status & !status, GitSubmoduleStatusFlags::NONE);
    }

    #[test]
    fn status_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitSubmoduleStatusFlags>(),
            size_of::<ffi::git_submodule_status_t>()
        );
        assert_eq!(
            align_of::<GitSubmoduleStatusFlags>(),
            align_of::<ffi::git_submodule_status_t>()
        );
    }
}
