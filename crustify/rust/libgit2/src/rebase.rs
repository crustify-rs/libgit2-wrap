//! Safe wrappers for libgit2 rebase APIs.

use core::ptr::{NonNull, addr_of};

use ffibox::{CBox, CDropped};

use crate::annotated_commit::AnnotatedCommitRef;
use crate::api::rebase::{GitRebaseOptionsMut, GitRebaseOptionsRef};
use crate::api::types::GitSignatureRef;
use crate::ffi;
use crate::index::{GitIndex, GitIndexOwned};
use crate::oid::{Oid, OidRef};

/// Wraps: git_rebase_operation_t
/// An instruction in a rebase sequence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum RebaseOperationType {
    /// Cherry-pick the commit; the client commits and continues when the
    /// cherry-pick produced no conflicts.
    Pick = ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_PICK,
    /// Cherry-pick the commit after allowing its message to be rewritten.
    Reword = ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_REWORD,
    /// Cherry-pick the commit, then pause so its changes can be edited.
    Edit = ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_EDIT,
    /// Combine the commit and its message with the preceding commit.
    Squash = ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_SQUASH,
    /// Combine the commit while discarding its commit message.
    Fixup = ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_FIXUP,
    /// Run a command instead of cherry-picking a commit.
    Exec = ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_EXEC,
}

/// A raw value that is not a published [`RebaseOperationType`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidRebaseOperationType(ffi::git_rebase_operation_t);

impl InvalidRebaseOperationType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_rebase_operation_t {
        self.0
    }
}

impl From<RebaseOperationType> for ffi::git_rebase_operation_t {
    fn from(operation: RebaseOperationType) -> Self {
        operation as Self
    }
}

impl TryFrom<ffi::git_rebase_operation_t> for RebaseOperationType {
    type Error = InvalidRebaseOperationType;

    fn try_from(operation: ffi::git_rebase_operation_t) -> Result<Self, Self::Error> {
        match operation {
            ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_PICK => Ok(Self::Pick),
            ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_REWORD => Ok(Self::Reword),
            ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_EDIT => Ok(Self::Edit),
            ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_SQUASH => Ok(Self::Squash),
            ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_FIXUP => Ok(Self::Fixup),
            ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_EXEC => Ok(Self::Exec),
            value => Err(InvalidRebaseOperationType(value)),
        }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_rebase
    /// An opaque in-progress rebase managed by libgit2.
    ///
    /// Owned rebases use [`GitRebaseOwned`] and are released by
    /// `git_rebase_free`. Libgit2 gives a rebase neither a reference count nor
    /// a duplication entry point, so an owner is unique and not cloneable. A
    /// rebase retains a borrowed repository pointer, so safe constructors must
    /// keep that repository alive for the full lifetime of the returned owner.
    GitRebase,
    GitRebaseRef,
    GitRebaseMut,
    ffi::git_rebase
);

/// An exclusively owned rebase tied to the repository and callback data it retains.
pub struct GitRebaseOwned<'repo, 'data> {
    inner: CBox<GitRebase>,
    _repository: core::marker::PhantomData<crate::repository::GitRepositoryMut<'repo>>,
    _data: core::marker::PhantomData<&'data mut ()>,
}

impl GitRebaseOwned<'_, '_> {
    /// Borrows the rebase shared.
    #[must_use]
    pub fn as_ref(&self) -> GitRebaseRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the rebase exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitRebaseMut<'_> {
        self.inner.as_mut()
    }
}

fn rebase_result<'repo, 'data>(
    status: i32,
    inner: Option<CBox<GitRebase>>,
) -> Result<GitRebaseOwned<'repo, 'data>, i32> {
    if status == 0 {
        let inner = inner.ok_or(ffi::git_error_code_GIT_ERROR)?;
        Ok(GitRebaseOwned {
            inner,
            _repository: core::marker::PhantomData,
            _data: core::marker::PhantomData,
        })
    } else {
        drop(inner);
        Err(status)
    }
}

/// Wraps: git_rebase_free
// SAFETY: `git_rebase_free` is the public destructor for a fully constructed
// `git_rebase`. It releases every owned field and the allocation, and accepts
// null although `CBox` supplies one live non-null allocation exactly once.
unsafe impl CDropped for GitRebase {
    unsafe fn c_drop(object: NonNull<Self>) {
        // SAFETY: the trait contract supplies one complete live rebase and the
        // wrapper is transparent over `ffi::git_rebase`.
        unsafe { ffi::git_rebase_free(object.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn operation_types_round_trip_through_the_c_type() {
        for operation in [
            RebaseOperationType::Pick,
            RebaseOperationType::Reword,
            RebaseOperationType::Edit,
            RebaseOperationType::Squash,
            RebaseOperationType::Fixup,
            RebaseOperationType::Exec,
        ] {
            let raw = ffi::git_rebase_operation_t::from(operation);
            assert_eq!(RebaseOperationType::try_from(raw), Ok(operation));
        }
    }

    #[test]
    fn unknown_operation_type_is_rejected() {
        let invalid = ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_EXEC + 1;
        let error = RebaseOperationType::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn operation_type_matches_the_c_abi_scalar() {
        assert_eq!(
            size_of::<RebaseOperationType>(),
            size_of::<ffi::git_rebase_operation_t>()
        );
        assert_eq!(
            align_of::<RebaseOperationType>(),
            align_of::<ffi::git_rebase_operation_t>()
        );
    }

    #[test]
    fn opaque_rebase_preserves_the_c_layout_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitRebase>();
        assert_dropped::<GitRebase>();
        // `struct git_rebase` is defined only in `src/libgit2/rebase.c`, so
        // the binding is an opaque marker: no field is reachable from Rust and
        // every operation crosses the FFI seam.
        assert_eq!(size_of::<ffi::git_rebase>(), 0);
        assert_eq!(size_of::<GitRebase>(), size_of::<ffi::git_rebase>());
        assert_eq!(align_of::<GitRebase>(), align_of::<ffi::git_rebase>());
        assert_eq!(
            size_of::<GitRebaseRef<'_>>(),
            size_of::<*const ffi::git_rebase>()
        );
        assert_eq!(
            size_of::<GitRebaseMut<'_>>(),
            size_of::<*mut ffi::git_rebase>()
        );
        assert_eq!(
            size_of::<GitRebaseOwned<'static, 'static>>(),
            size_of::<*mut ffi::git_rebase>()
        );
    }

    #[test]
    fn borrowed_rebase_handles_preserve_the_ffi_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_rebase>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_rebase>();

        {
            // SAFETY: `raw` addresses live, suitably aligned storage for the
            // opaque FFI type and remains live for this shared handle.
            let shared = unsafe { GitRebaseRef::from_ptr(raw) }
                .expect("the Box-derived pointer is non-null");
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitRebaseMut::from_ptr(raw) }
                .expect("the Box-derived pointer is non-null");
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from `Box::into_raw`, no handle remains, and the
        // cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_rebase>>()) });
    }

    #[test]
    fn read_only_accessors_are_reachable_from_a_shared_handle() {
        // Each of these C bodies is a bare field read behind a non-const
        // declaration, so the safe surface must not demand exclusive access.
        // Naming the function types is what pins that down: a signature taking
        // `&mut GitRebaseMut` would not coerce here, and the returned borrows
        // must stay tied to the shared handle they came from.
        let _: fn(GitRebaseRef<'_>) -> Option<usize> = git_rebase_operation_current;
        let _: fn(GitRebaseRef<'_>) -> usize = git_rebase_operation_entrycount;
        let _: fn(GitRebaseRef<'_>) -> OidRef<'_> = git_rebase_orig_head_id;
        let _: fn(GitRebaseRef<'_>) -> Option<&core::ffi::CStr> = git_rebase_orig_head_name;
    }

    #[test]
    fn null_rebase_seams_create_no_handle() {
        // SAFETY: each conversion accepts null and returns `None` without
        // borrowing or adopting an object.
        unsafe {
            assert!(GitRebaseRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRebaseMut::from_ptr(ptr::null_mut()).is_none());
            assert!(CBox::<GitRebase>::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn rebase_operation_layout_and_fields_match_the_c_value() {
        fn assert_cell<T: CCell>() {}

        let mut raw = ffi::git_rebase_operation {
            type_: ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_EXEC,
            id: ffi::git_oid {
                type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
                id: [0x5a; 32],
            },
            exec: c"make test".as_ptr(),
        };

        assert_cell::<GitRebaseOperation>();
        assert_eq!(
            size_of::<GitRebaseOperation>(),
            size_of::<ffi::git_rebase_operation>()
        );
        assert_eq!(
            align_of::<GitRebaseOperation>(),
            align_of::<ffi::git_rebase_operation>()
        );

        // SAFETY: `raw` is fully initialized and remains live and unchanged
        // for the shared handle's lifetime.
        let operation = unsafe { GitRebaseOperationRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(operation.operation_type(), Ok(RebaseOperationType::Exec));
        assert_eq!(operation.exec(), Ok(Some(c"make test")));
        assert!(matches!(operation.id(), Ok(None)));

        raw.type_ = ffi::git_rebase_operation_t_GIT_REBASE_OPERATION_PICK;
        raw.exec = core::ptr::null();
        // SAFETY: `raw` remains fully initialized and live; no prior handle is
        // used after this new shared borrow is created.
        let operation = unsafe { GitRebaseOperationRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(operation.exec(), Ok(None));
        assert_eq!(
            operation.id().unwrap().unwrap().raw_bytes().elem(0),
            Some(0x5a)
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_rebase_operation
    /// A rebase instruction borrowed from its enclosing rebase.
    ///
    /// Libgit2 stores operations inline and invalidates their addresses when
    /// the rebase is freed. The optional command string follows the same
    /// lifetime.
    GitRebaseOperation,
    GitRebaseOperationRef,
    GitRebaseOperationMut,
    ffi::git_rebase_operation
);

impl<'a> GitRebaseOperationRef<'a> {
    /// Field: git_rebase_operation.type
    /// Returns the instruction kind after validating the C discriminant.
    pub fn operation_type(&self) -> Result<RebaseOperationType, InvalidRebaseOperationType> {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to C-visible operation storage.
        let raw = unsafe { addr_of!((*self.as_ptr()).type_).read() };
        RebaseOperationType::try_from(raw)
    }

    /// Field: git_rebase_operation.id
    /// Borrows the commit ID, or returns `None` for an `Exec` instruction.
    pub fn id(&self) -> Result<Option<OidRef<'a>>, InvalidRebaseOperationType> {
        if self.operation_type()? == RebaseOperationType::Exec {
            return Ok(None);
        }

        // SAFETY: raw-place projection reaches the inline initialized OID
        // of a non-`Exec` operation without forming a reference, and the field
        // lives for this operation handle's full borrow.
        let ptr = unsafe { addr_of!((*self.as_ptr()).id) }.cast_mut();
        // SAFETY: `ptr` addresses the live inline OID and inherits `'a` from
        // the enclosing operation handle.
        Ok(Some(
            unsafe { OidRef::from_ptr(ptr) }.expect("an inline field is non-null"),
        ))
    }

    /// Field: git_rebase_operation.exec
    /// Borrows the command for an `Exec` instruction, if present.
    pub fn exec(&self) -> Result<Option<&'a core::ffi::CStr>, InvalidRebaseOperationType> {
        if self.operation_type()? != RebaseOperationType::Exec {
            return Ok(None);
        }

        // SAFETY: this shared handle permits a raw-place read of the pointer
        // populated for an `Exec` operation without forming a reference to
        // C-visible operation storage.
        let ptr = unsafe { addr_of!((*self.as_ptr()).exec).read() };
        if ptr.is_null() {
            Ok(None)
        } else {
            // SAFETY: a non-null libgit2 command points to a live
            // NUL-terminated string for the operation's lifetime.
            Ok(Some(unsafe { core::ffi::CStr::from_ptr(ptr) }))
        }
    }
}

/// Wraps: git_rebase_abort
/// Aborts the in-progress rebase and restores its original checkout state.
pub fn git_rebase_abort(rebase: &mut GitRebaseMut<'_>) -> Result<(), i32> {
    // SAFETY: the exclusive handle permits every mutation performed while
    // aborting, and libgit2 retains no new pointer.
    let status = unsafe { ffi::git_rebase_abort(rebase.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_rebase_operation_current
/// Returns the current operation index, or `None` before the first operation.
///
/// The C declaration takes a non-const rebase, but the body only reads
/// `started` and `current`, so a shared borrow states the real contract; that
/// is also what lets the index be read while other borrowed views are live.
#[must_use]
pub fn git_rebase_operation_current(rebase: GitRebaseRef<'_>) -> Option<usize> {
    // SAFETY: `rebase` is a live shared handle and the call performs no write;
    // restoring mutability at the seam only satisfies the C declaration.
    let current = unsafe { ffi::git_rebase_operation_current(rebase.as_ptr().cast_mut()) };
    (current != usize::MAX).then_some(current)
}

/// Wraps: git_rebase_operation_entrycount
/// Returns the number of operations in the rebase plan.
///
/// Read-only for the same reason as [`git_rebase_operation_current`]: the body
/// returns `git_array_size(rebase->operations)` and writes nothing.
#[must_use]
pub fn git_rebase_operation_entrycount(rebase: GitRebaseRef<'_>) -> usize {
    // SAFETY: `rebase` is a live shared handle and the call performs no write;
    // restoring mutability at the seam only satisfies the C declaration.
    unsafe { ffi::git_rebase_operation_entrycount(rebase.as_ptr().cast_mut()) }
}

/// Wraps: git_rebase_orig_head_id
/// Borrows the original HEAD object ID from a merge rebase.
///
/// The body returns `&rebase->orig_head_id`, the address of an inline field,
/// so the result is never null and never outlives the rebase borrow it came
/// from. The non-const C declaration performs no write.
#[must_use]
pub fn git_rebase_orig_head_id<'a>(rebase: GitRebaseRef<'a>) -> OidRef<'a> {
    // SAFETY: `rebase` is a live shared handle and the call performs no write;
    // restoring mutability at the seam only satisfies the C declaration.
    let id = unsafe { ffi::git_rebase_orig_head_id(rebase.as_ptr().cast_mut()) };
    // SAFETY: libgit2 returns the non-null address of the initialized inline
    // `orig_head_id`; its lifetime is bounded by the shared rebase borrow.
    unsafe { OidRef::from_ptr(id.cast_mut()) }.expect("an inline OID is non-null")
}

/// Wraps: git_rebase_orig_head_name
/// Borrows the optional original HEAD name from a merge rebase.
///
/// `orig_head_name` is a rebase-owned string that is null for a rebase started
/// from a detached HEAD. The non-const C declaration performs no write.
#[must_use]
pub fn git_rebase_orig_head_name<'a>(rebase: GitRebaseRef<'a>) -> Option<&'a core::ffi::CStr> {
    // SAFETY: `rebase` is a live shared handle and the call performs no write;
    // restoring mutability at the seam only satisfies the C declaration.
    let name = unsafe { ffi::git_rebase_orig_head_name(rebase.as_ptr().cast_mut()) };
    if name.is_null() {
        None
    } else {
        // SAFETY: a non-null libgit2 result is NUL-terminated and owned by the
        // rebase for the duration of this shared borrow.
        Some(unsafe { core::ffi::CStr::from_ptr(name) })
    }
}

/// Wraps: git_rebase_commit
/// Commits the current rebase operation and returns the new commit ID.
pub fn git_rebase_commit(
    rebase: &mut GitRebaseMut<'_>,
    author: Option<GitSignatureRef<'_>>,
    committer: GitSignatureRef<'_>,
    message_encoding: Option<&core::ffi::CStr>,
    message: Option<&core::ffi::CStr>,
) -> Result<Oid, i32> {
    let mut id = Oid::zeroed();
    // SAFETY: `id` is writable, the rebase is exclusively borrowed, and all
    // optional inputs are null or live borrowed values retained only for the
    // call. The committer is required and non-null by its typed handle.
    let status = unsafe {
        ffi::git_rebase_commit(
            core::ptr::addr_of_mut!(id).cast(),
            rebase.as_mut_ptr(),
            author.map_or(core::ptr::null(), |value| value.as_ptr()),
            committer.as_ptr(),
            message_encoding.map_or(core::ptr::null(), core::ffi::CStr::as_ptr),
            message.map_or(core::ptr::null(), core::ffi::CStr::as_ptr),
        )
    };
    if status == 0 { Ok(id) } else { Err(status) }
}

/// Wraps: git_rebase_finish
/// Finishes a rebase and optionally uses `signature` when copying notes.
pub fn git_rebase_finish(
    rebase: &mut GitRebaseMut<'_>,
    signature: Option<GitSignatureRef<'_>>,
) -> Result<(), i32> {
    // SAFETY: the rebase is exclusively borrowed and the optional signature
    // is null or live for this synchronous call; neither pointer is retained.
    let status = unsafe {
        ffi::git_rebase_finish(
            rebase.as_mut_ptr(),
            signature.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_rebase_inmemory_index
/// Acquires an independently owned count on an in-memory rebase index.
pub fn git_rebase_inmemory_index(rebase: GitRebaseRef<'_>) -> Result<GitIndexOwned, i32> {
    let mut index = core::ptr::null_mut();
    // SAFETY: the output slot is writable and the live shared rebase remains
    // valid while libgit2 reads its index and increments that index's count.
    let status = unsafe {
        ffi::git_rebase_inmemory_index(core::ptr::addr_of_mut!(index), rebase.as_ptr().cast_mut())
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete index reference count.
    unsafe { CBox::<GitIndex>::from_raw(index) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_rebase_next
/// Advances the rebase and borrows the current operation.
pub fn git_rebase_next<'a>(
    rebase: &'a mut GitRebaseMut<'_>,
) -> Result<GitRebaseOperationRef<'a>, i32> {
    let mut operation = core::ptr::null_mut();
    // SAFETY: the output slot is writable and `rebase` is exclusively
    // borrowed for the state transition. The returned operation is stored in
    // the rebase and is tied to this exclusive reborrow.
    let status =
        unsafe { ffi::git_rebase_next(core::ptr::addr_of_mut!(operation), rebase.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns a live operation owned by `rebase`.
    unsafe { GitRebaseOperationRef::from_ptr(operation) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_rebase_operation_byindex
/// Borrows an operation by index, returning `None` when out of range.
#[must_use]
pub fn git_rebase_operation_byindex<'a>(
    rebase: GitRebaseRef<'a>,
    index: usize,
) -> Option<GitRebaseOperationRef<'a>> {
    // SAFETY: the shared rebase stays live for `'a`; the body only reads its
    // operations array. A non-null result points into that array.
    let operation = unsafe { ffi::git_rebase_operation_byindex(rebase.as_ptr().cast_mut(), index) };
    // SAFETY: the result is null or a live rebase-owned operation for `'a`.
    unsafe { GitRebaseOperationRef::from_ptr(operation) }
}

/// Wraps: git_rebase_init
/// Starts a rebase and returns an owner tied to the repository and options data.
///
/// At least one of `upstream` and `onto` is required; C asserts that argument
/// pair, so the wrapper rejects the empty case before the call and reports it
/// as `GIT_EINVALID` rather than the generic `-1` the assertion would return.
///
/// The returned owner carries both retained borrows. `rebase->repo = repo` is
/// stored without a reference count, and `rebase_alloc` memcpy's the options
/// into the rebase: it duplicates `rewrite_notes_ref`, `default_driver`, the
/// checkout labels, the target directory and the baseline tree, but leaves the
/// checkout `paths` array, `baseline_index` and every callback payload as the
/// caller's pointers — which is what `'data` records.
#[allow(clippy::too_many_arguments)]
pub fn git_rebase_init<'repo, 'data>(
    mut repository: crate::repository::GitRepositoryMut<'repo>,
    branch: Option<AnnotatedCommitRef<'_>>,
    upstream: Option<AnnotatedCommitRef<'_>>,
    onto: Option<AnnotatedCommitRef<'_>>,
    options: Option<GitRebaseOptionsRef<'_, 'data>>,
) -> Result<GitRebaseOwned<'repo, 'data>, i32> {
    if upstream.is_none() && onto.is_none() {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable, every optional typed input is live for the
    // call, and the returned owner carries both retained lifetime bounds.
    let status = unsafe {
        ffi::git_rebase_init(
            &mut out,
            repository.as_mut_ptr(),
            branch.map_or(core::ptr::null(), |commit| commit.as_ptr()),
            upstream.map_or(core::ptr::null(), |commit| commit.as_ptr()),
            onto.map_or(core::ptr::null(), |commit| commit.as_ptr()),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    // SAFETY: the constructor leaves `out` null or transfers one complete
    // rebase allocation, including on a surprising error output. Adopting it
    // here keeps the raw ownership seam beside the call that produced it.
    let rebase = unsafe { CBox::<GitRebase>::from_raw(out) };
    rebase_result(status, rebase)
}

/// Wraps: git_rebase_init_options
/// Initializes deprecated rebase options for `version`.
pub fn git_rebase_init_options(
    options: &mut GitRebaseOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable options storage and C
    // retains no pointer to it after initialization.
    let status = unsafe { ffi::git_rebase_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_rebase_open
/// Opens an existing rebase and ties it to the repository and options data.
pub fn git_rebase_open<'repo, 'data>(
    mut repository: crate::repository::GitRepositoryMut<'repo>,
    options: Option<GitRebaseOptionsRef<'_, 'data>>,
) -> Result<GitRebaseOwned<'repo, 'data>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and the returned owner records the lifetimes
    // of the repository and copied option payloads retained by C.
    let status = unsafe {
        ffi::git_rebase_open(
            &mut out,
            repository.as_mut_ptr(),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    // SAFETY: the constructor leaves `out` null or transfers one complete
    // rebase allocation, including on a surprising error output. Adopting it
    // here keeps the raw ownership seam beside the call that produced it.
    let rebase = unsafe { CBox::<GitRebase>::from_raw(out) };
    rebase_result(status, rebase)
}

#[cfg(test)]
mod scheduled_constructor_tests {
    use super::*;
    use crate::api::rebase::GitRebaseOptions;

    #[test]
    fn deprecated_initializer_writes_the_current_version() {
        let mut options = GitRebaseOptions::new();
        git_rebase_init_options(&mut options.as_mut(), ffi::GIT_REBASE_OPTIONS_VERSION).unwrap();
        assert_eq!(options.as_ref().version(), ffi::GIT_REBASE_OPTIONS_VERSION);
    }
}

/// Wraps: git_rebase_onto_id
/// Borrows the object ID of the rebase's `onto` commit.
#[must_use]
pub fn git_rebase_onto_id<'a>(rebase: GitRebaseRef<'a>) -> OidRef<'a> {
    // SAFETY: the rebase is live and C returns its embedded initialized OID.
    let oid = unsafe { ffi::git_rebase_onto_id(rebase.as_ptr().cast_mut()) };
    // SAFETY: the C function returns the non-null address of the embedded
    // `onto_id`, which remains live for the rebase borrow.
    unsafe { OidRef::from_ptr(oid.cast_mut()) }.expect("an embedded OID is non-null")
}

/// Wraps: git_rebase_onto_name
/// Borrows the optional name of the rebase's `onto` reference.
#[must_use]
pub fn git_rebase_onto_name<'a>(rebase: GitRebaseRef<'a>) -> Option<&'a core::ffi::CStr> {
    // SAFETY: the rebase is live and C returns null or a rebase-owned string.
    let name = unsafe { ffi::git_rebase_onto_name(rebase.as_ptr().cast_mut()) };
    if name.is_null() {
        None
    } else {
        // SAFETY: a non-null result is NUL-terminated and owned by the rebase,
        // so it remains live and read-only for the handle borrow.
        Some(unsafe { core::ffi::CStr::from_ptr(name) })
    }
}
