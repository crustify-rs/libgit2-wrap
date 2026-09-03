//! Safe wrappers for libgit2 email APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::diff::{
    DiffFindOptionsMut, DiffFindOptionsRef, DiffOptions, GitDiffOptionsMut, GitDiffOptionsRef,
};
use crate::ffi;

/// Wraps: git_email_create_flags_t
/// A checked set of formatting options for generated patch emails.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitEmailCreateFlags(ffi::git_email_create_flags_t);

impl GitEmailCreateFlags {
    /// The default patch-email formatting behavior.
    pub const DEFAULT: Self = Self(ffi::git_email_create_flags_t_GIT_EMAIL_CREATE_DEFAULT);
    /// Omit patch numbers from the subject prefix.
    pub const OMIT_NUMBERS: Self =
        Self(ffi::git_email_create_flags_t_GIT_EMAIL_CREATE_OMIT_NUMBERS);
    /// Include patch numbers even for a single-commit series.
    pub const ALWAYS_NUMBER: Self =
        Self(ffi::git_email_create_flags_t_GIT_EMAIL_CREATE_ALWAYS_NUMBER);
    /// Disable rename and similarity detection.
    pub const NO_RENAMES: Self = Self(ffi::git_email_create_flags_t_GIT_EMAIL_CREATE_NO_RENAMES);
    /// Every option currently published by libgit2.
    pub const ALL: Self = Self(Self::OMIT_NUMBERS.0 | Self::ALWAYS_NUMBER.0 | Self::NO_RENAMES.0);

    /// Converts a C bit set when every set bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_email_create_flags_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 bit set.
    #[must_use]
    pub const fn bits(self) -> ffi::git_email_create_flags_t {
        self.0
    }

    /// Returns whether no formatting option is enabled.
    #[must_use]
    pub const fn is_default(self) -> bool {
        self.0 == Self::DEFAULT.0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for GitEmailCreateFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitEmailCreateFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitEmailCreateFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitEmailCreateFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl From<GitEmailCreateFlags> for ffi::git_email_create_flags_t {
    fn from(flags: GitEmailCreateFlags) -> Self {
        flags.bits()
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_email_flags_form_checked_bit_sets() {
        let mut flags = GitEmailCreateFlags::DEFAULT;
        assert!(flags.is_default());

        flags |= GitEmailCreateFlags::OMIT_NUMBERS | GitEmailCreateFlags::NO_RENAMES;
        assert!(flags.contains(GitEmailCreateFlags::OMIT_NUMBERS));
        assert!(flags.contains(GitEmailCreateFlags::NO_RENAMES));
        assert!(!flags.contains(GitEmailCreateFlags::ALWAYS_NUMBER));

        flags &= GitEmailCreateFlags::NO_RENAMES | GitEmailCreateFlags::ALWAYS_NUMBER;
        assert_eq!(flags, GitEmailCreateFlags::NO_RENAMES);
        assert_eq!(ffi::git_email_create_flags_t::from(flags), flags.bits());
    }

    #[test]
    fn raw_email_flag_bits_are_validated() {
        for flags in [
            GitEmailCreateFlags::DEFAULT,
            GitEmailCreateFlags::OMIT_NUMBERS,
            GitEmailCreateFlags::ALWAYS_NUMBER,
            GitEmailCreateFlags::NO_RENAMES,
            GitEmailCreateFlags::ALL,
        ] {
            assert_eq!(GitEmailCreateFlags::from_bits(flags.bits()), Some(flags));
        }

        assert_eq!(
            GitEmailCreateFlags::from_bits(GitEmailCreateFlags::ALL.bits() << 1),
            None
        );
    }

    #[test]
    fn email_flags_match_the_c_abi_scalar() {
        assert_eq!(
            size_of::<GitEmailCreateFlags>(),
            size_of::<ffi::git_email_create_flags_t>()
        );
        assert_eq!(
            align_of::<GitEmailCreateFlags>(),
            align_of::<ffi::git_email_create_flags_t>()
        );
    }
}

/// Wraps: git_email_create_options
/// Layout-compatible email options borrowing the subject prefix and nested
/// diff-option data for `'data`.
///
/// `'data` is invariant. [`GitEmailCreateOptionsMut::set_subject_prefix`]
/// stores a `&'data` string in the C struct while
/// [`GitEmailCreateOptionsRef::subject_prefix`] hands one back out, and the
/// nested diff options behave the same way. A covariant `'data` would let
/// safe code shrink the parameter on the exclusive handle, install a
/// shorter-lived prefix, and then read the released string back through a
/// handle still typed at the longer lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::email::GitEmailCreateOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitEmailCreateOptionsMut<'object, 'static>,
/// ) -> GitEmailCreateOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitEmailCreateOptions<'data> {
    inner: CType<ffi::git_email_create_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: core::marker::PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitEmailCreateOptions`].
#[repr(transparent)]
pub struct GitEmailCreateOptionsRef<'object, 'data>(CPtr<'object, GitEmailCreateOptions<'data>>);

impl Clone for GitEmailCreateOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitEmailCreateOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitEmailCreateOptions`].
#[repr(transparent)]
pub struct GitEmailCreateOptionsMut<'object, 'data>(GitEmailCreateOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitEmailCreateOptions<'data> {
    type C = ffi::git_email_create_options;
    type Ref<'object>
        = GitEmailCreateOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitEmailCreateOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitEmailCreateOptionsRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitEmailCreateOptionsMut(GitEmailCreateOptionsRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: this public options record only borrows its pointer fields. Neither
// it nor its nested options owns a resource that an inline disposer must free.
unsafe impl CValued for GitEmailCreateOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitEmailCreateOptions<'data> {
    /// Constructs options equivalent to `GIT_EMAIL_CREATE_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: all fields in the C options record admit the all-zero bit
        // pattern. Required versions and documented nonzero defaults are set
        // through exclusive field projections before return.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: core::marker::PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_EMAIL_CREATE_OPTIONS_VERSION);
            view.diff_options_mut()
                .set_version(ffi::GIT_DIFF_OPTIONS_VERSION);
            view.diff_options_mut().set_flags(DiffOptions::SHOW_BINARY);
            view.diff_options_mut()
                .set_ignore_submodules(crate::api::types::GitSubmoduleIgnore::Unspecified);
            view.diff_options_mut().set_context_lines(3);
            view.diff_find_options_mut()
                .set_version(ffi::GIT_DIFF_FIND_OPTIONS_VERSION);
        }
        options
    }
}

impl<'object, 'data> GitEmailCreateOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify initialized options that live for `'object`.
    /// Every configured string and nested borrowed value must remain valid for
    /// `'data`, which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_email_create_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitEmailCreateOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_email_create_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_email_create_options.flags
    /// Returns the checked set of email-formatting flags.
    pub fn flags(&self) -> Result<GitEmailCreateFlags, ffi::git_email_create_flags_t> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let flags = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitEmailCreateFlags::from_bits(flags).ok_or(flags)
    }

    /// Field: git_email_create_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_email_create_options.reroll_number
    /// Returns the patch-series reroll number, or zero when absent.
    #[must_use]
    pub fn reroll_number(&self) -> usize {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).reroll_number).read() }
    }

    /// Field: git_email_create_options.start_number
    /// Returns the requested first patch number, or zero for libgit2's default.
    #[must_use]
    pub fn start_number(&self) -> usize {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).start_number).read() }
    }

    /// Field: git_email_create_options.subject_prefix
    /// Borrows the optional NUL-terminated subject prefix.
    #[must_use]
    pub fn subject_prefix(&self) -> Option<&'object core::ffi::CStr> {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        let prefix = unsafe { addr_of!((*self.as_ptr()).subject_prefix).read() };
        if prefix.is_null() {
            None
        } else {
            // SAFETY: the wrapper contract keeps the configured NUL string
            // valid for at least this options borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(prefix) })
        }
    }

    /// Field: git_email_create_options.diff_find_opts
    /// Borrows the inline similarity-detection options.
    #[must_use]
    pub fn diff_find_options(&self) -> DiffFindOptionsRef<'object> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // without forming a reference to C-visible storage.
        let ptr = unsafe { addr_of!((*self.as_ptr()).diff_find_opts).cast_mut() };
        // SAFETY: the projected inline field lives for this outer borrow.
        unsafe { DiffFindOptionsRef::from_ptr(ptr) }.expect("an inline field is non-null")
    }

    /// Field: git_email_create_options.diff_opts
    /// Borrows the inline diff-generation options.
    #[must_use]
    pub fn diff_options(&self) -> GitDiffOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // without forming a reference to C-visible storage.
        let ptr = unsafe { addr_of!((*self.as_ptr()).diff_opts).cast_mut() };
        // SAFETY: the projected field lives for this outer borrow and shares
        // the outer record's borrowed-data lifetime.
        unsafe { GitDiffOptionsRef::from_ptr(ptr) }.expect("an inline field is non-null")
    }
}

impl<'object, 'data> GitEmailCreateOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path may use
    /// the options value for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_email_create_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitEmailCreateOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_email_create_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitEmailCreateOptionsRef<'_, 'data> {
        GitEmailCreateOptionsRef(self.0.0)
    }

    /// Replaces the email-formatting flags.
    pub fn set_flags(&mut self, flags: GitEmailCreateFlags) {
        // SAFETY: this live exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Sets the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this live exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Sets the patch-series reroll number, or zero to omit it.
    pub fn set_reroll_number(&mut self, number: usize) {
        // SAFETY: this live exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).reroll_number).write(number) }
    }

    /// Sets the first patch number. Zero selects libgit2's default of one.
    pub fn set_start_number(&mut self, number: usize) {
        // SAFETY: this live exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).start_number).write(number) }
    }

    /// Stores an optional borrowed subject prefix.
    pub fn set_subject_prefix(&mut self, prefix: Option<&'data core::ffi::CStr>) {
        let prefix = prefix.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        // SAFETY: this live exclusive handle permits the pointer write, and
        // the wrapper lifetime keeps a non-null NUL string live.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).subject_prefix).write(prefix) }
    }

    /// Exclusively borrows the inline similarity-detection options.
    #[must_use]
    pub fn diff_find_options_mut(&mut self) -> DiffFindOptionsMut<'_> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // under this exclusive outer reborrow.
        let ptr = unsafe { addr_of_mut!((*self.as_mut_ptr()).diff_find_opts) };
        // SAFETY: the projected field is exclusively accessible for the
        // returned handle's reborrow.
        unsafe { DiffFindOptionsMut::from_ptr(ptr) }.expect("an inline field is non-null")
    }

    /// Exclusively borrows the inline diff-generation options.
    #[must_use]
    pub fn diff_options_mut(&mut self) -> GitDiffOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // under this exclusive outer reborrow.
        let ptr = unsafe { addr_of_mut!((*self.as_mut_ptr()).diff_opts) };
        // SAFETY: the projected field is exclusively accessible and shares
        // the outer record's borrowed-data lifetime.
        unsafe { GitDiffOptionsMut::from_ptr(ptr) }.expect("an inline field is non-null")
    }
}

#[cfg(test)]
mod email_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn options_preserve_layout_and_public_defaults() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}
        assert_cell::<GitEmailCreateOptions<'static>>();
        assert_valued::<GitEmailCreateOptions<'static>>();
        assert_eq!(
            size_of::<GitEmailCreateOptions<'static>>(),
            size_of::<ffi::git_email_create_options>()
        );
        assert_eq!(
            align_of::<GitEmailCreateOptions<'static>>(),
            align_of::<ffi::git_email_create_options>()
        );
        assert_eq!(
            size_of::<GitEmailCreateOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_email_create_options>()
        );

        let options = GitEmailCreateOptions::new();
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_EMAIL_CREATE_OPTIONS_VERSION);
        assert_eq!(view.flags(), Ok(GitEmailCreateFlags::DEFAULT));
        assert_eq!(view.diff_options().flags(), Ok(DiffOptions::SHOW_BINARY));
        assert_eq!(view.diff_options().context_lines(), 3);
        assert_eq!(
            view.diff_find_options().version(),
            ffi::GIT_DIFF_FIND_OPTIONS_VERSION
        );
        assert!(view.subject_prefix().is_none());
        assert_eq!(view.start_number(), 0);
        assert_eq!(view.reroll_number(), 0);
    }

    #[test]
    fn scalar_string_and_nested_fields_round_trip() {
        let mut options = GitEmailCreateOptions::new();
        {
            let mut view = options.as_mut();
            view.set_flags(GitEmailCreateFlags::ALWAYS_NUMBER);
            view.set_subject_prefix(Some(c"SERIES"));
            view.set_start_number(4);
            view.set_reroll_number(2);
            view.diff_find_options_mut().set_rename_limit(77);
            view.diff_options_mut().set_context_lines(8);
        }

        let view = options.as_ref();
        assert_eq!(view.flags(), Ok(GitEmailCreateFlags::ALWAYS_NUMBER));
        assert_eq!(view.subject_prefix(), Some(c"SERIES"));
        assert_eq!(view.start_number(), 4);
        assert_eq!(view.reroll_number(), 2);
        assert_eq!(view.diff_find_options().rename_limit(), 77);
        assert_eq!(view.diff_options().context_lines(), 8);
    }
}
