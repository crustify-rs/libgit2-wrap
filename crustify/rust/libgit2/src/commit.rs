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
///
/// Rust can only call a C variadic with a compile-time argument list, so a
/// runtime-length parent list has to reach libgit2 as an array. This delegates
/// to [`git_commit_create`], which C's own `git_commit_create_v` matches field
/// for field: both build `GIT_COMMIT_CREATE_EXT_OPTIONS_INIT` with the same
/// `update_ref` and `message_encoding` and hand it to
/// `git_commit__create_internal`; only the parent callback differs.
///
/// That callback is the one behavioural difference: the array form also
/// checks `git_commit_owner(parent) == repo` for every parent, which the
/// varargs form does not. Parents from another repository are therefore
/// rejected here where C's variadic would have accepted them.
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

/// Wraps: git_commit_amend_from_stage
/// Amends `HEAD` using the repository's staged tree.
pub fn git_commit_amend_from_stage(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    message: Option<&core::ffi::CStr>,
    options: Option<crate::api::commit::GitCommitCreateOptionsRef<'_>>,
) -> Result<(), i32> {
    // SAFETY: the output and repository are exclusively borrowed, and every
    // optional input remains live for this synchronous, non-retaining call.
    status_result(unsafe {
        ffi::git_commit_amend_from_stage(
            id.as_mut_ptr(),
            repo.as_mut_ptr(),
            optional_string(message),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    })
}

/// Wraps: git_commit_amend_from_tree
/// Amends `HEAD` using `tree`, retaining the old message when omitted.
///
/// # Composition
///
/// Every safe route to a [`GitTreeRef`](crate::tree::GitTreeRef) — a
/// [`RepositoryTree`](crate::tree::RepositoryTree) from
/// [`git_tree_lookup`](crate::object_api::git_tree_lookup) or a
/// [`CommitTree`] from [`git_commit_tree`] — pins a *shared*
/// borrow of the repository owner, while this wrapper takes that owner
/// exclusively, so the two cannot be held at once. Reaching this entry point
/// therefore still needs a tree adopted through the raw seam. Closing the gap
/// belongs to the tree lookups: returning a `RepositoryTree<'repo>` from a
/// transient `&mut GitRepositoryMut<'repo>` reborrow, the way
/// [`git_diff_tree_to_tree`](crate::diff_generate::git_diff_tree_to_tree)
/// already returns its diff, would make the pair compose.
pub fn git_commit_amend_from_tree(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    tree: crate::tree::GitTreeRef<'_>,
    message: Option<&core::ffi::CStr>,
    options: Option<crate::api::commit::GitCommitCreateOptionsRef<'_>>,
) -> Result<(), i32> {
    // SAFETY: the output and repository are exclusively borrowed, and the
    // tree and optional inputs remain live for this non-retaining call.
    status_result(unsafe {
        ffi::git_commit_amend_from_tree(
            id.as_mut_ptr(),
            repo.as_mut_ptr(),
            tree.as_ptr(),
            optional_string(message),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    })
}

/// Wraps: git_commit_create_from_stage
/// Creates a commit from the repository's staged changes.
pub fn git_commit_create_from_stage(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    message: &core::ffi::CStr,
    options: Option<crate::api::commit::GitCommitCreateOptionsRef<'_>>,
) -> Result<(), i32> {
    // SAFETY: the output and repository are exclusively borrowed; the
    // message and optional options are live and are not retained.
    status_result(unsafe {
        ffi::git_commit_create_from_stage(
            id.as_mut_ptr(),
            repo.as_mut_ptr(),
            message.as_ptr(),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    })
}

/// Wraps: git_commit_create_from_tree
/// Creates a commit from an existing tree.
///
/// `git_commit_create_ext` asserts `git_tree_owner(tree) == repo`, so `tree`
/// has to come from `repo` rather than from any repository handle.
///
/// # Composition
///
/// Every safe route to a [`GitTreeRef`](crate::tree::GitTreeRef) — a
/// [`RepositoryTree`](crate::tree::RepositoryTree) from
/// [`git_tree_lookup`](crate::object_api::git_tree_lookup) or a
/// [`CommitTree`] from [`git_commit_tree`] — pins a *shared*
/// borrow of the repository owner, while this wrapper takes that owner
/// exclusively, so the two cannot be held at once. Reaching this entry point
/// therefore still needs a tree adopted through the raw seam. Closing the gap
/// belongs to the tree lookups: returning a `RepositoryTree<'repo>` from a
/// transient `&mut GitRepositoryMut<'repo>` reborrow, the way
/// [`git_diff_tree_to_tree`](crate::diff_generate::git_diff_tree_to_tree)
/// already returns its diff, would make the pair compose.
pub fn git_commit_create_from_tree(
    id: &mut crate::oid::OidMut<'_>,
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    tree: crate::tree::GitTreeRef<'_>,
    message: &core::ffi::CStr,
    options: Option<crate::api::commit::GitCommitCreateOptionsRef<'_>>,
) -> Result<(), i32> {
    // SAFETY: the output and repository are exclusively borrowed; the tree,
    // message and optional options are live and are not retained.
    status_result(unsafe {
        ffi::git_commit_create_from_tree(
            id.as_mut_ptr(),
            repo.as_mut_ptr(),
            tree.as_ptr(),
            message.as_ptr(),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    })
}

/// Wraps: git_commit_create_options_init
/// Creates simple commit options initialized for `version`.
pub fn git_commit_create_options_init(
    version: core::ffi::c_uint,
) -> Result<ffibox::CVal<crate::api::commit::GitCommitCreateOptions>, i32> {
    let mut options = crate::api::commit::GitCommitCreateOptions::new();
    // SAFETY: the inline options storage is exclusively writable and C
    // retains no pointer to it.
    let status =
        unsafe { ffi::git_commit_create_options_init(options.as_mut().as_mut_ptr(), version) };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_commitarray_dispose
/// Releases every commit and the pointer array owned by `array`.
pub fn git_commitarray_dispose(array: ffibox::CVal<crate::api::commit::GitCommitArray>) {
    drop(array);
}

#[cfg(test)]
mod simple_commit_api_tests {
    use super::*;

    #[test]
    fn simple_options_initializer_and_empty_array_disposer_are_safe() {
        let options = git_commit_create_options_init(ffi::GIT_COMMIT_CREATE_OPTIONS_VERSION)
            .expect("the published options version initializes");
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_COMMIT_CREATE_OPTIONS_VERSION
        );
        assert!(!options.as_ref().allow_empty_commit());

        git_commitarray_dispose(crate::api::commit::GitCommitArray::new());
    }
}

#[cfg(test)]
mod scheduled_creation_and_amend_tests {
    use core::ptr::{addr_of, addr_of_mut};

    use super::*;
    use crate::api::commit::{GitCommitCreateOptions, GitCommitCreateOptionsRef};
    use crate::oid::{Oid, OidMut, OidRef, OidType};
    use crate::repository::{GitRepositoryInitFlags, GitRepositoryOwned};

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

    /// Reads the message of the commit `HEAD` currently names.
    fn head_message(repository: &mut GitRepositoryOwned) -> std::ffi::CString {
        let id = crate::refs::git_reference_name_to_id(&mut repository.as_mut(), c"HEAD")
            .expect("HEAD resolves to an object");
        // SAFETY: `id` is live, initialized local storage borrowed only by
        // this shared handle.
        let id = unsafe { OidRef::from_ptr(addr_of!(id).cast_mut().cast()) }
            .expect("the address of a local value is non-null");
        let commit = crate::object_api::git_commit_lookup(repository.as_ref(), id)
            .expect("HEAD names a commit");
        let message = crate::commit::git_commit_message(commit.as_ref())
            .expect("a created commit carries a message");
        message.to_owned()
    }

    /// Runs one scheduled creation entry point over a fresh output slot.
    fn create(
        repository: &mut GitRepositoryOwned,
        options: GitCommitCreateOptionsRef<'_>,
        call: impl FnOnce(
            &mut OidMut<'_>,
            &mut crate::repository::GitRepositoryMut<'_>,
            GitCommitCreateOptionsRef<'_>,
        ) -> Result<(), i32>,
    ) -> Oid {
        let mut created = Oid::zeroed();
        {
            // SAFETY: `created` is live, initialized, exclusively borrowed
            // local storage for the whole life of this handle.
            let mut out = unsafe { OidMut::from_ptr(addr_of_mut!(created).cast()) }
                .expect("the address of a local value is non-null");
            call(&mut out, &mut repository.as_mut(), options).expect("the commit is created");
        }
        created
    }

    /// Walks `HEAD` through all four scheduled commit entry points.
    ///
    /// Every one of them ends in `git_commit__create_internal` with
    /// `update_ref` pointing at `HEAD`, so each call has to leave `HEAD`
    /// naming the object it just wrote. The two amending forms additionally
    /// keep the previous commit's message when none is supplied, which is the
    /// only place the wrapper's optional message is observable.
    #[test]
    fn creating_and_amending_walk_head_through_the_scheduled_entry_points() {
        let _libgit2 = Libgit2Init::acquire();

        let directory = std::env::temp_dir().join(format!(
            "crustify-commit-entry-points-{}-{:?}",
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
        init_options.as_mut().set_oid_type(Some(OidType::Sha256));
        init_options
            .as_mut()
            .set_flags(GitRepositoryInitFlags::MKPATH | GitRepositoryInitFlags::BARE);
        let mut repository =
            crate::repository::git_repository_init_ext(&path, &mut init_options.as_mut())
                .expect("a fresh directory initializes as a bare repository");

        let signature = crate::signature::git_signature_now(c"Crustify", c"crustify@example.com")
            .expect("a signature stamped with the current time");
        let mut options = GitCommitCreateOptions::new();
        // Every commit below has the same (empty) tree as its parent, so the
        // creating forms need the empty-commit gate opened.
        options.as_mut().set_allow_empty_commit(true);
        // SAFETY: the signature owner outlives the options and every call
        // that reads them below.
        unsafe {
            options
                .as_mut()
                .set_borrowed_author(Some(signature.as_ref()));
            options
                .as_mut()
                .set_borrowed_committer(Some(signature.as_ref()));
        }

        let initial = create(&mut repository, options.as_ref(), |out, repo, opts| {
            git_commit_create_from_stage(out, repo, c"initial", Some(opts))
        });
        assert_eq!(head_message(&mut repository), c"initial".to_owned());

        // The staged tree is the tree of the commit just written, so it is
        // also what the by-tree forms are handed.
        //
        // Reaching it goes through the raw seam on purpose. A tree looked up
        // with `git_tree_lookup` comes back as a `RepositoryTree<'repo>` whose
        // `'repo` is a *shared* borrow of the repository owner, while the
        // by-tree entry points below take that same owner exclusively, so safe
        // code cannot hold both at once. The adopted owner here carries no
        // repository tether, and this test keeps the repository alive around
        // every use of it.
        let tree_id = {
            // SAFETY: `initial` is live, initialized local storage borrowed
            // only by this shared handle.
            let initial = unsafe { OidRef::from_ptr(addr_of!(initial).cast_mut().cast()) }
                .expect("the address of a local value is non-null");
            let commit = crate::object_api::git_commit_lookup(repository.as_ref(), initial)
                .expect("the created commit is readable");
            crate::oid::git_oid_cpy(git_commit_tree_id(commit.as_ref()))
        };
        let mut raw_tree = core::ptr::null_mut();
        let status = {
            // SAFETY: `tree_id` is live, initialized local storage borrowed
            // only by this shared handle.
            let tree_id = unsafe { OidRef::from_ptr(addr_of!(tree_id).cast_mut().cast()) }
                .expect("the address of a local value is non-null");
            // SAFETY: the output slot is writable and both the repository and
            // the object ID are live for this synchronous lookup.
            unsafe {
                ffi::git_tree_lookup(
                    addr_of_mut!(raw_tree),
                    repository.as_ref().as_ptr().cast_mut(),
                    tree_id.as_ptr(),
                )
            }
        };
        assert_eq!(status, 0, "the staged tree is readable");
        // SAFETY: success transferred one complete owned tree reference, and
        // `repository` outlives every use of it below.
        let tree = unsafe { crate::tree::GitTreeOwned::from_raw(raw_tree) }
            .expect("a successful lookup returns a tree");

        let from_tree = create(&mut repository, options.as_ref(), |out, repo, opts| {
            git_commit_create_from_tree(out, repo, tree.as_ref(), c"from tree", Some(opts))
        });
        assert_ne!(
            crate::oid::git_oid_tostr_s(
                // SAFETY: live, initialized local storage borrowed only here.
                unsafe { OidRef::from_ptr(addr_of!(from_tree).cast_mut().cast()) }.unwrap()
            ),
            crate::oid::git_oid_tostr_s(
                // SAFETY: live, initialized local storage borrowed only here.
                unsafe { OidRef::from_ptr(addr_of!(initial).cast_mut().cast()) }.unwrap()
            ),
        );
        assert_eq!(head_message(&mut repository), c"from tree".to_owned());

        // Amending without a message keeps the amended commit's own message.
        let _ = create(&mut repository, options.as_ref(), |out, repo, opts| {
            git_commit_amend_from_tree(out, repo, tree.as_ref(), None, Some(opts))
        });
        assert_eq!(head_message(&mut repository), c"from tree".to_owned());

        let _ = create(&mut repository, options.as_ref(), |out, repo, opts| {
            git_commit_amend_from_tree(out, repo, tree.as_ref(), Some(c"retitled"), Some(opts))
        });
        assert_eq!(head_message(&mut repository), c"retitled".to_owned());

        // The staged form reaches the same amend through the repository index.
        let _ = create(&mut repository, options.as_ref(), |out, repo, opts| {
            git_commit_amend_from_stage(out, repo, Some(c"amended"), Some(opts))
        });
        assert_eq!(head_message(&mut repository), c"amended".to_owned());

        let _ = create(&mut repository, options.as_ref(), |out, repo, opts| {
            git_commit_amend_from_stage(out, repo, None, Some(opts))
        });
        assert_eq!(head_message(&mut repository), c"amended".to_owned());

        drop(tree);
        drop(options);
        drop(signature);
        drop(repository);
        let _ = std::fs::remove_dir_all(&directory);
    }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use core::ffi::CStr;

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, RawBuf, safe_buf_bytes};

    #[derive(Debug, Eq, PartialEq)]
    struct CommitObservation {
        message: Vec<u8>,
        message_raw: Vec<u8>,
        summary: Vec<u8>,
        body: Vec<u8>,
        encoding: Vec<u8>,
        raw_header: Vec<u8>,
        tree_header: Vec<u8>,
        author: (Vec<u8>, Vec<u8>, i64, i32),
        committer: (Vec<u8>, Vec<u8>, i64, i32),
        mapped_author: (Vec<u8>, Vec<u8>),
        parent_count: u32,
        parent_id: Vec<u8>,
        ancestor_id: Vec<u8>,
        tree_id: Vec<u8>,
        tree_entries: Vec<(Vec<u8>, u32, i32)>,
        short_id: Vec<u8>,
    }

    unsafe fn optional_cstr_bytes(value: *const core::ffi::c_char) -> Vec<u8> {
        if value.is_null() {
            Vec::new()
        } else {
            unsafe { CStr::from_ptr(value) }.to_bytes().to_vec()
        }
    }

    unsafe fn raw_signature(value: *const ffi::git_signature) -> (Vec<u8>, Vec<u8>, i64, i32) {
        assert!(!value.is_null());
        (
            unsafe { CStr::from_ptr((*value).name) }.to_bytes().to_vec(),
            unsafe { CStr::from_ptr((*value).email) }
                .to_bytes()
                .to_vec(),
            unsafe { (*value).when.time },
            unsafe { (*value).when.offset },
        )
    }

    fn safe_signature(
        value: crate::api::types::GitSignatureRef<'_>,
    ) -> (Vec<u8>, Vec<u8>, i64, i32) {
        (
            value.name().to_bytes().to_vec(),
            value.email().to_bytes().to_vec(),
            value.when().time(),
            value.when().offset(),
        )
    }

    unsafe fn raw_observation(fixture: &HistoryFixture) -> CommitObservation {
        let repository = fixture.repository.as_ptr();
        let mut head_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head_id, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut commit, repository, &head_id) },
            0
        );

        let message = unsafe { optional_cstr_bytes(ffi::git_commit_message(commit)) };
        let message_raw = unsafe { optional_cstr_bytes(ffi::git_commit_message_raw(commit)) };
        let summary = unsafe { optional_cstr_bytes(ffi::git_commit_summary(commit)) };
        let body = unsafe { optional_cstr_bytes(ffi::git_commit_body(commit)) };
        let encoding = unsafe { optional_cstr_bytes(ffi::git_commit_message_encoding(commit)) };
        let raw_header = unsafe { CStr::from_ptr(ffi::git_commit_raw_header(commit)) }
            .to_bytes()
            .to_vec();
        let author = unsafe { raw_signature(ffi::git_commit_author(commit)) };
        let committer = unsafe { raw_signature(ffi::git_commit_committer(commit)) };

        let mut mapped = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_author_with_mailmap(&mut mapped, commit, core::ptr::null()) },
            0
        );
        let mapped_author = (
            unsafe { CStr::from_ptr((*mapped).name) }
                .to_bytes()
                .to_vec(),
            unsafe { CStr::from_ptr((*mapped).email) }
                .to_bytes()
                .to_vec(),
        );
        unsafe { ffi::git_signature_free(mapped) };

        let parent_count = unsafe { ffi::git_commit_parentcount(commit) };
        let parent_id = unsafe { ffi::git_commit_parent_id(commit, 0) };
        assert!(!parent_id.is_null());
        let parent_id = unsafe { (*parent_id).id.to_vec() };
        let mut ancestor = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_nth_gen_ancestor(&mut ancestor, commit, 2) },
            0
        );
        let ancestor_id = unsafe { (*ffi::git_commit_id(ancestor)).id.to_vec() };
        unsafe { ffi::git_commit_free(ancestor) };

        let tree_id = unsafe { (*ffi::git_commit_tree_id(commit)).id.to_vec() };
        let mut tree = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_commit_tree(&mut tree, commit) }, 0);
        let mut tree_entries = Vec::new();
        for index in 0..unsafe { ffi::git_tree_entrycount(tree) } {
            let entry = unsafe { ffi::git_tree_entry_byindex(tree, index) };
            tree_entries.push((
                unsafe { CStr::from_ptr(ffi::git_tree_entry_name(entry)) }
                    .to_bytes()
                    .to_vec(),
                unsafe { ffi::git_tree_entry_filemode(entry) },
                unsafe { ffi::git_tree_entry_type(entry) },
            ));
        }
        unsafe { ffi::git_tree_free(tree) };

        let mut tree_header = RawBuf::new();
        assert_eq!(
            unsafe { ffi::git_commit_header_field(&mut tree_header.0, commit, c"tree".as_ptr()) },
            0
        );
        let tree_header = tree_header.bytes();
        let mut short_id = RawBuf::new();
        assert_eq!(
            unsafe { ffi::git_object_short_id(&mut short_id.0, commit.cast()) },
            0
        );
        let short_id = short_id.bytes();

        unsafe { ffi::git_commit_free(commit) };

        CommitObservation {
            message,
            message_raw,
            summary,
            body,
            encoding,
            raw_header,
            tree_header,
            author,
            committer,
            mapped_author,
            parent_count,
            parent_id,
            ancestor_id,
            tree_id,
            tree_entries,
            short_id,
        }
    }

    fn safe_observation(fixture: &HistoryFixture) -> CommitObservation {
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        let mut head_id = crate::refs::git_reference_name_to_id(&mut repository, c"HEAD").unwrap();
        let head_id =
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(head_id).cast()) }
                .unwrap();
        let mut commit =
            crate::object_api::git_commit_lookup(repository.as_ref(), head_id).unwrap();

        let message = git_commit_message(commit.as_ref())
            .map_or_else(Vec::new, |value| value.to_bytes().to_vec());
        let message_raw = git_commit_message_raw(commit.as_ref())
            .map_or_else(Vec::new, |value| value.to_bytes().to_vec());
        let summary = git_commit_summary(commit.as_mut())
            .map_or_else(Vec::new, |value| value.to_bytes().to_vec());
        let body = git_commit_body(commit.as_mut())
            .map_or_else(Vec::new, |value| value.to_bytes().to_vec());
        let encoding = git_commit_message_encoding(commit.as_ref())
            .map_or_else(Vec::new, |value| value.to_bytes().to_vec());
        let raw_header = git_commit_raw_header(commit.as_ref()).to_bytes().to_vec();
        let author = safe_signature(git_commit_author(commit.as_ref()));
        let committer = safe_signature(git_commit_committer(commit.as_ref()));

        let mapped = git_commit_author_with_mailmap(commit.as_ref(), None).unwrap();
        let mapped_author = (
            mapped.as_ref().name().to_bytes().to_vec(),
            mapped.as_ref().email().to_bytes().to_vec(),
        );
        drop(mapped);

        let parent_count = git_commit_parentcount(commit.as_ref());
        let parent_id: Vec<u8> = git_commit_parent_id(commit.as_ref(), 0)
            .unwrap()
            .raw_bytes()
            .elems()
            .collect();
        let ancestor = git_commit_nth_gen_ancestor(commit.as_ref(), 2).unwrap();
        let ancestor_id = crate::object_api::git_commit_id(ancestor.as_ref())
            .raw_bytes()
            .elems()
            .collect();

        let tree_id = git_commit_tree_id(commit.as_ref())
            .raw_bytes()
            .elems()
            .collect();
        let tree = git_commit_tree(commit.as_ref()).unwrap();
        let mut tree_entries = Vec::new();
        for index in 0..crate::tree::git_tree_entrycount(tree.as_ref()) {
            let entry = crate::tree::git_tree_entry_byindex(tree.as_ref(), index).unwrap();
            tree_entries.push((
                crate::tree::git_tree_entry_name(entry).to_bytes().to_vec(),
                crate::tree::git_tree_entry_filemode(entry).as_raw(),
                crate::tree::git_tree_entry_type(entry).as_raw(),
            ));
        }

        let mut tree_header = crate::api::buffer::GitBuf::new();
        git_commit_header_field(&mut tree_header.as_mut(), commit.as_ref(), c"tree").unwrap();
        let tree_header = safe_buf_bytes(tree_header.as_ref());
        let object = unsafe {
            crate::object::GitObjectRef::from_ptr(commit.as_ref().as_ptr().cast_mut().cast())
        }
        .unwrap();
        let short_id = crate::object::git_object_short_id(object).unwrap();
        let short_id = safe_buf_bytes(short_id.as_ref());

        CommitObservation {
            message,
            message_raw,
            summary,
            body,
            encoding,
            raw_header,
            tree_header,
            author,
            committer,
            mapped_author,
            parent_count,
            parent_id,
            ancestor_id,
            tree_id,
            tree_entries,
            short_id,
        }
    }

    #[test]
    fn io_equiv_commit_metadata_parents_tree_and_object_views() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("commit-surface-raw");
        let safe = HistoryFixture::new("commit-surface-safe");

        let raw_observation = unsafe { raw_observation(&raw) };
        let safe_observation = safe_observation(&safe);
        assert_eq!(raw_observation, safe_observation);
        assert_eq!(raw_observation.parent_count, 1);
        assert_eq!(raw_observation.tree_entries.len(), 2);
    }

    #[derive(Debug, Eq, PartialEq)]
    struct BufferObservation {
        content: Vec<u8>,
        created_id: Vec<u8>,
        extract_status: i32,
    }

    unsafe fn raw_buffer(repository: *mut ffi::git_repository) -> BufferObservation {
        let mut commit = core::ptr::null_mut();
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut id, repository, c"HEAD".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut commit, repository, &id) },
            0
        );
        let mut tree = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_commit_tree(&mut tree, commit) }, 0);
        let mut content = RawBuf::new();
        let mut parents = [commit.cast_const()];
        assert_eq!(
            unsafe {
                ffi::git_commit_create_buffer(
                    &mut content.0,
                    repository,
                    ffi::git_commit_author(commit),
                    ffi::git_commit_committer(commit),
                    core::ptr::null(),
                    c"buffered equivalence commit".as_ptr(),
                    tree,
                    parents.len(),
                    parents.as_mut_ptr(),
                )
            },
            0
        );
        let content_bytes = content.bytes();
        let content_string = unsafe { CStr::from_ptr(content.0.ptr) };
        let mut created = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_commit_create_with_signature(
                    &mut created,
                    repository,
                    content_string.as_ptr(),
                    core::ptr::null(),
                    core::ptr::null(),
                )
            },
            0
        );
        let mut signature = RawBuf::new();
        let mut signed = RawBuf::new();
        let extract_status = unsafe {
            ffi::git_commit_extract_signature(
                &mut signature.0,
                &mut signed.0,
                repository,
                &mut created,
                core::ptr::null(),
            )
        };
        unsafe {
            ffi::git_tree_free(tree);
            ffi::git_commit_free(commit);
        }
        BufferObservation {
            content: content_bytes,
            created_id: created.id.to_vec(),
            extract_status,
        }
    }

    fn safe_buffer(repository: *mut ffi::git_repository) -> BufferObservation {
        let mut commit = core::ptr::null_mut();
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut id, repository, c"HEAD".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut commit, repository, &id) },
            0
        );
        let mut tree = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_commit_tree(&mut tree, commit) }, 0);
        let commit_ref = unsafe { GitCommitRef::from_ptr(commit) }.unwrap();
        let tree_ref = unsafe { crate::tree::GitTreeRef::from_ptr(tree) }.unwrap();
        let author = git_commit_author(commit_ref);
        let committer = git_commit_committer(commit_ref);
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut content = crate::api::buffer::GitBuf::new();
        git_commit_create_buffer(
            &mut content.as_mut(),
            &mut repository_view,
            author,
            committer,
            None,
            c"buffered equivalence commit",
            tree_ref,
            &[commit_ref],
        )
        .unwrap();
        let content_bytes = safe_buf_bytes(content.as_ref());
        let content_string = unsafe { CStr::from_ptr((*content.as_ref().as_ptr()).ptr) };
        let mut created = crate::oid::Oid::zeroed();
        let mut created_mut = unsafe {
            crate::oid::OidMut::from_ptr(core::ptr::from_mut(&mut created).cast::<ffi::git_oid>())
        }
        .unwrap();
        git_commit_create_with_signature(
            &mut created_mut,
            &mut repository_view,
            content_string,
            None,
            None,
        )
        .unwrap();
        let created_ref = unsafe {
            crate::oid::OidRef::from_ptr(core::ptr::from_mut(&mut created).cast::<ffi::git_oid>())
        }
        .unwrap();
        let mut signature = crate::api::buffer::GitBuf::new();
        let mut signed = crate::api::buffer::GitBuf::new();
        let extract_status = match git_commit_extract_signature(
            &mut signature.as_mut(),
            &mut signed.as_mut(),
            &mut repository_view,
            created_ref,
            None,
        ) {
            Ok(()) => 0,
            Err(error) => error,
        };
        let created_id = created_ref.raw_bytes().elems().collect();
        unsafe {
            ffi::git_tree_free(tree);
            ffi::git_commit_free(commit);
        }
        BufferObservation {
            content: content_bytes,
            created_id,
            extract_status,
        }
    }

    #[test]
    fn io_equiv_commit_buffer_creation_and_unsigned_signature_lookup() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("commit-buffer-raw");
        let safe = HistoryFixture::new("commit-buffer-safe");
        let raw_observation = unsafe { raw_buffer(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_buffer(safe.repository.as_ptr()));
        assert_eq!(
            raw_observation.extract_status,
            ffi::git_error_code_GIT_ENOTFOUND
        );
    }

    fn prepare_stage(fixture: &HistoryFixture, contents: &[u8]) {
        std::fs::write(fixture.directory.path().join("signed-stage.txt"), contents).unwrap();
        let status = std::process::Command::new("git")
            .current_dir(fixture.directory.path())
            .args(["add", "signed-stage.txt"])
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[derive(Debug, Eq, PartialEq)]
    struct StageCommitObservation {
        created: Vec<u8>,
        amended: Vec<u8>,
        signature_header: Vec<u8>,
        message: Vec<u8>,
        ancestor: Vec<u8>,
    }

    unsafe extern "C" fn raw_sign(
        builder: *mut ffi::git_commitbuilder,
        _: *mut ffi::git_repository,
        _: *const core::ffi::c_char,
        _: *mut core::ffi::c_void,
    ) -> i32 {
        unsafe {
            ffi::git_commitbuilder_add_header(
                builder,
                c"x-crustify-signature".as_ptr(),
                c"deterministic-signature".as_ptr(),
            )
        }
    }

    unsafe fn refresh_index(repository: *mut ffi::git_repository) {
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, repository) },
            0
        );
        assert_eq!(unsafe { ffi::git_index_read(index, 1) }, 0);
        unsafe { ffi::git_index_free(index) };
    }

    unsafe fn raw_stage_commit(fixture: &HistoryFixture) -> StageCommitObservation {
        let repository = fixture.repository.as_ptr();
        unsafe { refresh_index(repository) };
        let mut signature = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_signature_new(
                    &mut signature,
                    c"Stage Author".as_ptr(),
                    c"stage@example.com".as_ptr(),
                    1_700_400_000,
                    0,
                )
            },
            0
        );
        let mut options = unsafe { core::mem::zeroed::<ffi::git_commit_create_options>() };
        assert_eq!(
            unsafe {
                ffi::git_commit_create_options_init(
                    &mut options,
                    ffi::GIT_COMMIT_CREATE_OPTIONS_VERSION,
                )
            },
            0
        );
        options.author = signature;
        options.committer = signature;
        options.sign = Some(raw_sign);
        let mut created = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_commit_create_from_stage(
                    &mut created,
                    repository,
                    c"created from staged state\n\nbody\n".as_ptr(),
                    &options,
                )
            },
            0
        );
        prepare_stage(fixture, b"amended staged contents\n");
        unsafe { refresh_index(repository) };
        let mut amended = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_commit_amend_from_stage(
                    &mut amended,
                    repository,
                    c"amended from staged state\n\nnew body\n".as_ptr(),
                    &options,
                )
            },
            0
        );
        let mut commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut commit, repository, &amended) },
            0
        );
        let mut header = crate::io_equiv_support::RawBuf::new();
        assert_eq!(
            unsafe {
                ffi::git_commit_header_field(
                    &mut header.0,
                    commit,
                    c"x-crustify-signature".as_ptr(),
                )
            },
            0
        );
        let message = unsafe { CStr::from_ptr(ffi::git_commit_message(commit)) }
            .to_bytes()
            .to_vec();
        let mut ancestor = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_nth_gen_ancestor(&mut ancestor, commit, 2) },
            0
        );
        let ancestor_id = unsafe { (*ffi::git_commit_id(ancestor)).id }.to_vec();
        unsafe {
            ffi::git_commit_free(ancestor);
            ffi::git_commit_free(commit);
            ffi::git_signature_free(signature);
        }
        StageCommitObservation {
            created: created.id.to_vec(),
            amended: amended.id.to_vec(),
            signature_header: header.bytes(),
            message,
            ancestor: ancestor_id,
        }
    }

    fn safe_stage_commit(fixture: &HistoryFixture) -> StageCommitObservation {
        unsafe { refresh_index(fixture.repository.as_ptr()) };
        let signature = crate::signature::git_signature_new(
            c"Stage Author",
            c"stage@example.com",
            1_700_400_000,
            0,
        )
        .unwrap();
        let mut options =
            git_commit_create_options_init(ffi::GIT_COMMIT_CREATE_OPTIONS_VERSION).unwrap();
        unsafe {
            options
                .as_mut()
                .set_borrowed_author(Some(signature.as_ref()));
            options
                .as_mut()
                .set_borrowed_committer(Some(signature.as_ref()));
        }
        let mut signer = |mut builder: GitCommitbuilderMut<'_>,
                          _: crate::repository::GitRepositoryMut<'_>,
                          _: &CStr| {
            git_commitbuilder_add_header(
                &mut builder,
                c"x-crustify-signature",
                c"deterministic-signature",
            )
            .map_or_else(|error| error, |_| 0)
        };
        unsafe {
            options.as_mut().set_signing_callback(&mut signer);
        }
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(fixture.repository.as_ptr()) }
                .unwrap();
        let mut created = crate::oid::Oid::zeroed();
        let mut created_view =
            unsafe { crate::oid::OidMut::from_ptr(core::ptr::addr_of_mut!(created).cast()) }
                .unwrap();
        git_commit_create_from_stage(
            &mut created_view,
            &mut repository,
            c"created from staged state\n\nbody\n",
            Some(options.as_ref()),
        )
        .unwrap();
        prepare_stage(fixture, b"amended staged contents\n");
        unsafe { refresh_index(fixture.repository.as_ptr()) };
        let mut amended = crate::oid::Oid::zeroed();
        let mut amended_view =
            unsafe { crate::oid::OidMut::from_ptr(core::ptr::addr_of_mut!(amended).cast()) }
                .unwrap();
        git_commit_amend_from_stage(
            &mut amended_view,
            &mut repository,
            Some(c"amended from staged state\n\nnew body\n"),
            Some(options.as_ref()),
        )
        .unwrap();
        let amended_ref =
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(amended).cast()) }
                .unwrap();
        let commit =
            crate::object_api::git_commit_lookup(repository.as_ref(), amended_ref).unwrap();
        let mut header = crate::api::buffer::GitBuf::new();
        git_commit_header_field(
            &mut header.as_mut(),
            commit.as_ref(),
            c"x-crustify-signature",
        )
        .unwrap();
        let message = git_commit_message(commit.as_ref())
            .unwrap()
            .to_bytes()
            .to_vec();
        let ancestor = git_commit_nth_gen_ancestor(commit.as_ref(), 2).unwrap();
        let bytes = |value: crate::oid::OidRef<'_>| value.raw_bytes().elems().collect();
        StageCommitObservation {
            created: bytes(
                unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(created).cast()) }
                    .unwrap(),
            ),
            amended: bytes(amended_ref),
            signature_header: safe_buf_bytes(header.as_ref()),
            message,
            ancestor: bytes(crate::object_api::git_commit_id(ancestor.as_ref())),
        }
    }

    #[test]
    fn io_equiv_commit_create_and_amend_from_stage_with_signing_callback() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("commit-stage-raw");
        let safe = HistoryFixture::new("commit-stage-safe");
        prepare_stage(&raw, b"created staged contents\n");
        prepare_stage(&safe, b"created staged contents\n");
        let raw_observation = unsafe { raw_stage_commit(&raw) };
        assert_eq!(raw_observation, safe_stage_commit(&safe));
        assert_eq!(raw_observation.signature_header, b"deterministic-signature");
        assert!(
            raw_observation
                .message
                .starts_with(b"amended from staged state")
        );
    }

    unsafe fn raw_legacy_create_and_amend(
        repository: *mut ffi::git_repository,
    ) -> (Vec<u8>, Vec<u8>, i32, Vec<u8>) {
        let mut parent_object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut parent_object, repository, c"HEAD".as_ptr()) },
            0
        );
        let parent = parent_object.cast::<ffi::git_commit>();
        let mut tree = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_commit_tree(&mut tree, parent) }, 0);
        let mut signature = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_signature_new(
                    &mut signature,
                    c"Legacy Author".as_ptr(),
                    c"legacy@example.com".as_ptr(),
                    1_700_500_000,
                    90,
                )
            },
            0
        );
        let mut parents = [parent.cast_const()];
        let mut created = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_commit_create(
                    &mut created,
                    repository,
                    core::ptr::null(),
                    signature,
                    signature,
                    c"UTF-8".as_ptr(),
                    c"legacy create\n\nbody\n".as_ptr(),
                    tree,
                    parents.len(),
                    parents.as_mut_ptr(),
                )
            },
            0
        );
        let mut commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut commit, repository, &created) },
            0
        );
        let mut amended = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_commit_amend(
                    &mut amended,
                    commit,
                    core::ptr::null(),
                    core::ptr::null(),
                    signature,
                    core::ptr::null(),
                    c"legacy amend\n\nnew body\n".as_ptr(),
                    core::ptr::null(),
                )
            },
            0
        );
        let mut amended_commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut amended_commit, repository, &amended) },
            0
        );
        let offset = unsafe { ffi::git_commit_time_offset(amended_commit) };
        let message = unsafe { CStr::from_ptr(ffi::git_commit_message(amended_commit)) }
            .to_bytes()
            .to_vec();
        unsafe {
            ffi::git_commit_free(amended_commit);
            ffi::git_commit_free(commit);
            ffi::git_signature_free(signature);
            ffi::git_tree_free(tree);
            ffi::git_object_free(parent_object);
        }
        (created.id.to_vec(), amended.id.to_vec(), offset, message)
    }

    fn safe_legacy_create_and_amend(
        repository: *mut ffi::git_repository,
    ) -> (Vec<u8>, Vec<u8>, i32, Vec<u8>) {
        let mut parent_object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut parent_object, repository, c"HEAD".as_ptr()) },
            0
        );
        let parent = unsafe { GitCommitRef::from_ptr(parent_object.cast()) }.unwrap();
        let mut tree = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_tree(&mut tree, parent.as_ptr()) },
            0
        );
        let tree_view = unsafe { crate::tree::GitTreeRef::from_ptr(tree) }.unwrap();
        let signature = crate::signature::git_signature_new(
            c"Legacy Author",
            c"legacy@example.com",
            1_700_500_000,
            90,
        )
        .unwrap();
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut created = crate::oid::Oid::zeroed();
        let mut created_view =
            unsafe { crate::oid::OidMut::from_ptr(core::ptr::addr_of_mut!(created).cast()) }
                .unwrap();
        git_commit_create(
            &mut created_view,
            &mut repository_view,
            None,
            signature.as_ref(),
            signature.as_ref(),
            Some(c"UTF-8"),
            c"legacy create\n\nbody\n",
            tree_view,
            &[parent],
        )
        .unwrap();
        let created_ref =
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(created).cast()) }
                .unwrap();
        let commit =
            crate::object_api::git_commit_lookup(repository_view.as_ref(), created_ref).unwrap();
        let mut amended = crate::oid::Oid::zeroed();
        let mut amended_view =
            unsafe { crate::oid::OidMut::from_ptr(core::ptr::addr_of_mut!(amended).cast()) }
                .unwrap();
        git_commit_amend(
            &mut amended_view,
            commit.as_ref(),
            None,
            None,
            Some(signature.as_ref()),
            None,
            Some(c"legacy amend\n\nnew body\n"),
            None,
        )
        .unwrap();
        let amended_ref =
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(amended).cast()) }
                .unwrap();
        let amended_commit =
            crate::object_api::git_commit_lookup(repository_view.as_ref(), amended_ref).unwrap();
        let observation = (
            created_ref.raw_bytes().elems().collect(),
            amended_ref.raw_bytes().elems().collect(),
            git_commit_time_offset(amended_commit.as_ref()),
            git_commit_message(amended_commit.as_ref())
                .unwrap()
                .to_bytes()
                .to_vec(),
        );
        drop(amended_commit);
        drop(commit);
        unsafe {
            ffi::git_tree_free(tree);
            ffi::git_object_free(parent_object);
        }
        observation
    }

    #[test]
    fn io_equiv_legacy_commit_create_and_amend() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("commit-legacy-raw");
        let safe = HistoryFixture::new("commit-legacy-safe");
        let raw = unsafe { raw_legacy_create_and_amend(raw.repository.as_ptr()) };
        assert_eq!(raw, safe_legacy_create_and_amend(safe.repository.as_ptr()));
        assert_eq!(raw.2, 90);
        assert!(raw.3.starts_with(b"legacy amend"));
    }
}
