//! Safe wrappers for libgit2 pathspec APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::NonNull;

use ffibox::{CBox, CDropped};

use crate::diff::{DiffMut, DiffRef};
use crate::ffi;
use crate::index::GitIndexRef;
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
/// accessor on `datatype == PATHSPEC_DATATYPE_STRINGS`. The matching accessor
/// is `git_pathspec_match_list_diff_entry`, still unwrapped because it hands
/// back a `const git_diff_delta *` and `git_diff_delta` has no safe wrapper
/// yet. Failure entries are pool-owned strings and read back normally.
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
    flags: u32,
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
            flags,
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
    flags: u32,
    pathspec: &mut GitPathspecMut<'_>,
) -> Result<GitPathspecMatchListOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: `output` is writable and both input handles provide the access
    // required by C. The result copies matched names and owns a pathspec count.
    let status = unsafe {
        ffi::git_pathspec_match_workdir(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            flags,
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
pub fn git_pathspec_matches_path(pathspec: GitPathspecRef<'_>, flags: u32, path: &CStr) -> bool {
    // SAFETY: both borrowed inputs are live for the call and neither pointer
    // is retained.
    unsafe { ffi::git_pathspec_matches_path(pathspec.as_ptr(), flags, path.as_ptr()) != 0 }
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
mod tests {
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
            0,
            c"src/main.c"
        ));
        assert!(!git_pathspec_matches_path(
            compiled.as_ref(),
            0,
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
pub fn git_pathspec_match_index(
    index: GitIndexRef<'_>,
    flags: u32,
    pathspec: GitPathspecRef<'_>,
) -> Result<GitPathspecMatchListOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and both shared inputs remain live
    // for the synchronous traversal. The result owns its pathspec count and
    // copies matched index pathnames.
    let status = unsafe {
        ffi::git_pathspec_match_index(
            core::ptr::addr_of_mut!(output),
            index.as_ptr().cast_mut(),
            flags,
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
    flags: u32,
    pathspec: GitPathspecRef<'_>,
) -> Result<GitPathspecMatchListOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and both shared inputs remain live
    // throughout the synchronous traversal; no input pointer is retained.
    let status = unsafe {
        ffi::git_pathspec_match_tree(
            core::ptr::addr_of_mut!(output),
            tree.as_ptr().cast_mut(),
            flags,
            pathspec.as_ptr().cast_mut(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete match-list allocation.
    unsafe { GitPathspecMatchListOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}
