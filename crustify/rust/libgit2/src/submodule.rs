//! Safe wrappers for libgit2 submodule APIs.

use core::ffi::CStr;
use core::ptr::NonNull;

use ffibox::{CBox, CCloned};

use crate::api::types::{GitSubmoduleIgnore, GitSubmoduleUpdate, InvalidGitSubmoduleUpdate};
use crate::ffi;
use crate::oid::OidRef;
use crate::repository::GitRepositoryMut;

ffibox::define_ctype!(
    /// Wraps: git_submodule
    /// An opaque, reference-counted description of a repository submodule.
    ///
    /// Owned handles release one reference with `git_submodule_free` and
    /// cloning acquires another reference with `git_submodule_dup`. The
    /// submodule internally borrows its parent repository, so operations that
    /// use that repository require it to remain alive.
    GitSubmodule,
    GitSubmoduleRef,
    GitSubmoduleMut,
    ffi::git_submodule
);

/// An owned reference to a libgit2 submodule.
pub type GitSubmoduleOwned = CBox<GitSubmodule>;

// SAFETY: `git_submodule_free` consumes one reference to a fully initialized
// `git_submodule`. Its refcount decrement releases the allocation only after
// the final reference, and `GitSubmodule` is transparent over the bindgen type.
ffibox::impl_dropped!(GitSubmodule, ffi::git_submodule, ffi::git_submodule_free);

// SAFETY: `git_submodule_dup` increments the live source's refcount and writes
// the same pointer to the non-null output slot. That new reference is released
// independently by the `CDropped` implementation above.
unsafe impl CCloned for GitSubmodule {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live submodule; `duplicate`
        // is a valid output slot, and the wrapper is layout-compatible with
        // `ffi::git_submodule`.
        let result = unsafe {
            ffi::git_submodule_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_submodule>(),
            )
        };
        debug_assert_eq!(result, 0);
        NonNull::new(duplicate.cast::<Self>())
    }
}

/// Wraps: git_submodule_set_update
/// Stores a submodule update strategy in the repository configuration.
pub fn git_submodule_set_update(
    repo: &mut GitRepositoryMut<'_>,
    name: &CStr,
    update: GitSubmoduleUpdate,
) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusively borrowed, `name` is a
    // live C string, and the checked strategy is passed by value. Nothing is
    // retained after the call.
    let status =
        unsafe { ffi::git_submodule_set_update(repo.as_mut_ptr(), name.as_ptr(), update.into()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_submodule_set_url
/// Stores a submodule URL in the repository configuration.
pub fn git_submodule_set_url(
    repo: &mut GitRepositoryMut<'_>,
    name: &CStr,
    url: &CStr,
) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusive and both strings are live
    // for this synchronous call; libgit2 copies their contents as needed.
    let status =
        unsafe { ffi::git_submodule_set_url(repo.as_mut_ptr(), name.as_ptr(), url.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_submodule_status
/// Computes the public submodule status bit set using `ignore`.
pub fn git_submodule_status(
    repo: &mut GitRepositoryMut<'_>,
    name: &CStr,
    ignore: GitSubmoduleIgnore,
) -> Result<u32, i32> {
    let mut output = 0;
    // SAFETY: `output` is writable, the repository is live and exclusive for
    // lookup and lazy cache work, and `name` is a live C string for the call.
    let status = unsafe {
        ffi::git_submodule_status(&mut output, repo.as_mut_ptr(), name.as_ptr(), ignore.into())
    };
    if status == 0 { Ok(output) } else { Err(status) }
}

/// Wraps: git_submodule_sync
/// Copies a submodule's configured URL into its checked-out repository.
pub fn git_submodule_sync(submodule: &mut GitSubmoduleMut<'_>) -> Result<(), i32> {
    // SAFETY: the submodule is live and exclusively borrowed for any lazy
    // state changes while its borrowed parent repository remains live.
    let status = unsafe { ffi::git_submodule_sync(submodule.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_submodule_update_strategy
/// Returns the effective update strategy selected for a submodule.
pub fn git_submodule_update_strategy(
    submodule: GitSubmoduleRef<'_>,
) -> Result<GitSubmoduleUpdate, InvalidGitSubmoduleUpdate> {
    // SAFETY: the getter only reads the live submodule. Restoring mutability at
    // the seam satisfies the historical C declaration without writing.
    let raw = unsafe { ffi::git_submodule_update_strategy(submodule.as_ptr().cast_mut()) };
    GitSubmoduleUpdate::try_from(raw)
}

/// Wraps: git_submodule_url
/// Borrows the configured URL, if the submodule has one.
#[must_use]
pub fn git_submodule_url<'a>(submodule: GitSubmoduleRef<'a>) -> Option<&'a CStr> {
    // SAFETY: this getter only reads the live submodule and any non-null result
    // is the submodule-owned NUL string valid for the handle lifetime.
    let url = unsafe { ffi::git_submodule_url(submodule.as_ptr().cast_mut()) };
    if url.is_null() {
        None
    } else {
        // SAFETY: the non-null result follows the ownership contract above.
        Some(unsafe { CStr::from_ptr(url) })
    }
}

/// Wraps: git_submodule_wd_id
/// Lazily loads and borrows the checked-out submodule's `HEAD` ID.
#[must_use]
pub fn git_submodule_wd_id<'a>(submodule: &'a mut GitSubmoduleMut<'_>) -> Option<OidRef<'a>> {
    // SAFETY: the exclusive borrow permits the getter's lazy mutation. A
    // non-null result points at the submodule's inline OID for this reborrow.
    let oid = unsafe { ffi::git_submodule_wd_id(submodule.as_mut_ptr()) }.cast_mut();
    // SAFETY: the non-null case is an inline field tied to `submodule`.
    unsafe { OidRef::from_ptr(oid) }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::{CCloned, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<GitSubmodule>(), size_of::<ffi::git_submodule>());
        assert_eq!(align_of::<GitSubmodule>(), align_of::<ffi::git_submodule>());
        assert_eq!(
            size_of::<GitSubmoduleRef<'_>>(),
            size_of::<*const ffi::git_submodule>()
        );
        assert_eq!(
            size_of::<GitSubmoduleMut<'_>>(),
            size_of::<*mut ffi::git_submodule>()
        );
        assert_eq!(
            size_of::<Option<GitSubmoduleOwned>>(),
            size_of::<*mut ffi::git_submodule>()
        );
    }

    #[test]
    fn submodule_registers_refcount_lifecycle() {
        fn assert_refcounted<T: CDropped + CCloned>() {}
        assert_refcounted::<GitSubmodule>();
    }

    #[test]
    fn borrowed_handles_preserve_the_submodule_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_submodule>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_submodule>();

        {
            // SAFETY: `raw` addresses live storage for the bindgen opaque type
            // and remains allocated for the duration of this handle.
            let shared = unsafe { GitSubmoduleRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitSubmoduleMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: no borrowed handle remains and this cast recovers the exact
        // allocation returned by `Box::into_raw` above.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_submodule>>()) });
    }
}
