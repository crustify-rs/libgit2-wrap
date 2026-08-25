//! Safe wrappers for libgit2 branch APIs.

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_branch_iterator
    /// Opaque storage for a libgit2 branch iterator.
    ///
    /// Owned iterators are represented by [`ffibox::CBox<GitBranchIterator>`].
    GitBranchIterator,
    GitBranchIteratorRef,
    GitBranchIteratorMut,
    ffi::git_branch_iterator
);

ffibox::impl_dropped!(
    GitBranchIterator,
    ffi::git_branch_iterator,
    ffi::git_branch_iterator_free
);

/// Wraps: git_branch_name_is_valid
/// Checks whether `name` is a valid branch shorthand.
///
/// `None` mirrors libgit2's accepted null input and is reported as invalid.
pub fn git_branch_name_is_valid(name: Option<&core::ffi::CStr>) -> Result<bool, i32> {
    let mut valid = 0;
    let name = name.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `valid` is a writable scalar out-slot and `name` is null or a
    // live NUL-terminated string for the duration of the call.
    let status = unsafe { ffi::git_branch_name_is_valid(&mut valid, name) };
    if status == 0 {
        Ok(valid != 0)
    } else {
        Err(status)
    }
}

fn reference_result(
    status: i32,
    reference: Option<crate::refs::GitReferenceOwned>,
) -> Result<crate::refs::GitReferenceOwned, i32> {
    if status == 0 {
        Ok(reference.expect("libgit2 succeeded without returning a reference"))
    } else {
        drop(reference);
        Err(status)
    }
}

/// Wraps: git_branch_create_from_annotated
/// Creates a branch pointing at an annotated commit.
pub fn git_branch_create_from_annotated(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    branch_name: &core::ffi::CStr,
    target: crate::annotated_commit::AnnotatedCommitRef<'_>,
    force: bool,
) -> Result<crate::refs::GitReferenceOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: all handles and the string are live for this call and `out` is writable.
    let status = unsafe {
        ffi::git_branch_create_from_annotated(
            &mut out,
            repo.as_mut_ptr(),
            branch_name.as_ptr(),
            target.as_ptr(),
            i32::from(force),
        )
    };
    // SAFETY: `out` is null or a complete caller-owned reference produced by
    // the constructor, including on a later error path.
    let reference = unsafe { crate::refs::GitReferenceOwned::from_raw(out) };
    reference_result(status, reference)
}

/// Wraps: git_branch_delete
/// Deletes a branch and consumes the now-stale reference handle.
pub fn git_branch_delete(branch: crate::refs::GitReferenceOwned) -> Result<(), i32> {
    // SAFETY: `branch` uniquely owns a live reference for the call. It is
    // dropped immediately afterward on success or failure.
    let status = unsafe { ffi::git_branch_delete(branch.as_ptr()) };
    drop(branch);
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_branch_is_head
/// Tests whether the branch is the repository's current HEAD.
pub fn git_branch_is_head(branch: crate::refs::GitReferenceRef<'_>) -> Result<bool, i32> {
    // SAFETY: `branch` is live and this function only reads it.
    let status = unsafe { ffi::git_branch_is_head(branch.as_ptr()) };
    if status < 0 {
        Err(status)
    } else {
        Ok(status != 0)
    }
}

/// Wraps: git_branch_iterator_new
/// Creates an iterator over selected branch kinds.
pub fn git_branch_iterator_new(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    kinds: crate::api::types::GitBranchType,
) -> Result<ffibox::CBox<GitBranchIterator>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `repo` is live and exclusive and `out` is writable.
    let status = unsafe { ffi::git_branch_iterator_new(&mut out, repo.as_mut_ptr(), kinds.bits()) };
    if status == 0 {
        // SAFETY: success transfers the fresh iterator allocation.
        Ok(unsafe { ffibox::CBox::from_raw(out) }
            .expect("libgit2 succeeded without returning an iterator"))
    } else {
        Err(status)
    }
}

/// Wraps: git_branch_lookup
/// Looks up a local, remote, or either-kind branch.
pub fn git_branch_lookup(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    branch_name: &core::ffi::CStr,
    kind: crate::api::types::GitBranchType,
) -> Result<crate::refs::GitReferenceOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: arguments are live and `out` is writable.
    let status = unsafe {
        ffi::git_branch_lookup(
            &mut out,
            repo.as_mut_ptr(),
            branch_name.as_ptr(),
            kind.bits(),
        )
    };
    // SAFETY: `out` has the transferred-output contract described above.
    let reference = unsafe { crate::refs::GitReferenceOwned::from_raw(out) };
    reference_result(status, reference)
}

/// Wraps: git_branch_move
/// Renames a local branch and consumes the stale old reference.
pub fn git_branch_move(
    branch: crate::refs::GitReferenceOwned,
    new_name: &core::ffi::CStr,
    force: bool,
) -> Result<crate::refs::GitReferenceOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `branch` remains live for the call, `new_name` is a live C
    // string, and `out` is writable.
    let status = unsafe {
        ffi::git_branch_move(
            &mut out,
            branch.as_ptr(),
            new_name.as_ptr(),
            i32::from(force),
        )
    };
    drop(branch);
    // SAFETY: `out` has the transferred-output contract described above.
    let reference = unsafe { crate::refs::GitReferenceOwned::from_raw(out) };
    reference_result(status, reference)
}

/// Wraps: git_branch_name
/// Borrows a branch's shorthand name.
pub fn git_branch_name<'a>(
    branch: crate::refs::GitReferenceRef<'a>,
) -> Result<&'a core::ffi::CStr, i32> {
    let mut out = core::ptr::null();
    // SAFETY: `branch` is live and `out` is writable. Success returns a string
    // stored inside that reference.
    let status = unsafe { ffi::git_branch_name(&mut out, branch.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    debug_assert!(!out.is_null());
    // SAFETY: success returns a non-null NUL-terminated substring retained by `branch`.
    Ok(unsafe { core::ffi::CStr::from_ptr(out) })
}

/// Wraps: git_branch_next
/// Advances an iterator and returns its next owned reference and checked kind.
pub fn git_branch_next(
    iter: &mut GitBranchIteratorMut<'_>,
) -> Result<
    (
        crate::refs::GitReferenceOwned,
        crate::api::types::GitBranchType,
    ),
    i32,
> {
    let mut out = core::ptr::null_mut();
    let mut raw_kind = 0;
    // SAFETY: `iter` is live and exclusive and both output slots are writable.
    let status = unsafe { ffi::git_branch_next(&mut out, &mut raw_kind, iter.as_mut_ptr()) };
    // SAFETY: `out` is null or a complete caller-owned reference produced by
    // the iterator, including on a later error path.
    let reference = unsafe { crate::refs::GitReferenceOwned::from_raw(out) };
    let reference = reference_result(status, reference)?;
    let kind = crate::api::types::GitBranchType::from_bits(raw_kind)
        .expect("libgit2 returned an unknown branch kind");
    Ok((reference, kind))
}

fn branch_name_to_buffer(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    refname: &core::ffi::CStr,
    call: unsafe extern "C" fn(
        *mut ffi::git_buf,
        *mut ffi::git_repository,
        *const core::ffi::c_char,
    ) -> core::ffi::c_int,
) -> Result<(), i32> {
    // SAFETY: the two handles and string are live, and `call` is one of the
    // scheduled libgit2 functions with exactly this synchronous contract.
    let status = unsafe { call(out.as_mut_ptr(), repo.as_mut_ptr(), refname.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_branch_remote_name
/// Writes the matching remote name for a remote-tracking reference.
pub fn git_branch_remote_name(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    refname: &core::ffi::CStr,
) -> Result<(), i32> {
    branch_name_to_buffer(out, repo, refname, ffi::git_branch_remote_name)
}

/// Wraps: git_branch_set_upstream
/// Sets or clears a local branch's upstream configuration.
pub fn git_branch_set_upstream(
    branch: crate::refs::GitReferenceRef<'_>,
    upstream: Option<&core::ffi::CStr>,
) -> Result<(), i32> {
    let upstream = upstream.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: the branch and optional string are live. Source inspection shows
    // this API only reads the reference object while mutating repository config.
    let status = unsafe { ffi::git_branch_set_upstream(branch.as_ptr().cast_mut(), upstream) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_branch_upstream
/// Returns the branch's owned upstream reference.
pub fn git_branch_upstream(
    branch: crate::refs::GitReferenceRef<'_>,
) -> Result<crate::refs::GitReferenceOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `branch` is live and `out` is writable.
    let status = unsafe { ffi::git_branch_upstream(&mut out, branch.as_ptr()) };
    // SAFETY: `out` has the transferred-output contract described above.
    let reference = unsafe { crate::refs::GitReferenceOwned::from_raw(out) };
    reference_result(status, reference)
}

/// Wraps: git_branch_upstream_merge
/// Writes the configured upstream merge refspec.
pub fn git_branch_upstream_merge(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    refname: &core::ffi::CStr,
) -> Result<(), i32> {
    branch_name_to_buffer(out, repo, refname, ffi::git_branch_upstream_merge)
}

/// Wraps: git_branch_upstream_name
/// Resolves and writes the full upstream reference name.
pub fn git_branch_upstream_name(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    refname: &core::ffi::CStr,
) -> Result<(), i32> {
    branch_name_to_buffer(out, repo, refname, ffi::git_branch_upstream_name)
}

/// Wraps: git_branch_upstream_remote
/// Writes the configured upstream remote name.
pub fn git_branch_upstream_remote(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    refname: &core::ffi::CStr,
) -> Result<(), i32> {
    branch_name_to_buffer(out, repo, refname, ffi::git_branch_upstream_remote)
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn branch_names_are_checked_without_exposing_out_slots() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(git_branch_name_is_valid(Some(c"main")), Ok(true));
        assert_eq!(git_branch_name_is_valid(Some(c"-bad")), Ok(false));
        assert_eq!(git_branch_name_is_valid(None), Ok(false));
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn branch_iterator_preserves_the_ffi_layout() {
        assert_eq!(
            size_of::<GitBranchIterator>(),
            size_of::<ffi::git_branch_iterator>()
        );
        assert_eq!(
            align_of::<GitBranchIterator>(),
            align_of::<ffi::git_branch_iterator>()
        );
        assert_eq!(
            size_of::<GitBranchIteratorRef<'_>>(),
            size_of::<*mut ffi::git_branch_iterator>()
        );
        assert_eq!(
            size_of::<GitBranchIteratorMut<'_>>(),
            size_of::<*mut ffi::git_branch_iterator>()
        );
        assert_eq!(
            size_of::<Option<ffibox::CBox<GitBranchIterator>>>(),
            size_of::<*mut ffi::git_branch_iterator>()
        );
    }

    #[test]
    fn branch_iterator_handles_preserve_the_borrowed_pointer() {
        let mut storage = GitBranchIterator::zeroed();
        let ptr = core::ptr::addr_of_mut!(storage).cast::<ffi::git_branch_iterator>();

        // SAFETY: `ptr` addresses the live opaque storage above and the handle
        // does not escape its scope.
        let shared = unsafe { GitBranchIteratorRef::from_ptr(ptr) }.unwrap();
        assert_eq!(shared.as_ptr(), ptr.cast_const());

        // The shared handle is no longer used, so this test can exclusively
        // borrow the same live storage for the remainder of the scope.
        // SAFETY: `ptr` remains live and no other handle is subsequently used.
        let mut exclusive = unsafe { GitBranchIteratorMut::from_ptr(ptr) }.unwrap();
        assert_eq!(exclusive.as_mut_ptr(), ptr);
        assert_eq!(exclusive.as_ref().as_ptr(), ptr.cast_const());
    }

    #[test]
    fn failed_reference_result_accepts_an_empty_typed_owner() {
        assert!(matches!(reference_result(-123, None), Err(-123)));
    }
}

/// Wraps: git_branch_create
/// Creates a branch tied to the repository that owns its reference database.
pub fn git_branch_create<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    branch_name: &core::ffi::CStr,
    target: crate::commit::GitCommitRef<'_>,
    force: bool,
) -> Result<crate::refs::GitReferenceTetheredOwned<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the handles and name are live for the call, `out` is writable,
    // and success returns one reference whose repository borrow is retained.
    let status = unsafe {
        ffi::git_branch_create(
            core::ptr::addr_of_mut!(out),
            repo.as_ptr().cast_mut(),
            branch_name.as_ptr(),
            target.as_ptr(),
            i32::from(force),
        )
    };
    // SAFETY: `out` is null or a fresh reference owner from this constructor.
    let reference = unsafe { crate::refs::GitReferenceOwned::from_raw(out) };
    crate::refs::adopt_reference(status, reference)
}
