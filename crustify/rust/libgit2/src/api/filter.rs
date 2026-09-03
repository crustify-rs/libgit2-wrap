//! Safe wrappers for libgit2 filter APIs.

use core::marker::PhantomData;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::ffi;
use crate::oid::{OidMut, OidRef};

/// Wraps: git_filter_mode_t
/// A validated direction for transforming content through a filter list.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GitFilterMode {
    /// Export content from the object database to the working tree.
    ToWorktree = ffi::git_filter_mode_t_GIT_FILTER_TO_WORKTREE,
    /// Import content from the working tree into the object database.
    ToObjectDatabase = ffi::git_filter_mode_t_GIT_FILTER_TO_ODB,
}

impl GitFilterMode {
    /// The traditional name for a worktree-directed filter.
    pub const SMUDGE: Self = Self::ToWorktree;
    /// The traditional name for an object-database-directed filter.
    pub const CLEAN: Self = Self::ToObjectDatabase;
}

impl From<GitFilterMode> for ffi::git_filter_mode_t {
    fn from(mode: GitFilterMode) -> Self {
        mode as Self
    }
}

impl TryFrom<ffi::git_filter_mode_t> for GitFilterMode {
    type Error = InvalidGitFilterMode;

    fn try_from(mode: ffi::git_filter_mode_t) -> Result<Self, Self::Error> {
        match mode {
            ffi::git_filter_mode_t_GIT_FILTER_TO_WORKTREE => Ok(Self::ToWorktree),
            ffi::git_filter_mode_t_GIT_FILTER_TO_ODB => Ok(Self::ToObjectDatabase),
            value => Err(InvalidGitFilterMode(value)),
        }
    }
}

/// A raw value that is not a published [`GitFilterMode`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitFilterMode(ffi::git_filter_mode_t);

impl InvalidGitFilterMode {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_filter_mode_t {
        self.0
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn filter_modes_validate_and_preserve_aliases() {
        assert_eq!(GitFilterMode::SMUDGE, GitFilterMode::ToWorktree);
        assert_eq!(GitFilterMode::CLEAN, GitFilterMode::ToObjectDatabase);
        assert_eq!(
            GitFilterMode::try_from(ffi::git_filter_mode_t_GIT_FILTER_TO_WORKTREE),
            Ok(GitFilterMode::ToWorktree)
        );

        let invalid = ffi::git_filter_mode_t_GIT_FILTER_TO_ODB + 1;
        assert_eq!(
            GitFilterMode::try_from(invalid).unwrap_err().value(),
            invalid
        );
    }

    #[test]
    fn filter_mode_matches_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitFilterMode>(),
            size_of::<ffi::git_filter_mode_t>()
        );
        assert_eq!(
            align_of::<GitFilterMode>(),
            align_of::<ffi::git_filter_mode_t>()
        );
    }
}

/// Wraps: git_filter_flag_t
/// A checked set of options controlling attribute lookup and filter safety.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitFilterFlags(ffi::git_filter_flag_t);

impl GitFilterFlags {
    /// Use the default filter behavior.
    pub const DEFAULT: Self = Self(ffi::git_filter_flag_t_GIT_FILTER_DEFAULT);
    /// Continue despite `safecrlf` violations.
    pub const ALLOW_UNSAFE: Self = Self(ffi::git_filter_flag_t_GIT_FILTER_ALLOW_UNSAFE);
    /// Do not load system-level attributes.
    pub const NO_SYSTEM_ATTRIBUTES: Self =
        Self(ffi::git_filter_flag_t_GIT_FILTER_NO_SYSTEM_ATTRIBUTES);
    /// Also load attributes from the root of `HEAD`.
    pub const ATTRIBUTES_FROM_HEAD: Self =
        Self(ffi::git_filter_flag_t_GIT_FILTER_ATTRIBUTES_FROM_HEAD);
    /// Load attributes from the commit selected by filter options.
    pub const ATTRIBUTES_FROM_COMMIT: Self =
        Self(ffi::git_filter_flag_t_GIT_FILTER_ATTRIBUTES_FROM_COMMIT);
    /// Every filter flag published by these bindings.
    pub const ALL: Self = Self(
        Self::ALLOW_UNSAFE.0
            | Self::NO_SYSTEM_ATTRIBUTES.0
            | Self::ATTRIBUTES_FROM_HEAD.0
            | Self::ATTRIBUTES_FROM_COMMIT.0,
    );

    /// Converts raw bits when every bit is a published filter option.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_filter_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_filter_flag_t {
        self.0
    }

    /// Returns whether no filter option is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any option in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitFilterFlags> for ffi::git_filter_flag_t {
    fn from(flags: GitFilterFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_filter_flag_t> for GitFilterFlags {
    type Error = ffi::git_filter_flag_t;

    fn try_from(bits: ffi::git_filter_flag_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitFilterFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitFilterFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitFilterFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitFilterFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitFilterFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod filter_flag_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn filter_options_combine_and_validate() {
        let flags = GitFilterFlags::ALLOW_UNSAFE | GitFilterFlags::NO_SYSTEM_ATTRIBUTES;
        assert!(flags.contains(GitFilterFlags::ALLOW_UNSAFE));
        assert!(flags.intersects(GitFilterFlags::NO_SYSTEM_ATTRIBUTES));
        assert_eq!(GitFilterFlags::from_bits(flags.bits()), Some(flags));
        assert!(GitFilterFlags::DEFAULT.is_empty());

        let unknown = GitFilterFlags::ALL.bits() << 1;
        assert_eq!(GitFilterFlags::from_bits(unknown), None);
    }

    #[test]
    fn filter_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitFilterFlags>(),
            size_of::<ffi::git_filter_flag_t>()
        );
        assert_eq!(
            align_of::<GitFilterFlags>(),
            align_of::<ffi::git_filter_flag_t>()
        );
    }
}

/// Wraps: git_filter_options
/// Layout-compatible filtering options whose deprecated commit pointer, when
/// present, borrows an object ID for `'data`.
///
/// The lifetime is invariant because mutable access can install the pointer
/// and a later shared handle can borrow it back. Safe construction leaves the
/// conditional legacy/reserved ABI slot null.
#[repr(transparent)]
pub struct GitFilterOptions<'data> {
    inner: CType<ffi::git_filter_options>,
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitFilterOptions`].
#[repr(transparent)]
pub struct GitFilterOptionsRef<'object, 'data>(CPtr<'object, GitFilterOptions<'data>>);

impl Clone for GitFilterOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitFilterOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitFilterOptions`].
#[repr(transparent)]
pub struct GitFilterOptionsMut<'object, 'data>(GitFilterOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and use only raw-place projections. The
// shared handle exposes no writes.
unsafe impl<'data> CCell for GitFilterOptions<'data> {
    type C = ffi::git_filter_options;
    type Ref<'object>
        = GitFilterOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitFilterOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitFilterOptionsRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitFilterOptionsMut(GitFilterOptionsRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: the options own only an inline `git_oid`; the pointer field is a
// borrow and no field needs disposal when inline storage goes out of scope.
unsafe impl CValued for GitFilterOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitFilterOptions<'data> {
    /// Constructs options equivalent to `GIT_FILTER_OPTIONS_INIT`.
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
            .set_version(ffi::GIT_FILTER_OPTIONS_VERSION);
        options
    }
}

impl<'object, 'data> GitFilterOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify initialized options live for `'object`. A non-null
    /// deprecated commit ID must remain live and shared-only for `'data`, which
    /// must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_filter_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitFilterOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_filter_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_filter_options.flags
    /// Returns the checked set of filtering options.
    pub fn flags(&self) -> Result<GitFilterFlags, ffi::git_filter_flag_t> {
        // SAFETY: raw-place projection copies the initialized scalar field.
        let bits = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitFilterFlags::try_from(bits)
    }

    /// Field: git_filter_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_filter_options.attr_commit_id
    /// Borrows the inline commit used for attribute lookup.
    #[must_use]
    pub fn attr_commit_id(&self) -> OidRef<'object> {
        // SAFETY: raw-place projection locates the initialized inline field
        // without forming a reference over C-visible storage.
        let oid = unsafe { addr_of!((*self.as_ptr()).attr_commit_id).cast_mut() };
        // SAFETY: the projected field lives for the enclosing options borrow.
        unsafe { OidRef::from_ptr(oid) }.expect("an inline field is non-null")
    }

    /// Field: git_filter_options.commit_id
    /// Borrows the optional deprecated caller-owned commit ID.
    #[must_use]
    pub fn commit_id(&self) -> Option<OidRef<'object>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let oid = unsafe { addr_of!((*self.as_ptr()).commit_id).read() };
        // SAFETY: the wrapper contract keeps a non-null ID live and shared for
        // at least this options borrow.
        unsafe { OidRef::from_ptr(oid) }
    }

    /// Field: git_filter_options.reserved
    /// Reports whether the conditional legacy/reserved ABI slot is clear.
    ///
    /// This configured non-hard-deprecation layout names the slot `commit_id`;
    /// hard-deprecation headers name the same slot `reserved`. Safe
    /// construction and [`GitFilterOptionsMut::clear_conditional_slot`] leave
    /// either interpretation null.
    #[must_use]
    pub fn reserved_slot_is_clear(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized conditional
        // pointer slot without dereferencing it.
        unsafe { addr_of!((*self.as_ptr()).commit_id).read().is_null() }
    }
}

impl<'object, 'data> GitFilterOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path to the
    /// options value may be used for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_filter_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitFilterOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_filter_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitFilterOptionsRef<'_, 'data> {
        GitFilterOptionsRef(self.0.0)
    }

    /// Replaces the filtering flags.
    pub fn set_flags(&mut self, flags: GitFilterFlags) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Exclusively borrows the inline attribute commit ID.
    #[must_use]
    pub fn attr_commit_id_mut(&mut self) -> OidMut<'_> {
        // SAFETY: raw-place projection originates from this exclusive handle.
        let oid = unsafe { addr_of_mut!((*self.as_mut_ptr()).attr_commit_id) };
        // SAFETY: the projected field is initialized and exclusively borrowed.
        unsafe { OidMut::from_ptr(oid) }.expect("an inline field is non-null")
    }

    /// Copies an object ID into the inline attribute-commit field.
    pub fn set_attr_commit_id(&mut self, oid: OidRef<'_>) {
        // SAFETY: `oid` identifies an initialized layout-compatible value;
        // copying the inline C value transfers no ownership.
        let oid = unsafe { oid.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).attr_commit_id).write(oid) }
    }

    /// Stores an optional borrowed ID in the deprecated conditional slot.
    pub fn set_commit_id(&mut self, oid: Option<OidRef<'data>>) {
        let oid = oid.map_or(core::ptr::null(), |oid| oid.as_ptr());
        // SAFETY: this exclusive handle permits the pointer write, and the
        // invariant wrapper lifetime keeps a non-null ID live and shared.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).commit_id).write(oid.cast_mut()) }
    }

    /// Clears the conditional legacy/reserved ABI slot.
    pub fn clear_conditional_slot(&mut self) {
        // SAFETY: this exclusive handle permits the pointer write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).commit_id).write(core::ptr::null_mut()) }
    }
}

#[cfg(test)]
mod filter_options_tests {
    use core::mem::{align_of, size_of};

    use crate::oid::{Oid, OidType};

    use super::*;

    #[test]
    fn options_preserve_layout_defaults_and_borrowed_commit() {
        assert_eq!(
            size_of::<GitFilterOptions<'static>>(),
            size_of::<ffi::git_filter_options>()
        );
        assert_eq!(
            align_of::<GitFilterOptions<'static>>(),
            align_of::<ffi::git_filter_options>()
        );
        assert_eq!(
            size_of::<GitFilterOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_filter_options>()
        );

        let mut oid = Oid::zeroed();
        let oid_ptr = addr_of_mut!(oid).cast::<ffi::git_oid>();
        // SAFETY: the layout-compatible stack value remains initialized and
        // live while the options borrow it.
        let mut oid_mut = unsafe { OidMut::from_ptr(oid_ptr) }.unwrap();
        oid_mut.set_oid_type(OidType::Sha256);
        let oid = oid_mut.as_ref();

        let mut options = GitFilterOptions::new();
        {
            let mut view = options.as_mut();
            assert_eq!(view.as_ref().version(), ffi::GIT_FILTER_OPTIONS_VERSION);
            assert_eq!(view.as_ref().flags(), Ok(GitFilterFlags::DEFAULT));
            assert!(view.as_ref().commit_id().is_none());
            assert!(view.as_ref().reserved_slot_is_clear());

            view.set_flags(GitFilterFlags::ATTRIBUTES_FROM_COMMIT);
            view.set_attr_commit_id(oid);
            view.set_commit_id(Some(oid));

            assert_eq!(
                view.as_ref().flags(),
                Ok(GitFilterFlags::ATTRIBUTES_FROM_COMMIT)
            );
            assert_eq!(
                view.as_ref().attr_commit_id().oid_type(),
                Ok(OidType::Sha256)
            );
            assert_eq!(
                view.as_ref().commit_id().unwrap().oid_type(),
                Ok(OidType::Sha256)
            );
            assert!(!view.as_ref().reserved_slot_is_clear());

            view.clear_conditional_slot();
            assert!(view.as_ref().reserved_slot_is_clear());
        }
    }
}
