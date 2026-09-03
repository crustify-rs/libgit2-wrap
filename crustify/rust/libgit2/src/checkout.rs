//! Safe wrappers for libgit2 checkout APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_checkout_notify_t
///
/// A layout-compatible set of checkout notification flags. Unknown bits are
/// retained so values introduced by a newer libgit2 remain safe to carry.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CheckoutNotify(ffi::git_checkout_notify_t);

impl CheckoutNotify {
    /// No checkout notifications.
    pub const NONE: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_NONE);
    /// A conflict was found.
    pub const CONFLICT: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_CONFLICT);
    /// A dirty file was found.
    pub const DIRTY: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_DIRTY);
    /// A file was updated.
    pub const UPDATED: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_UPDATED);
    /// An untracked file was found.
    pub const UNTRACKED: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_UNTRACKED);
    /// An ignored file was found.
    pub const IGNORED: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_IGNORED);
    /// All notification classes recognized by libgit2.
    pub const ALL: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_ALL);

    /// Retain all bits from a raw libgit2 value, including unknown bits.
    #[inline]
    #[must_use]
    pub const fn from_bits_retain(bits: ffi::git_checkout_notify_t) -> Self {
        Self(bits)
    }

    /// Return the raw libgit2 flag bits.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> ffi::git_checkout_notify_t {
        self.0
    }

    /// Whether every bit in `other` is present.
    #[inline]
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether at least one bit in `other` is present.
    #[inline]
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether no notification bits are set.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl From<ffi::git_checkout_notify_t> for CheckoutNotify {
    #[inline]
    fn from(bits: ffi::git_checkout_notify_t) -> Self {
        Self::from_bits_retain(bits)
    }
}

impl From<CheckoutNotify> for ffi::git_checkout_notify_t {
    #[inline]
    fn from(flags: CheckoutNotify) -> Self {
        flags.bits()
    }
}

impl BitOr for CheckoutNotify {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for CheckoutNotify {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for CheckoutNotify {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for CheckoutNotify {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for CheckoutNotify {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

ffibox::define_ctype!(
    /// Wraps: git_checkout_perfdata
    CheckoutPerfData,
    CheckoutPerfDataRef,
    CheckoutPerfDataMut,
    ffi::git_checkout_perfdata
);

impl CheckoutPerfDataRef<'_> {
    /// Field: git_checkout_perfdata.stat_calls
    #[inline]
    #[must_use]
    pub fn stat_calls(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).stat_calls).read() }
    }

    /// Field: git_checkout_perfdata.chmod_calls
    #[inline]
    #[must_use]
    pub fn chmod_calls(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).chmod_calls).read() }
    }

    /// Field: git_checkout_perfdata.mkdir_calls
    #[inline]
    #[must_use]
    pub fn mkdir_calls(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).mkdir_calls).read() }
    }
}

impl CheckoutPerfDataMut<'_> {
    /// Set the number of `stat` calls.
    #[inline]
    pub fn set_stat_calls(&mut self, value: usize) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).stat_calls).write(value) }
    }

    /// Set the number of `chmod` calls.
    #[inline]
    pub fn set_chmod_calls(&mut self, value: usize) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).chmod_calls).write(value) }
    }

    /// Set the number of `mkdir` calls.
    #[inline]
    pub fn set_mkdir_calls(&mut self, value: usize) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).mkdir_calls).write(value) }
    }
}

/// Wraps: git_checkout_progress_cb
/// Safe callable surface for checkout progress notifications.
pub trait GitCheckoutProgressCallback {
    /// Reports progress; `path == None` is the initial zero-step baseline.
    fn call(&mut self, path: Option<&core::ffi::CStr>, completed: usize, total: usize);
}

impl<F> GitCheckoutProgressCallback for F
where
    F: FnMut(Option<&core::ffi::CStr>, usize, usize),
{
    fn call(&mut self, path: Option<&core::ffi::CStr>, completed: usize, total: usize) {
        self(path, completed, total)
    }
}

/// Wraps: git_checkout_perfdata_cb
/// Safe callable surface for checkout performance reports.
pub trait GitCheckoutPerfDataCallback {
    /// Receives one transient shared view of the checkout counters.
    fn call(&mut self, perfdata: CheckoutPerfDataRef<'_>);
}

impl<F> GitCheckoutPerfDataCallback for F
where
    F: for<'a> FnMut(CheckoutPerfDataRef<'a>),
{
    fn call(&mut self, perfdata: CheckoutPerfDataRef<'_>) {
        self(perfdata)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn notify_flags_combine_and_retain_unknown_bits() {
        let flags = CheckoutNotify::CONFLICT | CheckoutNotify::UPDATED;
        assert!(flags.contains(CheckoutNotify::CONFLICT));
        assert!(flags.intersects(CheckoutNotify::UPDATED));
        assert!(!flags.intersects(CheckoutNotify::DIRTY));

        let unknown = CheckoutNotify::from_bits_retain(1 << 31);
        assert_eq!(unknown.bits(), 1 << 31);
        assert_eq!(ffi::git_checkout_notify_t::from(unknown), 1 << 31);
    }

    #[test]
    fn perfdata_callback_receives_a_typed_transient_handle() {
        let mut raw = ffi::git_checkout_perfdata {
            mkdir_calls: 7,
            stat_calls: 8,
            chmod_calls: 9,
        };
        // SAFETY: `raw` remains initialized and live for this callback call.
        let data = unsafe { CheckoutPerfDataRef::from_ptr(&raw mut raw) }.unwrap();
        let mut seen = 0;
        let mut callback = |data: CheckoutPerfDataRef<'_>| {
            seen = data.mkdir_calls() + data.stat_calls() + data.chmod_calls();
        };
        GitCheckoutPerfDataCallback::call(&mut callback, data);
        assert_eq!(seen, 24);
    }

    #[test]
    fn perfdata_handles_read_and_write_all_fields() {
        let mut raw = ffi::git_checkout_perfdata {
            mkdir_calls: 1,
            stat_calls: 2,
            chmod_calls: 3,
        };

        // SAFETY: `raw` is initialized, non-null, exclusively borrowed for the
        // handle's lifetime, and remains live until the handle is last used.
        let mut data = unsafe { CheckoutPerfDataMut::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(data.as_ref().mkdir_calls(), 1);
        assert_eq!(data.as_ref().stat_calls(), 2);
        assert_eq!(data.as_ref().chmod_calls(), 3);

        data.set_mkdir_calls(4);
        data.set_stat_calls(5);
        data.set_chmod_calls(6);
        assert_eq!(data.as_ref().mkdir_calls(), 4);
        assert_eq!(data.as_ref().stat_calls(), 5);
        assert_eq!(data.as_ref().chmod_calls(), 6);
    }

    #[test]
    fn progress_callback_accepts_the_null_baseline_path() {
        let mut seen = None;
        let mut callback = |path: Option<&core::ffi::CStr>, completed, total| {
            seen = Some((path.is_none(), completed, total));
        };
        GitCheckoutProgressCallback::call(&mut callback, None, 0, 12);
        assert_eq!(seen, Some((true, 0, 12)));
    }

    #[test]
    fn perfdata_wrapper_preserves_c_layout() {
        assert_eq!(
            core::mem::size_of::<CheckoutPerfData>(),
            core::mem::size_of::<ffi::git_checkout_perfdata>()
        );
        assert_eq!(
            core::mem::align_of::<CheckoutPerfData>(),
            core::mem::align_of::<ffi::git_checkout_perfdata>()
        );
    }
}

/// Wraps: git_checkout_head
/// Updates the index and working tree to match the repository's HEAD.
pub fn git_checkout_head(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    options: Option<crate::api::checkout::GitCheckoutOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository is exclusively borrowed and the optional options
    // record, including every caller-maintained pointee, is live for the call.
    let status = unsafe { ffi::git_checkout_head(repo.as_mut_ptr(), options) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_checkout_index
/// Updates the working tree from `index`, or from the repository index when absent.
///
/// libgit2 also accepts a null repository together with an index, deriving the
/// repository from `git_index_owner`. This wrapper deliberately does not: that
/// form checks a working tree out of a repository no Rust handle borrows, and
/// it turns the "neither argument given" case into a runtime error instead of a
/// type error. Callers pass the repository they already hold.
pub fn git_checkout_index(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    mut index: Option<&mut crate::index::GitIndexMut<'_>>,
    options: Option<crate::api::checkout::GitCheckoutOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let index = index
        .as_mut()
        .map_or(core::ptr::null_mut(), |index| index.as_mut_ptr());
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository and optional index are exclusively borrowed; C
    // retains neither pointer, and all option pointees remain live for the call.
    let status = unsafe { ffi::git_checkout_index(repo.as_mut_ptr(), index, options) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_checkout_init_options
/// Initializes deprecated checkout-options storage for `version`.
pub fn git_checkout_init_options(
    options: &mut crate::api::checkout::GitCheckoutOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle exposes writable layout-compatible storage;
    // initialization stores no pointer to the options header.
    let status = unsafe { ffi::git_checkout_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_checkout_tree
/// Updates the index and worktree from `treeish`, or from HEAD when absent.
///
/// As with [`git_checkout_index`], libgit2's null-repository form — which takes
/// the repository from `git_object_owner(treeish)` — is deliberately not
/// exposed: it would write a working tree that no Rust handle borrows.
pub fn git_checkout_tree(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    treeish: Option<crate::object::GitObjectRef<'_>>,
    options: Option<crate::api::checkout::GitCheckoutOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let treeish = treeish.map_or(core::ptr::null(), |treeish| treeish.as_ptr());
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository is exclusive, the optional object and options
    // are live for this synchronous call, and libgit2 retains none of them.
    let status = unsafe { ffi::git_checkout_tree(repo.as_mut_ptr(), treeish, options) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, TempDir};

    #[derive(Debug, Eq, PartialEq)]
    struct CheckoutObservation {
        readme: Vec<u8>,
        alpha: Vec<u8>,
        beta: Vec<u8>,
        junk_exists: bool,
        guide_exists: bool,
    }

    fn dirty(fixture: &HistoryFixture) {
        std::fs::write(
            fixture.directory.path().join("README.md"),
            b"dirty contents\n",
        )
        .unwrap();
        std::fs::remove_file(fixture.directory.path().join("src/alpha.c")).unwrap();
        std::fs::write(fixture.directory.path().join("junk.tmp"), b"untracked\n").unwrap();
    }

    fn observation(fixture: &HistoryFixture) -> CheckoutObservation {
        CheckoutObservation {
            readme: std::fs::read(fixture.directory.path().join("README.md")).unwrap(),
            alpha: std::fs::read(fixture.directory.path().join("src/alpha.c")).unwrap(),
            beta: std::fs::read(fixture.directory.path().join("src/beta.c")).unwrap(),
            junk_exists: fixture.directory.path().join("junk.tmp").exists(),
            guide_exists: fixture.directory.path().join("docs/guide.txt").exists(),
        }
    }

    unsafe fn raw_checkout(fixture: &HistoryFixture) -> CheckoutObservation {
        let repository = fixture.repository.as_ptr();
        let mut options: ffi::git_checkout_options = unsafe { core::mem::zeroed() };
        assert_eq!(
            unsafe {
                ffi::git_checkout_options_init(&mut options, ffi::GIT_CHECKOUT_OPTIONS_VERSION)
            },
            0
        );
        options.checkout_strategy = ffi::git_checkout_strategy_t_GIT_CHECKOUT_FORCE
            | ffi::git_checkout_strategy_t_GIT_CHECKOUT_RECREATE_MISSING
            | ffi::git_checkout_strategy_t_GIT_CHECKOUT_REMOVE_UNTRACKED;
        assert_eq!(unsafe { ffi::git_checkout_head(repository, &options) }, 0);
        std::fs::write(fixture.directory.path().join("README.md"), b"dirty again\n").unwrap();
        assert_eq!(
            unsafe { ffi::git_checkout_index(repository, core::ptr::null_mut(), &options) },
            0
        );

        let mut prior = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut prior, repository, c"HEAD~2".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_checkout_tree(repository, prior, &options) },
            0
        );
        assert!(fixture.directory.path().join("docs/guide.txt").exists());
        assert_eq!(unsafe { ffi::git_checkout_head(repository, &options) }, 0);
        unsafe { ffi::git_object_free(prior) };
        observation(fixture)
    }

    fn safe_checkout(fixture: &HistoryFixture) -> CheckoutObservation {
        let repository = fixture.repository.as_ptr();
        let mut options = crate::api::checkout::GitCheckoutOptions::new();
        options.as_mut().set_checkout_strategy(
            crate::api::checkout::GitCheckoutStrategy::FORCE
                | crate::api::checkout::GitCheckoutStrategy::RECREATE_MISSING
                | crate::api::checkout::GitCheckoutStrategy::REMOVE_UNTRACKED,
        );
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        git_checkout_head(&mut repository_view, Some(options.as_ref())).unwrap();
        std::fs::write(fixture.directory.path().join("README.md"), b"dirty again\n").unwrap();
        git_checkout_index(&mut repository_view, None, Some(options.as_ref())).unwrap();
        let mut prior = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut prior, repository, c"HEAD~2".as_ptr()) },
            0
        );
        let prior_view = unsafe { crate::object::GitObjectRef::from_ptr(prior) }.unwrap();
        git_checkout_tree(
            &mut repository_view,
            Some(prior_view),
            Some(options.as_ref()),
        )
        .unwrap();
        assert!(fixture.directory.path().join("docs/guide.txt").exists());
        git_checkout_head(&mut repository_view, Some(options.as_ref())).unwrap();
        unsafe { ffi::git_object_free(prior) };
        observation(fixture)
    }

    #[test]
    fn io_equiv_force_checkout_head_index_and_tree() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("checkout-raw");
        let safe = HistoryFixture::new("checkout-safe");
        dirty(&raw);
        dirty(&safe);
        let raw_observation = unsafe { raw_checkout(&raw) };
        assert_eq!(raw_observation, safe_checkout(&safe));
        assert!(!raw_observation.junk_exists);
        assert!(!raw_observation.guide_exists);
    }

    #[derive(Debug, Eq, PartialEq)]
    struct TargetObservation {
        readme_exists: bool,
        alpha: Vec<u8>,
        beta: Vec<u8>,
        guide_exists: bool,
    }

    fn target_observation(target: &TempDir) -> TargetObservation {
        TargetObservation {
            readme_exists: target.path().join("README.md").exists(),
            alpha: std::fs::read(target.path().join("src/alpha.c")).unwrap(),
            beta: std::fs::read(target.path().join("src/beta.c")).unwrap(),
            guide_exists: target.path().join("docs/guide.txt").exists(),
        }
    }

    unsafe fn raw_checkout_paths(
        repository: *mut ffi::git_repository,
        target: &TempDir,
    ) -> TargetObservation {
        let mut options = unsafe { core::mem::zeroed::<ffi::git_checkout_options>() };
        assert_eq!(
            unsafe {
                ffi::git_checkout_options_init(&mut options, ffi::GIT_CHECKOUT_OPTIONS_VERSION)
            },
            0
        );
        options.checkout_strategy = ffi::git_checkout_strategy_t_GIT_CHECKOUT_FORCE
            | ffi::git_checkout_strategy_t_GIT_CHECKOUT_DONT_UPDATE_INDEX;
        let target_path = target.c_path();
        options.target_directory = target_path.as_ptr();
        let mut path = c"src/*".as_ptr().cast_mut();
        options.paths = ffi::git_strarray {
            strings: core::ptr::from_mut(&mut path),
            count: 1,
        };
        assert_eq!(unsafe { ffi::git_checkout_head(repository, &options) }, 0);
        target_observation(target)
    }

    fn safe_checkout_paths(
        repository: *mut ffi::git_repository,
        target: &TempDir,
    ) -> TargetObservation {
        let target_path = target.c_path();
        let mut path = c"src/*".as_ptr().cast_mut();
        let mut raw_paths = ffi::git_strarray {
            strings: core::ptr::from_mut(&mut path),
            count: 1,
        };
        let paths = unsafe {
            crate::strarray::GitStrArrayRef::from_ptr(core::ptr::from_mut(&mut raw_paths))
        }
        .unwrap();
        let mut options = crate::api::checkout::GitCheckoutOptions::new();
        options.as_mut().set_checkout_strategy(
            crate::api::checkout::GitCheckoutStrategy::FORCE
                | crate::api::checkout::GitCheckoutStrategy::DONT_UPDATE_INDEX,
        );
        options.as_mut().set_target_directory(Some(&target_path));
        options.as_mut().set_paths(paths);
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        git_checkout_head(&mut repository_view, Some(options.as_ref())).unwrap();
        target_observation(target)
    }

    #[test]
    fn io_equiv_checkout_pathspec_into_alternate_directory() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("checkout-paths-raw");
        let safe = HistoryFixture::new("checkout-paths-safe");
        let raw_target = TempDir::new("checkout-target-raw");
        let safe_target = TempDir::new("checkout-target-safe");
        let raw_observation = unsafe { raw_checkout_paths(raw.repository.as_ptr(), &raw_target) };
        assert_eq!(
            raw_observation,
            safe_checkout_paths(safe.repository.as_ptr(), &safe_target)
        );
        assert!(!raw_observation.readme_exists);
        assert!(!raw_observation.guide_exists);
    }

    fn prepare_conflict(fixture: &HistoryFixture) {
        let path = fixture.directory.path();
        let run = |arguments: &[&str], succeeds: bool| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(path)
                .args(arguments)
                .env("GIT_AUTHOR_DATE", "1700003000 +0000")
                .env("GIT_COMMITTER_DATE", "1700003000 +0000")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap();
            assert_eq!(status.success(), succeeds, "git command: {arguments:?}");
        };
        run(&["checkout", "-q", "-b", "checkout-side"], true);
        std::fs::write(path.join("README.md"), b"theirs from side\n").unwrap();
        run(&["add", "README.md"], true);
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "side change",
            ],
            true,
        );
        run(&["checkout", "-q", "master"], true);
        std::fs::write(path.join("README.md"), b"ours from master\n").unwrap();
        run(&["add", "README.md"], true);
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "master change",
            ],
            true,
        );
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "merge",
                "--no-commit",
                "checkout-side",
            ],
            false,
        );
    }

    unsafe fn raw_conflict_checkout(fixture: &HistoryFixture, ours: bool) -> Vec<u8> {
        let mut options = unsafe { core::mem::zeroed::<ffi::git_checkout_options>() };
        assert_eq!(
            unsafe {
                ffi::git_checkout_options_init(&mut options, ffi::GIT_CHECKOUT_OPTIONS_VERSION)
            },
            0
        );
        options.checkout_strategy = ffi::git_checkout_strategy_t_GIT_CHECKOUT_FORCE
            | if ours {
                ffi::git_checkout_strategy_t_GIT_CHECKOUT_USE_OURS
            } else {
                ffi::git_checkout_strategy_t_GIT_CHECKOUT_USE_THEIRS
            };
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, fixture.repository.as_ptr()) },
            0
        );
        assert_eq!(unsafe { ffi::git_index_read(index, 1) }, 0);
        assert_eq!(
            unsafe { ffi::git_checkout_index(fixture.repository.as_ptr(), index, &options,) },
            0
        );
        unsafe { ffi::git_index_free(index) };
        std::fs::read(fixture.directory.path().join("README.md")).unwrap()
    }

    fn safe_conflict_checkout(fixture: &HistoryFixture, ours: bool) -> Vec<u8> {
        let mut options = crate::api::checkout::GitCheckoutOptions::new();
        let side = if ours {
            crate::api::checkout::GitCheckoutStrategy::USE_OURS
        } else {
            crate::api::checkout::GitCheckoutStrategy::USE_THEIRS
        };
        options
            .as_mut()
            .set_checkout_strategy(crate::api::checkout::GitCheckoutStrategy::FORCE | side);
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        let mut index = crate::repository::git_repository_index(&mut repository).unwrap();
        crate::index::git_index_read(&mut index.as_mut(), true).unwrap();
        git_checkout_index(
            &mut repository,
            Some(&mut index.as_mut()),
            Some(options.as_ref()),
        )
        .unwrap();
        std::fs::read(fixture.directory.path().join("README.md")).unwrap()
    }

    #[test]
    fn io_equiv_checkout_conflicts_select_ours_and_theirs() {
        let _libgit2 = Libgit2Init::acquire();
        let raw_ours = HistoryFixture::new("checkout-conflict-raw-ours");
        let safe_ours = HistoryFixture::new("checkout-conflict-safe-ours");
        let raw_theirs = HistoryFixture::new("checkout-conflict-raw-theirs");
        let safe_theirs = HistoryFixture::new("checkout-conflict-safe-theirs");
        for fixture in [&raw_ours, &safe_ours, &raw_theirs, &safe_theirs] {
            prepare_conflict(fixture);
        }
        let ours = unsafe { raw_conflict_checkout(&raw_ours, true) };
        assert_eq!(ours, safe_conflict_checkout(&safe_ours, true));
        let theirs = unsafe { raw_conflict_checkout(&raw_theirs, false) };
        assert_eq!(theirs, safe_conflict_checkout(&safe_theirs, false));
        assert_eq!(ours, b"ours from master\n");
        assert_eq!(theirs, b"theirs from side\n");
    }

    unsafe fn raw_conflict_marker_checkout(fixture: &HistoryFixture, style: u32) -> Vec<u8> {
        std::fs::write(
            fixture.directory.path().join("README.md"),
            b"replace this with conflict markers\n",
        )
        .unwrap();
        let mut options = unsafe { core::mem::zeroed::<ffi::git_checkout_options>() };
        assert_eq!(
            unsafe {
                ffi::git_checkout_options_init(&mut options, ffi::GIT_CHECKOUT_OPTIONS_VERSION)
            },
            0
        );
        options.checkout_strategy = ffi::git_checkout_strategy_t_GIT_CHECKOUT_FORCE
            | ffi::git_checkout_strategy_t_GIT_CHECKOUT_ALLOW_CONFLICTS
            | style;
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, fixture.repository.as_ptr()) },
            0
        );
        assert_eq!(unsafe { ffi::git_index_read(index, 1) }, 0);
        assert_eq!(
            unsafe { ffi::git_checkout_index(fixture.repository.as_ptr(), index, &options) },
            0
        );
        unsafe { ffi::git_index_free(index) };
        std::fs::read(fixture.directory.path().join("README.md")).unwrap()
    }

    fn safe_conflict_marker_checkout(
        fixture: &HistoryFixture,
        style: crate::api::checkout::GitCheckoutStrategy,
    ) -> Vec<u8> {
        std::fs::write(
            fixture.directory.path().join("README.md"),
            b"replace this with conflict markers\n",
        )
        .unwrap();
        let mut options = crate::api::checkout::GitCheckoutOptions::new();
        options.as_mut().set_checkout_strategy(
            crate::api::checkout::GitCheckoutStrategy::FORCE
                | crate::api::checkout::GitCheckoutStrategy::ALLOW_CONFLICTS
                | style,
        );
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        let mut index = crate::repository::git_repository_index(&mut repository).unwrap();
        crate::index::git_index_read(&mut index.as_mut(), true).unwrap();
        git_checkout_index(
            &mut repository,
            Some(&mut index.as_mut()),
            Some(options.as_ref()),
        )
        .unwrap();
        std::fs::read(fixture.directory.path().join("README.md")).unwrap()
    }

    #[test]
    fn io_equiv_checkout_writes_merge_and_diff3_conflict_markers() {
        let _libgit2 = Libgit2Init::acquire();
        let raw_merge = HistoryFixture::new("checkout-marker-raw-merge");
        let safe_merge = HistoryFixture::new("checkout-marker-safe-merge");
        let raw_diff3 = HistoryFixture::new("checkout-marker-raw-diff3");
        let safe_diff3 = HistoryFixture::new("checkout-marker-safe-diff3");
        for fixture in [&raw_merge, &safe_merge, &raw_diff3, &safe_diff3] {
            prepare_conflict(fixture);
        }

        let merge = unsafe {
            raw_conflict_marker_checkout(
                &raw_merge,
                ffi::git_checkout_strategy_t_GIT_CHECKOUT_CONFLICT_STYLE_MERGE,
            )
        };
        assert_eq!(
            merge,
            safe_conflict_marker_checkout(
                &safe_merge,
                crate::api::checkout::GitCheckoutStrategy::CONFLICT_STYLE_MERGE,
            )
        );
        let diff3 = unsafe {
            raw_conflict_marker_checkout(
                &raw_diff3,
                ffi::git_checkout_strategy_t_GIT_CHECKOUT_CONFLICT_STYLE_DIFF3,
            )
        };
        assert_eq!(
            diff3,
            safe_conflict_marker_checkout(
                &safe_diff3,
                crate::api::checkout::GitCheckoutStrategy::CONFLICT_STYLE_DIFF3,
            )
        );
        assert!(merge.windows(7).any(|line| line == b"<<<<<<<"));
        assert!(diff3.windows(7).any(|line| line == b"|||||||"));
    }

    fn prepare_callbacks(fixture: &HistoryFixture) {
        std::fs::write(
            fixture.directory.path().join("README.md"),
            b"dirty callback contents\n",
        )
        .unwrap();
        std::fs::remove_file(fixture.directory.path().join("src/alpha.c")).unwrap();
        std::fs::write(
            fixture.directory.path().join("untracked.txt"),
            b"remove me\n",
        )
        .unwrap();
        std::fs::write(fixture.directory.path().join(".gitignore"), b"ignored/\n").unwrap();
        std::fs::create_dir_all(fixture.directory.path().join("ignored")).unwrap();
        std::fs::write(
            fixture.directory.path().join("ignored/cache.bin"),
            b"ignored\n",
        )
        .unwrap();
    }

    #[derive(Debug, Eq, PartialEq)]
    struct CallbackObservation {
        notify: Vec<(u32, Vec<u8>)>,
        progress: Vec<(Vec<u8>, usize, usize)>,
        perf: Vec<(usize, usize, usize)>,
        readme: Vec<u8>,
        alpha_exists: bool,
        untracked_exists: bool,
        ignored_exists: bool,
    }

    unsafe fn raw_callback_checkout(fixture: &HistoryFixture) -> CallbackObservation {
        unsafe extern "C" fn notify(
            why: u32,
            path: *const core::ffi::c_char,
            _: *const ffi::git_diff_file,
            _: *const ffi::git_diff_file,
            _: *const ffi::git_diff_file,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            let output = unsafe { &mut *payload.cast::<Vec<(u32, Vec<u8>)>>() };
            let path = if path.is_null() {
                Vec::new()
            } else {
                unsafe { core::ffi::CStr::from_ptr(path) }
                    .to_bytes()
                    .to_vec()
            };
            output.push((why, path));
            0
        }
        unsafe extern "C" fn progress(
            path: *const core::ffi::c_char,
            completed: usize,
            total: usize,
            payload: *mut core::ffi::c_void,
        ) {
            let output = unsafe { &mut *payload.cast::<Vec<(Vec<u8>, usize, usize)>>() };
            let path = if path.is_null() {
                Vec::new()
            } else {
                unsafe { core::ffi::CStr::from_ptr(path) }
                    .to_bytes()
                    .to_vec()
            };
            output.push((path, completed, total));
        }
        unsafe extern "C" fn perf(
            data: *const ffi::git_checkout_perfdata,
            payload: *mut core::ffi::c_void,
        ) {
            unsafe { &mut *payload.cast::<Vec<(usize, usize, usize)>>() }.push((
                unsafe { (*data).stat_calls },
                unsafe { (*data).chmod_calls },
                unsafe { (*data).mkdir_calls },
            ));
        }
        let mut notify_output = Vec::new();
        let mut progress_output = Vec::new();
        let mut perf_output = Vec::new();
        let mut options = unsafe { core::mem::zeroed::<ffi::git_checkout_options>() };
        assert_eq!(
            unsafe {
                ffi::git_checkout_options_init(&mut options, ffi::GIT_CHECKOUT_OPTIONS_VERSION)
            },
            0
        );
        options.checkout_strategy = ffi::git_checkout_strategy_t_GIT_CHECKOUT_FORCE
            | ffi::git_checkout_strategy_t_GIT_CHECKOUT_RECREATE_MISSING
            | ffi::git_checkout_strategy_t_GIT_CHECKOUT_REMOVE_UNTRACKED
            | ffi::git_checkout_strategy_t_GIT_CHECKOUT_REMOVE_IGNORED;
        options.notify_flags = ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_ALL;
        options.notify_cb = Some(notify);
        options.notify_payload = core::ptr::from_mut(&mut notify_output).cast();
        options.progress_cb = Some(progress);
        options.progress_payload = core::ptr::from_mut(&mut progress_output).cast();
        options.perfdata_cb = Some(perf);
        options.perfdata_payload = core::ptr::from_mut(&mut perf_output).cast();
        assert_eq!(
            unsafe { ffi::git_checkout_head(fixture.repository.as_ptr(), &options) },
            0
        );
        CallbackObservation {
            notify: notify_output,
            progress: progress_output,
            perf: perf_output,
            readme: std::fs::read(fixture.directory.path().join("README.md")).unwrap(),
            alpha_exists: fixture.directory.path().join("src/alpha.c").exists(),
            untracked_exists: fixture.directory.path().join("untracked.txt").exists(),
            ignored_exists: fixture.directory.path().join("ignored/cache.bin").exists(),
        }
    }

    fn safe_callback_checkout(fixture: &HistoryFixture) -> CallbackObservation {
        let mut notify_output = Vec::new();
        let mut progress_output = Vec::new();
        let mut perf_output = Vec::new();
        let mut notify = |why: CheckoutNotify,
                          path: Option<&core::ffi::CStr>,
                          _: Option<crate::diff::DiffFileRef<'_>>,
                          _: Option<crate::diff::DiffFileRef<'_>>,
                          _: Option<crate::diff::DiffFileRef<'_>>| {
            notify_output.push((
                why.bits(),
                path.map_or_else(Vec::new, |value| value.to_bytes().to_vec()),
            ));
            0
        };
        let mut progress = |path: Option<&core::ffi::CStr>, completed: usize, total: usize| {
            progress_output.push((
                path.map_or_else(Vec::new, |value| value.to_bytes().to_vec()),
                completed,
                total,
            ));
        };
        let mut perf = |data: CheckoutPerfDataRef<'_>| {
            perf_output.push((data.stat_calls(), data.chmod_calls(), data.mkdir_calls()));
        };
        let mut options = crate::api::checkout::GitCheckoutOptions::new();
        options.as_mut().set_checkout_strategy(
            crate::api::checkout::GitCheckoutStrategy::FORCE
                | crate::api::checkout::GitCheckoutStrategy::RECREATE_MISSING
                | crate::api::checkout::GitCheckoutStrategy::REMOVE_UNTRACKED
                | crate::api::checkout::GitCheckoutStrategy::REMOVE_IGNORED,
        );
        options.as_mut().set_notify_flags(CheckoutNotify::ALL);
        unsafe {
            options.as_mut().set_notify_callback(&mut notify);
            options.as_mut().set_progress_callback(&mut progress);
            options.as_mut().set_perfdata_callback(&mut perf);
        }
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        git_checkout_head(&mut repository, Some(options.as_ref())).unwrap();
        CallbackObservation {
            notify: notify_output,
            progress: progress_output,
            perf: perf_output,
            readme: std::fs::read(fixture.directory.path().join("README.md")).unwrap(),
            alpha_exists: fixture.directory.path().join("src/alpha.c").exists(),
            untracked_exists: fixture.directory.path().join("untracked.txt").exists(),
            ignored_exists: fixture.directory.path().join("ignored/cache.bin").exists(),
        }
    }

    #[test]
    fn io_equiv_checkout_notifications_progress_cleanup_and_perfdata() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("checkout-callbacks-raw");
        let safe = HistoryFixture::new("checkout-callbacks-safe");
        prepare_callbacks(&raw);
        prepare_callbacks(&safe);
        let raw_observation = unsafe { raw_callback_checkout(&raw) };
        assert_eq!(raw_observation, safe_callback_checkout(&safe));
        assert!(!raw_observation.notify.is_empty());
        assert!(!raw_observation.progress.is_empty());
        assert!(!raw_observation.untracked_exists);
        assert!(!raw_observation.ignored_exists);
    }

    fn prepare_blockers(fixture: &HistoryFixture) {
        std::fs::remove_dir_all(fixture.directory.path().join("src")).unwrap();
        std::fs::write(
            fixture.directory.path().join("src"),
            b"file blocks directory\n",
        )
        .unwrap();
        std::fs::remove_file(fixture.directory.path().join("README.md")).unwrap();
        std::fs::create_dir_all(fixture.directory.path().join("README.md")).unwrap();
        std::fs::write(
            fixture.directory.path().join("README.md/untracked-child"),
            b"directory blocks file\n",
        )
        .unwrap();
    }

    unsafe fn raw_checkout_blockers(fixture: &HistoryFixture) -> Vec<Vec<u8>> {
        let mut options = unsafe { core::mem::zeroed::<ffi::git_checkout_options>() };
        assert_eq!(
            unsafe {
                ffi::git_checkout_options_init(&mut options, ffi::GIT_CHECKOUT_OPTIONS_VERSION)
            },
            0
        );
        options.checkout_strategy = ffi::git_checkout_strategy_t_GIT_CHECKOUT_FORCE
            | ffi::git_checkout_strategy_t_GIT_CHECKOUT_REMOVE_UNTRACKED;
        assert_eq!(
            unsafe { ffi::git_checkout_head(fixture.repository.as_ptr(), &options) },
            0
        );
        std::fs::remove_file(fixture.directory.path().join("src/beta.c")).unwrap();
        std::fs::create_dir(fixture.directory.path().join("src/beta.c")).unwrap();
        assert_eq!(
            unsafe { ffi::git_checkout_head(fixture.repository.as_ptr(), &options) },
            0
        );
        ["README.md", "src/alpha.c", "src/beta.c"]
            .map(|path| std::fs::read(fixture.directory.path().join(path)).unwrap())
            .to_vec()
    }

    fn safe_checkout_blockers(fixture: &HistoryFixture) -> Vec<Vec<u8>> {
        let mut options = crate::api::checkout::GitCheckoutOptions::new();
        options.as_mut().set_checkout_strategy(
            crate::api::checkout::GitCheckoutStrategy::FORCE
                | crate::api::checkout::GitCheckoutStrategy::REMOVE_UNTRACKED,
        );
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        git_checkout_head(&mut repository, Some(options.as_ref())).unwrap();
        std::fs::remove_file(fixture.directory.path().join("src/beta.c")).unwrap();
        std::fs::create_dir(fixture.directory.path().join("src/beta.c")).unwrap();
        git_checkout_head(&mut repository, Some(options.as_ref())).unwrap();
        ["README.md", "src/alpha.c", "src/beta.c"]
            .map(|path| std::fs::read(fixture.directory.path().join(path)).unwrap())
            .to_vec()
    }

    #[test]
    fn io_equiv_checkout_replaces_file_directory_and_empty_directory_blockers() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("checkout-blockers-raw");
        let safe = HistoryFixture::new("checkout-blockers-safe");
        prepare_blockers(&raw);
        prepare_blockers(&safe);
        let raw_observation = unsafe { raw_checkout_blockers(&raw) };
        let safe_observation = safe_checkout_blockers(&safe);
        assert_eq!(raw_observation, safe_observation);
        assert_eq!(raw_observation[0], b"fixture\nwith a third revision\n");
    }
}

#[cfg(test)]
mod scheduled_symbol_tests {
    use super::*;

    #[test]
    fn deprecated_initializer_writes_current_checkout_defaults() {
        let mut options = crate::api::checkout::GitCheckoutOptions::new();
        git_checkout_init_options(&mut options.as_mut(), ffi::GIT_CHECKOUT_OPTIONS_VERSION)
            .unwrap();
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
    }

    #[test]
    fn current_initializer_returns_owned_checkout_options() {
        let options = git_checkout_options_init(ffi::GIT_CHECKOUT_OPTIONS_VERSION)
            .expect("the published checkout-options version initializes");
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
    }

    #[test]
    fn an_unsupported_version_yields_no_checkout_options_at_all() {
        // The rejection path reports through `git_error_set`, which needs the
        // thread state libgit2 installs at initialization.
        // SAFETY: libgit2 initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        // `git_error__check_version` accepts `0 < version <= current` only, so
        // both ends are refused and the wrapper hands back no options rather
        // than a partially written record.
        assert_eq!(git_checkout_options_init(0).err(), Some(-1));
        assert_eq!(
            git_checkout_options_init(ffi::GIT_CHECKOUT_OPTIONS_VERSION + 1).err(),
            Some(-1)
        );
        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

/// Wraps: git_checkout_options_init
/// Creates checkout options initialized for `version`.
pub fn git_checkout_options_init<'data>(
    version: core::ffi::c_uint,
) -> Result<ffibox::CVal<crate::api::checkout::GitCheckoutOptions<'data>>, i32> {
    let mut options = crate::api::checkout::GitCheckoutOptions::<'data>::new();
    // SAFETY: the inline options storage is exclusively writable and the C
    // initializer retains no pointer to it or to any of its cleared fields.
    let status = unsafe { ffi::git_checkout_options_init(options.as_mut().as_mut_ptr(), version) };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}
