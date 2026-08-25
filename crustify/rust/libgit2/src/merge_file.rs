//! Safe wrappers for libgit2 merge_file APIs.

use ffibox::CVal;

use crate::ffi;
use crate::index::IndexEntryRef;
use crate::merge::{MergeFileInputRef, MergeFileOptionsRef, MergeFileResult, MergeFileResultMut};
use crate::repository::GitRepositoryRef;

/// Wraps: git_merge_file_result_free
/// Disposes an owned merge-file result and all allocations detached into it.
pub fn git_merge_file_result_free(result: CVal<MergeFileResult>) {
    drop(result);
}

/// Wraps: git_merge_file
/// Merges in-memory file inputs into an owned result header.
pub fn git_merge_file(
    ancestor: Option<MergeFileInputRef<'_>>,
    ours: MergeFileInputRef<'_>,
    theirs: MergeFileInputRef<'_>,
    options: Option<MergeFileOptionsRef<'_>>,
) -> Result<CVal<MergeFileResult>, i32> {
    let mut result = MergeFileResult::new();
    let status = {
        let mut out: MergeFileResultMut<'_> = result.as_mut();
        // SAFETY: `out` is an empty exclusively borrowed result header and all
        // optional/non-null input handles stay live for this non-retaining
        // merge. Success installs allocations owned by `result`.
        unsafe {
            ffi::git_merge_file(
                out.as_mut_ptr(),
                ancestor.map_or(core::ptr::null(), |value| value.as_ptr()),
                ours.as_ptr(),
                theirs.as_ptr(),
                options.map_or(core::ptr::null(), |value| value.as_ptr()),
            )
        }
    };
    if status == 0 { Ok(result) } else { Err(status) }
}

/// Wraps: git_merge_file_from_index
/// Merges up to three staged index entries into an owned result header.
pub fn git_merge_file_from_index(
    repository: GitRepositoryRef<'_>,
    ancestor: Option<IndexEntryRef<'_>>,
    ours: Option<IndexEntryRef<'_>>,
    theirs: Option<IndexEntryRef<'_>>,
    options: Option<MergeFileOptionsRef<'_>>,
) -> Result<CVal<MergeFileResult>, i32> {
    if ancestor.is_none() && ours.is_none() && theirs.is_none() {
        return Err(-1);
    }
    let mut result = MergeFileResult::new();
    let status = {
        let mut out = result.as_mut();
        // SAFETY: `out` is empty writable owned storage, the repository and
        // every present borrowed handle remain live, and C retains none of
        // them after constructing the result.
        unsafe {
            ffi::git_merge_file_from_index(
                out.as_mut_ptr(),
                repository.as_ptr().cast_mut(),
                ancestor.map_or(core::ptr::null(), |value| value.as_ptr()),
                ours.map_or(core::ptr::null(), |value| value.as_ptr()),
                theirs.map_or(core::ptr::null(), |value| value.as_ptr()),
                options.map_or(core::ptr::null(), |value| value.as_ptr()),
            )
        }
    };
    if status == 0 { Ok(result) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_wrapper_tests {
    use super::*;
    use crate::merge::MergeFileInputMut;

    #[test]
    fn identical_in_memory_files_produce_an_owned_result() {
        // SAFETY: libgit2 initialization is refcounted and balanced after the
        // result owner has disposed its allocations.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut ours = crate::merge::git_merge_file_input_init(1).unwrap();
        let mut theirs = crate::merge::git_merge_file_input_init(1).unwrap();
        // SAFETY: each local input is initialized, exclusively accessed, and
        // the static content bytes outlive the complete merge call.
        unsafe {
            MergeFileInputMut::from_ptr(core::ptr::addr_of_mut!(ours).cast())
                .unwrap()
                .set_borrowed_contents(Some(b"same\n"));
            MergeFileInputMut::from_ptr(core::ptr::addr_of_mut!(theirs).cast())
                .unwrap()
                .set_borrowed_contents(Some(b"same\n"));
        }
        // SAFETY: both initialized locals remain live and shared for the call.
        let ours =
            unsafe { MergeFileInputRef::from_ptr(core::ptr::addr_of_mut!(ours).cast()) }.unwrap();
        // SAFETY: as above, for the independent `theirs` input.
        let theirs =
            unsafe { MergeFileInputRef::from_ptr(core::ptr::addr_of_mut!(theirs).cast()) }.unwrap();
        let result = git_merge_file(None, ours, theirs, None).unwrap();
        assert!(result.as_ref().is_automergeable());
        assert_eq!(result.as_ref().len(), 5);
        assert_eq!(result.as_ref().contents().unwrap().elem(0), Some(b's'));
        drop(result);
        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
