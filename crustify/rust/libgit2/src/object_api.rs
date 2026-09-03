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

/// Wraps: git_tag_owner
/// Borrows the repository that owns an annotated tag.
#[must_use]
pub fn git_tag_owner<'a>(tag: GitTagRef<'a>) -> GitRepositoryRef<'a> {
    // SAFETY: `tag` is live and libgit2 returns its non-null repository owner.
    let owner = unsafe { ffi::git_tag_owner(tag.as_ptr()) };
    // SAFETY: the returned repository remains live for the tag borrow.
    unsafe { GitRepositoryRef::from_ptr(owner) }.expect("a live tag has a repository owner")
}

/// Wraps: git_tree_lookup_prefix
/// Looks up a tree by a hexadecimal object-ID prefix.
pub fn git_tree_lookup_prefix<'repo>(
    repo: GitRepositoryRef<'repo>,
    id: OidRef<'_>,
    hex_len: usize,
) -> Result<RepositoryTree<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and both borrowed inputs remain live;
    // success transfers one tree cache reference tied to `repo`.
    let status = unsafe {
        ffi::git_tree_lookup_prefix(&mut output, repo.as_ptr().cast_mut(), id.as_ptr(), hex_len)
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one complete owned tree reference.
    let tree = unsafe { GitTreeOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryTree::from_owned(tree, repo))
}

/// Wraps: git_tree_owner
/// Borrows the repository that owns a tree.
#[must_use]
pub fn git_tree_owner<'a>(tree: GitTreeRef<'a>) -> GitRepositoryRef<'a> {
    // SAFETY: `tree` is live and libgit2 returns its non-null repository owner.
    let owner = unsafe { ffi::git_tree_owner(tree.as_ptr()) };
    // SAFETY: the returned repository remains live for the tree borrow.
    unsafe { GitRepositoryRef::from_ptr(owner) }.expect("a live tree has a repository owner")
}

/// Wraps: git_blob_lookup_prefix
/// Looks up a blob by an object-ID prefix and ties it to its repository.
pub fn git_blob_lookup_prefix<'repo>(
    repo: GitRepositoryRef<'repo>,
    id: OidRef<'_>,
    hex_len: usize,
) -> Result<RepositoryBlob<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output is writable and both handles are live; success transfers
    // one blob cache reference whose repository is retained by the result lifetime.
    let status = unsafe {
        ffi::git_blob_lookup_prefix(&mut out, repo.as_ptr().cast_mut(), id.as_ptr(), hex_len)
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete blob reference.
    let inner =
        unsafe { crate::blob::GitBlobOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryBlob {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_blob_owner
/// Borrows the repository that owns `blob`.
#[must_use]
pub fn git_blob_owner<'a>(blob: crate::blob::GitBlobRef<'a>) -> GitRepositoryRef<'a> {
    // SAFETY: a live cached blob has a non-null repository pointer that remains
    // live for at least the blob borrow.
    let raw = unsafe { ffi::git_blob_owner(blob.as_ptr()) };
    // SAFETY: the repository is the blob's retained owner and is non-null.
    unsafe { GitRepositoryRef::from_ptr(raw) }.expect("a live blob has an owner")
}

/// Wraps: git_commit_owner
/// Borrows the repository that owns `commit`.
#[must_use]
pub fn git_commit_owner<'a>(commit: crate::commit::GitCommitRef<'a>) -> GitRepositoryRef<'a> {
    // SAFETY: a live cached commit has a non-null repository pointer that
    // remains live for at least the commit borrow.
    let raw = unsafe { ffi::git_commit_owner(commit.as_ptr()) };
    // SAFETY: the repository is the commit's retained owner and is non-null.
    unsafe { GitRepositoryRef::from_ptr(raw) }.expect("a live commit has an owner")
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    type Observation = Vec<Vec<u8>>;

    unsafe fn raw_prefixes(repository: *mut ffi::git_repository) -> Observation {
        let mut commit_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut commit_id, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_lookup_prefix(&mut commit, repository, &commit_id, 10) },
            0
        );
        assert_eq!(unsafe { ffi::git_commit_owner(commit) }, repository);
        let mut tree = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_commit_tree(&mut tree, commit) }, 0);
        let tree_id = unsafe { *ffi::git_tree_id(tree) };
        let mut tree_prefix = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_tree_lookup_prefix(&mut tree_prefix, repository, &tree_id, 10) },
            0
        );
        assert_eq!(unsafe { ffi::git_tree_owner(tree_prefix) }, repository);
        let readme = unsafe { ffi::git_tree_entry_byname(tree_prefix, c"README.md".as_ptr()) };
        let blob_id = unsafe { *ffi::git_tree_entry_id(readme) };
        let mut blob = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_blob_lookup_prefix(&mut blob, repository, &blob_id, 10) },
            0
        );
        assert_eq!(unsafe { ffi::git_blob_owner(blob) }, repository);

        let mut tag_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(&mut tag_id, repository, c"refs/tags/v1.0".as_ptr())
            },
            0
        );
        let mut tag = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_tag_lookup_prefix(&mut tag, repository, &tag_id, 10) },
            0
        );
        assert_eq!(unsafe { ffi::git_tag_owner(tag) }, repository);
        let mut duplicate = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_tag_dup(&mut duplicate, tag) }, 0);
        let observed = vec![
            unsafe { (*ffi::git_commit_id(commit)).id }.to_vec(),
            unsafe { (*ffi::git_tree_id(tree_prefix)).id }.to_vec(),
            unsafe { (*ffi::git_blob_id(blob)).id }.to_vec(),
            unsafe { (*ffi::git_tag_id(tag)).id }.to_vec(),
            unsafe { (*ffi::git_tag_id(duplicate)).id }.to_vec(),
        ];
        unsafe {
            ffi::git_tag_free(duplicate);
            ffi::git_tag_free(tag);
            ffi::git_blob_free(blob);
            ffi::git_tree_free(tree_prefix);
            ffi::git_tree_free(tree);
            ffi::git_commit_free(commit);
        }
        observed
    }

    fn safe_prefixes(repository: *mut ffi::git_repository) -> Observation {
        let mut commit_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut commit_id, repository, c"HEAD".as_ptr()) },
            0
        );
        let commit_id = unsafe { OidRef::from_ptr(&raw mut commit_id) }.unwrap();
        let repository_ref = unsafe { GitRepositoryRef::from_ptr(repository) }.unwrap();
        let commit = git_commit_lookup_prefix(repository_ref, commit_id, 10).unwrap();
        assert_eq!(
            git_commit_owner(commit.as_ref()).as_ptr(),
            repository.cast_const()
        );
        let tree = crate::commit::git_commit_tree(commit.as_ref()).unwrap();
        let tree_prefix =
            git_tree_lookup_prefix(repository_ref, git_tree_id(tree.as_ref()), 10).unwrap();
        assert_eq!(
            git_tree_owner(tree_prefix.as_ref()).as_ptr(),
            repository.cast_const()
        );
        let readme =
            crate::tree::git_tree_entry_byname(tree_prefix.as_ref(), c"README.md").unwrap();
        let blob =
            git_blob_lookup_prefix(repository_ref, crate::tree::git_tree_entry_id(readme), 10)
                .unwrap();
        assert_eq!(
            git_blob_owner(blob.as_ref()).as_ptr(),
            repository.cast_const()
        );
        let mut tag_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(&mut tag_id, repository, c"refs/tags/v1.0".as_ptr())
            },
            0
        );
        let tag_id = unsafe { OidRef::from_ptr(&raw mut tag_id) }.unwrap();
        let repository_mut = unsafe { GitRepositoryMut::from_ptr(repository) }.unwrap();
        let tag = git_tag_lookup_prefix(repository_mut, tag_id, 10).unwrap();
        assert_eq!(
            git_tag_owner(tag.as_ref()).as_ptr(),
            repository.cast_const()
        );
        let duplicate = tag.clone();
        vec![
            git_commit_id(commit.as_ref()).raw_bytes().elems().collect(),
            git_tree_id(tree_prefix.as_ref())
                .raw_bytes()
                .elems()
                .collect(),
            git_blob_id(blob.as_ref()).raw_bytes().elems().collect(),
            git_tag_id(tag.as_ref()).raw_bytes().elems().collect(),
            git_tag_id(duplicate.as_ref()).raw_bytes().elems().collect(),
        ]
    }

    #[test]
    fn io_equiv_prefix_lookups_ids_owners_and_tag_duplication() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("object-prefix-raw");
        let safe = HistoryFixture::new("object-prefix-safe");
        let raw = unsafe { raw_prefixes(raw.repository.as_ptr()) };
        assert_eq!(raw, safe_prefixes(safe.repository.as_ptr()));
        assert_eq!(raw[3], raw[4]);
    }
}
