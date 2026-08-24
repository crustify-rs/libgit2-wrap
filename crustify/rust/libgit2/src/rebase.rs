//! Safe wrappers for libgit2 rebase APIs.

use core::ptr::{NonNull, addr_of};

use ffibox::{CBox, CDropped};

use crate::ffi;
use crate::oid::OidRef;

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

/// An exclusively owned, fully constructed libgit2 rebase.
pub type GitRebaseOwned = CBox<GitRebase>;

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
            size_of::<GitRebaseOwned>(),
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
    fn null_rebase_seams_create_no_handle() {
        // SAFETY: each conversion accepts null and returns `None` without
        // borrowing or adopting an object.
        unsafe {
            assert!(GitRebaseRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRebaseMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRebaseOwned::from_raw(ptr::null_mut()).is_none());
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
#[must_use]
pub fn git_rebase_operation_current(rebase: &mut GitRebaseMut<'_>) -> Option<usize> {
    // SAFETY: the exclusive handle satisfies the C signature; the call only
    // reads rebase state and retains no pointer.
    let current = unsafe { ffi::git_rebase_operation_current(rebase.as_mut_ptr()) };
    (current != usize::MAX).then_some(current)
}

/// Wraps: git_rebase_operation_entrycount
/// Returns the number of operations in the rebase plan.
#[must_use]
pub fn git_rebase_operation_entrycount(rebase: &mut GitRebaseMut<'_>) -> usize {
    // SAFETY: the exclusive handle satisfies the C signature; the call only
    // reads the initialized operation-array count.
    unsafe { ffi::git_rebase_operation_entrycount(rebase.as_mut_ptr()) }
}

/// Wraps: git_rebase_orig_head_id
/// Borrows the original HEAD object ID from a merge rebase.
#[must_use]
pub fn git_rebase_orig_head_id<'a>(rebase: &'a mut GitRebaseMut<'_>) -> OidRef<'a> {
    // SAFETY: the exclusive reborrow keeps the rebase and its inline OID live
    // and prevents mutation while the returned handle is usable.
    let id = unsafe { ffi::git_rebase_orig_head_id(rebase.as_mut_ptr()) };
    // SAFETY: libgit2 returns the non-null address of the initialized inline
    // `orig_head_id`; its lifetime is bounded by the rebase reborrow.
    unsafe { OidRef::from_ptr(id.cast_mut()) }.expect("an inline OID is non-null")
}

/// Wraps: git_rebase_orig_head_name
/// Borrows the optional original HEAD name from a merge rebase.
#[must_use]
pub fn git_rebase_orig_head_name<'a>(
    rebase: &'a mut GitRebaseMut<'_>,
) -> Option<&'a core::ffi::CStr> {
    // SAFETY: the exclusive reborrow keeps every rebase-owned string live and
    // prevents mutation for the returned borrow.
    let name = unsafe { ffi::git_rebase_orig_head_name(rebase.as_mut_ptr()) };
    if name.is_null() {
        None
    } else {
        // SAFETY: a non-null libgit2 result is NUL-terminated and owned by the
        // rebase for the duration of this exclusive reborrow.
        Some(unsafe { core::ffi::CStr::from_ptr(name) })
    }
}
