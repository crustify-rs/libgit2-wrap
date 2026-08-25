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

ffibox::define_ctype!(
    /// Wraps: git_commit_header
    /// A layout-compatible custom commit-header descriptor.
    ///
    /// Both pointer fields borrow NUL-terminated strings owned by the caller.
    /// Libgit2 reads them synchronously while creating a commit and neither
    /// retains nor frees them.
    GitCommitHeader,
    GitCommitHeaderRef,
    GitCommitHeaderMut,
    crate::ffi::git_commit_header
);

impl<'a> GitCommitHeaderRef<'a> {
    /// Field: git_commit_header.value
    /// Borrows the header value, or returns `None` for a defensively handled
    /// null field in an incomplete C value.
    #[must_use]
    pub fn value(&self) -> Option<&'a core::ffi::CStr> {
        let header = self.as_ptr();
        // SAFETY: raw-place projection reads the initialized pointer without
        // forming a reference to the C-visible header.
        let value = unsafe { core::ptr::addr_of!((*header).value).read() };
        if value.is_null() {
            None
        } else {
            // SAFETY: a valid header points to a NUL-terminated string that
            // remains live for the header handle's borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(value) })
        }
    }

    /// Field: git_commit_header.field
    /// Borrows the header name, or returns `None` for a defensively handled
    /// null field in an incomplete C value.
    #[must_use]
    pub fn field(&self) -> Option<&'a core::ffi::CStr> {
        let header = self.as_ptr();
        // SAFETY: raw-place projection reads the initialized pointer without
        // forming a reference to the C-visible header.
        let field = unsafe { core::ptr::addr_of!((*header).field).read() };
        if field.is_null() {
            None
        } else {
            // SAFETY: a valid header points to a NUL-terminated string that
            // remains live for the header handle's borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(field) })
        }
    }
}

impl GitCommitHeaderMut<'_> {
    /// Replaces the borrowed header value.
    ///
    /// # Safety
    ///
    /// `value` must remain live and NUL-terminated for every later use of the
    /// underlying `git_commit_header`, not merely for this mutable reborrow.
    pub unsafe fn set_value(&mut self, value: &core::ffi::CStr) {
        // SAFETY: the exclusive handle permits the pointer-field write; the
        // caller supplies the stored referent's unexpressible lifetime.
        unsafe {
            core::ptr::addr_of_mut!((*self.as_mut_ptr()).value).write(value.as_ptr());
        }
    }

    /// Replaces the borrowed header name.
    ///
    /// # Safety
    ///
    /// `field` must remain live and NUL-terminated for every later use of the
    /// underlying `git_commit_header`, not merely for this mutable reborrow.
    pub unsafe fn set_field(&mut self, field: &core::ffi::CStr) {
        // SAFETY: the exclusive handle permits the pointer-field write; the
        // caller supplies the stored referent's unexpressible lifetime.
        unsafe {
            core::ptr::addr_of_mut!((*self.as_mut_ptr()).field).write(field.as_ptr());
        }
    }
}

#[cfg(test)]
mod commit_header_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn commit_header_accessors_borrow_both_strings() {
        let mut raw = crate::ffi::git_commit_header {
            field: c"x-signature".as_ptr(),
            value: c"signed value".as_ptr(),
        };
        // SAFETY: `raw` and both static C strings remain live for the handle.
        let header = unsafe { GitCommitHeaderRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(header.field(), Some(c"x-signature"));
        assert_eq!(header.value(), Some(c"signed value"));
    }

    #[test]
    fn commit_header_mutation_replaces_borrowed_strings() {
        let mut raw = crate::ffi::git_commit_header {
            field: c"old-field".as_ptr(),
            value: c"old-value".as_ptr(),
        };
        // SAFETY: `raw` remains exclusively accessible through this handle.
        let mut header = unsafe { GitCommitHeaderMut::from_ptr(&raw mut raw) }.unwrap();
        // SAFETY: these static C strings outlive the underlying header.
        unsafe {
            header.set_field(c"new-field");
            header.set_value(c"new-value");
        }
        assert_eq!(header.as_ref().field(), Some(c"new-field"));
        assert_eq!(header.as_ref().value(), Some(c"new-value"));
    }

    #[test]
    fn commit_header_wrapper_matches_the_c_layout() {
        assert_eq!(
            size_of::<GitCommitHeader>(),
            size_of::<crate::ffi::git_commit_header>()
        );
        assert_eq!(
            align_of::<GitCommitHeader>(),
            align_of::<crate::ffi::git_commit_header>()
        );
    }
}
