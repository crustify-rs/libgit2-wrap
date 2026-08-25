//! Safe wrappers for libgit2 commit APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CCloned};

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_commit
    /// An opaque, reference-counted Git commit.
    ///
    /// Owned handles release one reference with `git_commit_free`, while
    /// cloning acquires another reference with `git_commit_dup`. A commit
    /// internally borrows its repository, so operations that consult the
    /// repository require it to remain alive.
    GitCommit,
    GitCommitRef,
    GitCommitMut,
    ffi::git_commit
);

/// An owned reference to a libgit2 commit.
pub type GitCommitOwned = CBox<GitCommit>;

// SAFETY: `git_commit_free` consumes one reference to a fully initialized
// `git_commit`; the underlying object refcount releases the allocation only
// after the final reference, and `GitCommit` is transparent over the bindgen
// type.
ffibox::impl_dropped!(GitCommit, ffi::git_commit, ffi::git_commit_free);

// SAFETY: `git_commit_dup` increments the live commit's underlying object
// refcount and writes the same pointer to its non-null output slot. The new
// reference is independently released by the `CDropped` implementation above.
unsafe impl CCloned for GitCommit {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live commit; `duplicate` is
        // a valid output slot, and the wrapper is layout-compatible with
        // `ffi::git_commit`.
        let result = unsafe {
            ffi::git_commit_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_commit>(),
            )
        };
        debug_assert_eq!(result, 0);
        NonNull::new(duplicate.cast::<Self>())
    }
}

/// Wraps: git_commit_create_with_signature
/// Creates a commit object from raw commit text and an optional signature.
pub fn git_commit_create_with_signature(
    out: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    commit_content: &core::ffi::CStr,
    signature: Option<&core::ffi::CStr>,
    signature_field: Option<&core::ffi::CStr>,
) -> Result<(), i32> {
    let signature = signature.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    let signature_field = signature_field.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: both handles are live and exclusive, strings are live or null,
    // and libgit2 retains none of the input string pointers.
    let status = unsafe {
        ffi::git_commit_create_with_signature(
            out.as_mut_ptr(),
            repo.as_mut_ptr(),
            commit_content.as_ptr(),
            signature,
            signature_field,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_commit_extract_signature
/// Extracts a commit's signature and signed payload into owned buffers.
pub fn git_commit_extract_signature(
    signature: &mut crate::api::buffer::GitBufMut<'_>,
    signed_data: &mut crate::api::buffer::GitBufMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    commit_id: crate::oid::OidRef<'_>,
    field: Option<&core::ffi::CStr>,
) -> Result<(), i32> {
    let field = field.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: output buffers and repository are live and exclusive. Source
    // inspection shows `commit_id` is only read despite the legacy non-const C
    // signature; `field` is null or a live string.
    let status = unsafe {
        ffi::git_commit_extract_signature(
            signature.as_mut_ptr(),
            signed_data.as_mut_ptr(),
            repo.as_mut_ptr(),
            commit_id.as_ptr().cast_mut(),
            field,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::{CCloned, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<GitCommit>(), size_of::<ffi::git_commit>());
        assert_eq!(align_of::<GitCommit>(), align_of::<ffi::git_commit>());
        assert_eq!(
            size_of::<GitCommitRef<'_>>(),
            size_of::<*const ffi::git_commit>()
        );
        assert_eq!(
            size_of::<GitCommitMut<'_>>(),
            size_of::<*mut ffi::git_commit>()
        );
        assert_eq!(
            size_of::<Option<GitCommitOwned>>(),
            size_of::<*mut ffi::git_commit>()
        );
    }

    #[test]
    fn commit_registers_refcount_lifecycle() {
        fn assert_refcounted<T: CDropped + CCloned>() {}
        assert_refcounted::<GitCommit>();
    }

    #[test]
    fn borrowed_handles_preserve_the_commit_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_commit>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_commit>();

        {
            // SAFETY: `raw` addresses live storage for the bindgen opaque type
            // and remains allocated for the duration of this shared handle.
            let shared = unsafe { GitCommitRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitCommitMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_commit>>()) });
    }
}

fn optional_string(value: Option<&core::ffi::CStr>) -> *const core::ffi::c_char {
    value.map_or(core::ptr::null(), core::ffi::CStr::as_ptr)
}

fn status_result(status: i32) -> Result<(), i32> {
    if status == 0 { Ok(()) } else { Err(status) }
}

fn parent_pointers(parents: &[GitCommitRef<'_>]) -> Vec<*const ffi::git_commit> {
    parents.iter().map(GitCommitRef::as_ptr).collect()
}

/// Wraps: git_commit_amend
/// Amends a commit, retaining each omitted field from the original.
#[allow(clippy::too_many_arguments)]
pub fn git_commit_amend(
    id: &mut crate::oid::OidMut<'_>,
    commit: GitCommitRef<'_>,
    update_ref: Option<&core::ffi::CStr>,
    author: Option<crate::api::types::GitSignatureRef<'_>>,
    committer: Option<crate::api::types::GitSignatureRef<'_>>,
    message_encoding: Option<&core::ffi::CStr>,
    message: Option<&core::ffi::CStr>,
    tree: Option<crate::tree::GitTreeRef<'_>>,
) -> Result<(), i32> {
    // SAFETY: `id` is writable; every handle and non-null string is live for
    // the call, and libgit2 retains none of the borrowed inputs.
    let status = unsafe {
        ffi::git_commit_amend(
            id.as_mut_ptr(),
            commit.as_ptr(),
            optional_string(update_ref),
            author.map_or(core::ptr::null(), |signature| signature.as_ptr()),
            committer.map_or(core::ptr::null(), |signature| signature.as_ptr()),
            optional_string(message_encoding),
            optional_string(message),
            tree.map_or(core::ptr::null(), |tree| tree.as_ptr()),
        )
    };
    status_result(status)
}

/// Wraps: git_commit_author
/// Borrows the commit's author signature.
#[must_use]
pub fn git_commit_author<'a>(commit: GitCommitRef<'a>) -> crate::api::types::GitSignatureRef<'a> {
    // SAFETY: a parsed live commit has an initialized author field.
    let raw = unsafe { ffi::git_commit_author(commit.as_ptr()) };
    // SAFETY: the signature is embedded in and kept alive by `commit`.
    unsafe { crate::api::types::GitSignatureRef::from_ptr(raw.cast_mut()) }
        .expect("a live commit has an author")
}

fn mapped_signature(
    commit: GitCommitRef<'_>,
    mailmap: Option<crate::mailmap::GitMailmapRef<'_>>,
    author: bool,
) -> Result<crate::api::types::GitSignatureOwned, i32> {
    let mut out = core::ptr::null_mut();
    let mailmap = mailmap.map_or(core::ptr::null(), |value| value.as_ptr());
    // SAFETY: `out` is writable and both optional/shared inputs are live; the
    // selected function transfers a freshly allocated signature on success.
    let status = unsafe {
        if author {
            ffi::git_commit_author_with_mailmap(&mut out, commit.as_ptr(), mailmap)
        } else {
            ffi::git_commit_committer_with_mailmap(&mut out, commit.as_ptr(), mailmap)
        }
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized signature owner.
    unsafe { crate::api::types::GitSignatureOwned::from_raw(out) }
        .ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_commit_author_with_mailmap
/// Resolves and owns the commit's author signature.
pub fn git_commit_author_with_mailmap(
    commit: GitCommitRef<'_>,
    mailmap: Option<crate::mailmap::GitMailmapRef<'_>>,
) -> Result<crate::api::types::GitSignatureOwned, i32> {
    mapped_signature(commit, mailmap, true)
}

/// Wraps: git_commit_body
/// Borrows the lazily parsed body, if present.
#[must_use]
pub fn git_commit_body<'a>(mut commit: GitCommitMut<'a>) -> Option<&'a core::ffi::CStr> {
    // SAFETY: the exclusive handle permits the function to populate its lazy
    // cache; any returned pointer is then kept alive by the commit.
    let raw = unsafe { ffi::git_commit_body(commit.as_mut_ptr()) };
    if raw.is_null() {
        None
    } else {
        // SAFETY: the non-null result is NUL-terminated and commit-owned.
        Some(unsafe { core::ffi::CStr::from_ptr(raw) })
    }
}

/// Wraps: git_commit_committer
/// Borrows the commit's committer signature.
#[must_use]
pub fn git_commit_committer<'a>(
    commit: GitCommitRef<'a>,
) -> crate::api::types::GitSignatureRef<'a> {
    // SAFETY: a parsed live commit has an initialized committer field.
    let raw = unsafe { ffi::git_commit_committer(commit.as_ptr()) };
    // SAFETY: the signature is embedded in and kept alive by `commit`.
    unsafe { crate::api::types::GitSignatureRef::from_ptr(raw.cast_mut()) }
        .expect("a live commit has a committer")
}

/// Wraps: git_commit_committer_with_mailmap
/// Resolves and owns the commit's committer signature.
pub fn git_commit_committer_with_mailmap(
    commit: GitCommitRef<'_>,
    mailmap: Option<crate::mailmap::GitMailmapRef<'_>>,
) -> Result<crate::api::types::GitSignatureOwned, i32> {
    mapped_signature(commit, mailmap, false)
}

/// Wraps: git_commit_create
/// Creates a commit and writes its ID into `id`.
#[allow(clippy::too_many_arguments)]
pub fn git_commit_create(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    update_ref: Option<&core::ffi::CStr>,
    author: crate::api::types::GitSignatureRef<'_>,
    committer: crate::api::types::GitSignatureRef<'_>,
    message_encoding: Option<&core::ffi::CStr>,
    message: &core::ffi::CStr,
    tree: crate::tree::GitTreeRef<'_>,
    parents: &[GitCommitRef<'_>],
) -> Result<(), i32> {
    let mut parent_pointers = parent_pointers(parents);
    // SAFETY: the output and repository are exclusive, every borrowed input
    // is live, and the pointer array contains exactly `parents.len()` entries.
    let status = unsafe {
        ffi::git_commit_create(
            id.as_mut_ptr(),
            repo.as_mut_ptr(),
            optional_string(update_ref),
            author.as_ptr(),
            committer.as_ptr(),
            optional_string(message_encoding),
            message.as_ptr(),
            tree.as_ptr(),
            parent_pointers.len(),
            parent_pointers.as_mut_ptr(),
        )
    };
    status_result(status)
}

/// Wraps: git_commit_create_buffer
/// Serializes a commit into `out` without writing the object database.
#[allow(clippy::too_many_arguments)]
pub fn git_commit_create_buffer(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    author: crate::api::types::GitSignatureRef<'_>,
    committer: crate::api::types::GitSignatureRef<'_>,
    message_encoding: Option<&core::ffi::CStr>,
    message: &core::ffi::CStr,
    tree: crate::tree::GitTreeRef<'_>,
    parents: &[GitCommitRef<'_>],
) -> Result<(), i32> {
    let mut parent_pointers = parent_pointers(parents);
    // SAFETY: both mutable handles are exclusive, all shared inputs are live,
    // and the temporary parent array matches its supplied count.
    let status = unsafe {
        ffi::git_commit_create_buffer(
            out.as_mut_ptr(),
            repo.as_mut_ptr(),
            author.as_ptr(),
            committer.as_ptr(),
            optional_string(message_encoding),
            message.as_ptr(),
            tree.as_ptr(),
            parent_pointers.len(),
            parent_pointers.as_mut_ptr(),
        )
    };
    status_result(status)
}

/// Wraps: git_commit_header_field
/// Copies one commit header field into `out`.
pub fn git_commit_header_field(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    commit: GitCommitRef<'_>,
    field: &core::ffi::CStr,
) -> Result<(), i32> {
    // SAFETY: `out` is exclusive and both shared inputs are live for the call.
    status_result(unsafe {
        ffi::git_commit_header_field(out.as_mut_ptr(), commit.as_ptr(), field.as_ptr())
    })
}

unsafe fn commit_string<'a>(raw: *const core::ffi::c_char) -> Option<&'a core::ffi::CStr> {
    if raw.is_null() {
        None
    } else {
        // SAFETY: callers only pass commit-owned NUL-terminated pointers and
        // bind `'a` to the commit handle that keeps them alive.
        Some(unsafe { core::ffi::CStr::from_ptr(raw) })
    }
}

/// Wraps: git_commit_message
/// Borrows the normalized commit message, if present.
#[must_use]
pub fn git_commit_message<'a>(commit: GitCommitRef<'a>) -> Option<&'a core::ffi::CStr> {
    // SAFETY: `commit` is live and the accessor only reads commit-owned data.
    // SAFETY: the returned string is commit-owned and bounded by `'a`.
    unsafe { commit_string(ffi::git_commit_message(commit.as_ptr())) }
}

/// Wraps: git_commit_message_encoding
/// Borrows the declared message encoding, if the header supplied one.
#[must_use]
pub fn git_commit_message_encoding<'a>(commit: GitCommitRef<'a>) -> Option<&'a core::ffi::CStr> {
    // SAFETY: `commit` is live and the accessor only reads commit-owned data.
    // SAFETY: the returned string is commit-owned and bounded by `'a`.
    unsafe { commit_string(ffi::git_commit_message_encoding(commit.as_ptr())) }
}

/// Wraps: git_commit_message_raw
/// Borrows the unmodified commit message, if present.
#[must_use]
pub fn git_commit_message_raw<'a>(commit: GitCommitRef<'a>) -> Option<&'a core::ffi::CStr> {
    // SAFETY: `commit` is live and the accessor only reads commit-owned data.
    // SAFETY: the returned string is commit-owned and bounded by `'a`.
    unsafe { commit_string(ffi::git_commit_message_raw(commit.as_ptr())) }
}

/// An owned parent commit tied to the commit whose repository it shares.
pub struct CommitParent<'a> {
    inner: GitCommitOwned,
    _commit: core::marker::PhantomData<GitCommitRef<'a>>,
}

impl CommitParent<'_> {
    /// Borrows the parent commit.
    #[must_use]
    pub fn as_ref(&self) -> GitCommitRef<'_> {
        self.inner.as_ref()
    }
}

/// Wraps: git_commit_parent
/// Loads parent `index` and ties it to the source commit's repository.
pub fn git_commit_parent<'a>(
    commit: GitCommitRef<'a>,
    index: u32,
) -> Result<CommitParent<'a>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and `commit` is live. Success transfers one
    // parent cache reference sharing the source commit's repository.
    let status = unsafe { ffi::git_commit_parent(&mut out, commit.as_ptr(), index) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one initialized commit owner.
    let inner = unsafe { GitCommitOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(CommitParent {
        inner,
        _commit: core::marker::PhantomData,
    })
}

/// Wraps: git_commit_parent_id
/// Borrows a parent ID, returning `None` when `index` is out of range.
#[must_use]
pub fn git_commit_parent_id<'a>(
    commit: GitCommitRef<'a>,
    index: u32,
) -> Option<crate::oid::OidRef<'a>> {
    // SAFETY: `commit` is live; C returns null or an inline commit-owned ID.
    let raw = unsafe { ffi::git_commit_parent_id(commit.as_ptr(), index) };
    // SAFETY: a non-null returned field stays live for the commit borrow.
    unsafe { crate::oid::OidRef::from_ptr(raw.cast_mut()) }
}

/// Wraps: git_commit_parentcount
/// Returns the number of parents.
#[must_use]
pub fn git_commit_parentcount(commit: GitCommitRef<'_>) -> u32 {
    // SAFETY: `commit` is live and the accessor only reads its parent vector.
    unsafe { ffi::git_commit_parentcount(commit.as_ptr()) }
}

/// Wraps: git_commit_raw_header
/// Borrows the commit's complete raw header.
#[must_use]
pub fn git_commit_raw_header<'a>(commit: GitCommitRef<'a>) -> &'a core::ffi::CStr {
    // SAFETY: a parsed live commit owns a NUL-terminated raw header.
    let raw = unsafe { ffi::git_commit_raw_header(commit.as_ptr()) };
    // SAFETY: the returned string is commit-owned and bounded by `'a`.
    unsafe { commit_string(raw) }.expect("a live commit has a raw header")
}

/// Wraps: git_commit_summary
/// Borrows the lazily parsed one-line summary, if present.
#[must_use]
pub fn git_commit_summary<'a>(mut commit: GitCommitMut<'a>) -> Option<&'a core::ffi::CStr> {
    // SAFETY: the exclusive handle permits lazy cache initialization.
    // SAFETY: the returned string is commit-owned and bounded by `'a`.
    unsafe { commit_string(ffi::git_commit_summary(commit.as_mut_ptr())) }
}

/// Wraps: git_commit_time
/// Returns the committer timestamp in seconds since the Unix epoch.
#[must_use]
pub fn git_commit_time(commit: GitCommitRef<'_>) -> i64 {
    // SAFETY: `commit` is live and the accessor only reads its signature.
    unsafe { ffi::git_commit_time(commit.as_ptr()) }
}

/// Wraps: git_commit_time_offset
/// Returns the committer timezone offset in minutes.
#[must_use]
pub fn git_commit_time_offset(commit: GitCommitRef<'_>) -> i32 {
    // SAFETY: `commit` is live and the accessor only reads its signature.
    unsafe { ffi::git_commit_time_offset(commit.as_ptr()) }
}

/// An owned tree tied to the commit whose repository it shares.
pub struct CommitTree<'a> {
    inner: crate::tree::GitTreeOwned,
    _commit: core::marker::PhantomData<GitCommitRef<'a>>,
}

impl CommitTree<'_> {
    /// Borrows the tree.
    #[must_use]
    pub fn as_ref(&self) -> crate::tree::GitTreeRef<'_> {
        self.inner.as_ref()
    }
}

/// Wraps: git_commit_tree
/// Loads the commit's tree and ties it to the commit's repository.
pub fn git_commit_tree<'a>(commit: GitCommitRef<'a>) -> Result<CommitTree<'a>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and `commit` is live; success transfers one
    // tree cache reference sharing the source commit's repository.
    let status = unsafe { ffi::git_commit_tree(&mut out, commit.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one initialized owned tree reference.
    let inner =
        unsafe { crate::tree::GitTreeOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(CommitTree {
        inner,
        _commit: core::marker::PhantomData,
    })
}

/// Wraps: git_commit_tree_id
/// Borrows the commit's tree ID.
#[must_use]
pub fn git_commit_tree_id<'a>(commit: GitCommitRef<'a>) -> crate::oid::OidRef<'a> {
    // SAFETY: a parsed live commit has a non-null inline tree ID.
    let raw = unsafe { ffi::git_commit_tree_id(commit.as_ptr()) };
    // SAFETY: the returned field remains live for the commit borrow.
    unsafe { crate::oid::OidRef::from_ptr(raw.cast_mut()) }.expect("a live commit has a tree ID")
}

ffibox::define_ctype!(
    /// Wraps: git_commitbuilder
    /// A transient, opaque accessor passed to commit-signing callbacks.
    ///
    /// The builder borrows the in-progress commit buffer for exactly the
    /// callback invocation. It is never independently allocated or freed;
    /// callback wrappers should therefore expose only `GitCommitbuilderMut`.
    GitCommitbuilder,
    GitCommitbuilderRef,
    GitCommitbuilderMut,
    ffi::git_commitbuilder
);

#[cfg(test)]
mod commitbuilder_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn commitbuilder_wrapper_matches_the_opaque_public_type() {
        assert_eq!(
            size_of::<GitCommitbuilder>(),
            size_of::<ffi::git_commitbuilder>()
        );
        assert_eq!(
            align_of::<GitCommitbuilder>(),
            align_of::<ffi::git_commitbuilder>()
        );
    }
}

/// Wraps: git_commitbuilder_add_header
/// Adds or replaces a header in a transient commit builder.
pub fn git_commitbuilder_add_header(
    builder: &mut GitCommitbuilderMut<'_>,
    field: &core::ffi::CStr,
    value: &core::ffi::CStr,
) -> Result<(), i32> {
    // SAFETY: the builder is exclusively borrowed and both strings remain
    // live while libgit2 copies them into builder-owned storage.
    let status = unsafe {
        ffi::git_commitbuilder_add_header(builder.as_mut_ptr(), field.as_ptr(), value.as_ptr())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_commit_create_ext
/// Creates a commit with extended creation options.
#[allow(clippy::too_many_arguments)]
pub fn git_commit_create_ext(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    author: crate::api::types::GitSignatureRef<'_>,
    committer: crate::api::types::GitSignatureRef<'_>,
    message: &core::ffi::CStr,
    tree: crate::tree::GitTreeRef<'_>,
    parents: &[GitCommitRef<'_>],
    options: crate::api::commit::GitCommitCreateExtOptionsRef<'_>,
) -> Result<(), i32> {
    let parent_pointers: Vec<_> = parents
        .iter()
        .map(|parent| parent.as_ptr().cast_mut())
        .collect();
    // SAFETY: all typed inputs are live, output and repository are exclusive,
    // and the parent pointer array has exactly the supplied count.
    let status = unsafe {
        ffi::git_commit_create_ext(
            id.as_mut_ptr(),
            repo.as_mut_ptr(),
            author.as_ptr(),
            committer.as_ptr(),
            message.as_ptr(),
            tree.as_ptr(),
            parent_pointers.len(),
            parent_pointers.as_ptr(),
            options.as_ptr(),
        )
    };
    status_result(status)
}

/// Wraps: git_commit_create_ext_options_init
/// Creates extended commit options initialized for this ABI.
pub fn git_commit_create_ext_options_init()
-> Result<ffibox::CVal<crate::api::commit::GitCommitCreateExtOptions>, i32> {
    let mut options = ffibox::CVal::new(crate::api::commit::GitCommitCreateExtOptions::zeroed());
    // SAFETY: the inline options storage is exclusively writable.
    let status = unsafe {
        ffi::git_commit_create_ext_options_init(
            options.as_mut().as_mut_ptr(),
            ffi::GIT_COMMIT_CREATE_EXT_OPTIONS_VERSION,
        )
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_commit_create_v
/// Safe slice-based equivalent of the C variadic commit constructor.
#[allow(clippy::too_many_arguments)]
pub fn git_commit_create_v(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    update_ref: Option<&core::ffi::CStr>,
    author: crate::api::types::GitSignatureRef<'_>,
    committer: crate::api::types::GitSignatureRef<'_>,
    message_encoding: Option<&core::ffi::CStr>,
    message: &core::ffi::CStr,
    tree: crate::tree::GitTreeRef<'_>,
    parents: &[GitCommitRef<'_>],
) -> Result<(), i32> {
    git_commit_create(
        id,
        repo,
        update_ref,
        author,
        committer,
        message_encoding,
        message,
        tree,
        parents,
    )
}

/// Wraps: git_commit_nth_gen_ancestor
/// Loads the `n`th first-parent ancestor, tied to the source repository.
pub fn git_commit_nth_gen_ancestor<'a>(
    commit: GitCommitRef<'a>,
    n: u32,
) -> Result<CommitParent<'a>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: output is writable and the source commit remains live; success
    // transfers one commit reference backed by the same repository.
    let status = unsafe { ffi::git_commit_nth_gen_ancestor(&mut out, commit.as_ptr(), n) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully initialized commit owner.
    let inner = unsafe { GitCommitOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(CommitParent {
        inner,
        _commit: core::marker::PhantomData,
    })
}
