//! Safe wrappers for libgit2 blame APIs.

use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CDropped, CValued, define_ctype};

use crate::api::blame::{GitBlameFlags, GitBlameHunkRef};
use crate::ffi;
use crate::oid::{OidMut, OidRef};

define_ctype!(
    /// Wraps: git_blame
    /// An opaque blame result owned by libgit2.
    ///
    /// Use [`ffibox::CBox<GitBlame>`] for an owning handle. Dropping that handle
    /// releases the result with `git_blame_free`. Shared and exclusive borrows
    /// are represented by [`GitBlameRef`] and [`GitBlameMut`], without forming
    /// Rust references to memory that libgit2 owns.
    ///
    /// Libgit2 retains a borrowed repository pointer inside each result.
    /// Construction wrappers must therefore keep that repository alive for as
    /// long as the owning blame handle can be used.
    GitBlame,
    GitBlameRef,
    GitBlameMut,
    ffi::git_blame
);

// SAFETY: `git_blame_free` is the public destructor for a fully constructed
// `git_blame`. `CBox::from_raw` requires callers to transfer one uniquely owned
// result allocated by libgit2, and `CBox` invokes this implementation once.
unsafe impl CDropped for GitBlame {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the `CDropped` contract guarantees that `obj` denotes one
        // live, uniquely owned `git_blame` allocation.
        unsafe { ffi::git_blame_free(obj.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};
    use core::ptr;

    use ffibox::{CBox, CCell};

    use super::*;

    #[test]
    fn opaque_blame_has_c_layout_and_lifecycle_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitBlame>();
        assert_dropped::<GitBlame>();
        assert_eq!(size_of::<GitBlame>(), size_of::<ffi::git_blame>());
        assert_eq!(align_of::<GitBlame>(), align_of::<ffi::git_blame>());
        assert_eq!(
            size_of::<GitBlameRef<'_>>(),
            size_of::<*mut ffi::git_blame>()
        );
        assert_eq!(
            size_of::<GitBlameMut<'_>>(),
            size_of::<*mut ffi::git_blame>()
        );
    }

    #[test]
    fn null_blame_seams_create_no_handle() {
        // SAFETY: each conversion explicitly accepts a null pointer and
        // returns `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitBlameRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitBlameMut::from_ptr(ptr::null_mut()).is_none());
            assert!(CBox::<GitBlame>::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn an_empty_buffer_reports_the_code_the_c_assertion_returns() {
        let storage = Box::new(MaybeUninit::<ffi::git_blame>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_blame>();

        {
            // SAFETY: `raw` is the unique address of live storage for the
            // zero-sized opaque binding, and `git_blame_buffer` returns on its
            // empty-buffer guard before this handle reaches libgit2.
            let base = unsafe { GitBlameRef::from_ptr(raw) }.expect("boxed storage is non-null");
            assert_eq!(
                super::git_blame_buffer(base, &[]).err(),
                Some(ffi::git_error_code_GIT_ERROR)
            );
        }

        // SAFETY: `raw` came from `Box::into_raw` above, no handle remains,
        // and casting back recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_blame>>()) });
    }
}

define_ctype!(
    /// Wraps: git_blame_options
    /// Options controlling the commits, lines, and copy tracking considered by
    /// a blame operation.
    GitBlameOptions,
    GitBlameOptionsRef,
    GitBlameOptionsMut,
    ffi::git_blame_options
);

// SAFETY: blame options contain only scalars and embedded object IDs and own
// no resources, so inline disposal is a no-op.
unsafe impl CValued for GitBlameOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'a> GitBlameOptionsRef<'a> {
    /// Field: git_blame_options.flags
    /// Returns the checked set of `git_blame_flag_t` options.
    pub fn flags(&self) -> Result<GitBlameFlags, ffi::git_blame_flag_t> {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        let bits = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitBlameFlags::try_from(bits)
    }

    /// Field: git_blame_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_blame_options.max_line
    /// Returns the inclusive last line to blame, or zero for the file's end.
    #[must_use]
    pub fn max_line(&self) -> usize {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).max_line).read() }
    }

    /// Field: git_blame_options.min_line
    /// Returns the inclusive first line to blame.
    #[must_use]
    pub fn min_line(&self) -> usize {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).min_line).read() }
    }

    /// Field: git_blame_options.oldest_commit
    /// Borrows the oldest commit boundary embedded in this options value.
    #[must_use]
    pub fn oldest_commit(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection points at the initialized embedded
        // `git_oid`, which remains live for this handle's `'a` borrow.
        unsafe { OidRef::from_ptr(addr_of!((*self.as_ptr()).oldest_commit).cast_mut()) }
            .expect("an embedded field has a non-null address")
    }

    /// Field: git_blame_options.newest_commit
    /// Borrows the newest commit boundary embedded in this options value.
    #[must_use]
    pub fn newest_commit(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection points at the initialized embedded
        // `git_oid`, which remains live for this handle's `'a` borrow.
        unsafe { OidRef::from_ptr(addr_of!((*self.as_ptr()).newest_commit).cast_mut()) }
            .expect("an embedded field has a non-null address")
    }

    /// Field: git_blame_options.min_match_characters
    /// Returns the copy-tracking match threshold.
    #[must_use]
    pub fn min_match_characters(&self) -> u16 {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).min_match_characters).read() }
    }
}

impl GitBlameOptionsMut<'_> {
    /// Replaces the checked set of `git_blame_flag_t` options.
    pub fn set_flags(&mut self, flags: GitBlameFlags) {
        // SAFETY: this exclusive handle permits a raw-place write of the
        // scalar without forming a reference to C-visible memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Sets the ABI version of this options value.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: as `set_flags`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Sets the inclusive last line to blame, using zero for the file's end.
    pub fn set_max_line(&mut self, line: usize) {
        // SAFETY: as `set_flags`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).max_line).write(line) }
    }

    /// Sets the inclusive first line to blame.
    pub fn set_min_line(&mut self, line: usize) {
        // SAFETY: as `set_flags`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).min_line).write(line) }
    }

    /// Borrows the embedded oldest-commit boundary exclusively.
    #[must_use]
    pub fn oldest_commit_mut(&mut self) -> OidMut<'_> {
        // SAFETY: raw-place projection points at the initialized embedded
        // `git_oid`, and the result is bounded by this exclusive reborrow.
        unsafe { OidMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).oldest_commit)) }
            .expect("an embedded field has a non-null address")
    }

    /// Borrows the embedded newest-commit boundary exclusively.
    #[must_use]
    pub fn newest_commit_mut(&mut self) -> OidMut<'_> {
        // SAFETY: raw-place projection points at the initialized embedded
        // `git_oid`, and the result is bounded by this exclusive reborrow.
        unsafe { OidMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).newest_commit)) }
            .expect("an embedded field has a non-null address")
    }

    /// Sets the copy-tracking match threshold.
    pub fn set_min_match_characters(&mut self, characters: u16) {
        // SAFETY: as `set_flags`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).min_match_characters).write(characters) }
    }
}

#[cfg(test)]
mod options_tests {
    use core::mem::{align_of, size_of};

    use crate::oid::OidType;

    use super::*;

    #[test]
    fn blame_options_preserve_the_c_layout() {
        assert_eq!(
            size_of::<GitBlameOptions>(),
            size_of::<ffi::git_blame_options>()
        );
        assert_eq!(
            align_of::<GitBlameOptions>(),
            align_of::<ffi::git_blame_options>()
        );
    }

    #[test]
    fn blame_options_handles_read_and_write_every_field() {
        let mut options = GitBlameOptions::zeroed();
        let raw = addr_of_mut!(options).cast::<ffi::git_blame_options>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above and this is its only active handle.
        let mut options = unsafe { GitBlameOptionsMut::from_ptr(raw) }
            .expect("the address of a stack value is non-null");

        options.set_version(1);
        let flags =
            GitBlameFlags::TRACK_COPIES_SAME_FILE | GitBlameFlags::TRACK_COPIES_SAME_COMMIT_COPIES;
        options.set_flags(flags);
        options.set_min_match_characters(42);
        options.set_min_line(7);
        options.set_max_line(19);
        options.newest_commit_mut().set_oid_type(OidType::Sha256);
        options.oldest_commit_mut().set_oid_type(OidType::Sha1);

        let shared = options.as_ref();
        assert_eq!(shared.version(), 1);
        assert_eq!(shared.flags(), Ok(flags));
        assert_eq!(shared.min_match_characters(), 42);
        assert_eq!(shared.min_line(), 7);
        assert_eq!(shared.max_line(), 19);
        assert_eq!(shared.newest_commit().oid_type(), Ok(OidType::Sha256));
        assert_eq!(shared.oldest_commit().oid_type(), Ok(OidType::Sha1));
    }

    #[test]
    fn blame_options_reject_unknown_flag_bits() {
        let unknown = GitBlameFlags::ALL.bits() + 1;
        let mut raw = ffi::git_blame_options {
            flags: unknown,
            // SAFETY: every bit pattern of the remaining integer fields and
            // embedded `git_oid` byte arrays is valid, and the test reads only
            // the initialized `flags` member.
            ..unsafe { core::mem::zeroed() }
        };

        // SAFETY: `raw` is initialized and remains live for this shared handle.
        let options = unsafe { GitBlameOptionsRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(options.flags(), Err(unknown));
    }
}

/// An owned blame result derived from another blame with [`git_blame_buffer`].
///
/// libgit2 copies everything it needs out of the base result, so this owner
/// holds no pointer into it. The one borrow it inherits is the base's
/// repository, which `git_blame__alloc` stores verbatim; the `'base` parameter
/// is the tightest bound on that repository a blame handle can express here.
pub struct GitBlameBuffer<'base> {
    inner: ffibox::CBox<GitBlame>,
    _base: core::marker::PhantomData<GitBlameRef<'base>>,
}

impl GitBlameBuffer<'_> {
    /// Borrows the result.
    #[must_use]
    pub fn as_ref(&self) -> GitBlameRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the result exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitBlameMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_blame_buffer
/// Recomputes blame information for modified buffer contents.
///
/// The call only reads `base`: it copies the base options by value, duplicates
/// the base path and every hunk into the new result, and diffs `buffer`
/// against the base's final blob. A shared handle is therefore the right
/// argument. The single pointer the result inherits is the base's borrowed
/// repository, so it must not outlive that repository; see [`GitBlame`] for
/// the obligation every blame construction carries.
///
/// An empty `buffer` is what C's `GIT_ASSERT_ARG(buffer && buffer_len)`
/// rejects with `GIT_ERROR`, so the wrapper reports that code rather than
/// handing libgit2 a zero-length slice's dangling pointer.
pub fn git_blame_buffer<'base>(
    base: GitBlameRef<'base>,
    buffer: &[u8],
) -> Result<GitBlameBuffer<'base>, i32> {
    if buffer.is_empty() {
        return Err(ffi::git_error_code_GIT_ERROR);
    }
    let mut out = core::ptr::null_mut();
    // SAFETY: `base` is live and this call only reads it, `buffer` supplies
    // exactly its readable byte length, and `out` is writable.
    let status = unsafe {
        ffi::git_blame_buffer(
            &mut out,
            base.as_ptr().cast_mut(),
            buffer.as_ptr().cast(),
            buffer.len(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns a fresh blame allocation owned by the caller.
    let inner = unsafe { ffibox::CBox::from_raw(out) }
        .expect("libgit2 succeeded without returning blame data");
    Ok(GitBlameBuffer {
        inner,
        _base: core::marker::PhantomData,
    })
}

/// Wraps: git_blame_get_hunk_count
/// Returns the number of hunks in a blame result.
#[must_use]
pub fn git_blame_get_hunk_count(blame: GitBlameRef<'_>) -> u32 {
    // SAFETY: `blame` is a live typed handle; this deprecated accessor only
    // reads its hunk vector.
    unsafe { ffi::git_blame_get_hunk_count(blame.as_ptr().cast_mut()) }
}

/// An owned blame result tied to the repository whose history it reads.
pub struct GitRepositoryBlame<'repo> {
    inner: ffibox::CBox<GitBlame>,
    _repository: core::marker::PhantomData<crate::repository::GitRepositoryRef<'repo>>,
}

impl<'repo> GitRepositoryBlame<'repo> {
    pub(crate) fn from_owned(inner: ffibox::CBox<GitBlame>) -> Self {
        Self {
            inner,
            _repository: core::marker::PhantomData,
        }
    }
}

impl GitRepositoryBlame<'_> {
    /// Borrows the result.
    #[must_use]
    pub fn as_ref(&self) -> GitBlameRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the result exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitBlameMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_blame_file
/// Computes blame data and ties the returned owner to `repo`.
pub fn git_blame_file<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    path: &core::ffi::CStr,
    options: Option<GitBlameOptionsRef<'_>>,
) -> Result<GitRepositoryBlame<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut());
    // SAFETY: `out` is writable, both borrows and `path` are live for the
    // synchronous call, and success transfers one blame owner tied to `repo`.
    let status = unsafe {
        ffi::git_blame_file(
            core::ptr::addr_of_mut!(out),
            repo.as_ptr().cast_mut(),
            path.as_ptr(),
            options,
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns a fresh, fully initialized blame allocation.
    let inner = unsafe { ffibox::CBox::from_raw(out) }
        .expect("libgit2 succeeded without returning blame data");
    Ok(GitRepositoryBlame {
        inner,
        _repository: core::marker::PhantomData,
    })
}

/// Wraps: git_blame_init_options
/// Initializes an options value for the requested ABI version.
pub fn git_blame_init_options(
    options: &mut GitBlameOptionsMut<'_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: `options` exclusively exposes a writable options header.
    let status = unsafe { ffi::git_blame_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_blame_get_hunk_byindex
/// Borrows a hunk by zero-based index.
#[must_use]
pub fn git_blame_get_hunk_byindex<'a>(
    blame: GitBlameRef<'a>,
    index: u32,
) -> Option<GitBlameHunkRef<'a>> {
    git_blame_hunk_byindex(blame, index as usize)
}

/// Wraps: git_blame_get_hunk_byline
/// Borrows the hunk containing a one-based line number.
///
/// The lookup takes the blame exclusively: `git_blame_hunk_byline` searches
/// with `git_vector_bsearch2`, which sorts the result's hunk vector in place
/// and records it as sorted before the binary search runs. The hunks
/// themselves are separate allocations, so an already borrowed
/// [`GitBlameHunkRef`] stays valid, but the vector this reorders is the same
/// storage [`git_blame_get_hunk_byindex`] indexes, and a shared handle may not
/// publish that write.
#[must_use]
pub fn git_blame_get_hunk_byline<'a>(
    blame: &'a mut GitBlameMut<'_>,
    line: usize,
) -> Option<GitBlameHunkRef<'a>> {
    git_blame_hunk_byline(blame, line)
}

#[cfg(test)]
mod hunk_lookup_tests {
    use super::*;

    #[test]
    fn hunk_results_are_tied_to_the_blame_borrow() {
        let _: for<'a> fn(GitBlameRef<'a>, u32) -> Option<GitBlameHunkRef<'a>> =
            git_blame_get_hunk_byindex;
        // The by-line lookup sorts the hunk vector, so it borrows exclusively
        // and the hunk it hands back is bounded by that exclusive borrow.
        let _: for<'a, 'b> fn(&'a mut GitBlameMut<'b>, usize) -> Option<GitBlameHunkRef<'a>> =
            git_blame_get_hunk_byline;
    }
}

/// Wraps: git_blame_hunkcount
/// Returns the number of hunks in a blame result.
#[must_use]
pub fn git_blame_hunkcount(blame: GitBlameRef<'_>) -> usize {
    // SAFETY: `blame` is live and the query only reads its hunk vector length.
    unsafe { ffi::git_blame_hunkcount(blame.as_ptr().cast_mut()) }
}

/// Wraps: git_blame_line_byindex
/// Borrows a one-based blame line, returning `None` when out of range.
#[must_use]
pub fn git_blame_line_byindex<'a>(
    blame: GitBlameRef<'a>,
    index: usize,
) -> Option<crate::api::blame::GitBlameLineRef<'a>> {
    // SAFETY: the live blame owns every non-null returned line record and its
    // backing bytes for the complete input borrow.
    let line = unsafe { ffi::git_blame_line_byindex(blame.as_ptr().cast_mut(), index) };
    // SAFETY: null means out of range; otherwise `blame` keeps the line live.
    unsafe { crate::api::blame::GitBlameLineRef::from_ptr(line.cast_mut()) }
}

/// Wraps: git_blame_linecount
/// Returns the number of indexed lines in a blame result.
#[must_use]
pub fn git_blame_linecount(blame: GitBlameRef<'_>) -> usize {
    // SAFETY: `blame` is live and the query only reads its line-index length.
    unsafe { ffi::git_blame_linecount(blame.as_ptr().cast_mut()) }
}

/// Wraps: git_blame_options_init
/// Creates blame options initialized for this ABI.
pub fn git_blame_options_init() -> Result<ffibox::CVal<GitBlameOptions>, i32> {
    let mut options = ffibox::CVal::new(GitBlameOptions::zeroed());
    // SAFETY: the inline options storage is exclusively writable.
    let status = unsafe {
        ffi::git_blame_options_init(
            options.as_mut().as_mut_ptr(),
            ffi::GIT_BLAME_OPTIONS_VERSION,
        )
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_blame_hunk_byindex
/// Borrows the hunk at `index`, returning `None` when it is out of range.
#[must_use]
pub fn git_blame_hunk_byindex<'a>(
    blame: GitBlameRef<'a>,
    index: usize,
) -> Option<GitBlameHunkRef<'a>> {
    // SAFETY: the live blame is only queried; every returned hunk remains
    // owned by it for the complete source borrow.
    let hunk = unsafe { ffi::git_blame_hunk_byindex(blame.as_ptr().cast_mut(), index) };
    // SAFETY: null denotes an invalid index; otherwise `blame` keeps the hunk
    // and every pointer it contains live for `'a`.
    unsafe { GitBlameHunkRef::from_ptr(hunk.cast_mut()) }
}

/// Wraps: git_blame_hunk_byline
/// Borrows the hunk containing the one-based final line `line`.
#[must_use]
pub fn git_blame_hunk_byline<'a>(
    blame: &'a mut GitBlameMut<'_>,
    line: usize,
) -> Option<GitBlameHunkRef<'a>> {
    // SAFETY: the exclusive handle permits `git_vector_bsearch2` to sort the
    // hunk vector in place; every returned hunk remains owned by the blame.
    let hunk = unsafe { ffi::git_blame_hunk_byline(blame.as_mut_ptr(), line) };
    // SAFETY: null means no hunk contains the line; otherwise `blame` keeps
    // the hunk and every pointer it contains live for `'a`.
    unsafe { GitBlameHunkRef::from_ptr(hunk.cast_mut()) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct BlameObservation {
        base_hunks: u32,
        buffer_hunks: u32,
        base_lines: usize,
        buffer_lines: usize,
        final_commits: Vec<Vec<u8>>,
    }

    unsafe fn raw_observation(repository: *mut ffi::git_repository) -> BlameObservation {
        let mut base = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_blame_file(
                    &mut base,
                    repository,
                    c"README.md".as_ptr(),
                    core::ptr::null_mut(),
                )
            },
            0
        );
        let changed = b"fixture\nrewritten by the blame buffer\nwith another line\n";
        let mut buffer = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_blame_buffer(&mut buffer, base, changed.as_ptr().cast(), changed.len())
            },
            0
        );
        let base_hunks = unsafe { ffi::git_blame_get_hunk_count(base) };
        let buffer_hunks = unsafe { ffi::git_blame_get_hunk_count(buffer) };
        let base_lines = unsafe { ffi::git_blame_linecount(base) };
        let buffer_lines = unsafe { ffi::git_blame_linecount(buffer) };
        let mut final_commits = Vec::new();
        for index in 0..buffer_hunks {
            let hunk = unsafe { ffi::git_blame_get_hunk_byindex(buffer, index) };
            assert!(!hunk.is_null());
            final_commits.push(unsafe { (*hunk).final_commit_id.id }.to_vec());
        }
        unsafe {
            ffi::git_blame_free(buffer);
            ffi::git_blame_free(base);
        }
        BlameObservation {
            base_hunks,
            buffer_hunks,
            base_lines,
            buffer_lines,
            final_commits,
        }
    }

    fn safe_observation(
        repository: &mut crate::repository::GitRepositoryMut<'_>,
    ) -> BlameObservation {
        let base = git_blame_file(repository.as_ref(), c"README.md", None).unwrap();
        let changed = b"fixture\nrewritten by the blame buffer\nwith another line\n";
        let buffer = git_blame_buffer(base.as_ref(), changed).unwrap();
        let base_hunks = git_blame_get_hunk_count(base.as_ref());
        let buffer_hunks = git_blame_get_hunk_count(buffer.as_ref());
        let base_lines = git_blame_linecount(base.as_ref());
        let buffer_lines = git_blame_linecount(buffer.as_ref());
        let final_commits = (0..buffer_hunks)
            .map(|index| {
                git_blame_get_hunk_byindex(buffer.as_ref(), index)
                    .unwrap()
                    .final_commit_id()
                    .raw_bytes()
                    .elems()
                    .collect()
            })
            .collect();
        BlameObservation {
            base_hunks,
            buffer_hunks,
            base_lines,
            buffer_lines,
            final_commits,
        }
    }

    #[test]
    fn io_equiv_file_and_buffer_blame() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("blame-raw");
        let safe = HistoryFixture::new("blame-safe");
        let raw_observation = unsafe { raw_observation(raw.repository.as_ptr()) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(raw_observation, safe_observation(&mut safe_repository));
        assert!(raw_observation.base_lines >= 2);
        assert!(raw_observation.buffer_hunks > 0);
    }
}

#[cfg(test)]
mod current_hunk_lookup_tests {
    use super::*;

    #[test]
    fn hunk_results_carry_the_source_blame_lifetime() {
        let _: for<'a> fn(GitBlameRef<'a>, usize) -> Option<GitBlameHunkRef<'a>> =
            git_blame_hunk_byindex;
        let _: for<'a, 'b> fn(&'a mut GitBlameMut<'b>, usize) -> Option<GitBlameHunkRef<'a>> =
            git_blame_hunk_byline;
    }
}

#[cfg(test)]
mod scheduled_hunk_lookup_tests {
    use super::*;

    /// Holds one libgit2 initialization count for the duration of a test.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and reference
            // counted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the initialization this guard represents,
            // after every libgit2 owner in the test has been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// Both scheduled lookups reach the same hunk storage, from opposite ends
    /// of the borrow split.
    ///
    /// `git_blame_hunk_byindex` is a plain `git_vector_get`, so it reads
    /// through a shared handle. `git_blame_hunk_byline` runs
    /// `git_vector_bsearch2`, which calls `git_vector_sort` on the result's
    /// hunk vector before searching it, so it needs the exclusive handle; the
    /// hunks are separate allocations, which is why the hunk it returns is
    /// still valid storage owned by the blame.
    #[test]
    fn the_two_lookups_agree_on_the_hunk_covering_a_line() {
        let _libgit2 = Libgit2Init::acquire();

        let directory = std::env::temp_dir().join(format!(
            "crustify-blame-hunks-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let path = std::ffi::CString::new(
            directory
                .to_str()
                .expect("a temporary directory path is UTF-8"),
        )
        .expect("a temporary directory path holds no interior NUL");

        // A SHA-256 repository sidesteps the bundled SHA1DC collision
        // detector, whose unaligned 32-bit loads trip the C build's UBSan on
        // every object it hashes.
        let mut init_options = crate::repository::git_repository_init_options_init(1)
            .expect("the current init-options version");
        init_options
            .as_mut()
            .set_oid_type(Some(crate::oid::OidType::Sha256));
        init_options
            .as_mut()
            .set_flags(crate::repository::GitRepositoryInitFlags::MKPATH);
        let mut repository =
            crate::repository::git_repository_init_ext(&path, &mut init_options.as_mut())
                .expect("a fresh directory initializes as a work-tree repository");

        std::fs::write(directory.join("lines.txt"), "one\ntwo\nthree\n")
            .expect("the blamed file is written into the work tree");
        {
            let mut index = crate::repository::git_repository_index(&mut repository.as_mut())
                .expect("the repository index opens");
            crate::index::git_index_add_bypath(&mut index.as_mut(), c"lines.txt")
                .expect("the new file is staged");
            crate::index::git_index_write(&mut index.as_mut()).expect("the index is persisted");
        }

        let signature = crate::signature::git_signature_now(c"Crustify", c"crustify@example.com")
            .expect("a signature stamped with the current time");
        let mut options = crate::api::commit::GitCommitCreateOptions::new();
        // SAFETY: the signature owner outlives the options and the call below.
        unsafe {
            options
                .as_mut()
                .set_borrowed_author(Some(signature.as_ref()));
            options
                .as_mut()
                .set_borrowed_committer(Some(signature.as_ref()));
        }
        let mut created = crate::oid::Oid::zeroed();
        {
            // SAFETY: `created` is live, initialized, exclusively borrowed
            // local storage for the whole life of this handle.
            let mut out =
                unsafe { crate::oid::OidMut::from_ptr(core::ptr::addr_of_mut!(created).cast()) }
                    .expect("the address of a local value is non-null");
            crate::commit::git_commit_create_from_stage(
                &mut out,
                &mut repository.as_mut(),
                c"add lines",
                Some(options.as_ref()),
            )
            .expect("the staged file commits");
        }

        let mut blame = git_blame_file(repository.as_ref(), c"lines.txt", None)
            .expect("the committed file is blamable");

        assert_eq!(git_blame_hunkcount(blame.as_ref()), 1);
        let hunk = git_blame_hunk_byindex(blame.as_ref(), 0).expect("the sole hunk");
        assert_eq!(hunk.final_start_line_number(), 1);
        assert_eq!(hunk.lines_in_hunk(), 3);
        assert!(
            git_blame_hunk_byindex(blame.as_ref(), 1).is_none(),
            "an out-of-range index is a clean absence"
        );

        // The by-line lookup selects the same hunk for every covered line and
        // reports nothing past the end of the file.
        for line in 1..=3 {
            let mut exclusive = blame.as_mut();
            let found = git_blame_hunk_byline(&mut exclusive, line)
                .expect("every committed line belongs to the sole hunk");
            assert_eq!(found.final_start_line_number(), 1);
            assert_eq!(found.lines_in_hunk(), 3);
        }
        assert!(
            git_blame_hunk_byline(&mut blame.as_mut(), 4).is_none(),
            "a line past the blamed file has no hunk"
        );
        assert!(
            git_blame_hunk_byline(&mut blame.as_mut(), 0).is_none(),
            "line numbers are one-based"
        );

        // The searched vector is the one the index lookup reads, so indexing
        // still resolves after the search sorted it.
        assert!(git_blame_hunk_byindex(blame.as_ref(), 0).is_some());

        drop(blame);
        drop(options);
        drop(signature);
        drop(repository);
        let _ = std::fs::remove_dir_all(&directory);
    }
}
