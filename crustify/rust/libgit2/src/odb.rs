//! Safe wrappers for libgit2 odb APIs.

use core::ffi::CStr;
use core::marker::PhantomData;

use ffibox::CBox;

use crate::api::odb_backend::{GitOdbStreamMut, GitOdbStreamOwned};
use crate::api::types::GitObjectType;
use crate::ffi;
use crate::oid::{Oid, OidRef};

ffibox::define_ctype!(
    /// Wraps: git_odb
    /// An opaque, reference-counted object database managed by libgit2.
    ///
    /// Each owned handle represents one reference count. Dropping it calls
    /// `git_odb_free`; libgit2 does not publish an operation for acquiring a
    /// second count directly, so the owner intentionally does not implement
    /// `Clone`.
    ///
    /// A database reached through `git_repository_odb` additionally carries
    /// the repository as its refcount owner, and `GIT_REFCOUNT_DEC` frees
    /// nothing while an owner is recorded. Dropping such a handle releases the
    /// count only; the repository still frees the database.
    GitOdb,
    GitOdbRef,
    GitOdbMut,
    ffi::git_odb
);

/// An owned reference to a libgit2 object database.
pub type GitOdbOwned = CBox<GitOdb>;

// SAFETY: `git_odb_free` consumes exactly one reference to a fully initialized
// object database and frees the object only when its reference count reaches
// zero and no repository owns it. It accepts null, although `CBox` supplies a
// live non-null object exactly once.
ffibox::impl_dropped!(GitOdb, ffi::git_odb, ffi::git_odb_free);

/// Wraps: git_odb_foreach_cb
/// Safe callable surface for object IDs visited by an object database.
pub trait GitOdbForeachCallback {
    /// Receives one transient object ID. Nonzero stops iteration.
    fn call(&mut self, oid: OidRef<'_>) -> i32;
}

impl<F> GitOdbForeachCallback for F
where
    F: for<'a> FnMut(OidRef<'a>) -> i32,
{
    fn call(&mut self, oid: OidRef<'_>) -> i32 {
        self(oid)
    }
}

/// Wraps: git_odb_hash
/// Hashes an in-memory object body with its Git object header.
pub fn git_odb_hash(data: &[u8], kind: GitObjectType) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable and `data` is a readable run of `len` bytes;
    // libgit2 retains neither pointer.
    let status = unsafe {
        ffi::git_odb_hash(
            core::ptr::addr_of_mut!(out).cast(),
            data.as_ptr().cast(),
            data.len(),
            kind.as_raw(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_odb_hashfile
/// Hashes the file at `path` as a Git object of `kind`.
pub fn git_odb_hashfile(path: &core::ffi::CStr, kind: GitObjectType) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable and `path` is a live C string retained only
    // for this call.
    let status = unsafe {
        ffi::git_odb_hashfile(
            core::ptr::addr_of_mut!(out).cast(),
            path.as_ptr(),
            kind.as_raw(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_odb_preserves_the_c_seam_and_drop_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitOdb>();
        assert_dropped::<GitOdb>();
        assert_eq!(size_of::<GitOdb>(), size_of::<ffi::git_odb>());
        assert_eq!(align_of::<GitOdb>(), align_of::<ffi::git_odb>());
        assert_eq!(size_of::<GitOdbRef<'_>>(), size_of::<*const ffi::git_odb>());
        assert_eq!(size_of::<GitOdbMut<'_>>(), size_of::<*mut ffi::git_odb>());
        assert_eq!(size_of::<GitOdbOwned>(), size_of::<*mut ffi::git_odb>());
    }

    #[test]
    fn null_odb_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitOdbRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitOdbMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitOdbOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// A stream whose backend remains protected by an exclusive ODB borrow.
pub struct GitOdbScopedStream<'odb> {
    inner: GitOdbStreamOwned,
    _odb: PhantomData<&'odb mut GitOdb>,
}

impl GitOdbScopedStream<'_> {
    /// Exclusively reborrows the underlying stream.
    pub fn as_mut(&mut self) -> GitOdbStreamMut<'_> {
        self.inner.as_mut()
    }
}

/// Checked lookup flags for [`git_odb_exists_ext`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GitOdbLookupFlags(u32);

impl GitOdbLookupFlags {
    /// Automatically refresh the ODB after a failed first lookup.
    pub const DEFAULT: Self = Self(0);
    /// Do not refresh after a failed lookup.
    pub const NO_REFRESH: Self = Self(ffi::git_odb_lookup_flags_t_GIT_ODB_LOOKUP_NO_REFRESH);

    /// Returns the raw published bit set.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }
}

/// A successful C call returned an unknown object kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidOdbObjectType(pub ffi::git_object_t);

fn status_result(status: i32) -> Result<(), i32> {
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_odb_add_disk_alternate
pub fn git_odb_add_disk_alternate(odb: &mut GitOdbMut<'_>, path: &CStr) -> Result<(), i32> {
    // SAFETY: the ODB is exclusively borrowed while it installs the backend,
    // and it copies the live path during this call.
    status_result(unsafe { ffi::git_odb_add_disk_alternate(odb.as_mut_ptr(), path.as_ptr()) })
}

/// Wraps: git_odb_exists
#[must_use]
pub fn git_odb_exists(odb: &mut GitOdbMut<'_>, id: OidRef<'_>) -> bool {
    // SAFETY: the ODB is exclusively borrowed because lookup may refresh its
    // backend set; `id` remains live for the call.
    unsafe { ffi::git_odb_exists(odb.as_mut_ptr(), id.as_ptr()) != 0 }
}

/// Wraps: git_odb_exists_ext
#[must_use]
pub fn git_odb_exists_ext(
    odb: &mut GitOdbMut<'_>,
    id: OidRef<'_>,
    flags: GitOdbLookupFlags,
) -> bool {
    // SAFETY: the ODB is exclusively borrowed and `id` is live; `flags`
    // contains only published bits by construction.
    unsafe { ffi::git_odb_exists_ext(odb.as_mut_ptr(), id.as_ptr(), flags.bits()) != 0 }
}

/// Wraps: git_odb_exists_prefix
pub fn git_odb_exists_prefix(
    odb: &mut GitOdbMut<'_>,
    short_id: OidRef<'_>,
    hex_len: usize,
) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable, the ODB is exclusively borrowed because the
    // lookup may refresh it, and the abbreviated ID remains live.
    let status = unsafe {
        ffi::git_odb_exists_prefix(
            core::ptr::addr_of_mut!(out).cast(),
            odb.as_mut_ptr(),
            short_id.as_ptr(),
            hex_len,
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

unsafe extern "C" fn foreach_trampoline<C: GitOdbForeachCallback>(
    id: *const ffi::git_oid,
    payload: *mut core::ffi::c_void,
) -> i32 {
    if id.is_null() || payload.is_null() {
        return -1;
    }
    // SAFETY: the wrapper supplies the exact live callback object as payload
    // for this synchronous traversal.
    let callback = unsafe { &mut *payload.cast::<C>() };
    // SAFETY: libgit2 supplies one complete transient OID for this invocation.
    let id = unsafe { OidRef::from_ptr(id.cast_mut()) }.expect("callback ID is non-null");
    callback.call(id)
}

/// Wraps: git_odb_foreach
pub fn git_odb_foreach<C: GitOdbForeachCallback>(
    odb: &mut GitOdbMut<'_>,
    callback: &mut C,
) -> Result<(), i32> {
    // SAFETY: the ODB and callback remain exclusively live for the synchronous
    // traversal; the trampoline reconstructs the exact callback type.
    status_result(unsafe {
        ffi::git_odb_foreach(
            odb.as_mut_ptr(),
            Some(foreach_trampoline::<C>),
            (callback as *mut C).cast(),
        )
    })
}

/// Wraps: git_odb_new
pub fn git_odb_new() -> Result<GitOdbOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable slot for one fresh owned ODB count.
    let status = unsafe { ffi::git_odb_new(&mut out) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized ODB count.
    unsafe { GitOdbOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_odb_open_rstream
/// Opens a read stream tied to the database that owns its backend.
pub fn git_odb_open_rstream<'odb>(
    odb: &'odb mut GitOdbMut<'_>,
    id: OidRef<'_>,
) -> Result<(GitOdbScopedStream<'odb>, usize, GitObjectType), i32> {
    let mut stream = core::ptr::null_mut();
    let mut len = 0;
    let mut kind = ffi::git_object_t_GIT_OBJECT_INVALID;
    // SAFETY: all outputs are writable, the ODB is exclusively borrowed for
    // the returned stream lifetime, and `id` remains live for this call.
    let status = unsafe {
        ffi::git_odb_open_rstream(
            &mut stream,
            &mut len,
            &mut kind,
            odb.as_mut_ptr(),
            id.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    let Some(kind) = GitObjectType::from_raw(kind) else {
        // A successful constructor still owns the stream; adopt it before
        // returning an error so its destructor runs.
        // SAFETY: success transferred one complete stream allocation.
        drop(unsafe { GitOdbStreamOwned::from_raw(stream) });
        return Err(ffi::git_error_code_GIT_ERROR);
    };
    // SAFETY: success transfers one fully constructed stream allocation whose
    // backend remains alive through the exclusive ODB borrow carried below.
    let inner = unsafe { GitOdbStreamOwned::from_raw(stream) }
        .expect("successful stream construction returns non-null");
    Ok((
        GitOdbScopedStream {
            inner,
            _odb: PhantomData,
        },
        len,
        kind,
    ))
}

/// Wraps: git_odb_open_wstream
/// Opens a write stream tied to the database that owns its backend.
pub fn git_odb_open_wstream<'odb>(
    odb: &'odb mut GitOdbMut<'_>,
    size: ffi::git_object_size_t,
    kind: GitObjectType,
) -> Result<GitOdbScopedStream<'odb>, i32> {
    let mut stream = core::ptr::null_mut();
    // SAFETY: `stream` is writable, the ODB stays exclusively borrowed for
    // the returned stream lifetime, and `kind` is a checked C value.
    let status =
        unsafe { ffi::git_odb_open_wstream(&mut stream, odb.as_mut_ptr(), size, kind.as_raw()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully constructed stream allocation whose
    // borrowed backend is protected by the carried ODB lifetime.
    let inner = unsafe { GitOdbStreamOwned::from_raw(stream) }
        .expect("successful stream construction returns non-null");
    Ok(GitOdbScopedStream {
        inner,
        _odb: PhantomData,
    })
}

/// Wraps: git_odb_read_header
pub fn git_odb_read_header(
    odb: &mut GitOdbMut<'_>,
    id: OidRef<'_>,
) -> Result<(usize, GitObjectType), i32> {
    let mut len = 0;
    let mut kind = ffi::git_object_t_GIT_OBJECT_INVALID;
    // SAFETY: both outputs are writable, the ODB is exclusively borrowed, and
    // `id` remains live for this possibly refreshing lookup.
    let status =
        unsafe { ffi::git_odb_read_header(&mut len, &mut kind, odb.as_mut_ptr(), id.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    GitObjectType::from_raw(kind)
        .map(|kind| (len, kind))
        .ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_odb_refresh
pub fn git_odb_refresh(odb: &mut GitOdbMut<'_>) -> Result<(), i32> {
    // SAFETY: the ODB is exclusively borrowed while C refreshes its backends.
    status_result(unsafe { ffi::git_odb_refresh(odb.as_mut_ptr()) })
}

/// Wraps: git_odb_stream_finalize_write
pub fn git_odb_stream_finalize_write(stream: &mut GitOdbScopedStream<'_>) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    let mut stream = stream.as_mut();
    // SAFETY: `out` is writable and the scoped owner exclusively borrows a
    // live stream whose database/backend remains alive.
    let status = unsafe {
        ffi::git_odb_stream_finalize_write(core::ptr::addr_of_mut!(out).cast(), stream.as_mut_ptr())
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_odb_stream_read
pub fn git_odb_stream_read(
    stream: &mut GitOdbScopedStream<'_>,
    buffer: &mut [u8],
) -> Result<usize, i32> {
    let mut stream = stream.as_mut();
    // SAFETY: the scoped owner exclusively borrows a live stream and `buffer`
    // supplies exactly its length in writable bytes for the synchronous read.
    let status = unsafe {
        ffi::git_odb_stream_read(
            stream.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
        )
    };
    if status < 0 {
        Err(status)
    } else {
        let count = usize::try_from(status).expect("a nonnegative C int fits usize");
        if count <= buffer.len() {
            Ok(count)
        } else {
            Err(ffi::git_error_code_GIT_ERROR)
        }
    }
}

/// Wraps: git_odb_stream_write
pub fn git_odb_stream_write(stream: &mut GitOdbScopedStream<'_>, buffer: &[u8]) -> Result<(), i32> {
    let mut stream = stream.as_mut();
    // SAFETY: the scoped owner exclusively borrows a live write stream and
    // `buffer` supplies exactly its length in readable bytes.
    status_result(unsafe {
        ffi::git_odb_stream_write(stream.as_mut_ptr(), buffer.as_ptr().cast(), buffer.len())
    })
}

#[cfg(test)]
mod scheduled_wrapper_tests {
    use super::*;

    #[test]
    fn new_empty_odb_is_owned_and_refreshable() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after the ODB owner is dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut odb = git_odb_new().unwrap();
        git_odb_refresh(&mut odb.as_mut()).unwrap();
        drop(odb);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn lookup_flags_expose_only_published_bits() {
        assert_eq!(GitOdbLookupFlags::DEFAULT.bits(), 0);
        assert_eq!(
            GitOdbLookupFlags::NO_REFRESH.bits(),
            ffi::git_odb_lookup_flags_t_GIT_ODB_LOOKUP_NO_REFRESH
        );
    }
}
