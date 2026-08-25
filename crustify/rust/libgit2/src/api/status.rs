//! Safe wrappers for libgit2 status APIs.

use core::marker::PhantomData;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::ffi;
use crate::status::{InvalidStatusShow, StatusShow};
use crate::strarray::GitStrArrayRef;
use crate::tree::GitTreeRef;

/// Wraps: git_status_opt_t
/// A checked set of options controlling status scans.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitStatusOptionFlags(ffi::git_status_opt_t);

impl GitStatusOptionFlags {
    /// No optional status behavior.
    pub const NONE: Self = Self(0);
    /// Include untracked paths.
    pub const INCLUDE_UNTRACKED: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_UNTRACKED);
    /// Include ignored paths.
    pub const INCLUDE_IGNORED: Self = Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_IGNORED);
    /// Include paths without changes.
    pub const INCLUDE_UNMODIFIED: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_UNMODIFIED);
    /// Skip submodules without pending type changes.
    pub const EXCLUDE_SUBMODULES: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_EXCLUDE_SUBMODULES);
    /// Recurse into untracked directories.
    pub const RECURSE_UNTRACKED_DIRS: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RECURSE_UNTRACKED_DIRS);
    /// Treat pathspec entries as literal paths.
    pub const DISABLE_PATHSPEC_MATCH: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_DISABLE_PATHSPEC_MATCH);
    /// Include the contents of ignored directories.
    pub const RECURSE_IGNORED_DIRS: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RECURSE_IGNORED_DIRS);
    /// Detect renames between `HEAD` and the index.
    pub const RENAMES_HEAD_TO_INDEX: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RENAMES_HEAD_TO_INDEX);
    /// Detect renames between the index and working directory.
    pub const RENAMES_INDEX_TO_WORKDIR: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RENAMES_INDEX_TO_WORKDIR);
    /// Sort status entries case-sensitively.
    pub const SORT_CASE_SENSITIVELY: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_SORT_CASE_SENSITIVELY);
    /// Sort status entries case-insensitively.
    pub const SORT_CASE_INSENSITIVELY: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_SORT_CASE_INSENSITIVELY);
    /// Consider rewritten files as rename sources.
    pub const RENAMES_FROM_REWRITES: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_RENAMES_FROM_REWRITES);
    /// Do not refresh the index from disk.
    pub const NO_REFRESH: Self = Self(ffi::git_status_opt_t_GIT_STATUS_OPT_NO_REFRESH);
    /// Refresh index stat information for unchanged files.
    pub const UPDATE_INDEX: Self = Self(ffi::git_status_opt_t_GIT_STATUS_OPT_UPDATE_INDEX);
    /// Report unreadable working-directory paths.
    pub const INCLUDE_UNREADABLE: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_UNREADABLE);
    /// Report unreadable paths as untracked instead.
    pub const INCLUDE_UNREADABLE_AS_UNTRACKED: Self =
        Self(ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_UNREADABLE_AS_UNTRACKED);
    /// Every status option published by this version of libgit2.
    pub const ALL: Self = Self(
        Self::INCLUDE_UNTRACKED.0
            | Self::INCLUDE_IGNORED.0
            | Self::INCLUDE_UNMODIFIED.0
            | Self::EXCLUDE_SUBMODULES.0
            | Self::RECURSE_UNTRACKED_DIRS.0
            | Self::DISABLE_PATHSPEC_MATCH.0
            | Self::RECURSE_IGNORED_DIRS.0
            | Self::RENAMES_HEAD_TO_INDEX.0
            | Self::RENAMES_INDEX_TO_WORKDIR.0
            | Self::SORT_CASE_SENSITIVELY.0
            | Self::SORT_CASE_INSENSITIVELY.0
            | Self::RENAMES_FROM_REWRITES.0
            | Self::NO_REFRESH.0
            | Self::UPDATE_INDEX.0
            | Self::INCLUDE_UNREADABLE.0
            | Self::INCLUDE_UNREADABLE_AS_UNTRACKED.0,
    );

    /// Converts raw bits when every bit is a published status option.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_status_opt_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_status_opt_t {
        self.0
    }

    /// Returns whether no option is set.
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

impl From<GitStatusOptionFlags> for ffi::git_status_opt_t {
    fn from(flags: GitStatusOptionFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_status_opt_t> for GitStatusOptionFlags {
    type Error = ffi::git_status_opt_t;

    fn try_from(bits: ffi::git_status_opt_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitStatusOptionFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitStatusOptionFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitStatusOptionFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitStatusOptionFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitStatusOptionFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn status_options_combine_and_validate() {
        let mut options = GitStatusOptionFlags::INCLUDE_UNTRACKED;
        options |= GitStatusOptionFlags::RECURSE_UNTRACKED_DIRS;
        assert!(options.contains(GitStatusOptionFlags::INCLUDE_UNTRACKED));
        assert!(options.intersects(GitStatusOptionFlags::RECURSE_UNTRACKED_DIRS));
        assert!(!options.intersects(GitStatusOptionFlags::INCLUDE_IGNORED));
        assert_eq!(
            GitStatusOptionFlags::from_bits(options.bits()),
            Some(options)
        );
        assert!(GitStatusOptionFlags::NONE.is_empty());

        let unknown = GitStatusOptionFlags::ALL.bits() + 1;
        assert_eq!(GitStatusOptionFlags::from_bits(unknown), None);
        assert_eq!(GitStatusOptionFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn status_options_complement_stays_within_published_bits() {
        let options =
            GitStatusOptionFlags::INCLUDE_UNTRACKED | GitStatusOptionFlags::INCLUDE_IGNORED;
        assert!(!(!options).intersects(options));
        assert_eq!(options | !options, GitStatusOptionFlags::ALL);
    }

    #[test]
    fn status_options_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitStatusOptionFlags>(),
            size_of::<ffi::git_status_opt_t>()
        );
        assert_eq!(
            align_of::<GitStatusOptionFlags>(),
            align_of::<ffi::git_status_opt_t>()
        );
    }
}

/// Wraps: git_status_options
/// Layout-compatible status options whose baseline and pathspec borrow
/// caller-owned values for the options lifetime.
#[repr(transparent)]
pub struct GitStatusOptions<'data> {
    inner: CType<ffi::git_status_options>,
    _data: PhantomData<&'data ()>,
}

/// Shared borrow of [`GitStatusOptions`].
#[repr(transparent)]
pub struct GitStatusOptionsRef<'object, 'data>(CPtr<'object, GitStatusOptions<'data>>);

impl Clone for GitStatusOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitStatusOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitStatusOptions`].
#[repr(transparent)]
pub struct GitStatusOptionsMut<'object, 'data>(GitStatusOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and never form references to C-visible
// storage, and the shared handle exposes no writes.
unsafe impl<'data> CCell for GitStatusOptions<'data> {
    type C = ffi::git_status_options;
    type Ref<'object>
        = GitStatusOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitStatusOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(p: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `p` is a live shared object.
        GitStatusOptionsRef(unsafe { CPtr::new(p) })
    }

    unsafe fn mut_from_raw<'object>(p: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitStatusOptionsMut(GitStatusOptionsRef(unsafe { CPtr::new(p) }))
    }
}

// SAFETY: this header only borrows its pointer fields and owns no resource;
// disposing inline storage therefore requires no action.
unsafe impl CValued for GitStatusOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitStatusOptions<'data> {
    /// Constructs options equivalent to `GIT_STATUS_OPTIONS_INIT`.
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
            .set_version(ffi::GIT_STATUS_OPTIONS_VERSION);
        options
    }
}

impl<'object, 'data> GitStatusOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify a valid initialized options value that lives for
    /// `'object`. Its baseline and every pathspec allocation must remain valid
    /// for `'data`, and `'data` must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_status_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitStatusOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_status_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_status_options.flags
    /// Returns the checked set of configured status options.
    pub fn flags(&self) -> Result<GitStatusOptionFlags, ffi::git_status_opt_t> {
        // SAFETY: raw-place projection copies the initialized scalar field.
        let flags = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitStatusOptionFlags::try_from(flags)
    }

    /// Field: git_status_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_status_options.baseline
    /// Borrows the optional tree used instead of `HEAD`.
    #[must_use]
    pub fn baseline(&self) -> Option<GitTreeRef<'object>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let baseline = unsafe { addr_of!((*self.as_ptr()).baseline).read() };
        // SAFETY: safe construction stores only a live tree whose data borrow
        // outlives this options borrow; null remains `None`.
        unsafe { GitTreeRef::from_ptr(baseline) }
    }

    /// Field: git_status_options.rename_threshold
    /// Returns the configured rename-similarity threshold.
    #[must_use]
    pub fn rename_threshold(&self) -> u16 {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).rename_threshold).read() }
    }

    /// Field: git_status_options.pathspec
    /// Borrows the inline pathspec-array header.
    #[must_use]
    pub fn pathspec(&self) -> GitStrArrayRef<'object> {
        // SAFETY: this projects the initialized inline header without forming
        // a reference to C-visible storage.
        let pathspec = unsafe { addr_of!((*self.as_ptr()).pathspec).cast_mut() };
        // SAFETY: the projected header lives for this options borrow.
        unsafe { GitStrArrayRef::from_ptr(pathspec) }.expect("an inline field is non-null")
    }

    /// Field: git_status_options.show
    /// Returns the selected comparison mode, rejecting unpublished values.
    pub fn show(&self) -> Result<StatusShow, InvalidStatusShow> {
        // SAFETY: raw-place projection copies the initialized scalar field.
        let show = unsafe { addr_of!((*self.as_ptr()).show).read() };
        StatusShow::try_from(show)
    }
}

impl<'object, 'data> GitStatusOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path to the
    /// options value may be used for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_status_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitStatusOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_status_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitStatusOptionsRef<'_, 'data> {
        GitStatusOptionsRef(self.0.0)
    }

    /// Replaces the configured status-option flags.
    pub fn set_flags(&mut self, flags: GitStatusOptionFlags) {
        // SAFETY: this exclusive handle permits the scalar write, and the
        // wrapper contains only published flag bits.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Stores an optional borrowed baseline tree.
    pub fn set_baseline(&mut self, baseline: Option<GitTreeRef<'data>>) {
        let baseline = baseline.map_or(core::ptr::null(), |value| value.as_ptr());
        // SAFETY: this exclusive handle permits the pointer-field write, and
        // the wrapper lifetime keeps any non-null tree alive.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).baseline).write(baseline.cast_mut()) }
    }

    /// Replaces the rename-similarity threshold.
    pub fn set_rename_threshold(&mut self, threshold: u16) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).rename_threshold).write(threshold) }
    }

    /// Borrows a pathspec-array header and copies its non-owning C view.
    pub fn set_pathspec(&mut self, pathspec: GitStrArrayRef<'data>) {
        // SAFETY: `pathspec` identifies a live initialized header. Copying the
        // C header transfers no ownership; the wrapper lifetime retains the
        // source allocation while these options may be used.
        let pathspec = unsafe { pathspec.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline header.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).pathspec).write(pathspec) }
    }

    /// Selects which comparisons the status scan performs.
    pub fn set_show(&mut self, show: StatusShow) {
        // SAFETY: this exclusive handle permits the scalar write, and `show`
        // is one of the published C enum values.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).show).write(show.into()) }
    }
}

#[cfg(test)]
mod options_tests {
    use core::mem::{align_of, size_of};

    use crate::strarray::GitStrArrayRef;
    use crate::tree::{GitTree, GitTreeRef};

    use super::*;

    #[test]
    fn options_preserve_layout_and_borrowed_fields() {
        assert_eq!(
            size_of::<GitStatusOptions<'static>>(),
            size_of::<ffi::git_status_options>()
        );
        assert_eq!(
            align_of::<GitStatusOptions<'static>>(),
            align_of::<ffi::git_status_options>()
        );
        assert_eq!(
            size_of::<GitStatusOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_status_options>()
        );

        let mut tree = GitTree::zeroed();
        let tree_ptr = addr_of_mut!(tree).cast::<ffi::git_tree>();
        // SAFETY: the opaque layout-compatible stack value remains live while
        // the options borrow it; no tree operation is called in this test.
        let tree = unsafe { GitTreeRef::from_ptr(tree_ptr) }.unwrap();

        let mut entries = [c"src/*.c".as_ptr().cast_mut()];
        let mut pathspec = ffi::git_strarray {
            strings: entries.as_mut_ptr(),
            count: entries.len(),
        };
        // SAFETY: the stack header, pointer slot and static string all outlive
        // the options value and are accessed shared-only.
        let pathspec = unsafe { GitStrArrayRef::from_ptr(addr_of_mut!(pathspec)) }.unwrap();

        let flags =
            GitStatusOptionFlags::INCLUDE_UNTRACKED | GitStatusOptionFlags::RECURSE_UNTRACKED_DIRS;
        let mut options = GitStatusOptions::new();
        {
            let mut view = options.as_mut();
            view.set_flags(flags);
            view.set_baseline(Some(tree));
            view.set_rename_threshold(75);
            view.set_pathspec(pathspec);
            view.set_show(StatusShow::IndexOnly);
        }

        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_STATUS_OPTIONS_VERSION);
        assert_eq!(view.flags(), Ok(flags));
        assert_eq!(view.baseline().unwrap().as_ptr(), tree_ptr);
        assert_eq!(view.rename_threshold(), 75);
        assert_eq!(view.pathspec().count(), 1);
        assert_eq!(view.pathspec().strings().unwrap().get(0), Some(c"src/*.c"));
        assert_eq!(view.show(), Ok(StatusShow::IndexOnly));
    }

    #[test]
    fn getters_reject_invalid_status_values() {
        let mut raw = ffi::git_status_options {
            version: ffi::GIT_STATUS_OPTIONS_VERSION,
            show: ffi::git_status_show_t_GIT_STATUS_SHOW_WORKDIR_ONLY + 1,
            flags: GitStatusOptionFlags::ALL.bits() + 1,
            pathspec: ffi::git_strarray {
                strings: core::ptr::null_mut(),
                count: 0,
            },
            baseline: core::ptr::null_mut(),
            rename_threshold: 0,
        };
        // SAFETY: every field is initialized, all pointer fields are null, and
        // `raw` remains live and unmodified for the shared handle.
        let options = unsafe { GitStatusOptionsRef::from_ptr(addr_of_mut!(raw)) }.unwrap();
        assert_eq!(options.flags(), Err(GitStatusOptionFlags::ALL.bits() + 1));
        assert_eq!(
            options.show().unwrap_err().value(),
            ffi::git_status_show_t_GIT_STATUS_SHOW_WORKDIR_ONLY + 1
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_status_entry
    /// A status-list-owned entry borrowing its optional deltas from the
    /// enclosing list's two diff objects.
    GitStatusEntry,
    GitStatusEntryRef,
    GitStatusEntryMut,
    ffi::git_status_entry
);

impl<'a> GitStatusEntryRef<'a> {
    /// Field: git_status_entry.status
    /// Returns all status bits, retaining flags introduced by newer libgit2
    /// versions.
    #[must_use]
    pub fn status(&self) -> crate::status::Status {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let status = unsafe { addr_of!((*self.as_ptr()).status).read() };
        crate::status::Status::from_bits_retain(status)
    }

    /// Field: git_status_entry.index_to_workdir
    /// Borrows the optional index-to-working-directory delta.
    #[must_use]
    pub fn index_to_workdir(&self) -> Option<crate::api::diff::DiffDeltaRef<'a>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let delta = unsafe { addr_of!((*self.as_ptr()).index_to_workdir).read() };
        // SAFETY: the enclosing status list owns the referenced diff and a
        // status-entry borrow cannot outlive that list; null remains `None`.
        unsafe { crate::api::diff::DiffDeltaRef::from_ptr(delta) }
    }

    /// Field: git_status_entry.head_to_index
    /// Borrows the optional `HEAD`-to-index delta.
    #[must_use]
    pub fn head_to_index(&self) -> Option<crate::api::diff::DiffDeltaRef<'a>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let delta = unsafe { addr_of!((*self.as_ptr()).head_to_index).read() };
        // SAFETY: the enclosing status list owns the referenced diff and a
        // status-entry borrow cannot outlive that list; null remains `None`.
        unsafe { crate::api::diff::DiffDeltaRef::from_ptr(delta) }
    }
}

#[cfg(test)]
mod entry_tests {
    use core::mem::{align_of, size_of};

    use ffibox::CCell;

    use super::*;

    #[test]
    fn entry_wrapper_preserves_layout_and_borrows_deltas() {
        fn assert_cell<T: CCell>() {}
        assert_cell::<GitStatusEntry>();
        assert_eq!(
            size_of::<GitStatusEntry>(),
            size_of::<ffi::git_status_entry>()
        );
        assert_eq!(
            align_of::<GitStatusEntry>(),
            align_of::<ffi::git_status_entry>()
        );
        assert_eq!(
            size_of::<GitStatusEntryRef<'_>>(),
            size_of::<*const ffi::git_status_entry>()
        );

        // SAFETY: all-zero is a valid delta with UNMODIFIED status, absent
        // optional paths and zero-valued scalar fields.
        let mut head_delta: ffi::git_diff_delta = unsafe { core::mem::zeroed() };
        head_delta.status = ffi::git_delta_t_GIT_DELTA_MODIFIED;
        let unknown_status = crate::status::Status::ALL.bits() | (1 << 5);
        let mut raw = ffi::git_status_entry {
            status: unknown_status,
            head_to_index: &raw mut head_delta,
            index_to_workdir: core::ptr::null_mut(),
        };
        // SAFETY: the entry and pointed-to delta remain live and unmodified
        // while this shared handle and its derived delta handle are used.
        let entry = unsafe { GitStatusEntryRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(entry.status().bits(), unknown_status);
        assert_eq!(
            entry.head_to_index().unwrap().status(),
            Ok(crate::diff::Delta::Modified)
        );
        assert!(entry.index_to_workdir().is_none());
    }
}
