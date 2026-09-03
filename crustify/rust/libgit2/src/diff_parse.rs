//! Safe wrappers for libgit2 diff_parse APIs.

use crate::diff::DiffOwned;
use crate::ffi;

/// Wraps: git_diff_from_buffer
/// Parses a Git patch from a counted byte buffer.
pub fn git_diff_from_buffer(content: &[u8]) -> Result<DiffOwned, i32> {
    let mut out = core::ptr::null_mut();
    let content_ptr = if content.is_empty() {
        b"".as_ptr()
    } else {
        content.as_ptr()
    };
    // SAFETY: `content` supplies exactly `content.len()` readable bytes and C
    // retains no pointer into it. `out` is a writable result slot.
    let status = unsafe { ffi::git_diff_from_buffer(&mut out, content_ptr.cast(), content.len()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one complete owned diff reference whose drop
    // contract calls `git_diff_free`.
    unsafe { DiffOwned::from_raw(out) }.ok_or(-1)
}

/// Wraps: git_diff_from_buffer_ext
/// Parses a patch using explicit parse options.
pub fn git_diff_from_buffer_ext(
    content: &[u8],
    options: &mut crate::api::diff::DiffParseOptionsMut<'_>,
) -> Result<DiffOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output is writable, content covers its exact byte length, and
    // options is exclusively borrowed for this non-retaining parse.
    let status = unsafe {
        ffi::git_diff_from_buffer_ext(
            &mut out,
            content.as_ptr().cast(),
            content.len(),
            options.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete owned diff.
    unsafe { DiffOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Holds one libgit2 initialization count for the duration of a test.
    ///
    /// Parsing reaches libgit2's allocator and thread-local error state,
    /// neither of which exists before `git_libgit2_init`; without this guard
    /// the test only survives when an unrelated test happens to hold a count.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and
            // refcounted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard, after every libgit2 owner has already been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    #[test]
    fn parses_a_minimal_patch_into_an_owned_diff() {
        let _libgit2 = Libgit2Init::acquire();
        let patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n";
        let mut diff = git_diff_from_buffer(patch).expect("valid patch");
        assert_eq!(crate::diff::git_diff_num_deltas(diff.as_ref()), 1);
        let options = crate::diff::git_diff_patchid_options_init().unwrap();
        let _oid = crate::diff::git_diff_patchid(&mut diff.as_mut(), Some(options)).unwrap();
    }
}

#[cfg(test)]
mod io_equiv {
    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, RawBuf, RawDiff, safe_buf_bytes};

    #[derive(Debug, Eq, PartialEq)]
    struct DeltaObservation {
        status: ffi::git_delta_t,
        old_path: Option<Vec<u8>>,
        new_path: Option<Vec<u8>>,
        old_mode: u32,
        new_mode: u32,
        file_count: u16,
    }

    unsafe fn raw_path(path: *const core::ffi::c_char) -> Option<Vec<u8>> {
        if path.is_null() {
            None
        } else {
            // SAFETY: the caller supplies a path owned by a live raw diff.
            Some(
                unsafe { core::ffi::CStr::from_ptr(path) }
                    .to_bytes()
                    .to_vec(),
            )
        }
    }

    #[test]
    fn io_equiv_git_diff_from_buffer() {
        let _init = Libgit2Init::acquire();
        let patch = b"diff --git a/a.txt b/a.txt\nindex 3367afd..3e75765 100644\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-old\n+new\n";

        let raw = RawDiff::from_buffer(patch).unwrap();
        // SAFETY: the successful raw parse owns at least the single delta
        // reported below, and retains its paths for the owner's lifetime.
        let raw_delta = unsafe { ffi::git_diff_get_delta(raw.as_ptr(), 0) };
        assert!(!raw_delta.is_null());
        // SAFETY: checked non-null and kept alive by `raw`.
        let raw_delta = unsafe { &*raw_delta };
        let raw_observation = DeltaObservation {
            status: raw_delta.status,
            // SAFETY: both paths, when present, belong to the live raw diff.
            old_path: unsafe { raw_path(raw_delta.old_file.path) },
            // SAFETY: as above, for the independently nullable new path.
            new_path: unsafe { raw_path(raw_delta.new_file.path) },
            old_mode: raw_delta.old_file.mode.into(),
            new_mode: raw_delta.new_file.mode.into(),
            file_count: raw_delta.nfiles,
        };

        let safe = git_diff_from_buffer(patch).unwrap();
        assert_eq!(crate::diff::git_diff_num_deltas(safe.as_ref()), 1);
        let safe_delta = crate::diff::git_diff_get_delta(safe.as_ref(), 0).unwrap();
        let safe_observation = DeltaObservation {
            status: safe_delta.status().unwrap().into(),
            old_path: safe_delta
                .old_file()
                .path()
                .map(|value| value.to_bytes().to_vec()),
            new_path: safe_delta
                .new_file()
                .path()
                .map(|value| value.to_bytes().to_vec()),
            old_mode: safe_delta.old_file().mode().unwrap().as_raw(),
            new_mode: safe_delta.new_file().mode().unwrap().as_raw(),
            file_count: safe_delta.file_count(),
        };

        assert_eq!(safe_observation, raw_observation);
    }

    fn complex_patch(fixture: &HistoryFixture) -> Vec<u8> {
        let path = fixture.directory.path();
        std::fs::write(
            path.join("binary.dat"),
            (0..2048)
                .map(|value| (value % 251) as u8)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let status = std::process::Command::new("git")
            .current_dir(path)
            .args(["add", "binary.dat"])
            .status()
            .unwrap();
        assert!(status.success());
        let status = std::process::Command::new("git")
            .current_dir(path)
            .args([
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "binary baseline",
            ])
            .env("GIT_AUTHOR_DATE", "1700300000 +0000")
            .env("GIT_COMMITTER_DATE", "1700300000 +0000")
            .status()
            .unwrap();
        assert!(status.success());
        std::fs::rename(path.join("README.md"), path.join("RENAMED.md")).unwrap();
        std::fs::write(
            path.join("RENAMED.md"),
            b"fixture\nwith a third revision\nand renamed content\n",
        )
        .unwrap();
        let mut binary = (0..2048)
            .map(|value| (value % 251) as u8)
            .collect::<Vec<_>>();
        binary[100..180].fill(0);
        binary.extend_from_slice(b"binary tail\0\xff");
        std::fs::write(path.join("binary.dat"), binary).unwrap();
        std::fs::remove_file(path.join("src/beta.c")).unwrap();
        std::fs::write(path.join("no-newline.txt"), b"no trailing newline").unwrap();
        let status = std::process::Command::new("git")
            .current_dir(path)
            .args(["add", "-A"])
            .status()
            .unwrap();
        assert!(status.success());
        let output = std::process::Command::new("git")
            .current_dir(path)
            .args([
                "diff",
                "--cached",
                "--binary",
                "--full-index",
                "--find-renames",
                "HEAD",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        output.stdout
    }

    #[derive(Debug, Eq, PartialEq)]
    struct ComplexObservation {
        deltas: Vec<(ffi::git_delta_t, Vec<u8>, Vec<u8>, u32, u32)>,
        formats: Vec<Vec<u8>>,
        stats: (usize, usize, usize),
        stat_formats: Vec<Vec<u8>>,
    }

    unsafe fn raw_complex(patch: &[u8]) -> ComplexObservation {
        let mut diff = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_diff_from_buffer(&mut diff, patch.as_ptr().cast(), patch.len()) },
            0
        );
        let deltas = (0..unsafe { ffi::git_diff_num_deltas(diff) })
            .map(|position| {
                let delta = unsafe { ffi::git_diff_get_delta(diff, position) };
                let old = unsafe { raw_path((*delta).old_file.path) }.unwrap_or_default();
                let new = unsafe { raw_path((*delta).new_file.path) }.unwrap_or_default();
                (
                    unsafe { (*delta).status },
                    old,
                    new,
                    unsafe { (*delta).old_file.mode }.into(),
                    unsafe { (*delta).new_file.mode }.into(),
                )
            })
            .collect();
        let formats = [
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH,
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH_HEADER,
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_RAW,
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_NAME_ONLY,
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_NAME_STATUS,
            ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH_ID,
        ]
        .map(|format| {
            let mut output = RawBuf::new();
            assert_eq!(
                unsafe { ffi::git_diff_to_buf(&mut output.0, diff, format) },
                0
            );
            output.bytes()
        })
        .to_vec();
        let mut stats = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_diff_get_stats(&mut stats, diff) }, 0);
        let counts = (
            unsafe { ffi::git_diff_stats_files_changed(stats) },
            unsafe { ffi::git_diff_stats_insertions(stats) },
            unsafe { ffi::git_diff_stats_deletions(stats) },
        );
        let stat_formats = [
            ffi::git_diff_stats_format_t_GIT_DIFF_STATS_FULL
                | ffi::git_diff_stats_format_t_GIT_DIFF_STATS_INCLUDE_SUMMARY,
            ffi::git_diff_stats_format_t_GIT_DIFF_STATS_SHORT,
            ffi::git_diff_stats_format_t_GIT_DIFF_STATS_NUMBER,
        ]
        .map(|format| {
            let mut output = RawBuf::new();
            assert_eq!(
                unsafe { ffi::git_diff_stats_to_buf(&mut output.0, stats, format, 60) },
                0
            );
            output.bytes()
        })
        .to_vec();
        unsafe {
            ffi::git_diff_stats_free(stats);
            ffi::git_diff_free(diff);
        }
        ComplexObservation {
            deltas,
            formats,
            stats: counts,
            stat_formats,
        }
    }

    fn safe_complex(patch: &[u8]) -> ComplexObservation {
        let mut diff = git_diff_from_buffer(patch).unwrap();
        let deltas = (0..crate::diff::git_diff_num_deltas(diff.as_ref()))
            .map(|position| {
                let delta = crate::diff::git_diff_get_delta(diff.as_ref(), position).unwrap();
                (
                    delta.status().unwrap().into(),
                    delta
                        .old_file()
                        .path()
                        .map_or_else(Vec::new, |value| value.to_bytes().to_vec()),
                    delta
                        .new_file()
                        .path()
                        .map_or_else(Vec::new, |value| value.to_bytes().to_vec()),
                    delta.old_file().mode().unwrap().as_raw(),
                    delta.new_file().mode().unwrap().as_raw(),
                )
            })
            .collect();
        let formats = [
            crate::diff::DiffFormat::Patch,
            crate::diff::DiffFormat::PatchHeader,
            crate::diff::DiffFormat::Raw,
            crate::diff::DiffFormat::NameOnly,
            crate::diff::DiffFormat::NameStatus,
            crate::diff::DiffFormat::PatchId,
        ]
        .map(|format| {
            safe_buf_bytes(
                crate::diff_print::git_diff_to_buf(&mut diff.as_mut(), format)
                    .unwrap()
                    .as_ref(),
            )
        })
        .to_vec();
        let stats = crate::diff_stats::git_diff_get_stats(&mut diff.as_mut()).unwrap();
        let counts = (
            crate::diff_stats::git_diff_stats_files_changed(stats.as_ref()),
            crate::diff_stats::git_diff_stats_insertions(stats.as_ref()),
            crate::diff_stats::git_diff_stats_deletions(stats.as_ref()),
        );
        let stat_formats = [
            crate::diff::DiffStatsFormat::FULL | crate::diff::DiffStatsFormat::INCLUDE_SUMMARY,
            crate::diff::DiffStatsFormat::SHORT,
            crate::diff::DiffStatsFormat::NUMBER,
        ]
        .map(|format| {
            let mut output = crate::api::buffer::GitBuf::new();
            crate::diff_stats::git_diff_stats_to_buf(&mut output, stats.as_ref(), format, 60)
                .unwrap();
            safe_buf_bytes(output.as_ref())
        })
        .to_vec();
        ComplexObservation {
            deltas,
            formats,
            stats: counts,
            stat_formats,
        }
    }

    #[test]
    fn io_equiv_complex_binary_rename_delete_and_no_newline_patch_rendering() {
        let _init = Libgit2Init::acquire();
        let fixture = HistoryFixture::new("complex-patch");
        let patch = complex_patch(&fixture);
        let raw = unsafe { raw_complex(&patch) };
        assert_eq!(raw, safe_complex(&patch));
        assert!(raw.deltas.len() >= 4);
        assert!(
            raw.formats[0]
                .windows(b"GIT binary patch".len())
                .any(|window| window == b"GIT binary patch")
        );
    }
}
