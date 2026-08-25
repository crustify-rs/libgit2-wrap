//! Safe wrappers for libgit2 commit APIs.

use core::marker::PhantomData;
use core::ptr::NonNull;

/// Borrowed view over the parent-pointer array passed to commit callbacks.
#[derive(Clone, Copy)]
pub struct GitCommitParents<'a> {
    ptr: NonNull<*const crate::ffi::git_commit>,
    len: usize,
    _borrow: PhantomData<crate::commit::GitCommitRef<'a>>,
}

impl<'a> GitCommitParents<'a> {
    /// Constructs the transient view used by a callback trampoline.
    ///
    /// # Safety
    ///
    /// `ptr` must address `len` readable pointers to live commits for `'a`,
    /// and every element must be non-null.
    pub(crate) unsafe fn from_raw(
        ptr: *const *const crate::ffi::git_commit,
        len: usize,
    ) -> Option<Self> {
        let ptr = if len == 0 && ptr.is_null() {
            NonNull::dangling()
        } else {
            NonNull::new(ptr.cast_mut())?
        };
        Some(Self {
            ptr,
            len,
            _borrow: PhantomData,
        })
    }

    /// Returns the number of parent commits.
    #[must_use]
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns whether there are no parents.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrows one parent by position.
    #[must_use]
    pub fn get(self, index: usize) -> Option<crate::commit::GitCommitRef<'a>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: construction guarantees a readable `len`-element pointer
        // array, and the bounds check selects one initialized element.
        let parent = unsafe { self.ptr.as_ptr().add(index).read() };
        // SAFETY: construction requires each element to name a live commit for
        // `'a`; a null element is defensively rejected.
        unsafe { crate::commit::GitCommitRef::from_ptr(parent.cast_mut()) }
    }
}

/// Wraps: git_commit_create_cb
/// Safe callable surface for custom commit creation during a rebase.
pub trait GitCommitCreateCallback {
    /// Creates a commit ID or returns `GIT_PASSTHROUGH`/another error code.
    #[allow(clippy::too_many_arguments)]
    fn create(
        &mut self,
        out: &mut crate::oid::OidMut<'_>,
        author: crate::api::types::GitSignatureRef<'_>,
        committer: crate::api::types::GitSignatureRef<'_>,
        message_encoding: Option<&core::ffi::CStr>,
        message: &core::ffi::CStr,
        tree: crate::tree::GitTreeRef<'_>,
        parents: GitCommitParents<'_>,
    ) -> i32;
}

impl<F> GitCommitCreateCallback for F
where
    F: FnMut(
        &mut crate::oid::OidMut<'_>,
        crate::api::types::GitSignatureRef<'_>,
        crate::api::types::GitSignatureRef<'_>,
        Option<&core::ffi::CStr>,
        &core::ffi::CStr,
        crate::tree::GitTreeRef<'_>,
        GitCommitParents<'_>,
    ) -> i32,
{
    fn create(
        &mut self,
        out: &mut crate::oid::OidMut<'_>,
        author: crate::api::types::GitSignatureRef<'_>,
        committer: crate::api::types::GitSignatureRef<'_>,
        message_encoding: Option<&core::ffi::CStr>,
        message: &core::ffi::CStr,
        tree: crate::tree::GitTreeRef<'_>,
        parents: GitCommitParents<'_>,
    ) -> i32 {
        self(
            out,
            author,
            committer,
            message_encoding,
            message,
            tree,
            parents,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_implement_commit_creation() {
        fn accepts<C: GitCommitCreateCallback>(_callback: C) {}
        accepts(
            |_: &mut crate::oid::OidMut<'_>,
             _: crate::api::types::GitSignatureRef<'_>,
             _: crate::api::types::GitSignatureRef<'_>,
             _: Option<&core::ffi::CStr>,
             _: &core::ffi::CStr,
             _: crate::tree::GitTreeRef<'_>,
             _: GitCommitParents<'_>| 0,
        );
    }

    #[test]
    fn parent_pointer_view_checks_bounds_without_forming_a_slice() {
        let mut commit = crate::commit::GitCommit::zeroed();
        let pointers = [core::ptr::addr_of_mut!(commit)
            .cast::<crate::ffi::git_commit>()
            .cast_const()];
        // SAFETY: the local pointer array and opaque commit storage remain
        // live for this view, and its sole element is non-null.
        let parents = unsafe { GitCommitParents::from_raw(pointers.as_ptr(), 1) }.unwrap();
        assert_eq!(parents.len(), 1);
        assert!(!parents.is_empty());
        assert_eq!(parents.get(0).unwrap().as_ptr(), pointers[0]);
        assert!(parents.get(1).is_none());
    }
}
