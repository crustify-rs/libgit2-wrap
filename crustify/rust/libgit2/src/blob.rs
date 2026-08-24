//! Safe wrappers for libgit2 blob APIs.

use core::ptr::{NonNull, addr_of_mut};

use ffibox::{CBox, CCloned};

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_blob
    /// An opaque, reference-counted blob managed by libgit2.
    ///
    /// Owned handles release one cache reference with `git_blob_free`.
    /// Cloning an owned handle uses `git_blob_dup` to acquire an independent
    /// reference to the same blob.
    GitBlob,
    GitBlobRef,
    GitBlobMut,
    ffi::git_blob
);

/// An owned reference to a libgit2 blob.
pub type GitBlobOwned = CBox<GitBlob>;

// SAFETY: `git_blob_free` consumes one reference to a fully initialized blob
// and releases the allocation only when its cache reference count reaches zero.
ffibox::impl_dropped!(GitBlob, ffi::git_blob, ffi::git_blob_free);

// SAFETY: `git_blob_dup` increments the live source blob's cache reference
// count and writes the same pointer to its required output slot. That new
// reference is independently released by `GitBlob`'s `CDropped` contract.
unsafe impl CCloned for GitBlob {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live blob, `duplicate` is a
        // valid output slot, and `GitBlob` is layout-compatible with
        // `ffi::git_blob`.
        let result = unsafe {
            ffi::git_blob_dup(
                addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_blob>(),
            )
        };
        debug_assert_eq!(result, 0);
        NonNull::new(duplicate.cast::<Self>())
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_blob_preserves_layout_and_refcount_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_refcounted<T: CDropped + CCloned>() {}

        assert_cell::<GitBlob>();
        assert_refcounted::<GitBlob>();
        assert_eq!(size_of::<GitBlob>(), size_of::<ffi::git_blob>());
        assert_eq!(align_of::<GitBlob>(), align_of::<ffi::git_blob>());
        assert_eq!(
            size_of::<GitBlobRef<'_>>(),
            size_of::<*const ffi::git_blob>()
        );
        assert_eq!(size_of::<GitBlobMut<'_>>(), size_of::<*mut ffi::git_blob>());
        assert_eq!(size_of::<GitBlobOwned>(), size_of::<*mut ffi::git_blob>());
    }

    #[test]
    fn null_blob_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitBlobRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitBlobMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitBlobOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// A blob write stream tied to the repository pointer retained by libgit2.
pub struct GitBlobWriteStream<'repo> {
    inner: crate::api::types::GitWriteStreamOwned,
    _repository: core::marker::PhantomData<crate::repository::GitRepositoryRef<'repo>>,
}

impl GitBlobWriteStream<'_> {
    /// Borrows the stream exclusively for writing.
    #[must_use]
    pub fn as_mut(&mut self) -> crate::api::types::GitWriteStreamMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_blob_create_frombuffer
/// Writes `buffer` as a blob and stores its ID in `id`.
pub fn git_blob_create_frombuffer(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    buffer: &[u8],
) -> Result<(), i32> {
    // SAFETY: both handles are exclusive and live, and `buffer` supplies the
    // exact readable length retained only for this call.
    let status = unsafe {
        ffi::git_blob_create_frombuffer(
            id.as_mut_ptr(),
            repo.as_mut_ptr(),
            buffer.as_ptr().cast(),
            buffer.len(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_blob_create_fromdisk
/// Creates a blob from the file at `path`.
pub fn git_blob_create_fromdisk(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    path: &core::ffi::CStr,
) -> Result<(), i32> {
    // SAFETY: the output and repository handles are live and exclusive, and
    // `path` is a live NUL-terminated string for the synchronous call.
    let status =
        unsafe { ffi::git_blob_create_fromdisk(id.as_mut_ptr(), repo.as_mut_ptr(), path.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_blob_create_fromstream
/// Opens a stream that retains a borrow of `repo` until it is committed or dropped.
pub fn git_blob_create_fromstream<'repo>(
    mut repo: crate::repository::GitRepositoryMut<'repo>,
    hint_path: Option<&core::ffi::CStr>,
) -> Result<GitBlobWriteStream<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let hint_path = hint_path.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `repo` is live and exclusive, `hint_path` is null or a live C
    // string, and `out` is writable. The returned type carries the repo borrow.
    let status = unsafe { ffi::git_blob_create_fromstream(&mut out, repo.as_mut_ptr(), hint_path) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers a fully initialized stream whose installed
    // `free` callback is its destructor.
    let inner = unsafe { crate::api::types::GitWriteStreamOwned::from_raw(out) }
        .expect("libgit2 succeeded without returning a write stream");
    Ok(GitBlobWriteStream {
        inner,
        _repository: core::marker::PhantomData,
    })
}

/// Wraps: git_blob_create_fromstream_commit
/// Commits and consumes a blob stream, storing the resulting object ID.
pub fn git_blob_create_fromstream_commit(
    id: &mut crate::oid::OidMut<'_>,
    stream: GitBlobWriteStream<'_>,
) -> Result<(), i32> {
    let raw = stream.inner.into_raw();
    // SAFETY: `raw` transfers the one owned stream to this function, whose C
    // implementation frees it on every return path; `id` is writable.
    let status = unsafe { ffi::git_blob_create_fromstream_commit(id.as_mut_ptr(), raw) };
    if status == 0 { Ok(()) } else { Err(status) }
}
