//! Safe wrappers for libgit2 transaction APIs.

use core::ffi::CStr;
use core::marker::PhantomData;

use ffibox::CBox;

use crate::api::types::GitSignatureRef;
use crate::ffi;
use crate::oid::OidRef;
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

/// Wraps: git_transaction_set_symbolic_target
/// Sets a locked reference's symbolic target and copies all optional reflog
/// data into transaction-owned storage.
pub fn git_transaction_set_symbolic_target(
    transaction: &mut GitTransactionMut<'_>,
    refname: &CStr,
    target: &CStr,
    signature: Option<GitSignatureRef<'_>>,
    message: Option<&CStr>,
) -> Result<(), i32> {
    let signature = signature.map_or(core::ptr::null(), |value| value.as_ptr());
    let message = message.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the transaction is live and exclusive, required strings are
    // live C strings, and optional pointers are null or live for the call.
    // Libgit2 copies the target, signature, and message before returning.
    let status = unsafe {
        ffi::git_transaction_set_symbolic_target(
            transaction.as_mut_ptr(),
            refname.as_ptr(),
            target.as_ptr(),
            signature,
            message,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_transaction_set_target
/// Sets a locked reference's direct target and copies all optional reflog data
/// into transaction-owned storage.
pub fn git_transaction_set_target(
    transaction: &mut GitTransactionMut<'_>,
    refname: &CStr,
    target: OidRef<'_>,
    signature: Option<GitSignatureRef<'_>>,
    message: Option<&CStr>,
) -> Result<(), i32> {
    let signature = signature.map_or(core::ptr::null(), |value| value.as_ptr());
    let message = message.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the transaction is live and exclusive; all required inputs are
    // live for the call; optional pointers are null or live. Libgit2 copies
    // the target and optional reflog data before returning.
    let status = unsafe {
        ffi::git_transaction_set_target(
            transaction.as_mut_ptr(),
            refname.as_ptr(),
            target.as_ptr(),
            signature,
            message,
        )
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

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    unsafe fn seed(repository: *mut ffi::git_repository) -> ffi::git_oid {
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut head_commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut head_commit, repository, &head) },
            0
        );
        let parent_id = unsafe { ffi::git_commit_parent_id(head_commit, 0) };
        assert!(!parent_id.is_null());
        let prior = unsafe { *parent_id };
        unsafe { ffi::git_commit_free(head_commit) };
        for name in [
            c"refs/heads/tx-one",
            c"refs/heads/tx-two",
            c"refs/heads/logcopy",
        ] {
            let mut reference = core::ptr::null_mut();
            assert_eq!(
                unsafe {
                    ffi::git_reference_create(
                        &mut reference,
                        repository,
                        name.as_ptr(),
                        &head,
                        0,
                        c"transaction seed".as_ptr(),
                    )
                },
                0
            );
            unsafe { ffi::git_reference_free(reference) };
        }
        let mut alias = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reference_symbolic_create(
                    &mut alias,
                    repository,
                    c"refs/heads/tx-alias".as_ptr(),
                    c"refs/heads/tx-one".as_ptr(),
                    0,
                    c"transaction alias seed".as_ptr(),
                )
            },
            0
        );
        unsafe { ffi::git_reference_free(alias) };
        prior
    }

    unsafe fn raw_transaction(
        repository: *mut ffi::git_repository,
    ) -> (Vec<u8>, Vec<u8>, i32, usize) {
        let prior = unsafe { seed(repository) };
        let mut reflog = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_reflog_read(&mut reflog, repository, c"refs/heads/master".as_ptr()) },
            0
        );
        let mut transaction = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_transaction_new(&mut transaction, repository) },
            0
        );
        for name in [
            c"refs/heads/tx-one",
            c"refs/heads/tx-two",
            c"refs/heads/tx-alias",
            c"refs/heads/logcopy",
        ] {
            assert_eq!(
                unsafe { ffi::git_transaction_lock_ref(transaction, name.as_ptr()) },
                0
            );
        }
        assert_eq!(
            unsafe {
                ffi::git_transaction_set_target(
                    transaction,
                    c"refs/heads/tx-one".as_ptr(),
                    &prior,
                    core::ptr::null(),
                    c"transaction target".as_ptr(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_transaction_set_symbolic_target(
                    transaction,
                    c"refs/heads/tx-alias".as_ptr(),
                    c"refs/heads/master".as_ptr(),
                    core::ptr::null(),
                    c"transaction symbolic".as_ptr(),
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_transaction_remove(transaction, c"refs/heads/tx-two".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_transaction_set_reflog(transaction, c"refs/heads/logcopy".as_ptr(), reflog)
            },
            0
        );
        assert_eq!(unsafe { ffi::git_transaction_commit(transaction) }, 0);
        unsafe {
            ffi::git_transaction_free(transaction);
            ffi::git_reflog_free(reflog);
        }
        unsafe { observe(repository) }
    }

    fn safe_transaction(repository: *mut ffi::git_repository) -> (Vec<u8>, Vec<u8>, i32, usize) {
        let mut prior = unsafe { seed(repository) };
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut raw_reflog = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reflog_read(
                    &mut raw_reflog,
                    repository_view.as_mut_ptr(),
                    c"refs/heads/master".as_ptr(),
                )
            },
            0
        );
        // SAFETY: the successful read returned one complete caller-owned
        // reflog, and this fixture keeps its repository alive through `drop`.
        let reflog = unsafe { crate::reflog::GitReflogOwned::from_raw(raw_reflog) }.unwrap();
        let mut transaction = git_transaction_new(&mut repository_view).unwrap();
        for name in [
            c"refs/heads/tx-one",
            c"refs/heads/tx-two",
            c"refs/heads/tx-alias",
            c"refs/heads/logcopy",
        ] {
            git_transaction_lock_ref(&mut transaction.as_mut(), name).unwrap();
        }
        let prior = unsafe { OidRef::from_ptr(core::ptr::from_mut(&mut prior)) }.unwrap();
        git_transaction_set_target(
            &mut transaction.as_mut(),
            c"refs/heads/tx-one",
            prior,
            None,
            Some(c"transaction target"),
        )
        .unwrap();
        git_transaction_set_symbolic_target(
            &mut transaction.as_mut(),
            c"refs/heads/tx-alias",
            c"refs/heads/master",
            None,
            Some(c"transaction symbolic"),
        )
        .unwrap();
        git_transaction_remove(&mut transaction.as_mut(), c"refs/heads/tx-two").unwrap();
        git_transaction_set_reflog(
            &mut transaction.as_mut(),
            c"refs/heads/logcopy",
            reflog.as_ref(),
        )
        .unwrap();
        git_transaction_commit(&mut transaction.as_mut()).unwrap();
        drop(transaction);
        drop(reflog);
        unsafe { observe(repository) }
    }

    unsafe fn observe(repository: *mut ffi::git_repository) -> (Vec<u8>, Vec<u8>, i32, usize) {
        let mut direct = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut direct,
                    repository,
                    c"refs/heads/tx-one".as_ptr(),
                )
            },
            0
        );
        let mut alias = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reference_lookup(&mut alias, repository, c"refs/heads/tx-alias".as_ptr())
            },
            0
        );
        let alias_target = unsafe { CStr::from_ptr(ffi::git_reference_symbolic_target(alias)) }
            .to_bytes()
            .to_vec();
        unsafe { ffi::git_reference_free(alias) };
        let mut removed = core::ptr::null_mut();
        let removed_status = unsafe {
            ffi::git_reference_lookup(&mut removed, repository, c"refs/heads/tx-two".as_ptr())
        };
        let mut reflog = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reflog_read(&mut reflog, repository, c"refs/heads/logcopy".as_ptr())
            },
            0
        );
        let entries = unsafe { ffi::git_reflog_entrycount(reflog) };
        unsafe { ffi::git_reflog_free(reflog) };
        (direct.id.to_vec(), alias_target, removed_status, entries)
    }

    #[test]
    fn io_equiv_reference_transaction_targets_symbolic_remove_and_reflog() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("transaction-raw");
        let safe = HistoryFixture::new("transaction-safe");
        let raw_observation = unsafe { raw_transaction(raw.repository.as_ptr()) };
        let safe_observation = safe_transaction(safe.repository.as_ptr());
        assert_eq!(raw_observation, safe_observation);
        assert_eq!(raw_observation.1, b"refs/heads/master");
        assert_eq!(raw_observation.2, ffi::git_error_code_GIT_ENOTFOUND);
    }
}
