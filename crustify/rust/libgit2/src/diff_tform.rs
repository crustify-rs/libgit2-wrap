//! Safe wrappers for libgit2 diff_tform APIs.

use crate::diff::{DiffMut, DiffRef};
use crate::ffi;

/// Wraps: git_diff_merge
/// Merges every delta from `from` into `onto`.
pub fn git_diff_merge(onto: &mut DiffMut<'_>, from: DiffRef<'_>) -> Result<(), i32> {
    // SAFETY: `onto` is exclusively borrowed, `from` is shared and distinct,
    // and C copies all retained data into `onto` before returning.
    let status = unsafe { ffi::git_diff_merge(onto.as_mut_ptr(), from.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_diff_find_similar
/// Detects renames and copies, transforming `diff` in place.
pub fn git_diff_find_similar(
    diff: &mut DiffMut<'_>,
    options: Option<crate::api::diff::DiffFindOptionsRef<'_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the diff is exclusively borrowed and the optional options and
    // custom metric it names remain live for the synchronous transformation.
    let status = unsafe { ffi::git_diff_find_similar(diff.as_mut_ptr(), options) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct SimilarObservation {
        deltas: Vec<(ffi::git_delta_t, Vec<u8>, Vec<u8>, u16)>,
        renamed: usize,
        patch_id: Vec<u8>,
    }

    fn stage_rename(fixture: &HistoryFixture) {
        std::fs::rename(
            fixture.directory.path().join("src/beta.c"),
            fixture.directory.path().join("src/gamma.c"),
        )
        .unwrap();
        std::fs::write(
            fixture.directory.path().join("src/alpha.c"),
            b"int alpha(void) { return 42; }\n",
        )
        .unwrap();
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args(["add", "-A"])
            .status()
            .unwrap();
        assert!(status.success());
    }

    unsafe fn raw_similar(repository: *mut ffi::git_repository) -> SimilarObservation {
        let mut tree = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut tree, repository, c"HEAD^{tree}".as_ptr()) },
            0
        );
        let mut diff = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_diff_tree_to_index(
                    &mut diff,
                    repository,
                    tree.cast(),
                    core::ptr::null_mut(),
                    core::ptr::null(),
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_diff_find_similar(diff, core::ptr::null()) },
            0
        );
        let mut deltas = Vec::new();
        for index in 0..unsafe { ffi::git_diff_num_deltas(diff) } {
            let delta = unsafe { ffi::git_diff_get_delta(diff, index) };
            let bytes = |path: *const core::ffi::c_char| {
                if path.is_null() {
                    Vec::new()
                } else {
                    unsafe { core::ffi::CStr::from_ptr(path) }
                        .to_bytes()
                        .to_vec()
                }
            };
            deltas.push((
                unsafe { (*delta).status },
                bytes(unsafe { (*delta).old_file.path }),
                bytes(unsafe { (*delta).new_file.path }),
                unsafe { (*delta).similarity },
            ));
        }
        let renamed =
            unsafe { ffi::git_diff_num_deltas_of_type(diff, ffi::git_delta_t_GIT_DELTA_RENAMED) };
        let mut patch_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_diff_patchid(&mut patch_id, diff, core::ptr::null_mut()) },
            0
        );
        unsafe {
            ffi::git_diff_free(diff);
            ffi::git_object_free(tree);
        }
        SimilarObservation {
            deltas,
            renamed,
            patch_id: patch_id.id.to_vec(),
        }
    }

    fn safe_similar(repository: *mut ffi::git_repository) -> SimilarObservation {
        let mut tree = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut tree, repository, c"HEAD^{tree}".as_ptr()) },
            0
        );
        let tree_ref = unsafe { crate::tree::GitTreeRef::from_ptr(tree.cast()) }.unwrap();
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut diff = crate::diff_generate::git_diff_tree_to_index(
            &mut repository_view,
            Some(tree_ref),
            None,
            None,
        )
        .unwrap();
        git_diff_find_similar(&mut diff.as_mut(), None).unwrap();
        let mut deltas = Vec::new();
        for index in 0..crate::diff::git_diff_num_deltas(diff.as_ref()) {
            let delta = crate::diff::git_diff_get_delta(diff.as_ref(), index).unwrap();
            deltas.push((
                delta.status().unwrap().into(),
                delta
                    .old_file()
                    .path()
                    .map_or_else(Vec::new, |path| path.to_bytes().to_vec()),
                delta
                    .new_file()
                    .path()
                    .map_or_else(Vec::new, |path| path.to_bytes().to_vec()),
                delta.similarity(),
            ));
        }
        let renamed =
            crate::diff::git_diff_num_deltas_of_type(diff.as_ref(), crate::diff::Delta::Renamed);
        let mut patch_id = crate::diff::git_diff_patchid(&mut diff.as_mut(), None).unwrap();
        let patch_id = unsafe {
            crate::oid::OidRef::from_ptr(core::ptr::from_mut(&mut patch_id).cast::<ffi::git_oid>())
        }
        .unwrap()
        .raw_bytes()
        .elems()
        .collect();
        unsafe { ffi::git_object_free(tree) };
        SimilarObservation {
            deltas,
            renamed,
            patch_id,
        }
    }

    #[test]
    fn io_equiv_find_renames_and_compute_patch_identity() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("diff-similar-raw");
        let safe = HistoryFixture::new("diff-similar-safe");
        stage_rename(&raw);
        stage_rename(&safe);
        let raw_observation = unsafe { raw_similar(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_similar(safe.repository.as_ptr()));
        assert_eq!(raw_observation.renamed, 1);
    }
}
