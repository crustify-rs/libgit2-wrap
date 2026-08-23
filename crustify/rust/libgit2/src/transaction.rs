//! Safe wrappers for libgit2 transaction APIs.

use ffibox::CBox;

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_transaction
    /// An opaque transaction that batches reference or configuration updates.
    ///
    /// Owned transactions release outstanding locks and variant-specific
    /// resources through `git_transaction_free` when dropped.
    GitTransaction,
    GitTransactionRef,
    GitTransactionMut,
    ffi::git_transaction
);

/// An owned libgit2 transaction.
pub type GitTransactionOwned = CBox<GitTransaction>;

// SAFETY: `git_transaction_free` is the public destructor for a complete
// transaction, releases all resources selected by its internal variant, and
// accepts null, although `CDropped` supplies a live non-null allocation.
ffibox::impl_dropped!(
    GitTransaction,
    ffi::git_transaction,
    ffi::git_transaction_free
);

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
