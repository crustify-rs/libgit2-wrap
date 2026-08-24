//! Safe wrappers for libgit2 merge_file APIs.

use ffibox::CVal;

use crate::merge::MergeFileResult;

/// Wraps: git_merge_file_result_free
/// Disposes an owned merge-file result and all allocations detached into it.
pub fn git_merge_file_result_free(result: CVal<MergeFileResult>) {
    drop(result);
}
