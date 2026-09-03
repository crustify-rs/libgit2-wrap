//! Safe wrappers for FETCH_HEAD traversal.

use core::ffi::{CStr, c_char, c_void};

use crate::ffi;
use crate::oid::OidRef;
use crate::repository::{GitRepositoryFetchheadForeachCallback, GitRepositoryRef};

/// Wraps: git_repository_fetchhead_foreach
/// Visits each transient entry parsed from a repository's `FETCH_HEAD`.
pub fn git_repository_fetchhead_foreach<C>(
    repository: GitRepositoryRef<'_>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitRepositoryFetchheadForeachCallback,
{
    unsafe extern "C" fn trampoline<C: GitRepositoryFetchheadForeachCallback>(
        reference_name: *const c_char,
        remote_url: *const c_char,
        oid: *const ffi::git_oid,
        is_merge: u32,
        payload: *mut c_void,
    ) -> i32 {
        if oid.is_null() || payload.is_null() {
            return -1;
        }
        // SAFETY: the synchronous wrapper supplies this live exclusive
        // callback payload for the entire traversal.
        let callback = unsafe { &mut *payload.cast::<C>() };
        let string = |value: *const c_char| {
            if value.is_null() {
                None
            } else {
                // SAFETY: libgit2 documents each non-null callback string as
                // NUL-terminated and live for this invocation.
                Some(unsafe { CStr::from_ptr(value) })
            }
        };
        // SAFETY: the non-null OID is a live transient value for this call.
        let oid = unsafe { OidRef::from_ptr(oid.cast_mut()) }.expect("checked non-null");
        callback.call(
            string(reference_name),
            string(remote_url),
            oid,
            is_merge != 0,
        )
    }

    // SAFETY: the repository and callback remain live for this synchronous
    // traversal, and the trampoline reconstructs exactly `C` from the payload.
    let status = unsafe {
        ffi::git_repository_fetchhead_foreach(
            repository.as_ptr().cast_mut(),
            Some(trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    type Entry = (Option<Vec<u8>>, Option<Vec<u8>>, Vec<u8>, bool);

    fn write_fetch_head(fixture: &HistoryFixture) {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let id = String::from_utf8(output.stdout).unwrap();
        let id = id.trim();
        std::fs::write(
            fixture.directory.path().join(".git/FETCH_HEAD"),
            format!(
                "{id}\t\tbranch 'master' of https://example.invalid/source.git\n{id}\tnot-for-merge\tbranch 'topic' of https://example.invalid/source.git\n"
            ),
        )
        .unwrap();
    }

    unsafe fn raw_entries(repository: *mut ffi::git_repository) -> Vec<Entry> {
        unsafe extern "C" fn collect(
            name: *const c_char,
            url: *const c_char,
            id: *const ffi::git_oid,
            merge: u32,
            payload: *mut c_void,
        ) -> i32 {
            let output = unsafe { &mut *payload.cast::<Vec<Entry>>() };
            let bytes = |value: *const c_char| {
                (!value.is_null()).then(|| unsafe { CStr::from_ptr(value) }.to_bytes().to_vec())
            };
            output.push((
                bytes(name),
                bytes(url),
                unsafe { (*id).id }.to_vec(),
                merge != 0,
            ));
            0
        }
        let mut entries = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_repository_fetchhead_foreach(
                    repository,
                    Some(collect),
                    core::ptr::from_mut(&mut entries).cast(),
                )
            },
            0
        );
        entries
    }

    fn safe_entries(repository: *mut ffi::git_repository) -> Vec<Entry> {
        let repository = unsafe { GitRepositoryRef::from_ptr(repository) }.unwrap();
        let mut entries = Vec::new();
        git_repository_fetchhead_foreach(
            repository,
            &mut |name: Option<&CStr>, url: Option<&CStr>, id: OidRef<'_>, merge: bool| {
                entries.push((
                    name.map(|value| value.to_bytes().to_vec()),
                    url.map(|value| value.to_bytes().to_vec()),
                    id.raw_bytes().elems().collect(),
                    merge,
                ));
                0
            },
        )
        .unwrap();
        entries
    }

    #[test]
    fn io_equiv_fetchhead_traversal_merge_and_nonmerge_entries() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("fetchhead-raw");
        let safe = HistoryFixture::new("fetchhead-safe");
        write_fetch_head(&raw);
        write_fetch_head(&safe);
        let raw = unsafe { raw_entries(raw.repository.as_ptr()) };
        assert_eq!(raw, safe_entries(safe.repository.as_ptr()));
        assert_eq!(raw.len(), 2);
        assert!(raw[0].3);
        assert!(!raw[1].3);
    }
}
