//! Safe wrappers for libgit2 worktree APIs.

use core::marker::PhantomData;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::ffi;
use crate::refs::GitReferenceRef;

/// Wraps: git_worktree_prune_t
/// A checked set of overrides for pruning a linked worktree.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitWorktreePruneFlags(ffi::git_worktree_prune_t);

impl GitWorktreePruneFlags {
    /// Apply the normal worktree pruning checks.
    pub const NONE: Self = Self(0);
    /// Prune even when the worktree is otherwise valid.
    pub const VALID: Self = Self(ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_VALID);
    /// Prune even when the worktree is locked.
    pub const LOCKED: Self = Self(ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_LOCKED);
    /// Also delete the worktree's checked-out working directory.
    ///
    /// Unlike the other two overrides this relaxes no prunability check:
    /// `git_worktree_prune` always removes the administrative data under the
    /// parent repository's `worktrees/<name>`, and only removes the working
    /// directory named by the worktree's gitlink when this bit is set.
    pub const WORKING_TREE: Self = Self(ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_WORKING_TREE);
    /// Every prune override published by this version of libgit2.
    pub const ALL: Self = Self(Self::VALID.0 | Self::LOCKED.0 | Self::WORKING_TREE.0);

    /// Converts raw bits when every bit is a published prune override.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_worktree_prune_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_worktree_prune_t {
        self.0
    }

    /// Returns whether no override is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every override in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any override in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitWorktreePruneFlags> for ffi::git_worktree_prune_t {
    fn from(flags: GitWorktreePruneFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_worktree_prune_t> for GitWorktreePruneFlags {
    type Error = ffi::git_worktree_prune_t;

    fn try_from(bits: ffi::git_worktree_prune_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitWorktreePruneFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitWorktreePruneFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitWorktreePruneFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitWorktreePruneFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitWorktreePruneFlags {
    type Output = Self;

    /// Returns the published overrides this set omits, leaving every bit
    /// libgit2 does not define clear.
    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn prune_overrides_combine_and_validate() {
        let overrides = GitWorktreePruneFlags::VALID | GitWorktreePruneFlags::LOCKED;
        assert!(overrides.contains(GitWorktreePruneFlags::VALID));
        assert!(overrides.intersects(GitWorktreePruneFlags::LOCKED));
        assert!(!overrides.intersects(GitWorktreePruneFlags::WORKING_TREE));
        assert_eq!(
            GitWorktreePruneFlags::from_bits(overrides.bits()),
            Some(overrides)
        );
        assert!(GitWorktreePruneFlags::NONE.is_empty());

        let unknown = GitWorktreePruneFlags::ALL.bits() << 1;
        assert_eq!(GitWorktreePruneFlags::from_bits(unknown), None);
        assert_eq!(GitWorktreePruneFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn prune_override_complement_stays_within_published_bits() {
        assert_eq!(
            !GitWorktreePruneFlags::VALID,
            GitWorktreePruneFlags::LOCKED | GitWorktreePruneFlags::WORKING_TREE
        );
    }

    #[test]
    fn prune_overrides_accumulate_and_convert_to_the_c_enum() {
        let mut overrides = GitWorktreePruneFlags::NONE;
        overrides |= GitWorktreePruneFlags::VALID;
        overrides |= GitWorktreePruneFlags::WORKING_TREE;
        assert_eq!(
            ffi::git_worktree_prune_t::from(overrides),
            ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_VALID
                | ffi::git_worktree_prune_t_GIT_WORKTREE_PRUNE_WORKING_TREE
        );

        overrides &= !GitWorktreePruneFlags::WORKING_TREE;
        assert_eq!(overrides, GitWorktreePruneFlags::VALID);
        assert_eq!(
            GitWorktreePruneFlags::try_from(overrides.bits()),
            Ok(GitWorktreePruneFlags::VALID)
        );
    }

    #[test]
    fn prune_overrides_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitWorktreePruneFlags>(),
            size_of::<ffi::git_worktree_prune_t>()
        );
        assert_eq!(
            align_of::<GitWorktreePruneFlags>(),
            align_of::<ffi::git_worktree_prune_t>()
        );
    }
}

/// Wraps: git_worktree_add_options
/// Layout-compatible worktree-add options borrowing their optional reference
/// and all data nested in their checkout options for `'data`.
///
/// `'data` is invariant. [`GitWorktreeAddOptionsMut::set_reference`] stores a
/// `&'data` reference in the C struct while
/// [`GitWorktreeAddOptionsRef::reference`] hands it back out, and the nested
/// checkout options behave the same way. A covariant `'data` would let safe
/// code shrink the parameter on the exclusive handle, install a shorter-lived
/// reference, and then read it back through a handle still typed at the
/// longer lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::worktree::GitWorktreeAddOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitWorktreeAddOptionsMut<'object, 'static>,
/// ) -> GitWorktreeAddOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitWorktreeAddOptions<'data> {
    inner: CType<ffi::git_worktree_add_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitWorktreeAddOptions`].
#[repr(transparent)]
pub struct GitWorktreeAddOptionsRef<'object, 'data>(CPtr<'object, GitWorktreeAddOptions<'data>>);

impl Clone for GitWorktreeAddOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitWorktreeAddOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitWorktreeAddOptions`].
#[repr(transparent)]
pub struct GitWorktreeAddOptionsMut<'object, 'data>(GitWorktreeAddOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and never form references to C-visible
// storage, and the shared handle exposes no writes.
unsafe impl<'data> CCell for GitWorktreeAddOptions<'data> {
    type C = ffi::git_worktree_add_options;
    type Ref<'object>
        = GitWorktreeAddOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitWorktreeAddOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitWorktreeAddOptionsRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitWorktreeAddOptionsMut(GitWorktreeAddOptionsRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: this options header only borrows its reference and every pointer
// nested in its checkout options; disposing inline storage is a no-op.
unsafe impl CValued for GitWorktreeAddOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitWorktreeAddOptions<'data> {
    /// Constructs options equivalent to `GIT_WORKTREE_ADD_OPTIONS_INIT`.
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
            view.set_version(ffi::GIT_WORKTREE_ADD_OPTIONS_VERSION);
            view.checkout_options_mut()
                .set_version(ffi::GIT_CHECKOUT_OPTIONS_VERSION);
        }
        options
    }
}

impl<'object, 'data> GitWorktreeAddOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify initialized options live for `'object`. The
    /// optional reference and every value borrowed by the nested checkout
    /// options must remain live for `'data`, which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_worktree_add_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitWorktreeAddOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_worktree_add_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_worktree_add_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_worktree_add_options.ref
    /// Borrows the optional reference selected for the new worktree HEAD.
    #[must_use]
    pub fn reference(&self) -> Option<GitReferenceRef<'object>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let reference = unsafe { addr_of!((*self.as_ptr()).ref_).read() };
        // SAFETY: the outer options contract keeps a non-null reference live
        // for this shared object borrow.
        unsafe { GitReferenceRef::from_ptr(reference) }
    }

    /// Field: git_worktree_add_options.lock
    /// Returns whether the newly created worktree should be locked.
    #[must_use]
    pub fn lock(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).lock).read() != 0 }
    }

    /// Field: git_worktree_add_options.checkout_options
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

    /// Field: git_worktree_add_options.checkout_existing
    /// Returns whether an existing branch matching the worktree name may be
    /// checked out.
    #[must_use]
    pub fn checkout_existing(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).checkout_existing).read() != 0 }
    }
}

impl<'object, 'data> GitWorktreeAddOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path to the
    /// options value may be used for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_worktree_add_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitWorktreeAddOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_worktree_add_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitWorktreeAddOptionsRef<'_, 'data> {
        GitWorktreeAddOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Stores an optional borrowed reference for the new worktree HEAD.
    pub fn set_reference(&mut self, reference: Option<GitReferenceRef<'data>>) {
        let reference = reference.map_or(core::ptr::null(), |reference| reference.as_ptr());
        // SAFETY: this exclusive handle permits the pointer write, and the
        // wrapper lifetime keeps a non-null reference live and shared-only.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).ref_).write(reference.cast_mut()) }
    }

    /// Selects whether the newly created worktree should be locked.
    pub fn set_lock(&mut self, lock: bool) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).lock).write(core::ffi::c_int::from(lock)) }
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

    /// Selects whether an existing branch matching the worktree name may be
    /// checked out.
    pub fn set_checkout_existing(&mut self, checkout_existing: bool) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe {
            addr_of_mut!((*self.as_mut_ptr()).checkout_existing)
                .write(core::ffi::c_int::from(checkout_existing))
        }
    }
}

#[cfg(test)]
mod add_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn add_options_preserve_layout_defaults_and_mutable_access() {
        assert_eq!(
            size_of::<GitWorktreeAddOptions<'static>>(),
            size_of::<ffi::git_worktree_add_options>()
        );
        assert_eq!(
            align_of::<GitWorktreeAddOptions<'static>>(),
            align_of::<ffi::git_worktree_add_options>()
        );
        assert_eq!(
            size_of::<GitWorktreeAddOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_worktree_add_options>()
        );

        let mut options = GitWorktreeAddOptions::new();
        {
            let mut view = options.as_mut();
            view.set_lock(true);
            view.set_checkout_existing(true);
            view.checkout_options_mut().set_disable_filters(true);
        }
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_WORKTREE_ADD_OPTIONS_VERSION);
        assert!(view.reference().is_none());
        assert!(view.lock());
        assert!(view.checkout_existing());
        assert_eq!(
            view.checkout_options().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
        assert!(view.checkout_options().disable_filters());
    }

    #[test]
    fn checkout_options_header_can_be_copied_without_transferring_ownership() {
        let mut checkout = crate::api::checkout::GitCheckoutOptions::new();
        checkout.as_mut().set_file_mode(0o100644);

        let mut options = GitWorktreeAddOptions::new();
        options.as_mut().set_checkout_options(checkout.as_ref());
        assert_eq!(options.as_ref().checkout_options().file_mode(), 0o100644);
    }
}
