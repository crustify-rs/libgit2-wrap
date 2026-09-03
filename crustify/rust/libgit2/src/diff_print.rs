//! Safe wrappers for libgit2 diff_print APIs.

use ffibox::CVal;

use crate::api::buffer::GitBuf;
use crate::api::diff::{DiffDeltaRef, GitDiffLineCallback};
use crate::diff::{DiffHunkRef, DiffLineRef};
use crate::ffi;
use crate::patch::GitPatchMut;

/// Wraps: git_patch_to_buf
/// Serializes a patch into a newly owned libgit2 buffer.
pub fn git_patch_to_buf(patch: &mut GitPatchMut<'_>) -> Result<CVal<GitBuf>, i32> {
    let mut output = GitBuf::new();
    let status = {
        let mut output = output.as_mut();
        // SAFETY: `output` is an empty, exclusively borrowed buffer header and
        // `patch` is exclusively borrowed for any lazy formatting updates.
        // On success the buffer owns its allocation; on error it remains valid
        // for `CVal` to dispose.
        unsafe { ffi::git_patch_to_buf(output.as_mut_ptr(), patch.as_mut_ptr()) }
    };
    if status == 0 { Ok(output) } else { Err(status) }
}

/// Wraps: git_patch_print
/// Visits every formatted line of `patch` with a typed callback.
pub fn git_patch_print<C>(patch: &mut GitPatchMut<'_>, callback: &mut C) -> Result<(), i32>
where
    C: GitDiffLineCallback,
{
    unsafe extern "C" fn trampoline<C>(
        delta: *const ffi::git_diff_delta,
        hunk: *const ffi::git_diff_hunk,
        line: *const ffi::git_diff_line,
        payload: *mut core::ffi::c_void,
    ) -> i32
    where
        C: GitDiffLineCallback,
    {
        if delta.is_null() || line.is_null() || payload.is_null() {
            return ffi::git_error_code_GIT_ERROR;
        }
        // SAFETY: the wrapper supplies this exact live callback as the
        // synchronous payload and grants exclusive access for the traversal.
        let callback = unsafe { &mut *payload.cast::<C>() };
        // SAFETY: libgit2 supplies live transient records for this callback;
        // `hunk` is explicitly optional for formatted header lines.
        let delta =
            unsafe { DiffDeltaRef::from_ptr(delta.cast_mut()) }.expect("checked non-null delta");
        // SAFETY: null is an allowed absent hunk; otherwise it is transiently
        // live for this callback invocation.
        let hunk = unsafe { DiffHunkRef::from_ptr(hunk.cast_mut()) };
        // SAFETY: `line` was checked and remains live for this invocation.
        let line =
            unsafe { DiffLineRef::from_ptr(line.cast_mut()) }.expect("checked non-null line");
        callback.call(delta, hunk, line)
    }

    // SAFETY: the patch and exclusive callback remain live for this fully
    // synchronous traversal, and neither pointer is retained afterwards.
    let status = unsafe {
        ffi::git_patch_print(
            patch.as_mut_ptr(),
            Some(trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_diff_print
/// Formats a diff and delivers each transient output line to `callback`.
pub fn git_diff_print(
    diff: &mut crate::diff::DiffMut<'_>,
    format: crate::diff::DiffFormat,
    callback: &mut dyn crate::api::diff::GitDiffLineCallback,
) -> Result<(), i32> {
    let mut callbacks = crate::diff::DiffCallbacks {
        file: None,
        binary: None,
        hunk: None,
        line: Some(callback),
    };
    // SAFETY: the diff is exclusive and the callback payload remains live for
    // the complete synchronous formatting operation. The trampoline catches
    // Rust panics and validates every transient C pointer.
    let status = unsafe {
        ffi::git_diff_print(
            diff.as_mut_ptr(),
            format.into(),
            Some(crate::diff::diff_line_trampoline),
            core::ptr::from_mut(&mut callbacks).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod print_tests {
    use super::*;

    struct Counter<'a>(&'a mut usize);

    impl GitDiffLineCallback for Counter<'_> {
        fn call(
            &mut self,
            _delta: DiffDeltaRef<'_>,
            _hunk: Option<DiffHunkRef<'_>>,
            _line: DiffLineRef<'_>,
        ) -> i32 {
            *self.0 += 1;
            0
        }
    }

    struct Stop(i32);

    impl GitDiffLineCallback for Stop {
        fn call(
            &mut self,
            _delta: DiffDeltaRef<'_>,
            _hunk: Option<DiffHunkRef<'_>>,
            _line: DiffLineRef<'_>,
        ) -> i32 {
            self.0
        }
    }

    #[test]
    fn patch_print_forwards_lines_and_callback_stop_codes() {
        // SAFETY: libgit2 initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut patch = crate::patch_generate::git_patch_from_buffers(
            b"old\n",
            Some(c"file"),
            b"new\n",
            Some(c"file"),
            None,
        )
        .unwrap();
        let mut lines = 0usize;
        git_patch_print(&mut patch.as_mut(), &mut Counter(&mut lines)).unwrap();
        assert!(lines >= 4);

        let stop = 7;
        assert_eq!(
            git_patch_print(&mut patch.as_mut(), &mut Stop(stop)),
            Err(stop)
        );
        drop(patch);
        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

/// Wraps: git_diff_status_char
/// Returns the conventional one-byte status marker for a delta kind.
#[must_use]
pub fn git_diff_status_char(status: crate::diff::Delta) -> u8 {
    // SAFETY: `status` is a published C discriminant and the function returns
    // one ASCII byte for every input.
    unsafe { ffi::git_diff_status_char(status.into()) as u8 }
}

/// Wraps: git_diff_to_buf
/// Formats a diff into a newly owned libgit2 buffer.
pub fn git_diff_to_buf(
    diff: &mut crate::diff::DiffMut<'_>,
    format: crate::diff::DiffFormat,
) -> Result<CVal<GitBuf>, i32> {
    let mut out = GitBuf::new();
    // SAFETY: both values are exclusively writable/live and the format is a
    // checked C discriminant; no pointer is retained.
    let status = unsafe {
        ffi::git_diff_to_buf(out.as_mut().as_mut_ptr(), diff.as_mut_ptr(), format.into())
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_status_tests {
    use super::*;
    #[test]
    fn status_markers_match_git_conventions() {
        assert_eq!(git_diff_status_char(crate::diff::Delta::Added), b'A');
        assert_eq!(git_diff_status_char(crate::diff::Delta::Deleted), b'D');
        assert_eq!(git_diff_status_char(crate::diff::Delta::Untracked), b'?');
    }
}

#[cfg(test)]
mod io_equiv {
    use super::*;
    use crate::io_equiv_support::{Libgit2Init, RawDiff};

    #[derive(Debug, Eq, PartialEq)]
    struct LineObservation {
        origin: core::ffi::c_char,
        has_hunk: bool,
        old_line: i32,
        new_line: i32,
        line_count: i32,
        content: Vec<u8>,
    }

    unsafe extern "C" fn raw_line_callback(
        _delta: *const ffi::git_diff_delta,
        hunk: *const ffi::git_diff_hunk,
        line: *const ffi::git_diff_line,
        payload: *mut core::ffi::c_void,
    ) -> i32 {
        if line.is_null() || payload.is_null() {
            return ffi::git_error_code_GIT_ERROR;
        }
        // SAFETY: libgit2 supplies the transient line and this test supplies a
        // live exclusive vector as the synchronous payload.
        let line = unsafe { &*line };
        let content = if line.content.is_null() || line.content_len == 0 {
            Vec::new()
        } else {
            // SAFETY: the callback contract supplies `content_len` readable
            // bytes for the complete invocation.
            unsafe { core::slice::from_raw_parts(line.content.cast::<u8>(), line.content_len) }
                .to_vec()
        };
        let observation = LineObservation {
            origin: line.origin,
            has_hunk: !hunk.is_null(),
            old_line: line.old_lineno,
            new_line: line.new_lineno,
            line_count: line.num_lines,
            content,
        };
        // SAFETY: `payload` is the live vector passed below and callbacks are
        // synchronous and non-reentrant for this diff.
        unsafe { &mut *payload.cast::<Vec<LineObservation>>() }.push(observation);
        0
    }

    struct SafeTrace<'a>(&'a mut Vec<LineObservation>);

    impl crate::api::diff::GitDiffLineCallback for SafeTrace<'_> {
        fn call(
            &mut self,
            _delta: DiffDeltaRef<'_>,
            hunk: Option<DiffHunkRef<'_>>,
            line: DiffLineRef<'_>,
        ) -> i32 {
            self.0.push(LineObservation {
                origin: line.origin().unwrap().as_char(),
                has_hunk: hunk.is_some(),
                old_line: line.old_lineno(),
                new_line: line.new_lineno(),
                line_count: line.num_lines(),
                content: line
                    .content()
                    .map(|content| content.elems().collect())
                    .unwrap_or_default(),
            });
            0
        }
    }

    #[test]
    fn io_equiv_git_diff_print() {
        let _init = Libgit2Init::acquire();
        let patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,2 +1,2 @@\n-old\n same\n+new\n";
        let raw = RawDiff::from_buffer(patch).unwrap();
        let mut raw_trace = Vec::new();
        // SAFETY: the raw diff and callback payload remain live throughout
        // this synchronous traversal.
        let raw_status = unsafe {
            ffi::git_diff_print(
                raw.as_ptr(),
                ffi::git_diff_format_t_GIT_DIFF_FORMAT_PATCH,
                Some(raw_line_callback),
                core::ptr::from_mut(&mut raw_trace).cast(),
            )
        };

        let mut safe = crate::diff_parse::git_diff_from_buffer(patch).unwrap();
        let mut safe_trace = Vec::new();
        let safe_status = git_diff_print(
            &mut safe.as_mut(),
            crate::diff::DiffFormat::Patch,
            &mut SafeTrace(&mut safe_trace),
        );

        assert_eq!(
            safe_status,
            if raw_status == 0 {
                Ok(())
            } else {
                Err(raw_status)
            }
        );
        assert_eq!(safe_trace, raw_trace);
    }
}
