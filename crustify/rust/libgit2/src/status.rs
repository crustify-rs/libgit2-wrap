//! Safe wrappers for libgit2 status APIs.

use core::ffi::{CStr, c_void};
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use ffibox::CBox;

use crate::api::status::{GitStatusCallback, GitStatusOptionsMut, GitStatusOptionsRef};
use crate::ffi;
use crate::repository::GitRepositoryMut;

/// Wraps: git_status_show_t
/// Selects which comparisons libgit2 includes in a status scan.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum StatusShow {
    /// Compare both `HEAD` to the index and the index to the working directory.
    #[default]
    IndexAndWorkdir = ffi::git_status_show_t_GIT_STATUS_SHOW_INDEX_AND_WORKDIR,
    /// Compare `HEAD` to the index only.
    IndexOnly = ffi::git_status_show_t_GIT_STATUS_SHOW_INDEX_ONLY,
    /// Compare the index to the working directory only.
    WorkdirOnly = ffi::git_status_show_t_GIT_STATUS_SHOW_WORKDIR_ONLY,
}

/// An integer that is not a published [`StatusShow`] value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidStatusShow(ffi::git_status_show_t);

impl InvalidStatusShow {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_status_show_t {
        self.0
    }
}

impl From<StatusShow> for ffi::git_status_show_t {
    fn from(show: StatusShow) -> Self {
        show as Self
    }
}

impl TryFrom<ffi::git_status_show_t> for StatusShow {
    type Error = InvalidStatusShow;

    fn try_from(show: ffi::git_status_show_t) -> Result<Self, Self::Error> {
        match show {
            ffi::git_status_show_t_GIT_STATUS_SHOW_INDEX_AND_WORKDIR => Ok(Self::IndexAndWorkdir),
            ffi::git_status_show_t_GIT_STATUS_SHOW_INDEX_ONLY => Ok(Self::IndexOnly),
            ffi::git_status_show_t_GIT_STATUS_SHOW_WORKDIR_ONLY => Ok(Self::WorkdirOnly),
            value => Err(InvalidStatusShow(value)),
        }
    }
}

/// Wraps: git_status_t
/// A layout-compatible set of status flags for one path.
///
/// Unknown bits can be retained when carrying values from a newer libgit2,
/// while [`Self::from_bits`] validates values against the flags published by
/// the headers used to build this crate.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Status(ffi::git_status_t);

impl Status {
    /// The path is unchanged.
    pub const CURRENT: Self = Self(ffi::git_status_t_GIT_STATUS_CURRENT);
    /// The path is new in the index.
    pub const INDEX_NEW: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_NEW);
    /// The path is modified in the index.
    pub const INDEX_MODIFIED: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_MODIFIED);
    /// The path is deleted from the index.
    pub const INDEX_DELETED: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_DELETED);
    /// The path is renamed in the index.
    pub const INDEX_RENAMED: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_RENAMED);
    /// The path's type changed in the index.
    pub const INDEX_TYPECHANGE: Self = Self(ffi::git_status_t_GIT_STATUS_INDEX_TYPECHANGE);
    /// The path is new in the working directory.
    pub const WT_NEW: Self = Self(ffi::git_status_t_GIT_STATUS_WT_NEW);
    /// The path is modified in the working directory.
    pub const WT_MODIFIED: Self = Self(ffi::git_status_t_GIT_STATUS_WT_MODIFIED);
    /// The path is deleted from the working directory.
    pub const WT_DELETED: Self = Self(ffi::git_status_t_GIT_STATUS_WT_DELETED);
    /// The path's type changed in the working directory.
    pub const WT_TYPECHANGE: Self = Self(ffi::git_status_t_GIT_STATUS_WT_TYPECHANGE);
    /// The path is renamed in the working directory.
    pub const WT_RENAMED: Self = Self(ffi::git_status_t_GIT_STATUS_WT_RENAMED);
    /// The path cannot be read from the working directory.
    pub const WT_UNREADABLE: Self = Self(ffi::git_status_t_GIT_STATUS_WT_UNREADABLE);
    /// The path is ignored.
    pub const IGNORED: Self = Self(ffi::git_status_t_GIT_STATUS_IGNORED);
    /// The path is conflicted.
    pub const CONFLICTED: Self = Self(ffi::git_status_t_GIT_STATUS_CONFLICTED);
    /// Every status bit published by this version of libgit2.
    pub const ALL: Self = Self(
        Self::INDEX_NEW.0
            | Self::INDEX_MODIFIED.0
            | Self::INDEX_DELETED.0
            | Self::INDEX_RENAMED.0
            | Self::INDEX_TYPECHANGE.0
            | Self::WT_NEW.0
            | Self::WT_MODIFIED.0
            | Self::WT_DELETED.0
            | Self::WT_TYPECHANGE.0
            | Self::WT_RENAMED.0
            | Self::WT_UNREADABLE.0
            | Self::IGNORED.0
            | Self::CONFLICTED.0,
    );

    /// Converts raw bits when they contain only published status flags.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_status_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Retains all raw bits, including flags introduced by a newer libgit2.
    #[must_use]
    pub const fn from_bits_retain(bits: ffi::git_status_t) -> Self {
        Self(bits)
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_status_t {
        self.0
    }

    /// Returns whether the path has no changes.
    #[must_use]
    pub const fn is_current(self) -> bool {
        self.0 == Self::CURRENT.0
    }

    /// Returns whether every flag in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<ffi::git_status_t> for Status {
    fn from(bits: ffi::git_status_t) -> Self {
        Self::from_bits_retain(bits)
    }
}

impl From<Status> for ffi::git_status_t {
    fn from(status: Status) -> Self {
        status.bits()
    }
}

impl BitOr for Status {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Status {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Status {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Status {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for Status {
    type Output = Self;

    /// Complements within the flags published by this version of libgit2.
    ///
    /// Bits retained by [`Self::from_bits_retain`] but unknown to these
    /// headers are cleared rather than preserved, so `self & !self` is always
    /// empty.
    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

ffibox::define_ctype!(
    /// Wraps: git_status_list
    /// An opaque list of repository status entries managed by libgit2.
    ///
    /// Dropping an owner releases both optional diff objects, the collected
    /// entries, and the list allocation. Libgit2 publishes no operation for
    /// cloning a list or acquiring another ownership share.
    GitStatusList,
    GitStatusListRef,
    GitStatusListMut,
    ffi::git_status_list
);

/// An exclusively owned, fully constructed status list.
pub type GitStatusListOwned = CBox<GitStatusList>;

// SAFETY: `git_status_list_free` is the public destructor for a complete
// status-list allocation. It releases all owned fields and the allocation and
// accepts null, although `CBox` supplies one live non-null allocation once.
ffibox::impl_dropped!(
    GitStatusList,
    ffi::git_status_list,
    ffi::git_status_list_free
);

/// Wraps: git_status_file
/// Returns the status flags for one exact repository-relative path.
pub fn git_status_file(repository: &mut GitRepositoryMut<'_>, path: &CStr) -> Result<Status, i32> {
    let mut flags = 0;
    // SAFETY: the output slot and repository are exclusively borrowed and
    // `path` is a live C string. Libgit2 retains none of the pointers.
    let status = unsafe {
        ffi::git_status_file(
            core::ptr::addr_of_mut!(flags),
            repository.as_mut_ptr(),
            path.as_ptr(),
        )
    };
    if status == 0 {
        Ok(Status::from_bits_retain(flags))
    } else {
        Err(status)
    }
}

/// Wraps: git_status_list_entrycount
/// Returns the number of entries in a status list.
#[must_use]
pub fn git_status_list_entrycount(status: GitStatusListRef<'_>) -> usize {
    // SAFETY: the shared list is live; despite the historical non-const C
    // spelling, the body only reads the vector length and retains no pointer.
    unsafe { ffi::git_status_list_entrycount(status.as_ptr().cast_mut()) }
}

/// Wraps: git_status_should_ignore
/// Reports whether an exact repository-relative path is ignored.
pub fn git_status_should_ignore(
    repository: &mut GitRepositoryMut<'_>,
    path: &CStr,
) -> Result<bool, i32> {
    let mut ignored = 0;
    // SAFETY: the output slot and repository are exclusively borrowed and
    // `path` is a live C string; the callee retains no pointer.
    let status = unsafe {
        ffi::git_status_should_ignore(
            core::ptr::addr_of_mut!(ignored),
            repository.as_mut_ptr(),
            path.as_ptr(),
        )
    };
    if status == 0 {
        Ok(ignored != 0)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn status_show_values_round_trip_through_the_c_type() {
        for show in [
            StatusShow::IndexAndWorkdir,
            StatusShow::IndexOnly,
            StatusShow::WorkdirOnly,
        ] {
            let raw = ffi::git_status_show_t::from(show);
            assert_eq!(StatusShow::try_from(raw), Ok(show));
        }

        let invalid = ffi::git_status_show_t_GIT_STATUS_SHOW_WORKDIR_ONLY + 1;
        assert_eq!(StatusShow::try_from(invalid).unwrap_err().value(), invalid);
    }

    #[test]
    fn status_flags_combine_and_validate() {
        let status = Status::INDEX_MODIFIED | Status::WT_MODIFIED;
        assert!(status.contains(Status::INDEX_MODIFIED));
        assert!(status.intersects(Status::WT_MODIFIED));
        assert!(!status.intersects(Status::CONFLICTED));
        assert_eq!(Status::from_bits(status.bits()), Some(status));
        assert!(Status::CURRENT.is_current());
        assert!(Status::default().is_current());

        let unknown = Status::ALL.bits() + 1;
        assert_eq!(Status::from_bits(unknown), None);
        assert_eq!(Status::from_bits_retain(unknown).bits(), unknown);
    }

    #[test]
    fn status_complement_stays_inside_the_published_flags() {
        let known = Status::INDEX_NEW | Status::WT_NEW;
        assert_eq!(
            !known,
            Status::from_bits(Status::ALL.bits() & !known.bits()).unwrap()
        );
        assert!(!(!known).intersects(known));
        assert_eq!(!Status::CURRENT, Status::ALL);
        assert_eq!(!Status::ALL, Status::CURRENT);

        // Bit 5 is unassigned by these headers; a retained unknown bit must not
        // survive complementation, or `self & !self` would be non-empty.
        let retained = Status::from_bits_retain(known.bits() | (1 << 5));
        assert_eq!(Status::from_bits(retained.bits()), None);
        assert!(!(!retained).intersects(retained));
        assert_eq!(!retained, !known);
    }

    #[test]
    fn status_wrappers_preserve_the_c_enum_layouts() {
        assert_eq!(size_of::<StatusShow>(), size_of::<ffi::git_status_show_t>());
        assert_eq!(
            align_of::<StatusShow>(),
            align_of::<ffi::git_status_show_t>()
        );
        assert_eq!(size_of::<Status>(), size_of::<ffi::git_status_t>());
        assert_eq!(align_of::<Status>(), align_of::<ffi::git_status_t>());
    }

    #[test]
    fn status_list_preserves_the_opaque_c_seam_and_lifecycle() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitStatusList>();
        assert_dropped::<GitStatusList>();
        assert_eq!(
            size_of::<GitStatusList>(),
            size_of::<ffi::git_status_list>()
        );
        assert_eq!(
            align_of::<GitStatusList>(),
            align_of::<ffi::git_status_list>()
        );
        assert_eq!(
            size_of::<GitStatusListRef<'_>>(),
            size_of::<*const ffi::git_status_list>()
        );
        assert_eq!(
            size_of::<GitStatusListMut<'_>>(),
            size_of::<*mut ffi::git_status_list>()
        );
        assert_eq!(
            size_of::<GitStatusListOwned>(),
            size_of::<*mut ffi::git_status_list>()
        );
    }

    #[test]
    fn null_status_list_seams_create_no_handle() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitStatusListRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitStatusListMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitStatusListOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// Wraps: git_status_init_options
/// Initializes status options through the deprecated compatibility spelling.
pub fn git_status_init_options(
    options: &mut GitStatusOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle exposes writable layout-compatible options
    // storage, and initialization retains no pointer.
    let status = unsafe { ffi::git_status_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_status_list_new
/// Gathers repository status into a newly owned list.
pub fn git_status_list_new(
    repository: &mut GitRepositoryMut<'_>,
    options: Option<GitStatusOptionsRef<'_, '_>>,
) -> Result<GitStatusListOwned, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null(), |value| value.as_ptr());
    // SAFETY: `out` is writable, the repository is exclusively borrowed while
    // C may refresh its index, and all nested option borrows remain live for
    // this synchronous call. The result retains owned diffs, not those borrows.
    let status = unsafe { ffi::git_status_list_new(&mut out, repository.as_mut_ptr(), options) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized status-list allocation.
    unsafe { GitStatusListOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod options_initializer_tests {
    use super::*;
    use crate::api::status::GitStatusOptions;

    #[test]
    fn deprecated_initializer_writes_published_defaults() {
        let mut storage = GitStatusOptions::<'static>::new();
        let mut options = storage.as_mut();
        git_status_init_options(&mut options, ffi::GIT_STATUS_OPTIONS_VERSION).unwrap();
        assert_eq!(options.as_ref().version(), ffi::GIT_STATUS_OPTIONS_VERSION);
        assert_eq!(options.as_ref().show(), Ok(StatusShow::IndexAndWorkdir));
    }
}

/// Wraps: git_status_byindex
/// Borrows the entry at `index`, returning `None` when it is out of bounds.
#[must_use]
pub fn git_status_byindex<'a>(
    status: GitStatusListRef<'a>,
    index: usize,
) -> Option<crate::api::status::GitStatusEntryRef<'a>> {
    // SAFETY: the live list is only indexed; the returned entry is owned by
    // that list and this wrapper carries the list borrow into the result.
    let entry = unsafe { ffi::git_status_byindex(status.as_ptr().cast_mut(), index) };
    // SAFETY: null means out of range; otherwise the entry remains live for
    // the source list borrow.
    unsafe { crate::api::status::GitStatusEntryRef::from_ptr(entry.cast_mut()) }
}

unsafe extern "C" fn status_trampoline<C: GitStatusCallback>(
    path: *const core::ffi::c_char,
    status: ffi::git_status_t,
    payload: *mut c_void,
) -> i32 {
    // SAFETY: the traversal returns the exact callback pointer installed by
    // the wrapper and invokes it only before that wrapper returns.
    let callback = unsafe { &mut *payload.cast::<C>() };
    // SAFETY: libgit2 supplies a non-null transient NUL-terminated path for
    // this invocation. Unknown status bits are retained without invalid enums.
    let path = unsafe { CStr::from_ptr(path) };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        callback.call(path, Status::from_bits_retain(status))
    }))
    .unwrap_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_status_foreach
/// Visits each path with its status under libgit2's default options.
pub fn git_status_foreach<C>(
    repository: &mut GitRepositoryMut<'_>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitStatusCallback,
{
    // SAFETY: the repository and callback remain live for the synchronous
    // traversal; the payload type matches the trampoline and is not retained.
    let status = unsafe {
        ffi::git_status_foreach(
            repository.as_mut_ptr(),
            Some(status_trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_status_foreach_ext
/// Visits each selected path according to `options`.
pub fn git_status_foreach_ext<C>(
    repository: &mut GitRepositoryMut<'_>,
    options: Option<GitStatusOptionsRef<'_, '_>>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitStatusCallback,
{
    // SAFETY: the repository and callback are exclusively borrowed for the
    // synchronous traversal; all nested option borrows remain live and no
    // callback or payload pointer is retained.
    let status = unsafe {
        ffi::git_status_foreach_ext(
            repository.as_mut_ptr(),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
            Some(status_trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_status_options_init
/// Creates status options initialized for `version`.
pub fn git_status_options_init<'data>(
    version: core::ffi::c_uint,
) -> Result<ffibox::CVal<crate::api::status::GitStatusOptions<'data>>, i32> {
    let mut options = crate::api::status::GitStatusOptions::<'data>::new();
    // SAFETY: the inline options storage is exclusively writable and C
    // retains no pointer to it.
    let status = unsafe { ffi::git_status_options_init(options.as_mut().as_mut_ptr(), version) };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod current_options_initializer_tests {
    use super::*;

    #[test]
    fn current_initializer_writes_published_defaults() {
        let options = git_status_options_init(ffi::GIT_STATUS_OPTIONS_VERSION)
            .expect("the published status options version initializes");
        assert_eq!(options.as_ref().version(), ffi::GIT_STATUS_OPTIONS_VERSION);
        assert_eq!(options.as_ref().show(), Ok(StatusShow::IndexAndWorkdir));
    }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    fn prepare(fixture: &HistoryFixture) {
        std::fs::rename(
            fixture.directory.path().join("README.md"),
            fixture.directory.path().join("README-renamed.md"),
        )
        .unwrap();
        std::fs::write(
            fixture.directory.path().join("src/alpha.c"),
            b"int alpha(void) { return 99; }\n",
        )
        .unwrap();
        std::fs::remove_file(fixture.directory.path().join("src/beta.c")).unwrap();
        std::fs::write(fixture.directory.path().join("staged-new.txt"), b"staged\n").unwrap();
        std::fs::write(
            fixture.directory.path().join(".gitignore"),
            b"ignored/\n*.tmp\n",
        )
        .unwrap();
        std::fs::create_dir_all(fixture.directory.path().join("ignored/nested")).unwrap();
        std::fs::write(
            fixture.directory.path().join("ignored/nested/data.bin"),
            b"ignored\n",
        )
        .unwrap();
        std::fs::create_dir_all(fixture.directory.path().join("untracked/nested")).unwrap();
        std::fs::write(
            fixture.directory.path().join("untracked/nested/file.txt"),
            b"untracked\n",
        )
        .unwrap();
        std::fs::write(fixture.directory.path().join("scratch.tmp"), b"ignored\n").unwrap();
        let status = std::process::Command::new("git")
            .current_dir(fixture.directory.path())
            .args([
                "add",
                "README.md",
                "README-renamed.md",
                "staged-new.txt",
                ".gitignore",
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn path(pointer: *const core::ffi::c_char) -> Vec<u8> {
        if pointer.is_null() {
            Vec::new()
        } else {
            unsafe { core::ffi::CStr::from_ptr(pointer) }
                .to_bytes()
                .to_vec()
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    struct StatusObservation {
        entries: Vec<(u32, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)>,
        alpha: u32,
        ignored: [bool; 3],
        callback: Vec<(Vec<u8>, u32)>,
        stopped: i32,
    }

    unsafe fn raw_observe(repository: *mut ffi::git_repository) -> StatusObservation {
        let mut options = unsafe { core::mem::zeroed::<ffi::git_status_options>() };
        assert_eq!(
            unsafe { ffi::git_status_options_init(&mut options, ffi::GIT_STATUS_OPTIONS_VERSION) },
            0
        );
        options.flags = ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_UNTRACKED
            | ffi::git_status_opt_t_GIT_STATUS_OPT_RECURSE_UNTRACKED_DIRS
            | ffi::git_status_opt_t_GIT_STATUS_OPT_INCLUDE_IGNORED
            | ffi::git_status_opt_t_GIT_STATUS_OPT_RECURSE_IGNORED_DIRS
            | ffi::git_status_opt_t_GIT_STATUS_OPT_RENAMES_HEAD_TO_INDEX
            | ffi::git_status_opt_t_GIT_STATUS_OPT_RENAMES_INDEX_TO_WORKDIR
            | ffi::git_status_opt_t_GIT_STATUS_OPT_RENAMES_FROM_REWRITES
            | ffi::git_status_opt_t_GIT_STATUS_OPT_SORT_CASE_SENSITIVELY;
        options.rename_threshold = 45;
        let mut list = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_status_list_new(&mut list, repository, &options) },
            0
        );
        let count = unsafe { ffi::git_status_list_entrycount(list) };
        let mut entries = Vec::new();
        for index in 0..count {
            let entry = unsafe { ffi::git_status_byindex(list, index) };
            let (head_old, head_new) = if unsafe { (*entry).head_to_index }.is_null() {
                (Vec::new(), Vec::new())
            } else {
                let delta = unsafe { (*entry).head_to_index };
                (
                    path(unsafe { (*delta).old_file.path }),
                    path(unsafe { (*delta).new_file.path }),
                )
            };
            let (work_old, work_new) = if unsafe { (*entry).index_to_workdir }.is_null() {
                (Vec::new(), Vec::new())
            } else {
                let delta = unsafe { (*entry).index_to_workdir };
                (
                    path(unsafe { (*delta).old_file.path }),
                    path(unsafe { (*delta).new_file.path }),
                )
            };
            entries.push((
                unsafe { (*entry).status },
                head_old,
                head_new,
                work_old,
                work_new,
            ));
        }
        unsafe { ffi::git_status_list_free(list) };
        let mut alpha = 0;
        assert_eq!(
            unsafe { ffi::git_status_file(&mut alpha, repository, c"src/alpha.c".as_ptr()) },
            0
        );
        let ignored = [
            c"ignored/nested/data.bin",
            c"scratch.tmp",
            c"untracked/nested/file.txt",
        ]
        .map(|candidate| {
            let mut value = 0;
            assert_eq!(
                unsafe {
                    ffi::git_status_should_ignore(&mut value, repository, candidate.as_ptr())
                },
                0
            );
            value != 0
        });
        unsafe extern "C" fn collect(
            path: *const core::ffi::c_char,
            status: u32,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            let output = unsafe { &mut *payload.cast::<Vec<(Vec<u8>, u32)>>() };
            output.push((
                unsafe { core::ffi::CStr::from_ptr(path) }
                    .to_bytes()
                    .to_vec(),
                status,
            ));
            0
        }
        let mut callback = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_status_foreach_ext(
                    repository,
                    &options,
                    Some(collect),
                    core::ptr::from_mut(&mut callback).cast(),
                )
            },
            0
        );
        unsafe extern "C" fn stop(
            _: *const core::ffi::c_char,
            _: u32,
            _: *mut core::ffi::c_void,
        ) -> i32 {
            73
        }
        let stopped = unsafe {
            ffi::git_status_foreach_ext(repository, &options, Some(stop), core::ptr::null_mut())
        };
        StatusObservation {
            entries,
            alpha,
            ignored,
            callback,
            stopped,
        }
    }

    fn safe_observe(repository: *mut ffi::git_repository) -> StatusObservation {
        use crate::api::status::GitStatusOptionFlags;
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut options = git_status_options_init(ffi::GIT_STATUS_OPTIONS_VERSION).unwrap();
        options.as_mut().set_flags(
            GitStatusOptionFlags::INCLUDE_UNTRACKED
                | GitStatusOptionFlags::RECURSE_UNTRACKED_DIRS
                | GitStatusOptionFlags::INCLUDE_IGNORED
                | GitStatusOptionFlags::RECURSE_IGNORED_DIRS
                | GitStatusOptionFlags::RENAMES_HEAD_TO_INDEX
                | GitStatusOptionFlags::RENAMES_INDEX_TO_WORKDIR
                | GitStatusOptionFlags::RENAMES_FROM_REWRITES
                | GitStatusOptionFlags::SORT_CASE_SENSITIVELY,
        );
        options.as_mut().set_rename_threshold(45);
        let list = git_status_list_new(&mut repository, Some(options.as_ref())).unwrap();
        let mut entries = Vec::new();
        for index in 0..git_status_list_entrycount(list.as_ref()) {
            let entry = git_status_byindex(list.as_ref(), index).unwrap();
            let head = entry.head_to_index();
            let work = entry.index_to_workdir();
            entries.push((
                entry.status().bits(),
                head.and_then(|delta| delta.old_file().path())
                    .map_or_else(Vec::new, |value| value.to_bytes().to_vec()),
                head.and_then(|delta| delta.new_file().path())
                    .map_or_else(Vec::new, |value| value.to_bytes().to_vec()),
                work.and_then(|delta| delta.old_file().path())
                    .map_or_else(Vec::new, |value| value.to_bytes().to_vec()),
                work.and_then(|delta| delta.new_file().path())
                    .map_or_else(Vec::new, |value| value.to_bytes().to_vec()),
            ));
        }
        drop(list);
        let alpha = git_status_file(&mut repository, c"src/alpha.c")
            .unwrap()
            .bits();
        let ignored = [
            c"ignored/nested/data.bin",
            c"scratch.tmp",
            c"untracked/nested/file.txt",
        ]
        .map(|candidate| git_status_should_ignore(&mut repository, candidate).unwrap());
        let mut callback = Vec::new();
        git_status_foreach_ext(
            &mut repository,
            Some(options.as_ref()),
            &mut |path: &core::ffi::CStr, status: Status| {
                callback.push((path.to_bytes().to_vec(), status.bits()));
                0
            },
        )
        .unwrap();
        let stopped = git_status_foreach_ext(
            &mut repository,
            Some(options.as_ref()),
            &mut |_: &core::ffi::CStr, _: Status| 73,
        )
        .err()
        .unwrap();
        StatusObservation {
            entries,
            alpha,
            ignored,
            callback,
            stopped,
        }
    }

    #[test]
    fn io_equiv_status_lists_renames_ignored_paths_and_callbacks() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("status-raw");
        let safe = HistoryFixture::new("status-safe");
        prepare(&raw);
        prepare(&safe);
        let raw_observation = unsafe { raw_observe(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_observe(safe.repository.as_ptr()));
        assert_eq!(raw_observation.ignored, [true, true, false]);
        assert!(!raw_observation.entries.is_empty());
    }
}
