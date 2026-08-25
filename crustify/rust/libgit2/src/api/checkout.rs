//! Safe wrappers for libgit2 checkout APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_checkout_strategy_t
/// Checkout behavior selected for a libgit2 operation.
///
/// This is a layout-compatible bit set. Raw values retain unknown bits so
/// options from a newer libgit2 version can still be carried safely.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitCheckoutStrategy(ffi::git_checkout_strategy_t);

impl GitCheckoutStrategy {
    /// Apply only updates that do not overwrite uncommitted data.
    pub const SAFE: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_SAFE);
    /// Force the work directory to match the index, potentially losing data.
    pub const FORCE: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_FORCE);
    /// Recreate files that are missing from the work directory.
    pub const RECREATE_MISSING: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_RECREATE_MISSING);
    /// Apply safe updates even when conflicts exist.
    pub const ALLOW_CONFLICTS: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_ALLOW_CONFLICTS);
    /// Remove untracked, non-ignored files.
    pub const REMOVE_UNTRACKED: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_REMOVE_UNTRACKED);
    /// Remove ignored files.
    pub const REMOVE_IGNORED: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_REMOVE_IGNORED);
    /// Update existing files without creating or deleting files.
    pub const UPDATE_ONLY: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_UPDATE_ONLY);
    /// Do not update index entries while checking out files.
    pub const DONT_UPDATE_INDEX: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DONT_UPDATE_INDEX);
    /// Do not refresh the index, configuration, or attributes first.
    pub const NO_REFRESH: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_NO_REFRESH);
    /// Skip files with unmerged index entries.
    pub const SKIP_UNMERGED: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_SKIP_UNMERGED);
    /// Resolve unmerged files from the index's "ours" stage.
    pub const USE_OURS: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_USE_OURS);
    /// Resolve unmerged files from the index's "theirs" stage.
    pub const USE_THEIRS: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_USE_THEIRS);
    /// Treat pathspec entries as exact paths instead of patterns.
    pub const DISABLE_PATHSPEC_MATCH: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DISABLE_PATHSPEC_MATCH);
    /// Recursively checkout submodules (not implemented by this libgit2).
    pub const UPDATE_SUBMODULES: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_UPDATE_SUBMODULES);
    /// Recursively checkout changed submodules (not implemented by this libgit2).
    pub const UPDATE_SUBMODULES_IF_CHANGED: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_UPDATE_SUBMODULES_IF_CHANGED);
    /// Leave locked directories empty instead of failing.
    pub const SKIP_LOCKED_DIRECTORIES: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_SKIP_LOCKED_DIRECTORIES);
    /// Do not overwrite ignored files that exist in the checkout target.
    pub const DONT_OVERWRITE_IGNORED: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DONT_OVERWRITE_IGNORED);
    /// Write ordinary merge-style conflict files.
    pub const CONFLICT_STYLE_MERGE: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_CONFLICT_STYLE_MERGE);
    /// Write diff3-style conflict files with common-ancestor data.
    pub const CONFLICT_STYLE_DIFF3: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_CONFLICT_STYLE_DIFF3);
    /// Do not remove existing paths that collide on case-insensitive filesystems.
    pub const DONT_REMOVE_EXISTING: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DONT_REMOVE_EXISTING);
    /// Do not write the index when checkout completes.
    pub const DONT_WRITE_INDEX: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DONT_WRITE_INDEX);
    /// Report the actions checkout would take without changing files or index.
    pub const DRY_RUN: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_DRY_RUN);
    /// Write zdiff3-style conflict files with common-ancestor data.
    pub const CONFLICT_STYLE_ZDIFF3: Self =
        Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_CONFLICT_STYLE_ZDIFF3);
    /// Suppress checkout and checkout callbacks completely.
    pub const NONE: Self = Self(ffi::git_checkout_strategy_t_GIT_CHECKOUT_NONE);
    /// Every strategy bit published by this libgit2 API.
    pub const ALL: Self = Self(
        Self::FORCE.0
            | Self::RECREATE_MISSING.0
            | Self::ALLOW_CONFLICTS.0
            | Self::REMOVE_UNTRACKED.0
            | Self::REMOVE_IGNORED.0
            | Self::UPDATE_ONLY.0
            | Self::DONT_UPDATE_INDEX.0
            | Self::NO_REFRESH.0
            | Self::SKIP_UNMERGED.0
            | Self::USE_OURS.0
            | Self::USE_THEIRS.0
            | Self::DISABLE_PATHSPEC_MATCH.0
            | Self::UPDATE_SUBMODULES.0
            | Self::UPDATE_SUBMODULES_IF_CHANGED.0
            | Self::SKIP_LOCKED_DIRECTORIES.0
            | Self::DONT_OVERWRITE_IGNORED.0
            | Self::CONFLICT_STYLE_MERGE.0
            | Self::CONFLICT_STYLE_DIFF3.0
            | Self::DONT_REMOVE_EXISTING.0
            | Self::DONT_WRITE_INDEX.0
            | Self::DRY_RUN.0
            | Self::CONFLICT_STYLE_ZDIFF3.0
            | Self::NONE.0,
    );

    /// Retain all bits from a raw libgit2 value, including unknown bits.
    #[inline]
    #[must_use]
    pub const fn from_bits_retain(bits: ffi::git_checkout_strategy_t) -> Self {
        Self(bits)
    }

    /// Return the raw libgit2 strategy bits.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> ffi::git_checkout_strategy_t {
        self.0
    }

    /// Return whether every strategy in `other` is selected.
    #[inline]
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return whether at least one strategy in `other` is selected.
    #[inline]
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Return whether this is the default safe strategy with no modifier bits.
    #[inline]
    #[must_use]
    pub const fn is_safe(self) -> bool {
        self.0 == Self::SAFE.0
    }
}

impl From<ffi::git_checkout_strategy_t> for GitCheckoutStrategy {
    fn from(bits: ffi::git_checkout_strategy_t) -> Self {
        Self::from_bits_retain(bits)
    }
}

impl From<GitCheckoutStrategy> for ffi::git_checkout_strategy_t {
    fn from(strategy: GitCheckoutStrategy) -> Self {
        strategy.bits()
    }
}

impl BitOr for GitCheckoutStrategy {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitCheckoutStrategy {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitCheckoutStrategy {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitCheckoutStrategy {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitCheckoutStrategy {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn checkout_strategies_combine_and_clear() {
        let mut strategy = GitCheckoutStrategy::FORCE | GitCheckoutStrategy::REMOVE_UNTRACKED;
        assert!(strategy.contains(GitCheckoutStrategy::FORCE));
        assert!(strategy.intersects(GitCheckoutStrategy::REMOVE_UNTRACKED));
        assert!(!strategy.is_safe());

        strategy &= !GitCheckoutStrategy::FORCE;
        assert!(!strategy.contains(GitCheckoutStrategy::FORCE));
        assert!(strategy.contains(GitCheckoutStrategy::REMOVE_UNTRACKED));
        assert!(GitCheckoutStrategy::SAFE.is_safe());
    }

    #[test]
    fn checkout_strategies_retain_raw_bits_and_match_the_c_layout() {
        let unknown = GitCheckoutStrategy::from_bits_retain(1 << 29);
        assert_eq!(unknown.bits(), 1 << 29);
        assert_eq!(ffi::git_checkout_strategy_t::from(unknown), 1 << 29);
        assert_eq!(
            size_of::<GitCheckoutStrategy>(),
            size_of::<ffi::git_checkout_strategy_t>()
        );
        assert_eq!(
            align_of::<GitCheckoutStrategy>(),
            align_of::<ffi::git_checkout_strategy_t>()
        );
    }

    #[test]
    fn all_published_checkout_bits_are_accounted_for() {
        for strategy in [
            GitCheckoutStrategy::FORCE,
            GitCheckoutStrategy::RECREATE_MISSING,
            GitCheckoutStrategy::ALLOW_CONFLICTS,
            GitCheckoutStrategy::REMOVE_UNTRACKED,
            GitCheckoutStrategy::REMOVE_IGNORED,
            GitCheckoutStrategy::UPDATE_ONLY,
            GitCheckoutStrategy::DONT_UPDATE_INDEX,
            GitCheckoutStrategy::NO_REFRESH,
            GitCheckoutStrategy::SKIP_UNMERGED,
            GitCheckoutStrategy::USE_OURS,
            GitCheckoutStrategy::USE_THEIRS,
            GitCheckoutStrategy::DISABLE_PATHSPEC_MATCH,
            GitCheckoutStrategy::UPDATE_SUBMODULES,
            GitCheckoutStrategy::UPDATE_SUBMODULES_IF_CHANGED,
            GitCheckoutStrategy::SKIP_LOCKED_DIRECTORIES,
            GitCheckoutStrategy::DONT_OVERWRITE_IGNORED,
            GitCheckoutStrategy::CONFLICT_STYLE_MERGE,
            GitCheckoutStrategy::CONFLICT_STYLE_DIFF3,
            GitCheckoutStrategy::DONT_REMOVE_EXISTING,
            GitCheckoutStrategy::DONT_WRITE_INDEX,
            GitCheckoutStrategy::DRY_RUN,
            GitCheckoutStrategy::CONFLICT_STYLE_ZDIFF3,
            GitCheckoutStrategy::NONE,
        ] {
            assert!(GitCheckoutStrategy::ALL.contains(strategy));
        }
    }
}

/// Wraps: git_checkout_notify_cb
/// Safe callable surface for checkout notifications.
pub trait GitCheckoutNotifyCallback {
    /// Handles one checkout notification and returns zero to continue.
    fn notify(
        &mut self,
        why: crate::checkout::CheckoutNotify,
        path: Option<&core::ffi::CStr>,
        baseline: Option<crate::diff::DiffFileRef<'_>>,
        target: Option<crate::diff::DiffFileRef<'_>>,
        workdir: Option<crate::diff::DiffFileRef<'_>>,
    ) -> i32;
}

impl<F> GitCheckoutNotifyCallback for F
where
    F: FnMut(
        crate::checkout::CheckoutNotify,
        Option<&core::ffi::CStr>,
        Option<crate::diff::DiffFileRef<'_>>,
        Option<crate::diff::DiffFileRef<'_>>,
        Option<crate::diff::DiffFileRef<'_>>,
    ) -> i32,
{
    fn notify(
        &mut self,
        why: crate::checkout::CheckoutNotify,
        path: Option<&core::ffi::CStr>,
        baseline: Option<crate::diff::DiffFileRef<'_>>,
        target: Option<crate::diff::DiffFileRef<'_>>,
        workdir: Option<crate::diff::DiffFileRef<'_>>,
    ) -> i32 {
        self(why, path, baseline, target, workdir)
    }
}

#[cfg(test)]
mod callback_surface_tests {
    use super::*;

    #[test]
    fn closures_implement_checkout_notifications() {
        fn accepts<C: GitCheckoutNotifyCallback>(_callback: C) {}
        accepts(
            |_: crate::checkout::CheckoutNotify,
             _: Option<&core::ffi::CStr>,
             _: Option<crate::diff::DiffFileRef<'_>>,
             _: Option<crate::diff::DiffFileRef<'_>>,
             _: Option<crate::diff::DiffFileRef<'_>>| 0,
        );
    }
}

/// Wraps: git_checkout_options
/// Layout-compatible checkout options whose configured pointers borrow
/// caller-owned values for the options lifetime.
#[repr(transparent)]
pub struct GitCheckoutOptions<'data> {
    inner: ffibox::CType<ffi::git_checkout_options>,
    _data: core::marker::PhantomData<&'data ()>,
}

/// Shared borrow of [`GitCheckoutOptions`].
#[repr(transparent)]
pub struct GitCheckoutOptionsRef<'object, 'data>(ffibox::CPtr<'object, GitCheckoutOptions<'data>>);

impl Clone for GitCheckoutOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitCheckoutOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitCheckoutOptions`].
#[repr(transparent)]
pub struct GitCheckoutOptionsMut<'object, 'data>(GitCheckoutOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and never form references to C-visible
// storage, and the shared handle exposes no writes.
unsafe impl<'data> ffibox::CCell for GitCheckoutOptions<'data> {
    type C = ffi::git_checkout_options;
    type Ref<'object>
        = GitCheckoutOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitCheckoutOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: core::ptr::NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitCheckoutOptionsRef(unsafe { ffibox::CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: core::ptr::NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitCheckoutOptionsMut(GitCheckoutOptionsRef(unsafe { ffibox::CPtr::new(ptr) }))
    }
}

// SAFETY: this options header only borrows its pointer fields and owns no
// resource, so disposing inline storage requires no action.
unsafe impl ffibox::CValued for GitCheckoutOptions<'_> {
    unsafe fn c_dispose(_this: core::ptr::NonNull<Self>) {}
}

impl<'data> GitCheckoutOptions<'data> {
    /// Constructs options equivalent to `GIT_CHECKOUT_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> ffibox::CVal<Self> {
        // SAFETY: every field in the bindgen C struct admits an all-zero bit
        // pattern; the required ABI version is installed before return.
        let inner = unsafe { ffibox::CType::zeroed() };
        let mut options = ffibox::CVal::new(Self {
            inner,
            _data: core::marker::PhantomData,
        });
        options
            .as_mut()
            .set_version(ffi::GIT_CHECKOUT_OPTIONS_VERSION);
        options
    }
}

impl<'object, 'data> GitCheckoutOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify a valid initialized options value that lives for
    /// `'object`. Every configured object, string, path array, and callback
    /// payload must remain valid for `'data`, and `'data` must outlive
    /// `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_checkout_options) -> Option<Self> {
        core::ptr::NonNull::new(ptr.cast::<GitCheckoutOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { ffibox::CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_checkout_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_checkout_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_checkout_options.paths
    /// Borrows the inline path-array header.
    #[must_use]
    pub fn paths(&self) -> crate::strarray::GitStrArrayRef<'object> {
        // SAFETY: this projects the initialized inline header without forming
        // a reference to C-visible storage.
        let paths = unsafe { core::ptr::addr_of!((*self.as_ptr()).paths).cast_mut() };
        // SAFETY: the projected header lives for this options borrow.
        unsafe { crate::strarray::GitStrArrayRef::from_ptr(paths) }
            .expect("an inline field is non-null")
    }

    /// Field: git_checkout_options.progress_cb
    /// Returns whether a progress callback is installed.
    #[must_use]
    pub fn has_progress_callback(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized callback slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).progress_cb).read() }.is_some()
    }

    /// Field: git_checkout_options.file_mode
    /// Returns the requested file mode, or zero for libgit2's default.
    #[must_use]
    pub fn file_mode(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).file_mode).read() }
    }

    /// Field: git_checkout_options.checkout_strategy
    /// Returns the configured checkout strategy, retaining unknown bits.
    #[must_use]
    pub fn checkout_strategy(&self) -> GitCheckoutStrategy {
        // SAFETY: raw-place projection copies the initialized scalar field.
        let bits = unsafe { core::ptr::addr_of!((*self.as_ptr()).checkout_strategy).read() };
        GitCheckoutStrategy::from_bits_retain(bits)
    }

    /// Field: git_checkout_options.baseline
    /// Borrows the optional tree used as the expected worktree content.
    #[must_use]
    pub fn baseline(&self) -> Option<crate::tree::GitTreeRef<'object>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let baseline = unsafe { core::ptr::addr_of!((*self.as_ptr()).baseline).read() };
        // SAFETY: the options contract keeps a non-null tree live for this borrow.
        unsafe { crate::tree::GitTreeRef::from_ptr(baseline) }
    }

    /// Field: git_checkout_options.notify_cb
    /// Returns whether a checkout notification callback is installed.
    #[must_use]
    pub fn has_notify_callback(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized callback slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).notify_cb).read() }.is_some()
    }

    /// Field: git_checkout_options.their_label
    /// Borrows the optional label for the other side of conflicts.
    #[must_use]
    pub fn their_label(&self) -> Option<&'object core::ffi::CStr> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let label = unsafe { core::ptr::addr_of!((*self.as_ptr()).their_label).read() };
        // SAFETY: the valid options contract keeps a non-null NUL-terminated
        // string live for this shared options borrow.
        unsafe { c_string(label) }
    }

    /// Field: git_checkout_options.progress_payload
    /// Returns whether opaque progress-callback state is installed.
    #[must_use]
    pub fn has_progress_payload(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized opaque pointer.
        !unsafe { core::ptr::addr_of!((*self.as_ptr()).progress_payload).read() }.is_null()
    }

    /// Field: git_checkout_options.dir_mode
    /// Returns the requested directory mode, or zero for libgit2's default.
    #[must_use]
    pub fn dir_mode(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).dir_mode).read() }
    }

    /// Field: git_checkout_options.our_label
    /// Borrows the optional label for our side of conflicts.
    #[must_use]
    pub fn our_label(&self) -> Option<&'object core::ffi::CStr> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let label = unsafe { core::ptr::addr_of!((*self.as_ptr()).our_label).read() };
        // SAFETY: the valid options contract keeps a non-null NUL-terminated
        // string live for this shared options borrow.
        unsafe { c_string(label) }
    }

    /// Field: git_checkout_options.perfdata_cb
    /// Returns whether a performance-data callback is installed.
    #[must_use]
    pub fn has_perfdata_callback(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized callback slot.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).perfdata_cb).read() }.is_some()
    }

    /// Field: git_checkout_options.target_directory
    /// Borrows the optional alternate checkout directory.
    #[must_use]
    pub fn target_directory(&self) -> Option<&'object core::ffi::CStr> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let directory = unsafe { core::ptr::addr_of!((*self.as_ptr()).target_directory).read() };
        // SAFETY: the valid options contract keeps a non-null NUL-terminated
        // string live for this shared options borrow.
        unsafe { c_string(directory) }
    }

    /// Field: git_checkout_options.perfdata_payload
    /// Returns whether opaque performance-callback state is installed.
    #[must_use]
    pub fn has_perfdata_payload(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized opaque pointer.
        !unsafe { core::ptr::addr_of!((*self.as_ptr()).perfdata_payload).read() }.is_null()
    }

    /// Field: git_checkout_options.ancestor_label
    /// Borrows the optional label for the common ancestor of conflicts.
    #[must_use]
    pub fn ancestor_label(&self) -> Option<&'object core::ffi::CStr> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let label = unsafe { core::ptr::addr_of!((*self.as_ptr()).ancestor_label).read() };
        // SAFETY: the valid options contract keeps a non-null NUL-terminated
        // string live for this shared options borrow.
        unsafe { c_string(label) }
    }

    /// Field: git_checkout_options.baseline_index
    /// Borrows the optional index that overrides the tree baseline.
    #[must_use]
    pub fn baseline_index(&self) -> Option<crate::index::GitIndexRef<'object>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let index = unsafe { core::ptr::addr_of!((*self.as_ptr()).baseline_index).read() };
        // SAFETY: the options contract keeps a non-null index live for this borrow.
        unsafe { crate::index::GitIndexRef::from_ptr(index) }
    }

    /// Field: git_checkout_options.notify_payload
    /// Returns whether opaque notification-callback state is installed.
    #[must_use]
    pub fn has_notify_payload(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized opaque pointer.
        !unsafe { core::ptr::addr_of!((*self.as_ptr()).notify_payload).read() }.is_null()
    }

    /// Field: git_checkout_options.notify_flags
    /// Returns the notification classes selected for the callback.
    #[must_use]
    pub fn notify_flags(&self) -> crate::checkout::CheckoutNotify {
        // SAFETY: raw-place projection copies the initialized scalar field.
        let bits = unsafe { core::ptr::addr_of!((*self.as_ptr()).notify_flags).read() };
        crate::checkout::CheckoutNotify::from_bits_retain(bits)
    }

    /// Field: git_checkout_options.file_open_flags
    /// Returns the platform file-open flags, or zero for libgit2's default.
    #[must_use]
    pub fn file_open_flags(&self) -> core::ffi::c_int {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).file_open_flags).read() }
    }

    /// Field: git_checkout_options.disable_filters
    /// Returns whether content filters are disabled.
    #[must_use]
    pub fn disable_filters(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { core::ptr::addr_of!((*self.as_ptr()).disable_filters).read() != 0 }
    }
}

unsafe fn c_string<'a>(ptr: *const core::ffi::c_char) -> Option<&'a core::ffi::CStr> {
    if ptr.is_null() {
        None
    } else {
        // SAFETY: callers only pass configured options fields whose validity
        // and NUL termination are guaranteed for the returned borrow.
        Some(unsafe { core::ffi::CStr::from_ptr(ptr) })
    }
}

impl<'object, 'data> GitCheckoutOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path to the
    /// options value may be used for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_checkout_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitCheckoutOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_checkout_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitCheckoutOptionsRef<'_, 'data> {
        GitCheckoutOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Borrows a path-array header and copies its non-owning C view.
    pub fn set_paths(&mut self, paths: crate::strarray::GitStrArrayRef<'data>) {
        // SAFETY: `paths` identifies a live initialized header. Copying the C
        // header transfers no ownership and the wrapper retains the borrow.
        let paths = unsafe { paths.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline header.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).paths).write(paths) }
    }

    /// Replaces the requested file mode.
    pub fn set_file_mode(&mut self, mode: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).file_mode).write(mode) }
    }

    /// Replaces the checkout strategy.
    pub fn set_checkout_strategy(&mut self, strategy: GitCheckoutStrategy) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe {
            core::ptr::addr_of_mut!((*self.as_mut_ptr()).checkout_strategy).write(strategy.bits())
        }
    }

    /// Stores an optional borrowed baseline tree.
    pub fn set_baseline(&mut self, baseline: Option<crate::tree::GitTreeRef<'data>>) {
        let baseline = baseline.map_or(core::ptr::null(), |tree| tree.as_ptr());
        // SAFETY: this exclusive handle permits the pointer write and the
        // wrapper lifetime keeps a non-null tree live.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).baseline).write(baseline.cast_mut()) }
    }

    /// Replaces the requested directory mode.
    pub fn set_dir_mode(&mut self, mode: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).dir_mode).write(mode) }
    }

    /// Stores the optional alternate checkout directory.
    pub fn set_target_directory(&mut self, directory: Option<&'data core::ffi::CStr>) {
        let directory = directory.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write and the
        // wrapper lifetime keeps a non-null string live.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).target_directory).write(directory) }
    }

    /// Stores the optional common-ancestor conflict label.
    pub fn set_ancestor_label(&mut self, label: Option<&'data core::ffi::CStr>) {
        let label = label.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write and the
        // wrapper lifetime keeps a non-null string live.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).ancestor_label).write(label) }
    }

    /// Stores the optional label for our side of conflicts.
    pub fn set_our_label(&mut self, label: Option<&'data core::ffi::CStr>) {
        let label = label.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write and the
        // wrapper lifetime keeps a non-null string live.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).our_label).write(label) }
    }

    /// Stores the optional label for the other side of conflicts.
    pub fn set_their_label(&mut self, label: Option<&'data core::ffi::CStr>) {
        let label = label.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write and the
        // wrapper lifetime keeps a non-null string live.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).their_label).write(label) }
    }

    /// Stores an optional borrowed baseline index.
    pub fn set_baseline_index(&mut self, index: Option<crate::index::GitIndexRef<'data>>) {
        let index = index.map_or(core::ptr::null(), |index| index.as_ptr());
        // SAFETY: this exclusive handle permits the pointer write and the
        // wrapper lifetime keeps a non-null index live.
        unsafe {
            core::ptr::addr_of_mut!((*self.as_mut_ptr()).baseline_index).write(index.cast_mut())
        }
    }

    /// Replaces the notification classes selected for the callback.
    pub fn set_notify_flags(&mut self, flags: crate::checkout::CheckoutNotify) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).notify_flags).write(flags.bits()) }
    }

    /// Replaces the platform file-open flags.
    pub fn set_file_open_flags(&mut self, flags: core::ffi::c_int) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { core::ptr::addr_of_mut!((*self.as_mut_ptr()).file_open_flags).write(flags) }
    }

    /// Selects whether content filters are disabled.
    pub fn set_disable_filters(&mut self, disabled: bool) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe {
            core::ptr::addr_of_mut!((*self.as_mut_ptr()).disable_filters)
                .write(core::ffi::c_int::from(disabled))
        }
    }

    /// Installs a typed Rust checkout notification callback and payload.
    ///
    /// # Safety
    ///
    /// `callback` must remain live and exclusively reserved for every use of
    /// these options, including uses of any libgit2 object that copies them.
    /// Calls must not overlap unless the callback synchronizes its own state.
    pub unsafe fn set_notify_callback<C: GitCheckoutNotifyCallback>(&mut self, callback: &mut C) {
        unsafe extern "C" fn trampoline<C: GitCheckoutNotifyCallback>(
            why: ffi::git_checkout_notify_t,
            path: *const core::ffi::c_char,
            baseline: *const ffi::git_diff_file,
            target: *const ffi::git_diff_file,
            workdir: *const ffi::git_diff_file,
            payload: *mut core::ffi::c_void,
        ) -> core::ffi::c_int {
            let path = if path.is_null() {
                None
            } else {
                // SAFETY: libgit2 supplies a transient NUL-terminated path.
                Some(unsafe { core::ffi::CStr::from_ptr(path) })
            };
            // SAFETY: libgit2 supplies live transient records or null pointers.
            let baseline = unsafe { crate::diff::DiffFileRef::from_ptr(baseline.cast_mut()) };
            // SAFETY: libgit2 supplies live transient records or null pointers.
            let target = unsafe { crate::diff::DiffFileRef::from_ptr(target.cast_mut()) };
            // SAFETY: libgit2 supplies live transient records or null pointers.
            let workdir = unsafe { crate::diff::DiffFileRef::from_ptr(workdir.cast_mut()) };
            // SAFETY: the installation contract keeps the callback live and exclusive.
            let Some(callback) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return -1;
            };
            callback.notify(
                crate::checkout::CheckoutNotify::from_bits_retain(why),
                path,
                baseline,
                target,
                workdir,
            )
        }

        let options = self.as_mut_ptr();
        // SAFETY: the caller guarantees payload validity and this exclusive
        // handle permits installing both halves of the callback pair.
        unsafe {
            core::ptr::addr_of_mut!((*options).notify_cb).write(Some(trampoline::<C>));
            core::ptr::addr_of_mut!((*options).notify_payload)
                .write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears the notification callback and its payload together.
    pub fn clear_notify_callback(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both field writes.
        unsafe {
            core::ptr::addr_of_mut!((*options).notify_cb).write(None);
            core::ptr::addr_of_mut!((*options).notify_payload).write(core::ptr::null_mut());
        }
    }

    /// Installs a typed Rust checkout progress callback and payload.
    ///
    /// # Safety
    ///
    /// `callback` must remain live and exclusively reserved for every use of
    /// these options. Calls must not overlap unless it synchronizes its state.
    pub unsafe fn set_progress_callback<C: crate::checkout::GitCheckoutProgressCallback>(
        &mut self,
        callback: &mut C,
    ) {
        unsafe extern "C" fn trampoline<C: crate::checkout::GitCheckoutProgressCallback>(
            path: *const core::ffi::c_char,
            completed: usize,
            total: usize,
            payload: *mut core::ffi::c_void,
        ) {
            let path = if path.is_null() {
                None
            } else {
                // SAFETY: libgit2 supplies a transient NUL-terminated path.
                Some(unsafe { core::ffi::CStr::from_ptr(path) })
            };
            // SAFETY: the installation contract keeps the callback live and exclusive.
            let Some(callback) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return;
            };
            callback.call(path, completed, total);
        }

        let options = self.as_mut_ptr();
        // SAFETY: the caller guarantees payload validity and this exclusive
        // handle permits installing both halves of the callback pair.
        unsafe {
            core::ptr::addr_of_mut!((*options).progress_cb).write(Some(trampoline::<C>));
            core::ptr::addr_of_mut!((*options).progress_payload)
                .write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears the progress callback and its payload together.
    pub fn clear_progress_callback(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both field writes.
        unsafe {
            core::ptr::addr_of_mut!((*options).progress_cb).write(None);
            core::ptr::addr_of_mut!((*options).progress_payload).write(core::ptr::null_mut());
        }
    }

    /// Installs a typed Rust checkout performance callback and payload.
    ///
    /// # Safety
    ///
    /// `callback` must remain live and exclusively reserved for every use of
    /// these options. Calls must not overlap unless it synchronizes its state.
    pub unsafe fn set_perfdata_callback<C: crate::checkout::GitCheckoutPerfDataCallback>(
        &mut self,
        callback: &mut C,
    ) {
        unsafe extern "C" fn trampoline<C: crate::checkout::GitCheckoutPerfDataCallback>(
            perfdata: *const ffi::git_checkout_perfdata,
            payload: *mut core::ffi::c_void,
        ) {
            // SAFETY: libgit2 supplies a live transient performance record.
            let Some(perfdata) =
                (unsafe { crate::checkout::CheckoutPerfDataRef::from_ptr(perfdata.cast_mut()) })
            else {
                return;
            };
            // SAFETY: the installation contract keeps the callback live and exclusive.
            let Some(callback) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return;
            };
            callback.call(perfdata);
        }

        let options = self.as_mut_ptr();
        // SAFETY: the caller guarantees payload validity and this exclusive
        // handle permits installing both halves of the callback pair.
        unsafe {
            core::ptr::addr_of_mut!((*options).perfdata_cb).write(Some(trampoline::<C>));
            core::ptr::addr_of_mut!((*options).perfdata_payload)
                .write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears the performance callback and its payload together.
    pub fn clear_perfdata_callback(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both field writes.
        unsafe {
            core::ptr::addr_of_mut!((*options).perfdata_cb).write(None);
            core::ptr::addr_of_mut!((*options).perfdata_payload).write(core::ptr::null_mut());
        }
    }
}

#[cfg(test)]
mod checkout_options_tests {
    use core::mem::{align_of, size_of};
    use core::ptr::{addr_of, addr_of_mut};

    use ffibox::CCell;

    use super::*;

    #[test]
    fn checkout_options_preserve_layout_and_access_fields() {
        fn assert_cell<T: CCell>() {}
        assert_cell::<GitCheckoutOptions<'static>>();
        assert_eq!(
            size_of::<GitCheckoutOptions<'static>>(),
            size_of::<ffi::git_checkout_options>()
        );
        assert_eq!(
            align_of::<GitCheckoutOptions<'static>>(),
            align_of::<ffi::git_checkout_options>()
        );
        assert_eq!(
            size_of::<GitCheckoutOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_checkout_options>()
        );

        let mut entries = [c"src/lib.rs".as_ptr().cast_mut()];
        let mut paths = ffi::git_strarray {
            strings: entries.as_mut_ptr(),
            count: entries.len(),
        };
        // SAFETY: the stack header, pointer run, and static string remain live.
        let paths =
            unsafe { crate::strarray::GitStrArrayRef::from_ptr(addr_of_mut!(paths)) }.unwrap();

        let mut tree = crate::tree::GitTree::zeroed();
        // SAFETY: the opaque layout-compatible stack value stays live and no
        // tree operation inspects it in this test.
        let tree = unsafe { crate::tree::GitTreeRef::from_ptr(addr_of_mut!(tree).cast()) }.unwrap();
        let tree_ptr = tree.as_ptr();

        let mut index = crate::index::GitIndex::zeroed();
        // SAFETY: the opaque layout-compatible stack value stays live and no
        // index operation inspects it in this test.
        let index =
            unsafe { crate::index::GitIndexRef::from_ptr(addr_of_mut!(index).cast()) }.unwrap();
        let index_ptr = index.as_ptr();

        let mut options = GitCheckoutOptions::new();
        {
            let mut view = options.as_mut();
            view.set_paths(paths);
            view.set_file_mode(0o640);
            view.set_dir_mode(0o750);
            view.set_file_open_flags(7);
            view.set_disable_filters(true);
            view.set_checkout_strategy(
                GitCheckoutStrategy::FORCE | GitCheckoutStrategy::REMOVE_UNTRACKED,
            );
            view.set_notify_flags(
                crate::checkout::CheckoutNotify::CONFLICT
                    | crate::checkout::CheckoutNotify::UPDATED,
            );
            view.set_baseline(Some(tree));
            view.set_baseline_index(Some(index));
            view.set_target_directory(Some(c"alternate"));
            view.set_ancestor_label(Some(c"base"));
            view.set_our_label(Some(c"ours"));
            view.set_their_label(Some(c"theirs"));
        }

        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_CHECKOUT_OPTIONS_VERSION);
        assert_eq!(view.paths().strings().unwrap().get(0), Some(c"src/lib.rs"));
        assert_eq!(view.file_mode(), 0o640);
        assert_eq!(view.dir_mode(), 0o750);
        assert_eq!(view.file_open_flags(), 7);
        assert!(view.disable_filters());
        assert!(
            view.checkout_strategy()
                .contains(GitCheckoutStrategy::FORCE)
        );
        assert!(
            view.notify_flags()
                .contains(crate::checkout::CheckoutNotify::UPDATED)
        );
        assert_eq!(view.baseline().unwrap().as_ptr(), tree_ptr);
        assert_eq!(view.baseline_index().unwrap().as_ptr(), index_ptr);
        assert_eq!(view.target_directory(), Some(c"alternate"));
        assert_eq!(view.ancestor_label(), Some(c"base"));
        assert_eq!(view.our_label(), Some(c"ours"));
        assert_eq!(view.their_label(), Some(c"theirs"));
    }

    #[test]
    fn callback_pairs_use_typed_trampolines_and_clear_together() {
        let mut options = GitCheckoutOptions::new();
        let mut notifications = 0;
        let mut notify = |why: crate::checkout::CheckoutNotify,
                          path: Option<&core::ffi::CStr>,
                          _: Option<crate::diff::DiffFileRef<'_>>,
                          _: Option<crate::diff::DiffFileRef<'_>>,
                          _: Option<crate::diff::DiffFileRef<'_>>| {
            notifications += usize::from(
                why == crate::checkout::CheckoutNotify::UPDATED && path == Some(c"file"),
            );
            0
        };
        {
            let mut view = options.as_mut();
            // SAFETY: the callback stays live and reserved until it is cleared.
            unsafe { view.set_notify_callback(&mut notify) };
            assert!(view.as_ref().has_notify_callback());
            assert!(view.as_ref().has_notify_payload());
            let ptr = view.as_ref().as_ptr();
            // SAFETY: raw-place reads copy the coherently installed pair.
            let function = unsafe { addr_of!((*ptr).notify_cb).read() }.unwrap();
            // SAFETY: as above; this merely copies the opaque pointer.
            let payload = unsafe { addr_of!((*ptr).notify_payload).read() };
            // SAFETY: the callback and payload remain live and exclusively reserved.
            let rc = unsafe {
                function(
                    crate::checkout::CheckoutNotify::UPDATED.bits(),
                    c"file".as_ptr(),
                    core::ptr::null(),
                    core::ptr::null(),
                    core::ptr::null(),
                    payload,
                )
            };
            assert_eq!(rc, 0);
            view.clear_notify_callback();
            assert!(!view.as_ref().has_notify_callback());
            assert!(!view.as_ref().has_notify_payload());
        }
        assert_eq!(notifications, 1);

        let mut progress_calls = 0;
        let mut progress = |path: Option<&core::ffi::CStr>, done, total| {
            progress_calls += usize::from(path.is_none() && done == 0 && total == 3);
        };
        {
            let mut view = options.as_mut();
            // SAFETY: the callback stays live and reserved until it is cleared.
            unsafe { view.set_progress_callback(&mut progress) };
            assert!(view.as_ref().has_progress_callback());
            assert!(view.as_ref().has_progress_payload());
            view.clear_progress_callback();
        }
        assert_eq!(progress_calls, 0);

        let mut perf_calls = 0;
        let mut perf = |_: crate::checkout::CheckoutPerfDataRef<'_>| perf_calls += 1;
        {
            let mut view = options.as_mut();
            // SAFETY: the callback stays live and reserved until it is cleared.
            unsafe { view.set_perfdata_callback(&mut perf) };
            assert!(view.as_ref().has_perfdata_callback());
            assert!(view.as_ref().has_perfdata_payload());
            view.clear_perfdata_callback();
        }
        assert_eq!(perf_calls, 0);
    }
}
