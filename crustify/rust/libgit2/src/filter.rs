//! Safe wrappers for libgit2 filter APIs.

use core::ptr::NonNull;

use ffibox::{CCell, CPtr, CType, impl_dropped};

use crate::ffi;

/// Wraps: git_filter_list
/// An opaque ordered list of filters prepared by libgit2.
///
/// The public C API does not publish storage for this type, so it cannot be
/// constructed by value. An owning [`ffibox::CBox<GitFilterList>`] releases
/// the list, its entry buffer and every filter payload with
/// `git_filter_list_free`.
#[repr(transparent)]
pub struct GitFilterList(CType<ffi::git_filter_list>);

/// Shared borrow of a [`GitFilterList`].
#[repr(transparent)]
pub struct GitFilterListRef<'a>(CPtr<'a, GitFilterList>);

impl Clone for GitFilterListRef<'_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitFilterListRef<'_> {}

/// Exclusive borrow of a [`GitFilterList`].
#[repr(transparent)]
pub struct GitFilterListMut<'a>(GitFilterListRef<'a>);

// SAFETY: `GitFilterList` is transparent over the matching opaque bindgen
// type. Both handles are transparent over a lifetime-carrying pointer, never
// form a reference to the C object, and the shared handle exposes no writes.
unsafe impl CCell for GitFilterList {
    type C = ffi::git_filter_list;
    type Ref<'a> = GitFilterListRef<'a>;
    type Mut<'a> = GitFilterListMut<'a>;

    unsafe fn ref_from_raw<'a>(ptr: NonNull<Self>) -> Self::Ref<'a> {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitFilterListRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'a>(ptr: NonNull<Self>) -> Self::Mut<'a> {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitFilterListMut(GitFilterListRef(unsafe { CPtr::new(ptr) }))
    }
}

impl GitFilterListRef<'_> {
    /// Borrows a raw list pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify an initialized list that remains live and is not
    /// mutated for the returned handle's lifetime.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_filter_list) -> Option<Self> {
        NonNull::new(ptr.cast::<GitFilterList>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_filter_list {
        self.0.as_non_null().as_ptr().cast()
    }
}

impl GitFilterListMut<'_> {
    /// Exclusively borrows a raw list pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other handle to the list
    /// may be used for the returned handle's lifetime.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_filter_list) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitFilterListRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_filter_list {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitFilterListRef<'_> {
        GitFilterListRef(self.0.0)
    }
}

// SAFETY: `git_filter_list_free` is the public destructor for a complete list
// allocation. It accepts null, while `CDropped` supplies a live non-null value,
// runs each entry's cleanup callback, releases the entry buffer, and finally
// frees the list allocation. `GitFilterList` is transparent over the matching
// bindgen type.
impl_dropped!(
    GitFilterList,
    ffi::git_filter_list,
    ffi::git_filter_list_free
);

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use core::mem::size_of;
    use core::ptr;

    use ffibox::{CBox, CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_filter_list_has_pointer_sized_handles_and_lifecycle_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitFilterList>();
        assert_dropped::<GitFilterList>();
        assert_eq!(
            size_of::<GitFilterListRef<'_>>(),
            size_of::<*mut ffi::git_filter_list>()
        );
        assert_eq!(
            size_of::<GitFilterListMut<'_>>(),
            size_of::<*mut ffi::git_filter_list>()
        );
    }

    #[test]
    fn null_filter_list_seams_create_no_handle() {
        // SAFETY: each conversion explicitly accepts null and returns `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitFilterListRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitFilterListMut::from_ptr(ptr::null_mut()).is_none());
            assert!(CBox::<GitFilterList>::from_raw(ptr::null_mut()).is_none());
        }
    }
}

/// Wraps: git_filter_list_apply_to_buffer
/// Applies an optional filter list to length-delimited bytes.
pub fn git_filter_list_apply_to_buffer(
    filters: Option<&mut GitFilterListMut<'_>>,
    input: &[u8],
) -> Result<ffibox::CVal<crate::api::buffer::GitBuf>, i32> {
    let mut out = crate::api::buffer::GitBuf::new();
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: output is writable, input covers its exact length, and optional
    // filters remain live for this non-retaining operation.
    let status = unsafe {
        ffi::git_filter_list_apply_to_buffer(
            out.as_mut().as_mut_ptr(),
            filters,
            input.as_ptr().cast(),
            input.len(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_filter_list_apply_to_file
/// Applies an optional filter list to a file.
pub fn git_filter_list_apply_to_file(
    filters: Option<&mut GitFilterListMut<'_>>,
    repo: crate::repository::GitRepositoryRef<'_>,
    path: &core::ffi::CStr,
) -> Result<ffibox::CVal<crate::api::buffer::GitBuf>, i32> {
    let mut out = crate::api::buffer::GitBuf::new();
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: all handles and path are live and output is exclusively writable.
    let status = unsafe {
        ffi::git_filter_list_apply_to_file(
            out.as_mut().as_mut_ptr(),
            filters,
            repo.as_ptr().cast_mut(),
            path.as_ptr(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_filter_list_contains
/// Reports whether an optional filter list contains `name`.
#[must_use]
pub fn git_filter_list_contains(
    filters: Option<GitFilterListRef<'_>>,
    name: &core::ffi::CStr,
) -> bool {
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut());
    // SAFETY: the optional handle and required name are live for the query.
    unsafe { ffi::git_filter_list_contains(filters, name.as_ptr()) != 0 }
}

/// Wraps: git_filter_list_free
/// An owning filter list released automatically by its registered destructor.
pub type GitFilterListOwned = ffibox::CBox<GitFilterList>;

/// An owned filter list tied to the repository stored in its filter source.
pub struct RepositoryFilterList<'repo> {
    inner: GitFilterListOwned,
    _repository: core::marker::PhantomData<crate::repository::GitRepositoryRef<'repo>>,
}

impl RepositoryFilterList<'_> {
    #[must_use]
    pub fn as_ref(&self) -> GitFilterListRef<'_> {
        self.inner.as_ref()
    }
    #[must_use]
    pub fn as_mut(&mut self) -> GitFilterListMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_filter_list_stream_buffer
/// Streams filtered bytes into `target`.
pub fn git_filter_list_stream_buffer(
    filters: Option<&mut GitFilterListMut<'_>>,
    input: &[u8],
    target: &mut crate::api::types::GitWriteStreamMut<'_>,
) -> Result<(), i32> {
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: all borrowed storage is live and target is exclusive for the
    // complete synchronous streaming operation.
    let status = unsafe {
        ffi::git_filter_list_stream_buffer(
            filters,
            input.as_ptr().cast(),
            input.len(),
            target.as_mut_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_filter_list_stream_file
/// Streams a filtered file into `target`.
pub fn git_filter_list_stream_file(
    filters: Option<&mut GitFilterListMut<'_>>,
    repo: crate::repository::GitRepositoryRef<'_>,
    path: &core::ffi::CStr,
    target: &mut crate::api::types::GitWriteStreamMut<'_>,
) -> Result<(), i32> {
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: every input is live and target is exclusive for the synchronous call.
    let status = unsafe {
        ffi::git_filter_list_stream_file(
            filters,
            repo.as_ptr().cast_mut(),
            path.as_ptr(),
            target.as_mut_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_filter_tests {
    use super::*;
    #[test]
    fn null_filter_list_contains_nothing() {
        assert!(!git_filter_list_contains(None, c"ident"));
    }
}

/// Wraps: git_filter_list_apply_to_blob
/// Applies an optional filter list to a blob.
pub fn git_filter_list_apply_to_blob(
    filters: Option<&mut GitFilterListMut<'_>>,
    blob: &mut crate::blob::GitBlobMut<'_>,
) -> Result<ffibox::CVal<crate::api::buffer::GitBuf>, i32> {
    let mut out = crate::api::buffer::GitBuf::new();
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: output and blob are exclusive and optional filters are live.
    let status = unsafe {
        ffi::git_filter_list_apply_to_blob(out.as_mut().as_mut_ptr(), filters, blob.as_mut_ptr())
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

fn adopt_filter_list<'repo>(
    status: i32,
    out: *mut ffi::git_filter_list,
) -> Result<Option<RepositoryFilterList<'repo>>, i32> {
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete filter list, or null when no filters apply.
    let Some(inner) = (unsafe { GitFilterListOwned::from_raw(out) }) else {
        return Ok(None);
    };
    Ok(Some(RepositoryFilterList {
        inner,
        _repository: core::marker::PhantomData,
    }))
}

/// Wraps: git_filter_list_load
/// Loads filters and ties their retained source repository to the result.
pub fn git_filter_list_load<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    blob: Option<crate::blob::GitBlobRef<'_>>,
    path: &core::ffi::CStr,
    mode: crate::api::filter::GitFilterMode,
    flags: crate::api::filter::GitFilterFlags,
) -> Result<Option<RepositoryFilterList<'repo>>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output and all borrowed inputs are live; result copies blob ID/path and retains repo.
    let status = unsafe {
        ffi::git_filter_list_load(
            &mut out,
            repo.as_ptr().cast_mut(),
            blob.map_or(core::ptr::null_mut(), |b| b.as_ptr().cast_mut()),
            path.as_ptr(),
            mode.into(),
            flags.bits(),
        )
    };
    adopt_filter_list(status, out)
}

/// Wraps: git_filter_list_load_ext
/// Loads filters with extended options.
pub fn git_filter_list_load_ext<'repo, 'data>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    blob: Option<crate::blob::GitBlobRef<'_>>,
    path: &core::ffi::CStr,
    mode: crate::api::filter::GitFilterMode,
    options: &mut crate::api::filter::GitFilterOptionsMut<'_, 'data>,
) -> Result<Option<RepositoryFilterList<'repo>>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output and all borrowed inputs are live; options are copied and repo is lifetime-bound.
    let status = unsafe {
        ffi::git_filter_list_load_ext(
            &mut out,
            repo.as_ptr().cast_mut(),
            blob.map_or(core::ptr::null_mut(), |b| b.as_ptr().cast_mut()),
            path.as_ptr(),
            mode.into(),
            options.as_mut_ptr(),
        )
    };
    adopt_filter_list(status, out)
}

/// Wraps: git_filter_list_stream_blob
/// Streams a blob through an optional filter list.
pub fn git_filter_list_stream_blob(
    filters: Option<&mut GitFilterListMut<'_>>,
    blob: &mut crate::blob::GitBlobMut<'_>,
    target: &mut crate::api::types::GitWriteStreamMut<'_>,
) -> Result<(), i32> {
    let filters = filters.map_or(core::ptr::null_mut(), |value| value.as_mut_ptr());
    // SAFETY: all handles remain live and blob/target are exclusive for the call.
    let status = unsafe {
        ffi::git_filter_list_stream_blob(filters, blob.as_mut_ptr(), target.as_mut_ptr())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_filter_options_init
/// Creates filter options initialized for this ABI.
pub fn git_filter_options_init<'data>()
-> Result<ffibox::CVal<crate::api::filter::GitFilterOptions<'data>>, i32> {
    let mut options = crate::api::filter::GitFilterOptions::new();
    // SAFETY: inline options storage is exclusively writable and starts initialized.
    let status = unsafe {
        ffi::git_filter_options_init(
            options.as_mut().as_mut_ptr(),
            ffi::GIT_FILTER_OPTIONS_VERSION,
        )
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, safe_buf_bytes};

    #[derive(Debug, Eq, PartialEq)]
    struct FilterObservation {
        clean_buffer: Vec<u8>,
        clean_file: Vec<u8>,
        smudge_buffer: Vec<u8>,
        contains_crlf: bool,
    }

    unsafe fn take_buf(buffer: &mut ffi::git_buf) -> Vec<u8> {
        let bytes =
            unsafe { core::slice::from_raw_parts(buffer.ptr.cast::<u8>(), buffer.size).to_vec() };
        unsafe { ffi::git_buf_dispose(buffer) };
        bytes
    }

    unsafe fn raw_filters(repository: *mut ffi::git_repository) -> FilterObservation {
        let mut clean = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_filter_list_load(
                    &mut clean,
                    repository,
                    core::ptr::null_mut(),
                    c"src/alpha.c".as_ptr(),
                    ffi::git_filter_mode_t_GIT_FILTER_TO_ODB,
                    ffi::git_filter_flag_t_GIT_FILTER_DEFAULT,
                )
            },
            0
        );
        let input = b"one\r\ntwo\r\n";
        let mut clean_buffer = unsafe { core::mem::zeroed::<ffi::git_buf>() };
        assert_eq!(
            unsafe {
                ffi::git_filter_list_apply_to_buffer(
                    &mut clean_buffer,
                    clean,
                    input.as_ptr().cast(),
                    input.len(),
                )
            },
            0
        );
        let mut clean_file = unsafe { core::mem::zeroed::<ffi::git_buf>() };
        assert_eq!(
            unsafe {
                ffi::git_filter_list_apply_to_file(
                    &mut clean_file,
                    clean,
                    repository,
                    c"src/alpha.c".as_ptr(),
                )
            },
            0
        );
        let contains_crlf = unsafe { ffi::git_filter_list_contains(clean, c"crlf".as_ptr()) } != 0;
        unsafe { ffi::git_filter_list_free(clean) };

        let mut smudge = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_filter_list_load(
                    &mut smudge,
                    repository,
                    core::ptr::null_mut(),
                    c"src/alpha.c".as_ptr(),
                    ffi::git_filter_mode_t_GIT_FILTER_TO_WORKTREE,
                    ffi::git_filter_flag_t_GIT_FILTER_DEFAULT,
                )
            },
            0
        );
        let input = b"one\ntwo\n";
        let mut smudge_buffer = unsafe { core::mem::zeroed::<ffi::git_buf>() };
        assert_eq!(
            unsafe {
                ffi::git_filter_list_apply_to_buffer(
                    &mut smudge_buffer,
                    smudge,
                    input.as_ptr().cast(),
                    input.len(),
                )
            },
            0
        );
        unsafe { ffi::git_filter_list_free(smudge) };
        FilterObservation {
            clean_buffer: unsafe { take_buf(&mut clean_buffer) },
            clean_file: unsafe { take_buf(&mut clean_file) },
            smudge_buffer: unsafe { take_buf(&mut smudge_buffer) },
            contains_crlf,
        }
    }

    fn safe_filters(repository: *mut ffi::git_repository) -> FilterObservation {
        let repository_ref =
            unsafe { crate::repository::GitRepositoryRef::from_ptr(repository) }.unwrap();
        let mut clean = git_filter_list_load(
            repository_ref,
            None,
            c"src/alpha.c",
            crate::api::filter::GitFilterMode::CLEAN,
            crate::api::filter::GitFilterFlags::DEFAULT,
        )
        .unwrap()
        .unwrap();
        let clean_buffer = safe_buf_bytes(
            git_filter_list_apply_to_buffer(Some(&mut clean.as_mut()), b"one\r\ntwo\r\n")
                .unwrap()
                .as_ref(),
        );
        let clean_file = safe_buf_bytes(
            git_filter_list_apply_to_file(
                Some(&mut clean.as_mut()),
                repository_ref,
                c"src/alpha.c",
            )
            .unwrap()
            .as_ref(),
        );
        let contains_crlf = git_filter_list_contains(Some(clean.as_ref()), c"crlf");
        drop(clean);
        let mut smudge = git_filter_list_load(
            repository_ref,
            None,
            c"src/alpha.c",
            crate::api::filter::GitFilterMode::SMUDGE,
            crate::api::filter::GitFilterFlags::DEFAULT,
        )
        .unwrap()
        .unwrap();
        let smudge_buffer = safe_buf_bytes(
            git_filter_list_apply_to_buffer(Some(&mut smudge.as_mut()), b"one\ntwo\n")
                .unwrap()
                .as_ref(),
        );
        FilterObservation {
            clean_buffer,
            clean_file,
            smudge_buffer,
            contains_crlf,
        }
    }

    #[test]
    fn io_equiv_crlf_filters_apply_to_buffers_and_files() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("filters-raw");
        let safe = HistoryFixture::new("filters-safe");
        let attributes = b"*.c text eol=crlf\n";
        std::fs::write(raw.directory.path().join(".gitattributes"), attributes).unwrap();
        std::fs::write(safe.directory.path().join(".gitattributes"), attributes).unwrap();
        let raw_observation = unsafe { raw_filters(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_filters(safe.repository.as_ptr()));
        assert_eq!(raw_observation.clean_buffer, b"one\ntwo\n");
        assert_eq!(raw_observation.smudge_buffer, b"one\r\ntwo\r\n");
        assert!(raw_observation.contains_crlf);
    }

    #[repr(C)]
    struct CollectStream {
        stream: ffi::git_writestream,
        bytes: Vec<u8>,
    }

    impl CollectStream {
        fn new() -> Self {
            Self {
                stream: ffi::git_writestream {
                    write: Some(collect_write),
                    close: Some(collect_close),
                    free: Some(collect_free),
                },
                bytes: Vec::new(),
            }
        }
    }

    unsafe extern "C" fn collect_write(
        stream: *mut ffi::git_writestream,
        buffer: *const core::ffi::c_char,
        length: usize,
    ) -> i32 {
        let stream = unsafe { &mut *stream.cast::<CollectStream>() };
        stream
            .bytes
            .extend_from_slice(unsafe { core::slice::from_raw_parts(buffer.cast::<u8>(), length) });
        0
    }

    unsafe extern "C" fn collect_close(_: *mut ffi::git_writestream) -> i32 {
        0
    }

    unsafe extern "C" fn collect_free(_: *mut ffi::git_writestream) {}

    unsafe fn raw_blob_and_stream_filters(
        repository: *mut ffi::git_repository,
    ) -> (Vec<u8>, [Vec<u8>; 3], bool) {
        let input = b"blob one\r\nblob two\r\n";
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_blob_create_from_buffer(
                    &mut id,
                    repository,
                    input.as_ptr().cast(),
                    input.len(),
                )
            },
            0
        );
        let mut blob = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_blob_lookup(&mut blob, repository, &id) },
            0
        );
        let mut options = unsafe { core::mem::zeroed::<ffi::git_filter_options>() };
        assert_eq!(
            unsafe { ffi::git_filter_options_init(&mut options, ffi::GIT_FILTER_OPTIONS_VERSION) },
            0
        );
        let mut filters = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_filter_list_load_ext(
                    &mut filters,
                    repository,
                    blob,
                    c"src/generated.c".as_ptr(),
                    ffi::git_filter_mode_t_GIT_FILTER_TO_ODB,
                    &mut options,
                )
            },
            0
        );
        let mut applied = unsafe { core::mem::zeroed::<ffi::git_buf>() };
        assert_eq!(
            unsafe { ffi::git_filter_list_apply_to_blob(&mut applied, filters, blob) },
            0
        );
        let applied = unsafe { take_buf(&mut applied) };
        let mut streams = [
            CollectStream::new(),
            CollectStream::new(),
            CollectStream::new(),
        ];
        assert_eq!(
            unsafe {
                ffi::git_filter_list_stream_buffer(
                    filters,
                    input.as_ptr().cast(),
                    input.len(),
                    &mut streams[0].stream,
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_filter_list_stream_file(
                    filters,
                    repository,
                    c"src/alpha.c".as_ptr(),
                    &mut streams[1].stream,
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_filter_list_stream_blob(filters, blob, &mut streams[2].stream) },
            0
        );
        let contains = unsafe { ffi::git_filter_list_contains(filters, c"crlf".as_ptr()) } != 0;
        unsafe {
            ffi::git_filter_list_free(filters);
            ffi::git_blob_free(blob);
        }
        (applied, streams.map(|stream| stream.bytes), contains)
    }

    fn safe_blob_and_stream_filters(
        repository: *mut ffi::git_repository,
    ) -> (Vec<u8>, [Vec<u8>; 3], bool) {
        let input = b"blob one\r\nblob two\r\n";
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut raw_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        {
            let mut id = unsafe { crate::oid::OidMut::from_ptr(&mut raw_id) }.unwrap();
            crate::blob::git_blob_create_from_buffer(&mut id, &mut repository_view, input).unwrap();
        }
        let id = unsafe { crate::oid::OidRef::from_ptr(&mut raw_id) }.unwrap();
        let mut blob = crate::object_api::git_blob_lookup(repository_view.as_ref(), id).unwrap();
        let mut options = git_filter_options_init().unwrap();
        let mut filters = git_filter_list_load_ext(
            repository_view.as_ref(),
            Some(blob.as_ref()),
            c"src/generated.c",
            crate::api::filter::GitFilterMode::CLEAN,
            &mut options.as_mut(),
        )
        .unwrap()
        .unwrap();
        let applied = safe_buf_bytes(
            git_filter_list_apply_to_blob(Some(&mut filters.as_mut()), &mut blob.as_mut())
                .unwrap()
                .as_ref(),
        );
        let mut streams = [
            CollectStream::new(),
            CollectStream::new(),
            CollectStream::new(),
        ];
        {
            let mut targets = streams.each_mut().map(|stream| {
                unsafe { crate::api::types::GitWriteStreamMut::from_ptr(&mut stream.stream) }
                    .unwrap()
            });
            git_filter_list_stream_buffer(Some(&mut filters.as_mut()), input, &mut targets[0])
                .unwrap();
            git_filter_list_stream_file(
                Some(&mut filters.as_mut()),
                repository_view.as_ref(),
                c"src/alpha.c",
                &mut targets[1],
            )
            .unwrap();
            git_filter_list_stream_blob(
                Some(&mut filters.as_mut()),
                &mut blob.as_mut(),
                &mut targets[2],
            )
            .unwrap();
        }
        let contains = git_filter_list_contains(Some(filters.as_ref()), c"crlf");
        (applied, streams.map(|stream| stream.bytes), contains)
    }

    #[test]
    fn io_equiv_extended_blob_and_stream_filtering() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("filter-streams-raw");
        let safe = HistoryFixture::new("filter-streams-safe");
        let attributes = b"*.c text eol=lf\n";
        std::fs::write(raw.directory.path().join(".gitattributes"), attributes).unwrap();
        std::fs::write(safe.directory.path().join(".gitattributes"), attributes).unwrap();
        let raw = unsafe { raw_blob_and_stream_filters(raw.repository.as_ptr()) };
        assert_eq!(raw, safe_blob_and_stream_filters(safe.repository.as_ptr()));
        assert_eq!(raw.0, b"blob one\nblob two\n");
        assert_eq!(raw.1[0], raw.0);
        assert_eq!(raw.1[2], raw.0);
        assert!(raw.2);
    }
}
