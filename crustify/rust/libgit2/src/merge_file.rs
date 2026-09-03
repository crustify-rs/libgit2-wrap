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

#[cfg(test)]
mod io_equiv {
    use super::*;
    use crate::io_equiv_support::Libgit2Init;
    use crate::merge::MergeFileInputMut;

    #[derive(Debug, Eq, PartialEq)]
    struct MergeObservation {
        automergeable: bool,
        path: Option<Vec<u8>>,
        mode: u32,
        contents: Vec<u8>,
    }

    #[test]
    fn io_equiv_git_merge_file() {
        let _init = Libgit2Init::acquire();
        let ancestor_bytes = b"line\n";
        let ours_bytes = b"ours\n";
        let theirs_bytes = b"theirs\n";
        let path = c"file.txt";

        let empty_input = || ffi::git_merge_file_input {
            version: 0,
            ptr: core::ptr::null(),
            size: 0,
            path: core::ptr::null(),
            mode: 0,
        };
        let mut raw_ancestor = empty_input();
        let mut raw_ours = empty_input();
        let mut raw_theirs = empty_input();
        for input in [&mut raw_ancestor, &mut raw_ours, &mut raw_theirs] {
            // SAFETY: each input is writable caller-owned storage.
            assert_eq!(unsafe { ffi::git_merge_file_input_init(input, 1) }, 0);
            input.path = path.as_ptr();
            input.mode = 0o100644;
        }
        raw_ancestor.ptr = ancestor_bytes.as_ptr().cast();
        raw_ancestor.size = ancestor_bytes.len();
        raw_ours.ptr = ours_bytes.as_ptr().cast();
        raw_ours.size = ours_bytes.len();
        raw_theirs.ptr = theirs_bytes.as_ptr().cast();
        raw_theirs.size = theirs_bytes.len();
        let mut raw_result = ffi::git_merge_file_result {
            automergeable: 0,
            path: core::ptr::null(),
            mode: 0,
            ptr: core::ptr::null(),
            len: 0,
        };
        // SAFETY: every raw descriptor and its borrowed bytes/path remain live
        // for the synchronous call; the output header is empty and writable.
        let raw_status = unsafe {
            ffi::git_merge_file(
                &mut raw_result,
                &raw_ancestor,
                &raw_ours,
                &raw_theirs,
                core::ptr::null(),
            )
        };
        assert_eq!(raw_status, 0);
        let raw_observation = MergeObservation {
            automergeable: raw_result.automergeable != 0,
            path: (!raw_result.path.is_null()).then(|| {
                // SAFETY: a successful result's non-null path is owned and
                // NUL-terminated until the result is freed below.
                unsafe { core::ffi::CStr::from_ptr(raw_result.path) }
                    .to_bytes()
                    .to_vec()
            }),
            mode: raw_result.mode,
            contents: if raw_result.ptr.is_null() {
                Vec::new()
            } else {
                // SAFETY: success initialized `len` result bytes at `ptr`.
                unsafe { core::slice::from_raw_parts(raw_result.ptr.cast::<u8>(), raw_result.len) }
                    .to_vec()
            },
        };

        let mut safe_ancestor = crate::merge::git_merge_file_input_init(1).unwrap();
        let mut safe_ours = crate::merge::git_merge_file_input_init(1).unwrap();
        let mut safe_theirs = crate::merge::git_merge_file_input_init(1).unwrap();
        for (input, contents) in [
            (&mut safe_ancestor, ancestor_bytes.as_slice()),
            (&mut safe_ours, ours_bytes.as_slice()),
            (&mut safe_theirs, theirs_bytes.as_slice()),
        ] {
            // SAFETY: the static path and local byte arrays outlive the input
            // descriptors and the complete merge operation.
            let mut input =
                unsafe { MergeFileInputMut::from_ptr(input as *mut _ as *mut _) }.unwrap();
            // SAFETY: the static path and local byte arrays outlive both the
            // descriptor and the synchronous merge below.
            unsafe {
                input.set_borrowed_path(Some(path));
                input.set_borrowed_contents(Some(contents));
            }
            input.set_mode(0o100644);
        }
        // SAFETY: each initialized local remains live for the complete call.
        let safe_ancestor =
            unsafe { MergeFileInputRef::from_ptr(&mut safe_ancestor as *mut _ as *mut _) }.unwrap();
        // SAFETY: as above, for the independently initialized ours input.
        let safe_ours =
            unsafe { MergeFileInputRef::from_ptr(&mut safe_ours as *mut _ as *mut _) }.unwrap();
        // SAFETY: as above, for the independently initialized theirs input.
        let safe_theirs =
            unsafe { MergeFileInputRef::from_ptr(&mut safe_theirs as *mut _ as *mut _) }.unwrap();
        let safe_result =
            git_merge_file(Some(safe_ancestor), safe_ours, safe_theirs, None).unwrap();
        let safe_view = safe_result.as_ref();
        let safe_observation = MergeObservation {
            automergeable: safe_view.is_automergeable(),
            path: safe_view.path().map(|value| value.to_bytes().to_vec()),
            mode: safe_view.mode(),
            contents: safe_view
                .contents()
                .map(|contents| contents.elems().collect())
                .unwrap_or_default(),
        };

        assert_eq!(safe_observation, raw_observation);
        drop(safe_result);
        // SAFETY: frees the raw result fields exactly once.
        unsafe { ffi::git_merge_file_result_free(&mut raw_result) };
    }
}
