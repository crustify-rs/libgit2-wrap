//! Safe wrappers for libgit2 object_api APIs.

use core::marker::PhantomData;

use crate::ffi;
use crate::oid::OidRef;
use crate::repository::{GitRepositoryMut, GitRepositoryRef};
use crate::tag::{GitTagOwned, GitTagRef, RepositoryTag, adopt_repository_tag};
use crate::tree::{GitTreeOwned, GitTreeRef, RepositoryTree};

/// An owned blob tied to the repository that backs its object cache.
pub struct RepositoryBlob<'repo> {
    inner: crate::blob::GitBlobOwned,
    _repository: PhantomData<crate::repository::GitRepositoryRef<'repo>>,
}

impl RepositoryBlob<'_> {
    /// Borrows the blob.
    #[must_use]
    pub fn as_ref(&self) -> crate::blob::GitBlobRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the blob exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> crate::blob::GitBlobMut<'_> {
        self.inner.as_mut()
    }
}

/// An owned commit tied to the repository that backs its object cache.
pub struct RepositoryCommit<'repo> {
    inner: crate::commit::GitCommitOwned,
    _repository: PhantomData<crate::repository::GitRepositoryRef<'repo>>,
}

impl RepositoryCommit<'_> {
    /// Borrows the commit.
    #[must_use]
    pub fn as_ref(&self) -> crate::commit::GitCommitRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the commit exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> crate::commit::GitCommitMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_blob_id
/// Borrows a blob's inline object identifier.
#[must_use]
pub fn git_blob_id<'a>(blob: crate::blob::GitBlobRef<'a>) -> crate::oid::OidRef<'a> {
    // SAFETY: a live blob has a non-null inline ID kept alive by the blob.
    let id = unsafe { ffi::git_blob_id(blob.as_ptr()) };
    // SAFETY: the returned inline field is live for the input borrow.
    unsafe { crate::oid::OidRef::from_ptr(id.cast_mut()) }.expect("a live blob has an object ID")
}

/// Wraps: git_blob_lookup
/// Looks up a blob and ties it to its repository.
pub fn git_blob_lookup<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    id: crate::oid::OidRef<'_>,
) -> Result<RepositoryBlob<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and both input handles are live; success
    // transfers one blob cache reference tied to `repo`.
    let status = unsafe { ffi::git_blob_lookup(&mut out, repo.as_ptr().cast_mut(), id.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one fully initialized owned blob reference.
    let inner =
        unsafe { crate::blob::GitBlobOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryBlob {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_commit_id
/// Borrows a commit's inline object identifier.
#[must_use]
pub fn git_commit_id<'a>(commit: crate::commit::GitCommitRef<'a>) -> crate::oid::OidRef<'a> {
    // SAFETY: a live commit has a non-null inline ID kept alive by the commit.
    let id = unsafe { ffi::git_commit_id(commit.as_ptr()) };
    // SAFETY: the returned inline field is live for the input borrow.
    unsafe { crate::oid::OidRef::from_ptr(id.cast_mut()) }.expect("a live commit has an object ID")
}

/// Wraps: git_commit_lookup
/// Looks up a commit and ties it to its repository.
pub fn git_commit_lookup<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    id: crate::oid::OidRef<'_>,
) -> Result<RepositoryCommit<'repo>, i32> {
    commit_lookup_impl(repo, id, None)
}

/// Wraps: git_commit_lookup_prefix
/// Looks up a commit using the first `hex_len` hexadecimal ID digits.
pub fn git_commit_lookup_prefix<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    id: crate::oid::OidRef<'_>,
    hex_len: usize,
) -> Result<RepositoryCommit<'repo>, i32> {
    commit_lookup_impl(repo, id, Some(hex_len))
}

fn commit_lookup_impl<'repo>(
    repo: crate::repository::GitRepositoryRef<'repo>,
    id: crate::oid::OidRef<'_>,
    prefix: Option<usize>,
) -> Result<RepositoryCommit<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and both handles are live. Both constructors
    // transfer one commit cache reference on success.
    let status = unsafe {
        match prefix {
            Some(len) => {
                ffi::git_commit_lookup_prefix(&mut out, repo.as_ptr().cast_mut(), id.as_ptr(), len)
            }
            None => ffi::git_commit_lookup(&mut out, repo.as_ptr().cast_mut(), id.as_ptr()),
        }
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one fully initialized owned commit reference.
    let inner = unsafe { crate::commit::GitCommitOwned::from_raw(out) }
        .ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryCommit {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_tree_id
/// Borrows the tree's inline object ID.
#[must_use]
pub fn git_tree_id<'tree>(tree: GitTreeRef<'tree>) -> OidRef<'tree> {
    // SAFETY: `tree` is live and libgit2 returns its non-null inline ID.
    let id = unsafe { ffi::git_tree_id(tree.as_ptr()) }.cast_mut();
    // SAFETY: the inline ID remains live for the tree handle's lifetime.
    unsafe { OidRef::from_ptr(id) }.expect("a live tree has an object ID")
}

/// Wraps: git_tree_lookup
/// Looks up a tree and ties its owned cache reference to `repository`.
pub fn git_tree_lookup<'repo>(
    repository: GitRepositoryRef<'repo>,
    id: OidRef<'_>,
) -> Result<RepositoryTree<'repo>, i32> {
    let mut tree = core::ptr::null_mut();
    // SAFETY: the output slot is writable, both inputs are live, and success
    // writes one owned tree cache reference backed by `repository`.
    let status = unsafe {
        ffi::git_tree_lookup(
            core::ptr::addr_of_mut!(tree),
            repository.as_ptr().cast_mut(),
            id.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes one complete non-null owned tree reference.
    let tree = unsafe { GitTreeOwned::from_raw(tree) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryTree::from_owned(tree, repository))
}

/// Wraps: git_tag_id
/// Borrows the annotated tag object's own inline ID.
#[must_use]
pub fn git_tag_id<'a>(tag: GitTagRef<'a>) -> OidRef<'a> {
    // SAFETY: the live shared tag returns the non-null address of its inline
    // object ID, which remains valid for the tag borrow.
    let oid = unsafe { ffi::git_tag_id(tag.as_ptr()) };
    // SAFETY: the inline ID is non-null and live for `'a`.
    unsafe { OidRef::from_ptr(oid.cast_mut()) }.expect("an inline OID is non-null")
}

/// Wraps: git_tag_lookup
/// Looks up a repository-tethered annotated tag by full object ID.
pub fn git_tag_lookup<'repo>(
    mut repository: GitRepositoryMut<'repo>,
    id: OidRef<'_>,
) -> Result<RepositoryTag<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable, repository is exclusive for object-cache
    // access, and the borrowed ID remains live for the synchronous lookup.
    let status = unsafe {
        ffi::git_tag_lookup(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            id.as_ptr(),
        )
    };
    // SAFETY: the output is null on failure or one complete tag count.
    let output = unsafe { GitTagOwned::from_raw(output) };
    adopt_repository_tag(status, output)
}

/// Wraps: git_tag_lookup_prefix
/// Looks up a repository-tethered tag by an object-ID prefix length.
pub fn git_tag_lookup_prefix<'repo>(
    mut repository: GitRepositoryMut<'repo>,
    id: OidRef<'_>,
    prefix_len: usize,
) -> Result<RepositoryTag<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: as `git_tag_lookup`; libgit2 validates `prefix_len` against the
    // repository's configured object-ID algorithm.
    let status = unsafe {
        ffi::git_tag_lookup_prefix(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            id.as_ptr(),
            prefix_len,
        )
    };
    // SAFETY: the output is null on failure or one complete tag count.
    let output = unsafe { GitTagOwned::from_raw(output) };
    adopt_repository_tag(status, output)
}
