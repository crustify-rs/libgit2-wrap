//! Safe wrappers for libgit2 submodule APIs.

use core::ffi::{CStr, c_void};
use core::marker::PhantomData;
use core::ptr::NonNull;

use ffibox::{CBox, CCloned};

use crate::api::submodule::{GitSubmoduleCallback, GitSubmoduleUpdateOptionsRef};
use crate::api::types::{
    GitSubmoduleIgnore, GitSubmoduleUpdate, InvalidGitSubmoduleIgnore, InvalidGitSubmoduleUpdate,
};
use crate::ffi;
use crate::oid::OidRef;
use crate::repository::{GitRepositoryMut, GitRepositoryOwned, GitRepositoryRef};

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

/// An owned submodule reference tied to its borrowed parent repository.
pub struct GitSubmoduleOwned<'repo> {
    inner: CBox<GitSubmodule>,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl GitSubmoduleOwned<'_> {
    /// Borrows the submodule.
    #[must_use]
    pub fn as_ref(&self) -> GitSubmoduleRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the submodule exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitSubmoduleMut<'_> {
        self.inner.as_mut()
    }
}

impl Clone for GitSubmoduleOwned<'_> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _repository: PhantomData,
        }
    }
}

fn adopt_submodule<'repo>(
    status: i32,
    inner: Option<CBox<GitSubmodule>>,
) -> Result<GitSubmoduleOwned<'repo>, i32> {
    if status != 0 {
        return Err(status);
    }
    let inner = inner.expect("a successful submodule constructor returns a non-null owner");
    Ok(GitSubmoduleOwned {
        inner,
        _repository: PhantomData,
    })
}

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

/// Wraps: git_submodule_add_finalize
/// Adds a prepared submodule and `.gitmodules` to the parent index.
pub fn git_submodule_add_finalize(submodule: &mut GitSubmoduleMut<'_>) -> Result<(), i32> {
    // SAFETY: the submodule and its tethered parent repository are live and
    // the exclusive handle permits the index-affecting operation.
    status_result(unsafe { ffi::git_submodule_add_finalize(submodule.as_mut_ptr()) })
}

/// Wraps: git_submodule_add_setup
/// Prepares a new submodule and returns an owner tied to `repository`.
pub fn git_submodule_add_setup<'repo>(
    mut repository: GitRepositoryMut<'repo>,
    url: &CStr,
    path: &CStr,
    use_gitlink: bool,
) -> Result<GitSubmoduleOwned<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable, the repository is live and exclusive,
    // and both strings live for the call. The returned submodule stores the
    // repository pointer, whose borrow is carried by `GitSubmoduleOwned`.
    let status = unsafe {
        ffi::git_submodule_add_setup(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            url.as_ptr(),
            path.as_ptr(),
            i32::from(use_gitlink),
        )
    };
    // SAFETY: output is null or one complete owned submodule reference.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_submodule(status, inner)
}

/// Wraps: git_submodule_add_to_index
/// Records the submodule's current `HEAD` in the parent index.
pub fn git_submodule_add_to_index(
    submodule: &mut GitSubmoduleMut<'_>,
    write_index: bool,
) -> Result<(), i32> {
    // SAFETY: the submodule is live and exclusive for its cached-ID updates.
    status_result(unsafe {
        ffi::git_submodule_add_to_index(submodule.as_mut_ptr(), i32::from(write_index))
    })
}

/// Wraps: git_submodule_branch
/// Borrows the configured branch, when one is present.
#[must_use]
pub fn git_submodule_branch<'a>(submodule: GitSubmoduleRef<'a>) -> Option<&'a CStr> {
    // SAFETY: the shared handle is live; the getter only reads an owned field.
    let branch = unsafe { ffi::git_submodule_branch(submodule.as_ptr().cast_mut()) };
    optional_submodule_string(branch)
}

/// Wraps: git_submodule_head_id
/// Borrows the submodule ID recorded in the current `HEAD` tree.
#[must_use]
pub fn git_submodule_head_id<'a>(submodule: GitSubmoduleRef<'a>) -> Option<OidRef<'a>> {
    // SAFETY: the shared handle is live and the getter only reads cached fields.
    let oid = unsafe { ffi::git_submodule_head_id(submodule.as_ptr().cast_mut()) };
    // SAFETY: a non-null result is an inline field live for the submodule borrow.
    unsafe { OidRef::from_ptr(oid.cast_mut()) }
}

/// Wraps: git_submodule_ignore
/// Returns the checked ignore rule used for status calculations.
pub fn git_submodule_ignore(
    submodule: GitSubmoduleRef<'_>,
) -> Result<GitSubmoduleIgnore, InvalidGitSubmoduleIgnore> {
    // SAFETY: the getter only reads the live submodule's scalar field.
    let value = unsafe { ffi::git_submodule_ignore(submodule.as_ptr().cast_mut()) };
    GitSubmoduleIgnore::try_from(value)
}

/// Wraps: git_submodule_index_id
/// Borrows the submodule ID recorded in the index.
#[must_use]
pub fn git_submodule_index_id<'a>(submodule: GitSubmoduleRef<'a>) -> Option<OidRef<'a>> {
    // SAFETY: the shared handle is live and the getter only reads cached fields.
    let oid = unsafe { ffi::git_submodule_index_id(submodule.as_ptr().cast_mut()) };
    // SAFETY: a non-null result is an inline field live for the submodule borrow.
    unsafe { OidRef::from_ptr(oid.cast_mut()) }
}

/// Wraps: git_submodule_init
/// Copies submodule configuration into the parent repository configuration.
pub fn git_submodule_init(submodule: &mut GitSubmoduleMut<'_>, overwrite: bool) -> Result<(), i32> {
    // SAFETY: the submodule and its parent are live and the exclusive handle
    // permits configuration and cache access during the call.
    status_result(unsafe { ffi::git_submodule_init(submodule.as_mut_ptr(), i32::from(overwrite)) })
}

/// Wraps: git_submodule_lookup
/// Looks up a submodule and ties its returned reference to `repository`.
pub fn git_submodule_lookup<'repo>(
    mut repository: GitRepositoryMut<'repo>,
    name: &CStr,
) -> Result<GitSubmoduleOwned<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable, repository is live and exclusive, and
    // `name` is a live C string. The result's type carries the stored repo borrow.
    let status = unsafe {
        ffi::git_submodule_lookup(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            name.as_ptr(),
        )
    };
    // SAFETY: output is null or one complete owned submodule reference.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_submodule(status, inner)
}

/// Wraps: git_submodule_lookup
/// Tests for a submodule without requesting an owned output reference.
pub fn git_submodule_exists(repository: &mut GitRepositoryMut<'_>, name: &CStr) -> Result<(), i32> {
    // SAFETY: a null output is explicitly supported; repository and name are
    // live for the call and no caller pointer is retained.
    status_result(unsafe {
        ffi::git_submodule_lookup(
            core::ptr::null_mut(),
            repository.as_mut_ptr(),
            name.as_ptr(),
        )
    })
}

/// Wraps: git_submodule_name
/// Borrows the non-null submodule name.
#[must_use]
pub fn git_submodule_name<'a>(submodule: GitSubmoduleRef<'a>) -> &'a CStr {
    // SAFETY: every complete submodule owns a non-null NUL-terminated name.
    unsafe { CStr::from_ptr(ffi::git_submodule_name(submodule.as_ptr().cast_mut())) }
}

/// Wraps: git_submodule_open
/// Opens a distinct owned repository for a checked-out submodule.
pub fn git_submodule_open(submodule: &mut GitSubmoduleMut<'_>) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and the submodule is live and exclusive
    // for its working-directory status cache updates.
    let status = unsafe { ffi::git_submodule_open(&mut output, submodule.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete, independently owned repository.
    unsafe { GitRepositoryOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_submodule_path
/// Borrows the non-null repository-relative submodule path.
#[must_use]
pub fn git_submodule_path<'a>(submodule: GitSubmoduleRef<'a>) -> &'a CStr {
    // SAFETY: every complete submodule owns a non-null NUL-terminated path.
    unsafe { CStr::from_ptr(ffi::git_submodule_path(submodule.as_ptr().cast_mut())) }
}

/// Wraps: git_submodule_reload
/// Refreshes cached submodule data from configuration, index, and `HEAD`.
pub fn git_submodule_reload(submodule: &mut GitSubmoduleMut<'_>, force: bool) -> Result<(), i32> {
    // SAFETY: the submodule is live and exclusively borrowed while its cached
    // strings, IDs, and flags are replaced.
    status_result(unsafe { ffi::git_submodule_reload(submodule.as_mut_ptr(), i32::from(force)) })
}

/// Wraps: git_submodule_repo_init
/// Initializes and returns an owned repository for a submodule checkout.
pub fn git_submodule_repo_init(
    submodule: GitSubmoduleRef<'_>,
    use_gitlink: bool,
) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: output is writable and the shared submodule remains live for the
    // call; the returned repository is independently owned.
    let status = unsafe {
        ffi::git_submodule_repo_init(
            core::ptr::addr_of_mut!(output),
            submodule.as_ptr(),
            i32::from(use_gitlink),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete, independently owned repository.
    unsafe { GitRepositoryOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_submodule_set_branch
/// Sets the configured branch, or removes it with `None`.
pub fn git_submodule_set_branch(
    repository: &mut GitRepositoryMut<'_>,
    name: &CStr,
    branch: Option<&CStr>,
) -> Result<(), i32> {
    let branch = branch.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: repository is exclusive and both strings are live or null as
    // allowed. Libgit2 copies configuration data before returning.
    status_result(unsafe {
        ffi::git_submodule_set_branch(repository.as_mut_ptr(), name.as_ptr(), branch)
    })
}

/// Wraps: git_submodule_set_ignore
/// Stores a checked ignore rule in `.gitmodules`.
pub fn git_submodule_set_ignore(
    repository: &mut GitRepositoryMut<'_>,
    name: &CStr,
    ignore: GitSubmoduleIgnore,
) -> Result<(), i32> {
    // SAFETY: repository and name are live for the call, and `ignore` is a
    // validated C enum value. No pointer is retained.
    status_result(unsafe {
        ffi::git_submodule_set_ignore(
            repository.as_mut_ptr(),
            name.as_ptr(),
            ffi::git_submodule_ignore_t::from(ignore),
        )
    })
}

fn status_result(status: i32) -> Result<(), i32> {
    if status == 0 { Ok(()) } else { Err(status) }
}

fn optional_submodule_string<'a>(value: *const core::ffi::c_char) -> Option<&'a CStr> {
    if value.is_null() {
        None
    } else {
        // SAFETY: callers bind `'a` to the submodule handle that owns this
        // live NUL-terminated string.
        Some(unsafe { CStr::from_ptr(value) })
    }
}

fn submodule_clone_result(
    status: i32,
    repository: Option<GitRepositoryOwned>,
) -> Result<GitRepositoryOwned, i32> {
    if status == 0 {
        repository.ok_or(ffi::git_error_code_GIT_ERROR)
    } else {
        drop(repository);
        Err(status)
    }
}

/// Wraps: git_submodule_clone
/// Performs the clone step and returns the newly owned submodule repository.
pub fn git_submodule_clone(
    submodule: &mut GitSubmoduleMut<'_>,
    options: Option<GitSubmoduleUpdateOptionsRef<'_, '_>>,
) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the output is writable, the submodule is live and exclusive for
    // status-cache changes, and all optional nested option borrows remain live
    // through this synchronous clone and its callbacks.
    let status = unsafe {
        ffi::git_submodule_clone(
            core::ptr::addr_of_mut!(output),
            submodule.as_mut_ptr(),
            options,
        )
    };
    // SAFETY: a non-null output is one complete repository owner transferred
    // by a successful clone; failure leaves it null.
    let repository = unsafe { GitRepositoryOwned::from_raw(output) };
    submodule_clone_result(status, repository)
}

/// Wraps: git_submodule_clone
/// Performs the clone step while asking libgit2 to release its repository handle.
pub fn git_submodule_clone_without_repository(
    submodule: &mut GitSubmoduleMut<'_>,
    options: Option<GitSubmoduleUpdateOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: null selects the documented no-output ownership contract; the
    // exclusive submodule and every nested option borrow remain live through
    // the synchronous clone and its callbacks.
    let status =
        unsafe { ffi::git_submodule_clone(core::ptr::null_mut(), submodule.as_mut_ptr(), options) };
    status_result(status)
}

/// Wraps: git_submodule_update
/// Initializes when requested, fetches as needed, and checks out the index commit.
pub fn git_submodule_update(
    submodule: &mut GitSubmoduleMut<'_>,
    initialize: bool,
    options: Option<GitSubmoduleUpdateOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null_mut(), |options| options.as_ptr().cast_mut());
    // SAFETY: the submodule is exclusively borrowed while its repository and
    // caches may change. C only copies the historically mutable options
    // header, and its nested borrows remain live through synchronous callbacks.
    let status = unsafe {
        ffi::git_submodule_update(submodule.as_mut_ptr(), i32::from(initialize), options)
    };
    status_result(status)
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
            size_of::<Option<GitSubmoduleOwned<'_>>>(),
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

    #[test]
    fn clone_result_preserves_errors_and_rejects_missing_success_output() {
        assert!(matches!(submodule_clone_result(-123, None), Err(-123)));
        assert_eq!(
            submodule_clone_result(0, None).unwrap_err(),
            ffi::git_error_code_GIT_ERROR
        );
    }

    #[test]
    fn clone_and_update_surfaces_contain_no_raw_pointer_obligations() {
        let clone: fn(
            &mut GitSubmoduleMut<'static>,
            Option<GitSubmoduleUpdateOptionsRef<'static, 'static>>,
        ) -> Result<GitRepositoryOwned, i32> = git_submodule_clone;
        let clone_without_output: fn(
            &mut GitSubmoduleMut<'static>,
            Option<GitSubmoduleUpdateOptionsRef<'static, 'static>>,
        ) -> Result<(), i32> = git_submodule_clone_without_repository;
        let update: fn(
            &mut GitSubmoduleMut<'static>,
            bool,
            Option<GitSubmoduleUpdateOptionsRef<'static, 'static>>,
        ) -> Result<(), i32> = git_submodule_update;
        let _ = (clone, clone_without_output, update);
    }
}

/// Wraps: git_submodule_foreach
/// Visits a stable snapshot of the repository's submodules.
pub fn git_submodule_foreach<C>(
    repository: &mut GitRepositoryMut<'_>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitSubmoduleCallback,
{
    unsafe extern "C" fn trampoline<C: GitSubmoduleCallback>(
        submodule: *mut ffi::git_submodule,
        name: *const core::ffi::c_char,
        payload: *mut c_void,
    ) -> i32 {
        if submodule.is_null() || name.is_null() || payload.is_null() {
            return -1;
        }
        // SAFETY: the wrapper supplies this live exclusive callback throughout
        // the synchronous traversal.
        let callback = unsafe { &mut *payload.cast::<C>() };
        // SAFETY: the snapshot holds a count on this non-null submodule and
        // invokes callbacks serially, granting exclusive access for the call.
        let submodule = unsafe { GitSubmoduleMut::from_ptr(submodule) }.expect("checked non-null");
        // SAFETY: `name` is the submodule-owned NUL string, live for this call.
        let name = unsafe { CStr::from_ptr(name) };
        callback.call(submodule, name)
    }

    // SAFETY: the repository and callback remain exclusively borrowed for the
    // synchronous traversal and libgit2 retains neither pointer afterwards.
    let status = unsafe {
        ffi::git_submodule_foreach(
            repository.as_mut_ptr(),
            Some(trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}
