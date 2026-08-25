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

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};
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

    #[test]
    fn borrowed_handles_preserve_the_blob_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_blob>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_blob>();

        {
            // SAFETY: `raw` addresses live, suitably aligned storage for the
            // bindgen opaque type, and the shared handle stays in this scope.
            let shared = unsafe { GitBlobRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitBlobMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_blob>>()) });
    }
}

/// Wraps: git_blob_is_binary
/// Reports whether libgit2's content heuristic classifies the blob as binary.
#[must_use]
pub fn git_blob_is_binary(blob: GitBlobRef<'_>) -> bool {
    // SAFETY: `blob` is live and the operation only reads immutable content.
    unsafe { ffi::git_blob_is_binary(blob.as_ptr()) != 0 }
}

/// Wraps: git_blob_rawcontent
/// Borrows the blob's immutable bytes.
#[must_use]
pub fn git_blob_rawcontent<'a>(blob: GitBlobRef<'a>) -> &'a [u8] {
    // SAFETY: both accessors only read the live blob; libgit2 keeps its object
    // bytes immutable and alive for the blob borrow.
    let (data, size) = unsafe {
        (
            ffi::git_blob_rawcontent(blob.as_ptr()).cast::<u8>(),
            ffi::git_blob_rawsize(blob.as_ptr()),
        )
    };
    let size = usize::try_from(size).expect("a blob size must fit the address space");
    if size == 0 {
        return &[];
    }
    assert!(!data.is_null(), "a nonempty blob must have content");
    // SAFETY: a nonempty blob exposes exactly `size` initialized immutable
    // bytes at `data`, all kept alive by the input handle's `'a` lifetime.
    unsafe { core::slice::from_raw_parts(data, size) }
}

/// Wraps: git_blob_rawsize
/// Returns the blob's content length in bytes.
#[must_use]
pub fn git_blob_rawsize(blob: GitBlobRef<'_>) -> u64 {
    // SAFETY: `blob` is live and the accessor only reads its size.
    unsafe { ffi::git_blob_rawsize(blob.as_ptr()) }
}

/// Wraps: git_blob_data_is_binary
/// Reports whether libgit2's content heuristic classifies `data` as binary.
#[must_use]
pub fn git_blob_data_is_binary(data: &[u8]) -> bool {
    // SAFETY: `data` supplies exactly its readable length, and libgit2 retains
    // neither the byte pointer nor the temporary string view it constructs.
    unsafe { ffi::git_blob_data_is_binary(data.as_ptr().cast(), data.len()) != 0 }
}

#[cfg(test)]
mod binary_data_tests {
    use super::*;

    #[test]
    fn binary_data_classifier_accepts_slices() {
        assert!(!git_blob_data_is_binary(b"ordinary text\n"));
        assert!(git_blob_data_is_binary(b"text\0binary"));
        assert!(!git_blob_data_is_binary(&[]));
    }
}

/// Wraps: git_blob_create_from_buffer
/// Creates a blob through the current non-deprecated API.
pub fn git_blob_create_from_buffer(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    buffer: &[u8],
) -> Result<(), i32> {
    // SAFETY: typed handles are live and exclusive; borrowed input bytes or
    // strings remain live for this synchronous non-retaining call.
    let status = unsafe {
        ffi::git_blob_create_from_buffer(
            id.as_mut_ptr(),
            repo.as_mut_ptr(),
            buffer.as_ptr().cast(),
            buffer.len(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_blob_create_from_disk
/// Creates a blob through the current non-deprecated API.
pub fn git_blob_create_from_disk(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    path: &core::ffi::CStr,
) -> Result<(), i32> {
    // SAFETY: typed handles are live and exclusive; borrowed input bytes or
    // strings remain live for this synchronous non-retaining call.
    let status = unsafe {
        ffi::git_blob_create_from_disk(id.as_mut_ptr(), repo.as_mut_ptr(), path.as_ptr())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_blob_create_from_stream
/// Opens a blob write stream that retains the repository borrow.
pub fn git_blob_create_from_stream<'repo>(
    mut repo: crate::repository::GitRepositoryMut<'repo>,
    hint_path: Option<&core::ffi::CStr>,
) -> Result<GitBlobWriteStream<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let hint = hint_path.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: inputs are live, `out` is writable, and the result carries the
    // repository borrow stored by the C stream.
    let status = unsafe { ffi::git_blob_create_from_stream(&mut out, repo.as_mut_ptr(), hint) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one initialized stream with a free callback.
    let inner = unsafe { crate::api::types::GitWriteStreamOwned::from_raw(out) }
        .ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(GitBlobWriteStream {
        inner,
        _repository: core::marker::PhantomData,
    })
}

/// Wraps: git_blob_create_from_stream_commit
/// Commits and consumes a blob write stream.
pub fn git_blob_create_from_stream_commit(
    id: &mut crate::oid::OidMut<'_>,
    stream: GitBlobWriteStream<'_>,
) -> Result<(), i32> {
    let raw = stream.inner.into_raw();
    // SAFETY: ownership of `raw` is transferred to a function that frees it
    // on every path, and `id` is a live writable output.
    let status = unsafe { ffi::git_blob_create_from_stream_commit(id.as_mut_ptr(), raw) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_blob_create_from_workdir
/// Creates a blob through the current non-deprecated API.
pub fn git_blob_create_from_workdir(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    path: &core::ffi::CStr,
) -> Result<(), i32> {
    // SAFETY: typed handles are live and exclusive; borrowed input bytes or
    // strings remain live for this synchronous non-retaining call.
    let status = unsafe {
        ffi::git_blob_create_from_workdir(id.as_mut_ptr(), repo.as_mut_ptr(), path.as_ptr())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_blob_filter
/// Applies checkout filters to a blob and returns the owned output buffer.
pub fn git_blob_filter(
    blob: &mut GitBlobMut<'_>,
    path: &core::ffi::CStr,
    options: crate::api::blob::GitBlobFilterOptionsRef<'_>,
) -> Result<ffibox::CVal<crate::api::buffer::GitBuf>, i32> {
    let mut out = crate::api::buffer::GitBuf::new();
    // SAFETY: output and blob are exclusive, and path and options are live for
    // the synchronous operation. Libgit2 retains none of these borrows.
    let status = unsafe {
        ffi::git_blob_filter(
            out.as_mut().as_mut_ptr(),
            blob.as_mut_ptr(),
            path.as_ptr(),
            options.as_ptr().cast_mut(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_blob_filter_options_init
/// Creates blob-filter options initialized for this ABI.
pub fn git_blob_filter_options_init()
-> Result<ffibox::CVal<crate::api::blob::GitBlobFilterOptions>, i32> {
    let mut options = ffibox::CVal::new(crate::api::blob::GitBlobFilterOptions::zeroed());
    // SAFETY: the inline options storage is exclusively writable.
    let status = unsafe {
        ffi::git_blob_filter_options_init(
            options.as_mut().as_mut_ptr(),
            ffi::GIT_BLOB_FILTER_OPTIONS_VERSION,
        )
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}
