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
    AnnotatedCommit,
    AnnotatedCommitRef,
    AnnotatedCommitMut,
    ffi::git_annotated_commit
);

// SAFETY: `git_annotated_commit_free` is the public destructor for a complete
// `git_annotated_commit` allocation and accepts null, although `CDropped` only
// supplies a live non-null allocation. `AnnotatedCommit` is transparent over
// the corresponding bindgen C type.
impl_dropped!(
    AnnotatedCommit,
    ffi::git_annotated_commit,
    ffi::git_annotated_commit_free
);

#[cfg(test)]
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
}

/// An owned annotated commit whose lifetime is tied to its repository borrow.
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
    raw: *mut ffi::git_annotated_commit,
) -> Result<AnnotatedCommitOwned<'repo>, i32> {
    if status == 0 {
        // SAFETY: success transfers a fresh, non-null allocation owned by the
        // output slot.
        let inner = unsafe { ffibox::CBox::from_raw(raw) }
            .expect("libgit2 succeeded without returning an annotated commit");
        Ok(AnnotatedCommitOwned {
            inner,
            _repository: core::marker::PhantomData,
        })
    } else {
        if !raw.is_null() {
            // SAFETY: constructors may populate `out` before a later allocation
            // fails; any such complete annotated commit remains caller-owned.
            drop(unsafe { ffibox::CBox::<AnnotatedCommit>::from_raw(raw) });
        }
        Err(status)
    }
}

/// Wraps: git_annotated_commit_from_fetchhead
/// Creates an annotated commit from fetch-head metadata.
pub fn git_annotated_commit_from_fetchhead<'repo>(
    mut repo: crate::repository::GitRepositoryMut<'repo>,
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
    annotated_result(status, out)
}

/// Wraps: git_annotated_commit_from_ref
/// Creates an annotated commit from a reference in `repo`.
pub fn git_annotated_commit_from_ref<'repo>(
    mut repo: crate::repository::GitRepositoryMut<'repo>,
    reference: crate::refs::GitReferenceRef<'_>,
) -> Result<AnnotatedCommitOwned<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: all pointers come from live typed handles and `out` is writable.
    let status = unsafe {
        ffi::git_annotated_commit_from_ref(&mut out, repo.as_mut_ptr(), reference.as_ptr())
    };
    annotated_result(status, out)
}

/// Wraps: git_annotated_commit_id
/// Borrows the commit ID embedded in an annotated commit.
#[must_use]
pub fn git_annotated_commit_id<'a>(
    commit: AnnotatedCommitRef<'a>,
) -> Option<crate::oid::OidRef<'a>> {
    // SAFETY: `commit` is live; libgit2 returns null or an ID kept alive by it.
    let raw = unsafe { ffi::git_annotated_commit_id(commit.as_ptr()) };
    // SAFETY: a non-null result is borrowed from `commit` for `'a`.
    unsafe { crate::oid::OidRef::from_ptr(raw.cast_mut()) }
}

/// Wraps: git_annotated_commit_lookup
/// Creates an annotated commit by object ID.
pub fn git_annotated_commit_lookup<'repo>(
    mut repo: crate::repository::GitRepositoryMut<'repo>,
    id: crate::oid::OidRef<'_>,
) -> Result<AnnotatedCommitOwned<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the handles are live for the call and `out` is writable.
    let status =
        unsafe { ffi::git_annotated_commit_lookup(&mut out, repo.as_mut_ptr(), id.as_ptr()) };
    annotated_result(status, out)
}

/// Wraps: git_annotated_commit_ref
/// Borrows the optional reference name retained by an annotated commit.
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
