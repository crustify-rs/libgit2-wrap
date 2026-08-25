//! Safe wrappers for libgit2 merge APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::diff::DiffSimilarityMetricRef;
use crate::ffi;
use crate::merge::MergeFileFavor;

/// Wraps: git_merge_file_flag_t
/// Known behavior flags accepted by file-level merges.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MergeFileFlags(ffi::git_merge_file_flag_t);

impl MergeFileFlags {
    /// Use libgit2's default file-merge behavior.
    pub const DEFAULT: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DEFAULT);
    /// Produce standard two-sided conflict markers.
    pub const STYLE_MERGE: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_MERGE);
    /// Include the common ancestor in conflict markers.
    pub const STYLE_DIFF3: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_DIFF3);
    /// Condense non-alphanumeric regions while comparing.
    pub const SIMPLIFY_ALNUM: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_SIMPLIFY_ALNUM);
    /// Ignore all whitespace changes.
    pub const IGNORE_WHITESPACE: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_IGNORE_WHITESPACE);
    /// Ignore changes in the amount of whitespace.
    pub const IGNORE_WHITESPACE_CHANGE: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_IGNORE_WHITESPACE_CHANGE);
    /// Ignore whitespace changes at the ends of lines.
    pub const IGNORE_WHITESPACE_EOL: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_IGNORE_WHITESPACE_EOL);
    /// Use the patience-diff algorithm.
    pub const DIFF_PATIENCE: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DIFF_PATIENCE);
    /// Spend extra time finding a minimal diff.
    pub const DIFF_MINIMAL: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DIFF_MINIMAL);
    /// Produce zealous diff3 conflict markers.
    pub const STYLE_ZDIFF3: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_ZDIFF3);
    /// Accept output containing conflict markers as a merge result.
    pub const ACCEPT_CONFLICTS: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_ACCEPT_CONFLICTS);
    /// Every file-merge flag published by this libgit2 version.
    pub const ALL: Self = Self(
        Self::STYLE_MERGE.0
            | Self::STYLE_DIFF3.0
            | Self::SIMPLIFY_ALNUM.0
            | Self::IGNORE_WHITESPACE.0
            | Self::IGNORE_WHITESPACE_CHANGE.0
            | Self::IGNORE_WHITESPACE_EOL.0
            | Self::DIFF_PATIENCE.0
            | Self::DIFF_MINIMAL.0
            | Self::STYLE_ZDIFF3.0
            | Self::ACCEPT_CONFLICTS.0,
    );

    /// Converts raw bits when every bit is known to this libgit2 version.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_merge_file_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_merge_file_flag_t {
        self.0
    }

    /// Returns whether no behavior-changing flag is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for MergeFileFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for MergeFileFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for MergeFileFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for MergeFileFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for MergeFileFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

impl From<MergeFileFlags> for ffi::git_merge_file_flag_t {
    fn from(flags: MergeFileFlags) -> Self {
        flags.bits()
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn published_file_merge_flags_form_checked_sets() {
        let mut flags = MergeFileFlags::STYLE_DIFF3 | MergeFileFlags::DIFF_PATIENCE;
        assert!(flags.contains(MergeFileFlags::STYLE_DIFF3));
        flags |= MergeFileFlags::ACCEPT_CONFLICTS;
        assert!(flags.contains(MergeFileFlags::ACCEPT_CONFLICTS));
        assert_eq!(MergeFileFlags::from_bits(flags.bits()), Some(flags));
        assert_eq!(ffi::git_merge_file_flag_t::from(flags), flags.bits());
        assert!(MergeFileFlags::DEFAULT.is_empty());
    }

    #[test]
    fn unknown_file_merge_flags_are_rejected() {
        let unknown = MergeFileFlags::ALL.bits() << 1;
        assert_eq!(MergeFileFlags::from_bits(unknown), None);
    }

    #[test]
    fn file_merge_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<MergeFileFlags>(),
            size_of::<ffi::git_merge_file_flag_t>()
        );
        assert_eq!(
            align_of::<MergeFileFlags>(),
            align_of::<ffi::git_merge_file_flag_t>()
        );
    }
}

/// Wraps: git_merge_options
/// Layout-compatible merge options borrowing a default-driver string and
/// optional similarity table for `'data`.
///
/// `'data` is invariant. The setters store a `&'data` referent into the C
/// struct while the getters hand one back out, so a covariant `'data` would
/// let safe code shrink the parameter on the exclusive handle, install a
/// shorter-lived driver name or similarity table, and then read it back
/// through a handle still typed at the longer lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::merge::GitMergeOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitMergeOptionsMut<'object, 'static>,
/// ) -> GitMergeOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitMergeOptions<'data> {
    inner: CType<ffi::git_merge_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitMergeOptions`].
#[repr(transparent)]
pub struct GitMergeOptionsRef<'object, 'data>(CPtr<'object, GitMergeOptions<'data>>);

impl Clone for GitMergeOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitMergeOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitMergeOptions`].
#[repr(transparent)]
pub struct GitMergeOptionsMut<'object, 'data>(GitMergeOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitMergeOptions<'data> {
    type C = ffi::git_merge_options;
    type Ref<'object>
        = GitMergeOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitMergeOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(p: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `p` is a live shared object.
        GitMergeOptionsRef(unsafe { CPtr::new(p) })
    }

    unsafe fn mut_from_raw<'object>(p: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitMergeOptionsMut(GitMergeOptionsRef(unsafe { CPtr::new(p) }))
    }
}

// SAFETY: this options header only borrows its pointer fields and owns no
// resource, so disposing its inline storage requires no action.
unsafe impl CValued for GitMergeOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitMergeOptions<'data> {
    /// Constructs options equivalent to `GIT_MERGE_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every field of the C options struct admits the all-zero bit
        // pattern; published nonzero defaults are installed below.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_MERGE_OPTIONS_VERSION);
            view.set_flags(ffi::git_merge_flag_t_GIT_MERGE_FIND_RENAMES);
        }
        options
    }
}

impl<'object, 'data> GitMergeOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify an initialized options value that lives for
    /// `'object`. Its metric and default driver must remain valid for `'data`,
    /// which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_merge_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitMergeOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_merge_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_merge_options.flags
    /// Returns the merge behavior bits. Unknown bits are retained for forward
    /// compatibility with newer libgit2 versions.
    #[must_use]
    pub fn flags(&self) -> u32 {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Field: git_merge_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_merge_options.default_driver
    /// Borrows the optional default merge-driver name.
    #[must_use]
    pub fn default_driver(&self) -> Option<&'object CStr> {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        let driver = unsafe { addr_of!((*self.as_ptr()).default_driver).read() };
        if driver.is_null() {
            None
        } else {
            // SAFETY: the wrapper contract keeps the borrowed NUL string live
            // for at least this object borrow.
            Some(unsafe { CStr::from_ptr(driver) })
        }
    }

    /// Field: git_merge_options.metric
    /// Borrows the optional caller-supplied similarity table.
    #[must_use]
    pub fn metric(&self) -> Option<DiffSimilarityMetricRef<'object>> {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        let metric = unsafe { addr_of!((*self.as_ptr()).metric).read() };
        // SAFETY: the wrapper contract keeps a non-null borrowed table live
        // for at least this object borrow.
        unsafe { DiffSimilarityMetricRef::from_ptr(metric) }
    }

    /// Field: git_merge_options.rename_threshold
    /// Returns the percentage threshold used to recognize renames.
    #[must_use]
    pub fn rename_threshold(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).rename_threshold).read() }
    }

    /// Field: git_merge_options.file_flags
    /// Returns the checked flags passed to the standard file-merge driver.
    pub fn file_flags(&self) -> Result<MergeFileFlags, ffi::git_merge_file_flag_t> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let flags = unsafe { addr_of!((*self.as_ptr()).file_flags).read() };
        MergeFileFlags::from_bits(flags).ok_or(flags)
    }

    /// Field: git_merge_options.file_favor
    /// Returns the checked side-selection policy for content conflicts.
    pub fn file_favor(&self) -> Result<MergeFileFavor, ffi::git_merge_file_favor_t> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let favor = unsafe { addr_of!((*self.as_ptr()).file_favor).read() };
        MergeFileFavor::try_from(favor)
    }

    /// Field: git_merge_options.recursion_limit
    /// Returns the maximum recursive virtual-base depth, or zero for unlimited.
    #[must_use]
    pub fn recursion_limit(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).recursion_limit).read() }
    }

    /// Field: git_merge_options.target_limit
    /// Returns the maximum candidate sources examined for renames.
    #[must_use]
    pub fn target_limit(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).target_limit).read() }
    }
}

impl<'object, 'data> GitMergeOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path may use
    /// the options value for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_merge_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitMergeOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_merge_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitMergeOptionsRef<'_, 'data> {
        GitMergeOptionsRef(self.0.0)
    }

    /// Replaces the merge behavior bits, retaining unknown bits.
    pub fn set_flags(&mut self, flags: u32) {
        // SAFETY: this exclusive handle permits the scalar write. Libgit2
        // defines the field as a bit mask and ignores no storage invariant.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Sets the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Stores an optional borrowed default merge-driver name.
    pub fn set_default_driver(&mut self, driver: Option<&'data CStr>) {
        let driver = driver.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write, and the
        // wrapper lifetime keeps a non-null NUL string live.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).default_driver).write(driver) }
    }

    /// Stores an optional borrowed similarity table.
    pub fn set_metric(&mut self, metric: Option<DiffSimilarityMetricRef<'data>>) {
        let metric = metric.map_or(core::ptr::null_mut(), |metric| metric.as_ptr().cast_mut());
        // SAFETY: this exclusive handle permits the pointer write, and the
        // wrapper lifetime keeps a non-null table live. Libgit2 reads it only.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).metric).write(metric) }
    }

    /// Sets the percentage threshold used to recognize renames.
    pub fn set_rename_threshold(&mut self, threshold: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).rename_threshold).write(threshold) }
    }

    /// Replaces the flags passed to the standard file-merge driver.
    pub fn set_file_flags(&mut self, flags: MergeFileFlags) {
        // SAFETY: this exclusive handle permits the scalar write, and the
        // wrapper contains only published file-merge bits.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).file_flags).write(flags.bits()) }
    }

    /// Selects which side wins content conflicts.
    pub fn set_file_favor(&mut self, favor: MergeFileFavor) {
        // SAFETY: this exclusive handle permits the scalar write, and `favor`
        // is one of the published enum values.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).file_favor).write(favor.into()) }
    }

    /// Sets the maximum recursive virtual-base depth, or zero for unlimited.
    pub fn set_recursion_limit(&mut self, limit: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).recursion_limit).write(limit) }
    }

    /// Sets the maximum candidate sources examined for renames.
    pub fn set_target_limit(&mut self, limit: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).target_limit).write(limit) }
    }
}

#[cfg(test)]
mod merge_options_tests {
    use core::mem::{align_of, size_of};

    use crate::api::diff::DiffSimilarityMetric;

    use super::*;

    #[test]
    fn options_preserve_layout_and_defaults() {
        assert_eq!(
            size_of::<GitMergeOptions<'static>>(),
            size_of::<ffi::git_merge_options>()
        );
        assert_eq!(
            align_of::<GitMergeOptions<'static>>(),
            align_of::<ffi::git_merge_options>()
        );
        assert_eq!(
            size_of::<GitMergeOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_merge_options>()
        );

        let options = GitMergeOptions::new();
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_MERGE_OPTIONS_VERSION);
        assert_eq!(view.flags(), ffi::git_merge_flag_t_GIT_MERGE_FIND_RENAMES);
        assert_eq!(view.rename_threshold(), 0);
        assert_eq!(view.target_limit(), 0);
        assert!(view.metric().is_none());
        assert_eq!(view.recursion_limit(), 0);
        assert!(view.default_driver().is_none());
        assert_eq!(view.file_favor(), Ok(MergeFileFavor::Normal));
        assert_eq!(view.file_flags(), Ok(MergeFileFlags::DEFAULT));
    }

    #[test]
    fn borrowed_and_scalar_fields_round_trip() {
        let metric = CVal::new(DiffSimilarityMetric::zeroed());
        let mut options = GitMergeOptions::new();
        {
            let mut view = options.as_mut();
            view.set_flags(0x11);
            view.set_default_driver(Some(c"custom"));
            view.set_metric(Some(metric.as_ref()));
            view.set_rename_threshold(72);
            view.set_file_flags(MergeFileFlags::STYLE_DIFF3 | MergeFileFlags::DIFF_PATIENCE);
            view.set_file_favor(MergeFileFavor::Ours);
            view.set_recursion_limit(5);
            view.set_target_limit(240);
        }

        let view = options.as_ref();
        assert_eq!(view.flags(), 0x11);
        assert_eq!(view.default_driver(), Some(c"custom"));
        assert_eq!(view.metric().unwrap().as_ptr(), metric.as_ref().as_ptr());
        assert_eq!(view.rename_threshold(), 72);
        assert_eq!(
            view.file_flags(),
            Ok(MergeFileFlags::STYLE_DIFF3 | MergeFileFlags::DIFF_PATIENCE)
        );
        assert_eq!(view.file_favor(), Ok(MergeFileFavor::Ours));
        assert_eq!(view.recursion_limit(), 5);
        assert_eq!(view.target_limit(), 240);
    }

    #[test]
    fn options_keep_a_scoped_data_borrow_across_shorter_object_borrows() {
        // The `compile_fail` doctest on `GitMergeOptions` covers the direction
        // that must be rejected. This covers the direction that must keep
        // working: `'data` is a local scope rather than `'static`, and the
        // value is reborrowed for several shorter `'object` lifetimes.
        let driver = std::ffi::CString::new("scoped").unwrap();
        let mut options = GitMergeOptions::new();
        options.as_mut().set_default_driver(Some(driver.as_c_str()));
        assert_eq!(options.as_ref().default_driver(), Some(driver.as_c_str()));
        options.as_mut().set_target_limit(11);
        assert_eq!(options.as_ref().default_driver(), Some(driver.as_c_str()));
    }

    #[test]
    fn getters_reject_invalid_enums_and_file_flags() {
        let mut raw = ffi::git_merge_options {
            version: ffi::GIT_MERGE_OPTIONS_VERSION,
            flags: 0,
            rename_threshold: 0,
            target_limit: 0,
            metric: core::ptr::null_mut(),
            recursion_limit: 0,
            default_driver: core::ptr::null(),
            file_favor: ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_UNION + 1,
            file_flags: MergeFileFlags::ALL.bits() << 1,
        };
        // SAFETY: all fields are initialized, pointer fields are null, and the
        // raw value remains live and unmodified for this shared handle.
        let options = unsafe { GitMergeOptionsRef::from_ptr(addr_of_mut!(raw)) }.unwrap();
        assert!(options.file_favor().is_err());
        assert!(options.file_flags().is_err());
    }
}
