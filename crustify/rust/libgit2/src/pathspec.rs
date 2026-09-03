//! Safe wrappers for libgit2 pathspec APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::NonNull;

use ffibox::{CBox, CDropped};

use crate::api::diff::DiffDeltaRef;
use crate::api::pathspec::GitPathspecFlags;
use crate::diff::{DiffMut, DiffRef};
use crate::ffi;
use crate::index::GitIndexMut;
use crate::repository::GitRepositoryMut;
use crate::strarray::GitStrArrayRef;
use crate::tree::GitTreeRef;

ffibox::define_ctype!(
    /// Wraps: git_pathspec
    /// An opaque compiled pathspec managed by libgit2.
    ///
    /// Each owner represents one reference count. Dropping it calls
    /// `git_pathspec_free`; libgit2 does not publish an operation for acquiring
    /// another count, so the owner intentionally does not implement `Clone`.
    GitPathspec,
    GitPathspecRef,
    GitPathspecMut,
    ffi::git_pathspec
);

/// An owned reference count to a compiled pathspec.
pub type GitPathspecOwned = CBox<GitPathspec>;

/// Wraps: git_pathspec_free
// SAFETY: `git_pathspec_free` consumes exactly one reference to a complete
// `git_pathspec`, releasing its owned fields and allocation when the final
// count is dropped. It accepts null, although `CBox` supplies a live non-null
// object exactly once.
unsafe impl CDropped for GitPathspec {
    unsafe fn c_drop(object: NonNull<Self>) {
        // SAFETY: the trait contract supplies one live owned pathspec count
        // and the wrapper is transparent over `ffi::git_pathspec`.
        unsafe { ffi::git_pathspec_free(object.as_ptr().cast()) }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_pathspec_match_list
    /// An opaque list of pathspec matches owned by libgit2.
    ///
    /// Dropping an owner releases the list, its retained pathspec reference,
    /// and its private arrays and pool. Accessors for diff-backed match lists
    /// must additionally preserve their source diff's lifetime.
    GitPathspecMatchList,
    GitPathspecMatchListRef,
    GitPathspecMatchListMut,
    ffi::git_pathspec_match_list
);

/// An owning handle to a complete pathspec match list.
pub type GitPathspecMatchListOwned = CBox<GitPathspecMatchList>;

/// Wraps: git_pathspec_match_list_free
// SAFETY: `git_pathspec_match_list_free` is the public destructor for a
// complete match-list allocation. It accepts null, although `CBox` supplies a
// live non-null object exactly once, and releases every owned resource before
// freeing the header.
unsafe impl CDropped for GitPathspecMatchList {
    unsafe fn c_drop(object: NonNull<Self>) {
        // SAFETY: the trait contract supplies one complete live match list and
        // the wrapper is transparent over the matching FFI type.
        unsafe { ffi::git_pathspec_match_list_free(object.as_ptr().cast()) }
    }
}

/// A diff-backed match list whose internal delta pointers cannot outlive the
/// diff that produced them.
///
/// [`git_pathspec_match_list_entrycount`] counts its entries, but
/// [`git_pathspec_match_list_entry`] always yields `None`: C gates that
/// accessor on `datatype == PATHSPEC_DATATYPE_STRINGS`. Diff-backed entries
/// are available through [`git_pathspec_match_list_diff_entry`]. Failure
/// entries are pool-owned strings and read back normally.
pub struct GitPathspecDiffMatchListOwned<'a> {
    inner: GitPathspecMatchListOwned,
    _diff: PhantomData<DiffRef<'a>>,
}

impl GitPathspecDiffMatchListOwned<'_> {
    /// Borrows the match list.
    #[must_use]
    pub fn as_ref(&self) -> GitPathspecMatchListRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the match list exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitPathspecMatchListMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_pathspec_match_diff
/// Builds a match list tied to the diff whose internal deltas it may borrow.
pub fn git_pathspec_match_diff<'a>(
    diff: &'a mut DiffMut<'_>,
    flags: GitPathspecFlags,
    pathspec: &mut GitPathspecMut<'_>,
) -> Result<GitPathspecDiffMatchListOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: `output` is a writable owner slot, while `diff` and `pathspec`
    // provide the mutable C access their declarations require. On success the
    // list owns a pathspec count and may borrow delta pointers from `diff`.
    let status = unsafe {
        ffi::git_pathspec_match_diff(
            core::ptr::addr_of_mut!(output),
            diff.as_mut_ptr(),
            flags.bits(),
            pathspec.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: a successful call initializes `output` with one complete owned
    // match list, and the wrapper below preserves its diff keepalive lifetime.
    let inner = unsafe { GitPathspecMatchListOwned::from_raw(output) }.ok_or(status)?;
    Ok(GitPathspecDiffMatchListOwned {
        inner,
        _diff: PhantomData,
    })
}

/// Wraps: git_pathspec_match_list_entry
/// Borrows a matched pathname, or returns `None` for an out-of-range or
/// diff-backed entry.
#[must_use]
pub fn git_pathspec_match_list_entry<'a>(
    matches: GitPathspecMatchListRef<'a>,
    position: usize,
) -> Option<&'a CStr> {
    // SAFETY: `matches` is live and shared; the returned pointer, when
    // non-null, addresses a NUL-terminated string owned by the list.
    let entry = unsafe { ffi::git_pathspec_match_list_entry(matches.as_ptr(), position) };
    if entry.is_null() {
        None
    } else {
        // SAFETY: libgit2 guarantees the non-null entry remains a valid C
        // string for the match list borrow.
        Some(unsafe { CStr::from_ptr(entry) })
    }
}

/// Wraps: git_pathspec_match_list_entrycount
/// Returns the number of matched entries stored in the list.
#[must_use]
pub fn git_pathspec_match_list_entrycount(matches: GitPathspecMatchListRef<'_>) -> usize {
    // SAFETY: `matches` is live and shared and the call retains no pointer.
    unsafe { ffi::git_pathspec_match_list_entrycount(matches.as_ptr()) }
}

/// Wraps: git_pathspec_match_list_failed_entry
/// Borrows an unmatched pathspec pattern by index.
#[must_use]
pub fn git_pathspec_match_list_failed_entry<'a>(
    matches: GitPathspecMatchListRef<'a>,
    position: usize,
) -> Option<&'a CStr> {
    // SAFETY: `matches` is live and shared; a non-null result is owned by the
    // match list's pool.
    let entry = unsafe { ffi::git_pathspec_match_list_failed_entry(matches.as_ptr(), position) };
    if entry.is_null() {
        None
    } else {
        // SAFETY: the pool-owned result is NUL-terminated and lives for `'a`.
        Some(unsafe { CStr::from_ptr(entry) })
    }
}

/// Wraps: git_pathspec_match_list_failed_entrycount
/// Returns the number of unmatched pathspec patterns stored in the list.
#[must_use]
pub fn git_pathspec_match_list_failed_entrycount(matches: GitPathspecMatchListRef<'_>) -> usize {
    // SAFETY: `matches` is live and shared and the call retains no pointer.
    unsafe { ffi::git_pathspec_match_list_failed_entrycount(matches.as_ptr()) }
}

/// Wraps: git_pathspec_match_workdir
/// Matches against a repository workdir and returns an independently owned
/// list of copied pathnames.
pub fn git_pathspec_match_workdir(
    repository: &mut GitRepositoryMut<'_>,
    flags: GitPathspecFlags,
    pathspec: &mut GitPathspecMut<'_>,
) -> Result<GitPathspecMatchListOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: `output` is writable and both input handles provide the access
    // required by C. The result copies matched names and owns a pathspec count.
    let status = unsafe {
        ffi::git_pathspec_match_workdir(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            flags.bits(),
            pathspec.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete owned match-list allocation.
    unsafe { GitPathspecMatchListOwned::from_raw(output) }.ok_or(status)
}

/// Wraps: git_pathspec_matches_path
/// Tests a required NUL-terminated pathname against a compiled pathspec.
#[must_use]
pub fn git_pathspec_matches_path(
    pathspec: GitPathspecRef<'_>,
    flags: GitPathspecFlags,
    path: &CStr,
) -> bool {
    // SAFETY: both borrowed inputs are live for the call and neither pointer
    // is retained.
    unsafe { ffi::git_pathspec_matches_path(pathspec.as_ptr(), flags.bits(), path.as_ptr()) != 0 }
}

/// Wraps: git_pathspec_new
/// Compiles a borrowed string array into a new owned pathspec.
pub fn git_pathspec_new(pathspec: GitStrArrayRef<'_>) -> Result<GitPathspecOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: `output` is writable and `pathspec` is a live shared array whose
    // strings libgit2 copies before returning.
    let status =
        unsafe { ffi::git_pathspec_new(core::ptr::addr_of_mut!(output), pathspec.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete owned pathspec count.
    unsafe { GitPathspecOwned::from_raw(output) }.ok_or(status)
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_types_preserve_the_c_seam_and_lifecycle_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitPathspec>();
        assert_dropped::<GitPathspec>();
        assert_eq!(size_of::<GitPathspec>(), size_of::<ffi::git_pathspec>());
        assert_eq!(align_of::<GitPathspec>(), align_of::<ffi::git_pathspec>());
        assert_eq!(
            size_of::<GitPathspecRef<'_>>(),
            size_of::<*const ffi::git_pathspec>()
        );
        assert_eq!(
            size_of::<GitPathspecMut<'_>>(),
            size_of::<*mut ffi::git_pathspec>()
        );
        assert_eq!(
            size_of::<GitPathspecOwned>(),
            size_of::<*mut ffi::git_pathspec>()
        );

        assert_cell::<GitPathspecMatchList>();
        assert_dropped::<GitPathspecMatchList>();
        assert_eq!(
            size_of::<GitPathspecMatchList>(),
            size_of::<ffi::git_pathspec_match_list>()
        );
        assert_eq!(
            align_of::<GitPathspecMatchList>(),
            align_of::<ffi::git_pathspec_match_list>()
        );
        assert_eq!(
            size_of::<GitPathspecMatchListRef<'_>>(),
            size_of::<*const ffi::git_pathspec_match_list>()
        );
        assert_eq!(
            size_of::<GitPathspecMatchListMut<'_>>(),
            size_of::<*mut ffi::git_pathspec_match_list>()
        );
        assert_eq!(
            size_of::<GitPathspecMatchListOwned>(),
            size_of::<*mut ffi::git_pathspec_match_list>()
        );
    }

    #[test]
    fn null_seams_create_no_pathspec_handles() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting any object.
        unsafe {
            assert!(GitPathspecRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPathspecMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPathspecOwned::from_raw(ptr::null_mut()).is_none());
            assert!(GitPathspecMatchListRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPathspecMatchListMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitPathspecMatchListOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn compiled_pathspec_matches_borrowed_paths_and_drops_cleanly() {
        // SAFETY: libgit2 initialization is refcounted and balanced after the
        // compiled pathspec has released its allocation.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let mut strings = [c"src/*.c".as_ptr().cast_mut()];
        let mut raw = ffi::git_strarray {
            strings: strings.as_mut_ptr(),
            count: strings.len(),
        };
        // SAFETY: the stack header, pointer run, and static C string remain
        // live and unchanged while the compiler copies their contents.
        let source = unsafe { GitStrArrayRef::from_ptr(core::ptr::addr_of_mut!(raw)) }.unwrap();
        let compiled = git_pathspec_new(source).expect("the pattern should compile");

        assert!(git_pathspec_matches_path(
            compiled.as_ref(),
            GitPathspecFlags::DEFAULT,
            c"src/main.c"
        ));
        assert!(!git_pathspec_matches_path(
            compiled.as_ref(),
            GitPathspecFlags::DEFAULT,
            c"README.md"
        ));
        drop(compiled);

        // SAFETY: balances the successful initialization after all libgit2
        // allocations created by this test have been released.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

/// Wraps: git_pathspec_match_index
/// Matches an index and returns an independently owned list of copied paths.
///
/// The index is taken exclusively: building the traversal calls
/// `git_index_snapshot_new`, which sorts the index's entry vector in place and
/// registers a reader on it for the duration of the walk.
pub fn git_pathspec_match_index(
    index: &mut GitIndexMut<'_>,
    flags: GitPathspecFlags,
    pathspec: GitPathspecRef<'_>,
) -> Result<GitPathspecMatchListOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable, the index is exclusively borrowed
    // for the in-place sort and snapshot reader the traversal takes, and the
    // shared pathspec stays live for it. The result owns its pathspec count
    // and copies matched index pathnames.
    let status = unsafe {
        ffi::git_pathspec_match_index(
            core::ptr::addr_of_mut!(output),
            index.as_mut_ptr(),
            flags.bits(),
            pathspec.as_ptr().cast_mut(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete match-list allocation.
    unsafe { GitPathspecMatchListOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_pathspec_match_tree
/// Matches a tree and returns an independently owned list of copied paths.
pub fn git_pathspec_match_tree(
    tree: GitTreeRef<'_>,
    flags: GitPathspecFlags,
    pathspec: GitPathspecRef<'_>,
) -> Result<GitPathspecMatchListOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and both shared inputs remain live
    // throughout the synchronous traversal; no input pointer is retained.
    let status = unsafe {
        ffi::git_pathspec_match_tree(
            core::ptr::addr_of_mut!(output),
            tree.as_ptr().cast_mut(),
            flags.bits(),
            pathspec.as_ptr().cast_mut(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete match-list allocation.
    unsafe { GitPathspecMatchListOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_pathspec_match_list_diff_entry
/// Borrows a matched delta from a diff-backed match list.
#[must_use]
pub fn git_pathspec_match_list_diff_entry<'a>(
    matches: GitPathspecMatchListRef<'a>,
    position: usize,
) -> Option<DiffDeltaRef<'a>> {
    // SAFETY: the shared match list remains live and owns its array of borrowed
    // delta pointers for the duration of this access.
    let delta = unsafe { ffi::git_pathspec_match_list_diff_entry(matches.as_ptr(), position) };
    // SAFETY: null denotes the wrong list kind or an out-of-range position.
    // Otherwise the delta remains valid for the match-list borrow, whose
    // owning wrapper itself retains the source diff lifetime.
    unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct PathspecObservation {
        direct: Vec<bool>,
        matches: Vec<Vec<u8>>,
        failures: Vec<Vec<u8>>,
    }

    fn patterns(values: &mut [*mut core::ffi::c_char; 3]) -> ffi::git_strarray {
        ffi::git_strarray {
            strings: values.as_mut_ptr(),
            count: values.len(),
        }
    }

    unsafe fn raw_observation(repository: *mut ffi::git_repository) -> PathspecObservation {
        let mut values = [
            c"src/*.c".as_ptr().cast_mut(),
            c"README.md".as_ptr().cast_mut(),
            c"missing/*.txt".as_ptr().cast_mut(),
        ];
        let array = patterns(&mut values);
        let mut pathspec = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_pathspec_new(&mut pathspec, &array) }, 0);
        let direct = [c"src/alpha.c", c"README.md", c"docs/nope.txt"]
            .into_iter()
            .map(|path| unsafe { ffi::git_pathspec_matches_path(pathspec, 0, path.as_ptr()) != 0 })
            .collect();
        let mut list = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_pathspec_match_workdir(
                    &mut list,
                    repository,
                    ffi::git_pathspec_flag_t_GIT_PATHSPEC_FIND_FAILURES,
                    pathspec,
                )
            },
            0
        );
        let matches = (0..unsafe { ffi::git_pathspec_match_list_entrycount(list) })
            .map(|index| {
                let entry = unsafe { ffi::git_pathspec_match_list_entry(list, index) };
                unsafe { CStr::from_ptr(entry) }.to_bytes().to_vec()
            })
            .collect();
        let failures = (0..unsafe { ffi::git_pathspec_match_list_failed_entrycount(list) })
            .map(|index| {
                let entry = unsafe { ffi::git_pathspec_match_list_failed_entry(list, index) };
                unsafe { CStr::from_ptr(entry) }.to_bytes().to_vec()
            })
            .collect();
        unsafe {
            ffi::git_pathspec_match_list_free(list);
            ffi::git_pathspec_free(pathspec);
        }
        PathspecObservation {
            direct,
            matches,
            failures,
        }
    }

    fn safe_observation(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
    ) -> PathspecObservation {
        let mut values = [
            c"src/*.c".as_ptr().cast_mut(),
            c"README.md".as_ptr().cast_mut(),
            c"missing/*.txt".as_ptr().cast_mut(),
        ];
        let mut array = patterns(&mut values);
        let array = unsafe { GitStrArrayRef::from_ptr(&mut array) }.unwrap();
        let mut pathspec = git_pathspec_new(array).unwrap();
        let direct = [c"src/alpha.c", c"README.md", c"docs/nope.txt"]
            .into_iter()
            .map(|path| {
                git_pathspec_matches_path(pathspec.as_ref(), GitPathspecFlags::DEFAULT, path)
            })
            .collect();
        let list = git_pathspec_match_workdir(
            repository,
            GitPathspecFlags::FIND_FAILURES,
            &mut pathspec.as_mut(),
        )
        .unwrap();
        let matches = (0..git_pathspec_match_list_entrycount(list.as_ref()))
            .map(|index| {
                git_pathspec_match_list_entry(list.as_ref(), index)
                    .unwrap()
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        let failures = (0..git_pathspec_match_list_failed_entrycount(list.as_ref()))
            .map(|index| {
                git_pathspec_match_list_failed_entry(list.as_ref(), index)
                    .unwrap()
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        PathspecObservation {
            direct,
            matches,
            failures,
        }
    }

    unsafe fn raw_repository_sources(
        repository: *mut ffi::git_repository,
    ) -> Vec<PathspecObservation> {
        let mut values = [
            c"src/*".as_ptr().cast_mut(),
            c"README.md".as_ptr().cast_mut(),
            c"missing/*.txt".as_ptr().cast_mut(),
        ];
        let array = patterns(&mut values);
        let mut pathspec = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_pathspec_new(&mut pathspec, &array) }, 0);

        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, repository) },
            0
        );
        let mut tree_object = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_revparse_single(&mut tree_object, repository, c"HEAD^{tree}".as_ptr())
            },
            0
        );
        let tree = tree_object.cast::<ffi::git_tree>();
        let mut diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_tree_to_workdir(&mut diff, repository, tree, core::ptr::null())
            },
            0
        );

        let flags = ffi::git_pathspec_flag_t_GIT_PATHSPEC_FIND_FAILURES;
        let mut lists = Vec::new();
        for source in 0..3 {
            let mut list = core::ptr::null_mut();
            let status = match source {
                0 => unsafe { ffi::git_pathspec_match_index(&mut list, index, flags, pathspec) },
                1 => unsafe { ffi::git_pathspec_match_tree(&mut list, tree, flags, pathspec) },
                _ => unsafe { ffi::git_pathspec_match_diff(&mut list, diff, flags, pathspec) },
            };
            assert_eq!(status, 0);
            let matches = (0..unsafe { ffi::git_pathspec_match_list_entrycount(list) })
                .map(|position| {
                    if source == 2 {
                        let delta =
                            unsafe { ffi::git_pathspec_match_list_diff_entry(list, position) };
                        assert!(!delta.is_null());
                        let file = unsafe { &(*delta).new_file };
                        let path = if file.path.is_null() {
                            unsafe { (*delta).old_file.path }
                        } else {
                            file.path
                        };
                        unsafe { CStr::from_ptr(path) }.to_bytes().to_vec()
                    } else {
                        let entry = unsafe { ffi::git_pathspec_match_list_entry(list, position) };
                        unsafe { CStr::from_ptr(entry) }.to_bytes().to_vec()
                    }
                })
                .collect();
            let failures = (0..unsafe { ffi::git_pathspec_match_list_failed_entrycount(list) })
                .map(|position| {
                    let entry =
                        unsafe { ffi::git_pathspec_match_list_failed_entry(list, position) };
                    unsafe { CStr::from_ptr(entry) }.to_bytes().to_vec()
                })
                .collect();
            lists.push(PathspecObservation {
                direct: Vec::new(),
                matches,
                failures,
            });
            unsafe { ffi::git_pathspec_match_list_free(list) };
        }

        unsafe {
            ffi::git_diff_free(diff);
            ffi::git_object_free(tree_object);
            ffi::git_index_free(index);
            ffi::git_pathspec_free(pathspec);
        }
        lists
    }

    fn safe_repository_sources(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
    ) -> Vec<PathspecObservation> {
        let mut values = [
            c"src/*".as_ptr().cast_mut(),
            c"README.md".as_ptr().cast_mut(),
            c"missing/*.txt".as_ptr().cast_mut(),
        ];
        let mut array = patterns(&mut values);
        let array = unsafe { GitStrArrayRef::from_ptr(&mut array) }.unwrap();
        let mut pathspec = git_pathspec_new(array).unwrap();

        let raw_repository = repository.as_mut_ptr();
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, raw_repository) },
            0
        );
        let mut tree_object = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_revparse_single(&mut tree_object, raw_repository, c"HEAD^{tree}".as_ptr())
            },
            0
        );
        let tree = tree_object.cast::<ffi::git_tree>();
        let mut diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_tree_to_workdir(&mut diff, raw_repository, tree, core::ptr::null())
            },
            0
        );

        let raw_index = index;
        let raw_diff = diff;
        let mut index = unsafe { GitIndexMut::from_ptr(raw_index) }.unwrap();
        let tree = unsafe { GitTreeRef::from_ptr(tree) }.unwrap();
        let mut diff = unsafe { DiffMut::from_ptr(diff) }.unwrap();
        let flags = GitPathspecFlags::FIND_FAILURES;
        let mut lists = Vec::new();
        for source in 0..3 {
            let observation = if source == 0 {
                let list = git_pathspec_match_index(&mut index, flags, pathspec.as_ref()).unwrap();
                safe_string_list(list.as_ref())
            } else if source == 1 {
                let list = git_pathspec_match_tree(tree, flags, pathspec.as_ref()).unwrap();
                safe_string_list(list.as_ref())
            } else {
                let list =
                    git_pathspec_match_diff(&mut diff, flags, &mut pathspec.as_mut()).unwrap();
                let matches = (0..git_pathspec_match_list_entrycount(list.as_ref()))
                    .map(|position| {
                        let delta = git_pathspec_match_list_diff_entry(list.as_ref(), position)
                            .expect("diff-backed match");
                        delta
                            .new_file()
                            .path()
                            .or_else(|| delta.old_file().path())
                            .unwrap()
                            .to_bytes()
                            .to_vec()
                    })
                    .collect();
                let failures = safe_failures(list.as_ref());
                PathspecObservation {
                    direct: Vec::new(),
                    matches,
                    failures,
                }
            };
            lists.push(observation);
        }
        unsafe {
            ffi::git_diff_free(raw_diff);
            ffi::git_object_free(tree_object);
            ffi::git_index_free(raw_index);
        }
        lists
    }

    fn safe_string_list(list: GitPathspecMatchListRef<'_>) -> PathspecObservation {
        let matches = (0..git_pathspec_match_list_entrycount(list))
            .map(|position| {
                git_pathspec_match_list_entry(list, position)
                    .unwrap()
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        PathspecObservation {
            direct: Vec::new(),
            matches,
            failures: safe_failures(list),
        }
    }

    fn safe_failures(list: GitPathspecMatchListRef<'_>) -> Vec<Vec<u8>> {
        (0..git_pathspec_match_list_failed_entrycount(list))
            .map(|position| {
                git_pathspec_match_list_failed_entry(list, position)
                    .unwrap()
                    .to_bytes()
                    .to_vec()
            })
            .collect()
    }

    #[test]
    fn io_equiv_pathspec_workdir_matching() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("pathspec-raw");
        let safe = HistoryFixture::new("pathspec-safe");
        let raw_observation = unsafe { raw_observation(raw.repository.as_ptr()) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(raw_observation, safe_observation(&mut safe_repository));
        assert_eq!(raw_observation.direct, [true, true, false]);
        assert_eq!(raw_observation.failures, [b"missing/*.txt".to_vec()]);
    }

    #[test]
    fn io_equiv_pathspec_index_tree_and_diff_matching() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("pathspec-sources-raw");
        let safe = HistoryFixture::new("pathspec-sources-safe");
        std::fs::write(raw.directory.path().join("README.md"), b"raw change\n").unwrap();
        std::fs::write(safe.directory.path().join("README.md"), b"raw change\n").unwrap();
        std::fs::write(raw.directory.path().join("src/gamma.c"), b"int gamma;\n").unwrap();
        std::fs::write(safe.directory.path().join("src/gamma.c"), b"int gamma;\n").unwrap();

        let raw_observation = unsafe { raw_repository_sources(raw.repository.as_ptr()) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(
            raw_observation,
            safe_repository_sources(&mut safe_repository)
        );
        assert!(
            raw_observation
                .iter()
                .all(|source| source.failures.contains(&b"missing/*.txt".to_vec()))
        );
    }
}

#[cfg(test)]
mod index_match_tests {
    use super::*;

    #[test]
    fn index_match_takes_the_index_exclusively() {
        // Building the traversal calls `git_index_snapshot_new`, which sorts
        // the index's entry vector in place and registers a reader on it, so
        // the index cannot be passed as a shared borrow.
        let _: fn(
            &mut GitIndexMut<'_>,
            GitPathspecFlags,
            GitPathspecRef<'_>,
        ) -> Result<GitPathspecMatchListOwned, i32> = git_pathspec_match_index;

        // SAFETY: libgit2 initialization is refcounted and balanced once every
        // allocation this test creates has been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let mut strings = [c"*".as_ptr().cast_mut()];
        let mut raw = ffi::git_strarray {
            strings: strings.as_mut_ptr(),
            count: strings.len(),
        };
        // SAFETY: the stack header, pointer run, and static C string remain
        // live and unchanged while the compiler copies their contents.
        let source = unsafe { GitStrArrayRef::from_ptr(core::ptr::addr_of_mut!(raw)) }.unwrap();
        let compiled = git_pathspec_new(source).expect("the pattern should compile");

        let mut index = crate::index::git_index_new().expect("an in-memory index");
        let matches = git_pathspec_match_index(
            &mut index.as_mut(),
            GitPathspecFlags::DEFAULT,
            compiled.as_ref(),
        )
        .expect("an empty index matches nothing without failing");
        assert_eq!(git_pathspec_match_list_entrycount(matches.as_ref()), 0);

        drop(matches);
        drop(index);
        drop(compiled);

        // SAFETY: balances the successful initialization after all libgit2
        // allocations created by this test have been released.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

#[cfg(test)]
mod diff_entry_tests {
    use super::*;

    #[test]
    fn diff_entry_borrow_is_tied_to_the_match_list() {
        let _: for<'a> fn(GitPathspecMatchListRef<'a>, usize) -> Option<DiffDeltaRef<'a>> =
            git_pathspec_match_list_diff_entry;
    }
}
