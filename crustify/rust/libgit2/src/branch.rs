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

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

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
}
