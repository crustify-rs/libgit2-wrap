//! Safe wrappers for libgit2 patch_generate APIs.

use core::ffi::CStr;
use core::marker::PhantomData;

use crate::api::diff::GitDiffOptionsRef;
use crate::blob::GitBlobRef;
use crate::ffi;
use crate::patch::{GitPatchMut, GitPatchOwned, GitPatchRef};

/// An owned generated patch coupled to its source lifetime.
///
/// Buffer-backed patches retain spans into the source bytes. Blob-backed
/// patches acquire their own blob references, but those references retain the
/// same repository dependency as the source blobs. One conservative lifetime
/// keeps both forms from escaping their required inputs.
pub struct GitGeneratedPatch<'input> {
    patch: GitPatchOwned,
    _inputs: PhantomData<&'input ()>,
}

impl GitGeneratedPatch<'_> {
    /// Borrows the generated patch.
    #[must_use]
    pub fn as_ref(&self) -> GitPatchRef<'_> {
        self.patch.as_ref()
    }

    /// Borrows the generated patch exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitPatchMut<'_> {
        self.patch.as_mut()
    }
}

fn patch_result(status: i32, patch: Option<GitPatchOwned>) -> Result<GitPatchOwned, i32> {
    if status == 0 {
        patch.ok_or(ffi::git_error_code_GIT_ERROR)
    } else {
        drop(patch);
        Err(status)
    }
}

#[cfg(test)]
mod owner_result_tests {
    use super::*;

    #[test]
    fn typed_patch_result_rejects_a_null_success_output() {
        assert!(matches!(
            patch_result(0, None),
            Err(ffi::git_error_code_GIT_ERROR)
        ));
    }

    #[test]
    fn typed_patch_result_preserves_a_constructor_error() {
        assert!(matches!(patch_result(-7, None), Err(-7)));
    }
}

/// Wraps: git_patch_from_blob_and_buffer
/// Generates a patch from an optional blob and a borrowed byte buffer.
pub fn git_patch_from_blob_and_buffer<'input>(
    old_blob: Option<GitBlobRef<'input>>,
    old_path: Option<&CStr>,
    buffer: &'input [u8],
    buffer_path: Option<&CStr>,
    options: Option<GitDiffOptionsRef<'_, '_>>,
) -> Result<GitGeneratedPatch<'input>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable; every optional object, string and
    // options value is live for the call. Success transfers one complete patch
    // count. `buffer` stays borrowed because the patch retains spans into it.
    let (status, patch) = unsafe {
        let status = ffi::git_patch_from_blob_and_buffer(
            core::ptr::addr_of_mut!(raw),
            old_blob.map_or(core::ptr::null(), |blob| blob.as_ptr()),
            old_path.map_or(core::ptr::null(), |path| path.as_ptr()),
            buffer.as_ptr().cast(),
            buffer.len(),
            buffer_path.map_or(core::ptr::null(), |path| path.as_ptr()),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        let patch = (status == 0)
            .then(|| GitPatchOwned::from_raw(raw))
            .flatten();
        (status, patch)
    };
    Ok(GitGeneratedPatch {
        patch: patch_result(status, patch)?,
        _inputs: PhantomData,
    })
}

/// Wraps: git_patch_from_blobs
/// Generates a patch from two optional blobs, retaining their repository dependency.
pub fn git_patch_from_blobs<'input>(
    old_blob: Option<GitBlobRef<'input>>,
    old_path: Option<&CStr>,
    new_blob: Option<GitBlobRef<'input>>,
    new_path: Option<&CStr>,
    options: Option<GitDiffOptionsRef<'_, '_>>,
) -> Result<GitGeneratedPatch<'input>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable and all optional borrowed inputs
    // remain live for the call. Success transfers one complete patch count.
    // Libgit2 duplicates blob references and copies the retained paths.
    let (status, patch) = unsafe {
        let status = ffi::git_patch_from_blobs(
            core::ptr::addr_of_mut!(raw),
            old_blob.map_or(core::ptr::null(), |blob| blob.as_ptr()),
            old_path.map_or(core::ptr::null(), |path| path.as_ptr()),
            new_blob.map_or(core::ptr::null(), |blob| blob.as_ptr()),
            new_path.map_or(core::ptr::null(), |path| path.as_ptr()),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        let patch = (status == 0)
            .then(|| GitPatchOwned::from_raw(raw))
            .flatten();
        (status, patch)
    };
    Ok(GitGeneratedPatch {
        patch: patch_result(status, patch)?,
        _inputs: PhantomData,
    })
}

/// Wraps: git_patch_from_buffers
/// Generates a patch that borrows both source byte buffers.
pub fn git_patch_from_buffers<'input>(
    old_buffer: &'input [u8],
    old_path: Option<&CStr>,
    new_buffer: &'input [u8],
    new_path: Option<&CStr>,
    options: Option<GitDiffOptionsRef<'_, '_>>,
) -> Result<GitGeneratedPatch<'input>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable; strings and options are live for
    // the call, and success transfers one complete patch count. Both buffers
    // stay borrowed for the returned patch's life.
    let (status, patch) = unsafe {
        let status = ffi::git_patch_from_buffers(
            core::ptr::addr_of_mut!(raw),
            old_buffer.as_ptr().cast(),
            old_buffer.len(),
            old_path.map_or(core::ptr::null(), |path| path.as_ptr()),
            new_buffer.as_ptr().cast(),
            new_buffer.len(),
            new_path.map_or(core::ptr::null(), |path| path.as_ptr()),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        );
        let patch = (status == 0)
            .then(|| GitPatchOwned::from_raw(raw))
            .flatten();
        (status, patch)
    };
    Ok(GitGeneratedPatch {
        patch: patch_result(status, patch)?,
        _inputs: PhantomData,
    })
}

/// Wraps: git_diff_blobs
/// Compares two optional blobs and reports differences synchronously.
#[allow(clippy::too_many_arguments)]
pub fn git_diff_blobs<'callbacks>(
    old_blob: Option<crate::blob::GitBlobRef<'_>>,
    old_path: Option<&core::ffi::CStr>,
    new_blob: Option<crate::blob::GitBlobRef<'_>>,
    new_path: Option<&core::ffi::CStr>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
    file: Option<&'callbacks mut dyn crate::api::diff::GitDiffFileCallback>,
    binary: Option<&'callbacks mut dyn crate::api::diff::GitDiffBinaryCallback>,
    hunk: Option<&'callbacks mut dyn crate::api::diff::GitDiffHunkCallback>,
    line: Option<&'callbacks mut dyn crate::api::diff::GitDiffLineCallback>,
) -> Result<(), i32> {
    let old_blob = old_blob.map_or(core::ptr::null(), |blob| blob.as_ptr());
    let old_path = old_path.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    let new_blob = new_blob.map_or(core::ptr::null(), |blob| blob.as_ptr());
    let new_path = new_path.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    let mut callbacks = crate::diff::DiffCallbacks {
        file,
        binary,
        hunk,
        line,
    };
    let file = callbacks
        .file
        .as_ref()
        .map(|_| crate::diff::diff_file_trampoline as _);
    let binary = callbacks
        .binary
        .as_ref()
        .map(|_| crate::diff::diff_binary_trampoline as _);
    let hunk = callbacks
        .hunk
        .as_ref()
        .map(|_| crate::diff::diff_hunk_trampoline as _);
    let line = callbacks
        .line
        .as_ref()
        .map(|_| crate::diff::diff_line_trampoline as _);
    // SAFETY: every optional input is null or live, and callback state plus
    // payload remains exclusively borrowed for this synchronous comparison.
    let status = unsafe {
        crate::ffi::git_diff_blobs(
            old_blob,
            old_path,
            new_blob,
            new_path,
            options,
            file,
            binary,
            hunk,
            line,
            core::ptr::from_mut(&mut callbacks).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_buffer_patch_exposes_owned_patch_access() {
        // SAFETY: libgit2 initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let old = b"old\n";
        let new = b"new\n";
        let blob_patch = git_patch_from_blobs(None, None, None, None, None)
            .expect("two empty blob sides still produce a patch");
        assert_eq!(crate::patch::git_patch_num_hunks(blob_patch.as_ref()), 0);
        drop(blob_patch);
        let one_sided = git_patch_from_blob_and_buffer(None, None, new, None, None)
            .expect("an empty blob and one buffer produce a patch");
        assert_eq!(crate::patch::git_patch_num_hunks(one_sided.as_ref()), 1);
        drop(one_sided);
        let mut patch = git_patch_from_buffers(old, Some(c"a"), new, Some(c"a"), None)
            .expect("valid buffers produce a patch");
        assert_eq!(crate::patch::git_patch_num_hunks(patch.as_ref()), 1);
        assert_eq!(
            crate::patch::git_patch_num_lines_in_hunk(patch.as_ref(), 0).unwrap(),
            2
        );
        assert_eq!(
            crate::patch::git_patch_line_stats(patch.as_ref())
                .unwrap()
                .additions,
            1
        );
        let _ = patch.as_mut();
        drop(patch);
        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn direct_buffer_diffs_deliver_typed_callbacks() {
        // SAFETY: libgit2 initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let mut files = 0;
        let mut file = |_: crate::api::diff::DiffDeltaRef<'_>, _: f32| {
            files += 1;
            0
        };
        let mut lines = 0;
        let mut line = |_: crate::api::diff::DiffDeltaRef<'_>,
                        _: Option<crate::diff::DiffHunkRef<'_>>,
                        _: crate::diff::DiffLineRef<'_>| {
            lines += 1;
            0
        };
        git_diff_buffers(
            b"old\n",
            Some(c"file"),
            b"new\n",
            Some(c"file"),
            None,
            Some(&mut file),
            None,
            None,
            Some(&mut line),
        )
        .expect("valid buffers compare successfully");
        assert_eq!(files, 1);
        assert!(lines >= 2);

        git_diff_blob_to_buffer(
            None,
            None,
            b"new\n",
            Some(c"file"),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("an empty blob side compares with a buffer");

        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

/// Wraps: git_diff_blob_to_buffer
/// Compares an optional blob with a borrowed byte buffer and reports the
/// differences synchronously.
#[allow(clippy::too_many_arguments)]
pub fn git_diff_blob_to_buffer<'callbacks>(
    old_blob: Option<crate::blob::GitBlobRef<'_>>,
    old_path: Option<&core::ffi::CStr>,
    buffer: &[u8],
    buffer_path: Option<&core::ffi::CStr>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
    file: Option<&'callbacks mut dyn crate::api::diff::GitDiffFileCallback>,
    binary: Option<&'callbacks mut dyn crate::api::diff::GitDiffBinaryCallback>,
    hunk: Option<&'callbacks mut dyn crate::api::diff::GitDiffHunkCallback>,
    line: Option<&'callbacks mut dyn crate::api::diff::GitDiffLineCallback>,
) -> Result<(), i32> {
    let old_blob = old_blob.map_or(core::ptr::null(), |blob| blob.as_ptr());
    let old_path = old_path.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    let buffer_path = buffer_path.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    let mut callbacks = crate::diff::DiffCallbacks {
        file,
        binary,
        hunk,
        line,
    };
    let file = callbacks
        .file
        .as_ref()
        .map(|_| crate::diff::diff_file_trampoline as _);
    let binary = callbacks
        .binary
        .as_ref()
        .map(|_| crate::diff::diff_binary_trampoline as _);
    let hunk = callbacks
        .hunk
        .as_ref()
        .map(|_| crate::diff::diff_hunk_trampoline as _);
    let line = callbacks
        .line
        .as_ref()
        .map(|_| crate::diff::diff_line_trampoline as _);
    // SAFETY: all object, string and buffer inputs remain live for the call;
    // the buffer length bounds every C read, and callback state plus payload
    // stays exclusively borrowed until this synchronous comparison returns.
    let status = unsafe {
        ffi::git_diff_blob_to_buffer(
            old_blob,
            old_path,
            buffer.as_ptr().cast(),
            buffer.len(),
            buffer_path,
            options,
            file,
            binary,
            hunk,
            line,
            core::ptr::from_mut(&mut callbacks).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_diff_buffers
/// Compares two borrowed byte buffers and reports the differences
/// synchronously.
#[allow(clippy::too_many_arguments)]
pub fn git_diff_buffers<'callbacks>(
    old_buffer: &[u8],
    old_path: Option<&core::ffi::CStr>,
    new_buffer: &[u8],
    new_path: Option<&core::ffi::CStr>,
    options: Option<crate::api::diff::GitDiffOptionsRef<'_, '_>>,
    file: Option<&'callbacks mut dyn crate::api::diff::GitDiffFileCallback>,
    binary: Option<&'callbacks mut dyn crate::api::diff::GitDiffBinaryCallback>,
    hunk: Option<&'callbacks mut dyn crate::api::diff::GitDiffHunkCallback>,
    line: Option<&'callbacks mut dyn crate::api::diff::GitDiffLineCallback>,
) -> Result<(), i32> {
    let old_path = old_path.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    let new_path = new_path.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    let mut callbacks = crate::diff::DiffCallbacks {
        file,
        binary,
        hunk,
        line,
    };
    let file = callbacks
        .file
        .as_ref()
        .map(|_| crate::diff::diff_file_trampoline as _);
    let binary = callbacks
        .binary
        .as_ref()
        .map(|_| crate::diff::diff_binary_trampoline as _);
    let hunk = callbacks
        .hunk
        .as_ref()
        .map(|_| crate::diff::diff_hunk_trampoline as _);
    let line = callbacks
        .line
        .as_ref()
        .map(|_| crate::diff::diff_line_trampoline as _);
    // SAFETY: each slice pointer is readable for its paired length, optional
    // strings and options remain live, and callback state plus payload stays
    // exclusively borrowed until this synchronous comparison returns.
    let status = unsafe {
        ffi::git_diff_buffers(
            old_buffer.as_ptr().cast(),
            old_buffer.len(),
            old_path,
            new_buffer.as_ptr().cast(),
            new_buffer.len(),
            new_path,
            options,
            file,
            binary,
            hunk,
            line,
            core::ptr::from_mut(&mut callbacks).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}
