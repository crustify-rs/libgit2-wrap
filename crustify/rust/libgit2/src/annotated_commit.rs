//! Safe wrappers for libgit2 annotated_commit APIs.

use ffibox::{define_ctype, impl_dropped};

use crate::ffi;

define_ctype!(
    /// Wraps: git_annotated_commit
    /// An annotated commit owned by libgit2.
    ///
    /// The public C API keeps this type opaque. Owned pointers use
    /// [`ffibox::CBox<AnnotatedCommit>`], which releases the object with
    /// `git_annotated_commit_free`.
    ///
    /// Internally a discriminant selects two ownership variants: a "real"
    /// commit owns a duplicated `git_commit`, a lazily peeled `git_tree` and
    /// its duplicated strings, while a "virtual" merge base owns a
    /// `git_index` and a parent-id array. Both are released by the same
    /// destructor, so the wrapper needs no runtime drop state, but a value
    /// whose discriminant is neither reaches `abort()` in
    /// `git_annotated_commit_free`.
    ///
    /// A real commit stores the `git_repository *` it was looked up in inside
    /// the `git_object` header it owns, and every consumer of an annotated
    /// commit (merge, rebase, reset) re-enters that repository through it, so
    /// an owning handle is only usable while the repository is alive.
    /// [`AnnotatedCommitOwned`] carries that bound for the constructors here.
    AnnotatedCommit,
    AnnotatedCommitRef,
    AnnotatedCommitMut,
    ffi::git_annotated_commit
);

// SAFETY: `git_annotated_commit_free` is the public destructor for a complete
// `git_annotated_commit` allocation and accepts null, although `CDropped` only
// supplies a live non-null allocation. It releases whichever variant the
// discriminant selects and aborts on any other value, so `CBox` may only adopt
// a pointer a libgit2 constructor produced -- which is what `CBox::from_raw`
// already requires. `AnnotatedCommit` is transparent over the corresponding
// bindgen C type.
impl_dropped!(
    AnnotatedCommit,
    ffi::git_annotated_commit,
    ffi::git_annotated_commit_free
);

/// An owned annotated commit whose lifetime is tied to its repository borrow.
///
/// # Composition
///
/// Every consumer of an annotated commit — reset, merge, rebase, branch
/// creation and detaching `HEAD` — takes the same repository exclusively while
/// the annotated commit is alive. The constructors below therefore borrow a
/// transient `&mut GitRepositoryMut<'repo>` and tether the result to `'repo`,
/// the handle's own borrow of the repository owner, rather than consuming the
/// handle: consuming it would pin the exclusive borrow for as long as the
/// annotated commit lives and leave every one of those consumers unreachable
/// from safe code. This is the shape
/// [`git_diff_tree_to_tree`](crate::diff_generate::git_diff_tree_to_tree)
/// already uses for the diff it returns.
pub struct AnnotatedCommitOwned<'repo> {
    inner: ffibox::CBox<AnnotatedCommit>,
    _repository: core::marker::PhantomData<crate::repository::GitRepositoryRef<'repo>>,
}

impl AnnotatedCommitOwned<'_> {
    /// Borrows the annotated commit.
    #[must_use]
    pub fn as_ref(&self) -> AnnotatedCommitRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the annotated commit exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> AnnotatedCommitMut<'_> {
        self.inner.as_mut()
    }
}

fn annotated_result<'repo>(
    status: i32,
    inner: Option<ffibox::CBox<AnnotatedCommit>>,
) -> Result<AnnotatedCommitOwned<'repo>, i32> {
    if status == 0 {
        let inner = inner.expect("libgit2 succeeded without returning an annotated commit");
        Ok(AnnotatedCommitOwned {
            inner,
            _repository: core::marker::PhantomData,
        })
    } else {
        drop(inner);
        Err(status)
    }
}

/// Wraps: git_annotated_commit_from_fetchhead
/// Creates an annotated commit from fetch-head metadata.
pub fn git_annotated_commit_from_fetchhead<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    branch_name: &core::ffi::CStr,
    remote_url: &core::ffi::CStr,
    id: crate::oid::OidRef<'_>,
) -> Result<AnnotatedCommitOwned<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the handles and C strings are live for the synchronous call and
    // `out` is a writable output slot. The returned owner retains the repo
    // lifetime required by the commit stored inside it.
    let status = unsafe {
        ffi::git_annotated_commit_from_fetchhead(
            &mut out,
            repo.as_mut_ptr(),
            branch_name.as_ptr(),
            remote_url.as_ptr(),
            id.as_ptr(),
        )
    };
    // SAFETY: `out` is null or transfers a complete annotated commit from the
    // constructor's output slot, including on a later error path.
    let inner = unsafe { ffibox::CBox::<AnnotatedCommit>::from_raw(out) };
    annotated_result(status, inner)
}

/// Wraps: git_annotated_commit_from_ref
/// Creates an annotated commit from a reference in `repo`.
pub fn git_annotated_commit_from_ref<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    reference: crate::refs::GitReferenceRef<'_>,
) -> Result<AnnotatedCommitOwned<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: all pointers come from live typed handles and `out` is writable.
    let status = unsafe {
        ffi::git_annotated_commit_from_ref(&mut out, repo.as_mut_ptr(), reference.as_ptr())
    };
    // SAFETY: `out` has the transferred-output contract described above.
    let inner = unsafe { ffibox::CBox::<AnnotatedCommit>::from_raw(out) };
    annotated_result(status, inner)
}

/// Wraps: git_annotated_commit_id
/// Borrows the commit ID embedded in an annotated commit.
///
/// The result is `None` for a virtual merge base, which carries an index
/// rather than a commit: libgit2 forwards the null `commit` field to
/// `git_object_id`, which returns null for it.
#[must_use]
pub fn git_annotated_commit_id<'a>(
    commit: AnnotatedCommitRef<'a>,
) -> Option<crate::oid::OidRef<'a>> {
    // SAFETY: `commit` is live; libgit2 returns null or the cached id of the
    // `git_commit` this annotated commit owns, which it keeps alive.
    let raw = unsafe { ffi::git_annotated_commit_id(commit.as_ptr()) };
    // SAFETY: a non-null result is borrowed from `commit` for `'a`.
    unsafe { crate::oid::OidRef::from_ptr(raw.cast_mut()) }
}

/// Wraps: git_annotated_commit_lookup
/// Creates an annotated commit by object ID.
pub fn git_annotated_commit_lookup<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    id: crate::oid::OidRef<'_>,
) -> Result<AnnotatedCommitOwned<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the handles are live for the call and `out` is writable.
    let status =
        unsafe { ffi::git_annotated_commit_lookup(&mut out, repo.as_mut_ptr(), id.as_ptr()) };
    // SAFETY: `out` has the transferred-output contract described above.
    let inner = unsafe { ffibox::CBox::<AnnotatedCommit>::from_raw(out) };
    annotated_result(status, inner)
}

/// Wraps: git_annotated_commit_ref
/// Borrows the optional reference name retained by an annotated commit.
///
/// The name is a `git__strdup` allocation the annotated commit owns and its
/// destructor frees, so the borrow is valid for exactly `'a`. It is `None`
/// unless the commit was built from a reference or from fetch-head metadata.
#[must_use]
pub fn git_annotated_commit_ref<'a>(commit: AnnotatedCommitRef<'a>) -> Option<&'a core::ffi::CStr> {
    // SAFETY: `commit` is live; libgit2 returns null or a NUL-terminated string
    // kept alive by that annotated commit.
    let raw = unsafe { ffi::git_annotated_commit_ref(commit.as_ptr()) };
    if raw.is_null() {
        None
    } else {
        // SAFETY: the non-null result has the lifetime and string contract above.
        Some(unsafe { core::ffi::CStr::from_ptr(raw) })
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::CDropped;

    use super::*;

    #[test]
    fn opaque_representation_and_borrowed_handles_match_the_c_seam() {
        assert_eq!(
            size_of::<AnnotatedCommit>(),
            size_of::<ffi::git_annotated_commit>()
        );
        assert_eq!(
            align_of::<AnnotatedCommit>(),
            align_of::<ffi::git_annotated_commit>()
        );
        assert_eq!(
            size_of::<AnnotatedCommitRef<'static>>(),
            size_of::<*const ffi::git_annotated_commit>()
        );
        assert_eq!(
            size_of::<AnnotatedCommitMut<'static>>(),
            size_of::<*mut ffi::git_annotated_commit>()
        );

        let storage = Box::new(MaybeUninit::<ffi::git_annotated_commit>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_annotated_commit>();

        {
            // SAFETY: `raw` addresses live zero-initialized storage for the
            // bindgen opaque type and remains live for this scope.
            let shared = unsafe { AnnotatedCommitRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, `raw` remains live, and this
            // scope has exclusive access to the storage.
            let mut exclusive = unsafe { AnnotatedCommitMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from `Box::into_raw` above, no handle remains,
        // and casting back recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_annotated_commit>>()) });
    }

    #[test]
    fn annotated_commit_registers_its_c_destructor() {
        fn assert_dropped<T: CDropped>() {}
        assert_dropped::<AnnotatedCommit>();
    }

    #[test]
    fn failed_result_accepts_an_empty_typed_owner() {
        assert!(matches!(annotated_result(-123, None), Err(-123)));
    }
}

/// Wraps: git_annotated_commit_from_revspec
/// Resolves `revspec` to a commit and returns an annotated commit tied to `repo`.
pub fn git_annotated_commit_from_revspec<'repo>(
    repo: &mut crate::repository::GitRepositoryMut<'repo>,
    revspec: &core::ffi::CStr,
) -> Result<AnnotatedCommitOwned<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `repo` and `revspec` are live for the call, `out` is writable,
    // and the returned owner carries the repository lifetime it retains.
    let status = unsafe {
        ffi::git_annotated_commit_from_revspec(&mut out, repo.as_mut_ptr(), revspec.as_ptr())
    };
    // SAFETY: `out` is null or transfers a complete annotated commit.
    let inner = unsafe { ffibox::CBox::<AnnotatedCommit>::from_raw(out) };
    annotated_result(status, inner)
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    type Observation = Vec<(Vec<u8>, Option<Vec<u8>>)>;

    unsafe fn inspect(commit: *mut ffi::git_annotated_commit) -> (Vec<u8>, Option<Vec<u8>>) {
        let id = unsafe { (*ffi::git_annotated_commit_id(commit)).id }.to_vec();
        let name = unsafe { ffi::git_annotated_commit_ref(commit) };
        let name = if name.is_null() {
            None
        } else {
            Some(
                unsafe { core::ffi::CStr::from_ptr(name) }
                    .to_bytes()
                    .to_vec(),
            )
        };
        (id, name)
    }

    unsafe fn raw_annotated(repository: *mut ffi::git_repository) -> Observation {
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut output = Vec::new();

        let mut commit = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_annotated_commit_from_fetchhead(
                    &mut commit,
                    repository,
                    c"refs/heads/master".as_ptr(),
                    c"https://example.invalid/source.git".as_ptr(),
                    &head,
                )
            },
            0
        );
        output.push(unsafe { inspect(commit) });
        unsafe { ffi::git_annotated_commit_free(commit) };

        let mut reference = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reference_lookup(&mut reference, repository, c"refs/heads/master".as_ptr())
            },
            0
        );
        commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_annotated_commit_from_ref(&mut commit, repository, reference) },
            0
        );
        output.push(unsafe { inspect(commit) });
        unsafe {
            ffi::git_annotated_commit_free(commit);
            ffi::git_reference_free(reference);
        }

        commit = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_annotated_commit_from_revspec(&mut commit, repository, c"HEAD~1".as_ptr())
            },
            0
        );
        output.push(unsafe { inspect(commit) });
        unsafe { ffi::git_annotated_commit_free(commit) };

        commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_annotated_commit_lookup(&mut commit, repository, &head) },
            0
        );
        output.push(unsafe { inspect(commit) });
        unsafe { ffi::git_annotated_commit_free(commit) };
        output
    }

    fn safe_inspect(commit: AnnotatedCommitRef<'_>) -> (Vec<u8>, Option<Vec<u8>>) {
        let id = git_annotated_commit_id(commit)
            .unwrap()
            .raw_bytes()
            .elems()
            .collect();
        let name = git_annotated_commit_ref(commit).map(|name| name.to_bytes().to_vec());
        (id, name)
    }

    fn safe_annotated(repository: *mut ffi::git_repository) -> Observation {
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let head_ref = unsafe { crate::oid::OidRef::from_ptr(&raw mut head) }.unwrap();
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut output = Vec::new();

        let commit = git_annotated_commit_from_fetchhead(
            &mut repository_view,
            c"refs/heads/master",
            c"https://example.invalid/source.git",
            head_ref,
        )
        .unwrap();
        output.push(safe_inspect(commit.as_ref()));
        drop(commit);

        let mut raw_reference = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reference_lookup(
                    &mut raw_reference,
                    repository,
                    c"refs/heads/master".as_ptr(),
                )
            },
            0
        );
        let reference = unsafe { crate::refs::GitReferenceRef::from_ptr(raw_reference) }.unwrap();
        let commit = git_annotated_commit_from_ref(&mut repository_view, reference).unwrap();
        output.push(safe_inspect(commit.as_ref()));
        drop(commit);
        unsafe { ffi::git_reference_free(raw_reference) };

        let commit = git_annotated_commit_from_revspec(&mut repository_view, c"HEAD~1").unwrap();
        output.push(safe_inspect(commit.as_ref()));
        drop(commit);
        let commit = git_annotated_commit_lookup(&mut repository_view, head_ref).unwrap();
        output.push(safe_inspect(commit.as_ref()));
        output
    }

    #[test]
    fn io_equiv_annotated_commits_from_fetchhead_ref_revspec_and_lookup() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("annotated-raw");
        let safe = HistoryFixture::new("annotated-safe");
        let raw = unsafe { raw_annotated(raw.repository.as_ptr()) };
        assert_eq!(raw, safe_annotated(safe.repository.as_ptr()));
        assert_eq!(raw[0].1.as_deref(), Some(b"refs/heads/master".as_slice()));
        assert_eq!(raw[1].1.as_deref(), Some(b"refs/heads/master".as_slice()));
        assert!(raw[2].1.is_none());
        assert!(raw[3].1.is_none());
    }
}
