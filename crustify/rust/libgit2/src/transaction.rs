//! Safe wrappers for libgit2 transaction APIs.

use core::ffi::CStr;
use core::marker::PhantomData;

use ffibox::CBox;

use crate::ffi;
use crate::reflog::GitReflogRef;
use crate::repository::{GitRepositoryMut, GitRepositoryRef};

ffibox::define_ctype!(
    /// Wraps: git_transaction
    /// An opaque transaction that batches reference or configuration updates.
    ///
    /// Owned transactions release outstanding locks and variant-specific
    /// resources through `git_transaction_free` when dropped: a reference
    /// transaction unlocks whatever it still holds, drops its owned refdb
    /// count and clears the pool its own storage lives in, while a
    /// configuration transaction hands its owned backend-instance token back
    /// through `git_config_unlock` and frees the allocation.
    ///
    /// Both variants only *borrow* the object they were opened against — the
    /// repository for a reference transaction, the configuration for a
    /// configuration transaction. The wrapper carries no lifetime, so keeping
    /// that object alive for at least as long as the transaction is an
    /// obligation of whichever constructor seam produces the owner.
    ///
    /// Libgit2 publishes no operation for duplicating a transaction or
    /// acquiring another ownership share, so there is no `CCloned` impl.
    GitTransaction,
    GitTransactionRef,
    GitTransactionMut,
    ffi::git_transaction
);

/// An owned libgit2 transaction.
pub type GitTransactionOwned = CBox<GitTransaction>;

/// A transaction tied to the repository pointer retained by libgit2.
pub struct RepositoryTransaction<'repo> {
    inner: GitTransactionOwned,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl RepositoryTransaction<'_> {
    /// Borrows the transaction.
    #[must_use]
    pub fn as_ref(&self) -> GitTransactionRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the transaction exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitTransactionMut<'_> {
        self.inner.as_mut()
    }
}

// SAFETY: `git_transaction_free` is the public destructor for a complete
// transaction, releases all resources selected by its internal variant, and
// accepts null, although `CDropped` supplies a live non-null allocation.
ffibox::impl_dropped!(
    GitTransaction,
    ffi::git_transaction,
    ffi::git_transaction_free
);

/// Wraps: git_transaction_commit
/// Applies the updates queued in a transaction.
pub fn git_transaction_commit(transaction: &mut GitTransactionMut<'_>) -> Result<(), i32> {
    // SAFETY: the transaction is live and exclusive and its tethered repository
    // remains alive for every reference-database access.
    let status = unsafe { ffi::git_transaction_commit(transaction.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_transaction_lock_ref
/// Locks `refname` and adds it to the transaction.
pub fn git_transaction_lock_ref(
    transaction: &mut GitTransactionMut<'_>,
    refname: &CStr,
) -> Result<(), i32> {
    // SAFETY: the transaction is live and exclusive, and libgit2 copies the
    // live reference name into its own pool before returning.
    let status =
        unsafe { ffi::git_transaction_lock_ref(transaction.as_mut_ptr(), refname.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_transaction_new
/// Creates a reference transaction and keeps its borrowed repository alive.
pub fn git_transaction_new<'repo>(
    repo: &'repo mut GitRepositoryMut<'_>,
) -> Result<RepositoryTransaction<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable and the repository remains
    // exclusively borrowed for the returned transaction's full lifetime.
    let status = unsafe { ffi::git_transaction_new(&mut raw, repo.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes a non-null, complete caller-owned transaction.
    let inner =
        unsafe { GitTransactionOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryTransaction {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_transaction_remove
/// Marks a locked reference for removal at commit time.
pub fn git_transaction_remove(
    transaction: &mut GitTransactionMut<'_>,
    refname: &CStr,
) -> Result<(), i32> {
    // SAFETY: the transaction is live and exclusive and `refname` is a live C
    // string used only to find the already-owned transaction node.
    let status = unsafe { ffi::git_transaction_remove(transaction.as_mut_ptr(), refname.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_transaction_set_reflog
/// Deep-copies `reflog` into the locked reference's transaction pool.
pub fn git_transaction_set_reflog(
    transaction: &mut GitTransactionMut<'_>,
    refname: &CStr,
    reflog: GitReflogRef<'_>,
) -> Result<(), i32> {
    // SAFETY: all inputs are live for the call; the transaction is exclusive
    // and libgit2 deep-copies the reflog and its strings before returning.
    let status = unsafe {
        ffi::git_transaction_set_reflog(transaction.as_mut_ptr(), refname.as_ptr(), reflog.as_ptr())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::CDropped;

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        assert_eq!(
            size_of::<GitTransaction>(),
            size_of::<ffi::git_transaction>()
        );
        assert_eq!(
            align_of::<GitTransaction>(),
            align_of::<ffi::git_transaction>()
        );
        assert_eq!(
            size_of::<GitTransactionRef<'static>>(),
            size_of::<*const ffi::git_transaction>()
        );
        assert_eq!(
            size_of::<GitTransactionMut<'static>>(),
            size_of::<*mut ffi::git_transaction>()
        );
        assert_eq!(
            size_of::<Option<GitTransactionOwned>>(),
            size_of::<*mut ffi::git_transaction>()
        );
    }

    #[test]
    fn borrowed_handles_preserve_the_transaction_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_transaction>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_transaction>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage
            // and remains live for the duration of the shared handle.
            let shared = unsafe { GitTransactionRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live,
            // and this scope has exclusive access to it.
            let mut exclusive = unsafe { GitTransactionMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from `Box::into_raw`, no handle remains, and the
        // cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_transaction>>()) });
    }

    #[test]
    fn transaction_registers_its_c_destructor() {
        fn assert_dropped<T: CDropped>() {}
        assert_dropped::<GitTransaction>();
    }
}
