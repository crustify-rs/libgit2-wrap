//! Safe wrappers for libgit2 submodule APIs.

use core::ffi::{CStr, c_void};
use core::marker::PhantomData;
use core::ptr::NonNull;

use ffibox::{CBox, CCloned};

use crate::api::buffer::GitBuf;
use crate::api::submodule::{
    GitSubmoduleCallback, GitSubmoduleStatusFlags, GitSubmoduleUpdateOptionsMut,
    GitSubmoduleUpdateOptionsRef,
};
use crate::api::types::{
    GitSubmoduleIgnore, GitSubmoduleRecurse, GitSubmoduleUpdate, InvalidGitSubmoduleIgnore,
    InvalidGitSubmoduleRecurse, InvalidGitSubmoduleUpdate,
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
/// Computes the checked submodule status bit set using `ignore`.
pub fn git_submodule_status(
    repo: &mut GitRepositoryMut<'_>,
    name: &CStr,
    ignore: GitSubmoduleIgnore,
) -> Result<GitSubmoduleStatusFlags, i32> {
    let mut output = 0;
    // SAFETY: `output` is writable, the repository is live and exclusive for
    // lookup and lazy cache work, and `name` is a live C string for the call.
    let status = unsafe {
        ffi::git_submodule_status(&mut output, repo.as_mut_ptr(), name.as_ptr(), ignore.into())
    };
    if status != 0 {
        return Err(status);
    }
    // C clears its private bits before reporting, so only published status
    // bits can reach here; anything else is reported as a generic error.
    GitSubmoduleStatusFlags::try_from(output).map_err(|_| ffi::git_error_code_GIT_ERROR)
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
    // C assigns this slot only on the success path, after the clone has
    // completed; every failure jumps to the cleanup label ahead of that
    // assignment and leaves the caller's value in place. The null below is
    // therefore what makes a failure observable, not a defensive extra.
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
    // by a successful clone; a failure leaves the initialized null in place.
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
///
/// The options are taken as a shared handle even though C declares them
/// `git_submodule_update_options *` without `const`: the implementation copies
/// the record into a local and works on that copy alone, so the caller's
/// storage is never written. `git_submodule_clone` already declares the same
/// argument `const`, and the two agree in behaviour.
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

#[cfg(test)]
mod unit_tests {
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

    /// A balanced hold on the process-global libgit2 initialization, which
    /// every allocation below the FFI seam requires.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and refcounted;
            // `Drop` below balances this successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard; every owner taken under it is dropped first.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A temporary directory tree removed when the test ends.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("crustify-submodule-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }

        /// Writes the minimum object store, ref namespace, `HEAD` and config
        /// libgit2 needs to open a repository at `git_dir`.
        fn write_git_dir(git_dir: &std::path::Path, bare: bool) {
            std::fs::create_dir_all(git_dir.join("objects/info")).expect("an object directory");
            std::fs::create_dir_all(git_dir.join("objects/pack")).expect("a pack directory");
            std::fs::create_dir_all(git_dir.join("refs/heads")).expect("a branch namespace");
            std::fs::create_dir_all(git_dir.join("refs/tags")).expect("a tag namespace");
            std::fs::write(git_dir.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                git_dir.join("config"),
                format!("[core]\n\trepositoryformatversion = 0\n\tbare = {bare}\n"),
            )
            .expect("a config file");
        }

        /// An empty bare repository, usable as a submodule's clone source.
        fn into_bare_repository(self) -> Self {
            Self::write_git_dir(&self.0, true);
            self
        }

        /// An empty repository with a working directory, so it can own
        /// submodules.
        fn into_working_repository(self) -> Self {
            Self::write_git_dir(&self.0.join(".git"), false);
            self
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }

        fn c_path(&self) -> std::ffi::CString {
            std::ffi::CString::new(self.0.to_str().expect("a UTF-8 temporary path"))
                .expect("a path without interior NUL")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn submodule_clone_transfers_the_checked_out_repository() {
        let _init = Libgit2Init::acquire();
        let source = Scratch::new("clone-source").into_bare_repository();
        let parent = Scratch::new("clone-parent").into_working_repository();
        let mut repository = crate::repository::git_repository_open(&parent.c_path())
            .expect("the hand-built parent repository opens");
        let mut submodule =
            git_submodule_add_setup(repository.as_mut(), &source.c_path(), c"sub", true)
                .expect("the submodule structure is created");

        let checkout = git_submodule_clone(&mut submodule.as_mut(), None)
            .expect("the configured local source clones into the submodule");
        let path = crate::repository::git_repository_path(checkout.as_ref())
            .expect("an on-disk submodule repository path");
        assert_eq!(
            std::path::Path::new(path.to_str().expect("a UTF-8 path").trim_end_matches('/')),
            parent.path().join(".git/modules/sub"),
            "a gitlink submodule keeps its repository under the parent's modules directory"
        );

        // The same operation without an output slot releases the repository
        // inside C instead of transferring it.
        assert_eq!(
            git_submodule_clone_without_repository(&mut submodule.as_mut(), None),
            Ok(())
        );
    }

    #[test]
    fn submodule_update_reports_the_missing_index_entry() {
        let _init = Libgit2Init::acquire();
        let source = Scratch::new("update-source").into_bare_repository();
        let parent = Scratch::new("update-parent").into_working_repository();
        let mut repository = crate::repository::git_repository_open(&parent.c_path())
            .expect("the hand-built parent repository opens");
        let mut submodule =
            git_submodule_add_setup(repository.as_mut(), &source.c_path(), c"sub", true)
                .expect("the submodule structure is created");

        // `git_submodule_add_setup` prepares the working directory but does
        // not record the submodule in the parent index, which is where update
        // reads the commit it should check out.
        assert_eq!(
            git_submodule_update(&mut submodule.as_mut(), true, None),
            Err(-1)
        );
        let error = crate::util::errors::git_error_last();
        assert_eq!(
            error.message.as_deref(),
            Some(c"could not get ID of submodule in index")
        );
    }

    #[test]
    fn submodule_status_reports_checked_published_flags() {
        let _init = Libgit2Init::acquire();
        let source = Scratch::new("status-source").into_bare_repository();
        let parent = Scratch::new("status-parent").into_working_repository();
        let mut repository = crate::repository::git_repository_open(&parent.c_path())
            .expect("the hand-built parent repository opens");
        let mut submodule =
            git_submodule_add_setup(repository.as_mut(), &source.c_path(), c"sub", true)
                .expect("the submodule structure is created");

        // `git_submodule_location` asks for the same bit set with ignore
        // `ALL`, which restricts the answer to the four source bits.
        let location = git_submodule_location(&mut submodule.as_mut())
            .expect("the registered submodule has a location");
        let sources = GitSubmoduleStatusFlags::IN_HEAD
            | GitSubmoduleStatusFlags::IN_INDEX
            | GitSubmoduleStatusFlags::IN_CONFIG
            | GitSubmoduleStatusFlags::IN_WORKDIR;
        assert!(sources.contains(location));
        drop(submodule);

        let status = git_submodule_status(
            &mut repository.as_mut(),
            c"sub",
            GitSubmoduleIgnore::Unspecified,
        )
        .expect("the registered submodule has a status");
        assert!(
            status.contains(GitSubmoduleStatusFlags::IN_CONFIG),
            "`git_submodule_add_setup` records the submodule in .gitmodules"
        );
        assert!(
            GitSubmoduleStatusFlags::ALL.contains(status),
            "C clears its private bits, so only published bits are reported"
        );
        assert_eq!(status & sources, location);
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

/// Wraps: git_submodule_fetch_recurse_submodules
/// Returns the submodule's configured fetch-recursion policy.
pub fn git_submodule_fetch_recurse_submodules(
    submodule: GitSubmoduleRef<'_>,
) -> Result<GitSubmoduleRecurse, InvalidGitSubmoduleRecurse> {
    // SAFETY: the implementation only reads the live submodule field and
    // retains no pointer; the mutable C spelling is historical.
    GitSubmoduleRecurse::try_from(unsafe {
        ffi::git_submodule_fetch_recurse_submodules(submodule.as_ptr().cast_mut())
    })
}

/// Wraps: git_submodule_location
/// Returns the checked flags describing where submodule metadata is present.
pub fn git_submodule_location(
    submodule: &mut GitSubmoduleMut<'_>,
) -> Result<GitSubmoduleStatusFlags, i32> {
    let mut location = 0;
    // SAFETY: `location` is writable and the submodule is exclusive for the
    // status lookup and any cache activity it performs.
    let status = unsafe { ffi::git_submodule_location(&mut location, submodule.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    GitSubmoduleStatusFlags::try_from(location).map_err(|_| ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_submodule_owner
/// Borrows the parent repository of a submodule.
#[must_use]
pub fn git_submodule_owner<'a>(submodule: GitSubmoduleRef<'a>) -> GitRepositoryRef<'a> {
    // SAFETY: the submodule is live and C returns its required parent pointer.
    let repository = unsafe { ffi::git_submodule_owner(submodule.as_ptr().cast_mut()) };
    // SAFETY: a valid submodule's parent remains live for its borrow.
    unsafe { GitRepositoryRef::from_ptr(repository) }
        .expect("a valid submodule has a repository owner")
}

/// Wraps: git_submodule_resolve_url
/// Resolves a submodule URL relative to its parent repository.
pub fn git_submodule_resolve_url(
    repository: &mut GitRepositoryMut<'_>,
    url: &CStr,
) -> Result<ffibox::CVal<GitBuf>, i32> {
    let mut resolved = GitBuf::new();
    let status = {
        let mut output = resolved.as_mut();
        // SAFETY: `output` is an empty exclusive buffer, the repository is
        // exclusive for config lookup, and `url` is a live C string.
        unsafe {
            ffi::git_submodule_resolve_url(
                output.as_mut_ptr(),
                repository.as_mut_ptr(),
                url.as_ptr(),
            )
        }
    };
    if status == 0 {
        Ok(resolved)
    } else {
        Err(status)
    }
}

/// Wraps: git_submodule_set_fetch_recurse_submodules
/// Stores the fetch-recursion policy for a named submodule.
pub fn git_submodule_set_fetch_recurse_submodules(
    repository: &mut GitRepositoryMut<'_>,
    name: &CStr,
    recurse: GitSubmoduleRecurse,
) -> Result<(), i32> {
    // SAFETY: the repository is exclusive, `name` is live, and the checked
    // enum converts to a published C value. No pointer is retained.
    let status = unsafe {
        ffi::git_submodule_set_fetch_recurse_submodules(
            repository.as_mut_ptr(),
            name.as_ptr(),
            recurse.into(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_submodule_update_options_init
/// Restores caller-owned submodule-update options to the published defaults.
///
/// The record belongs to the caller throughout: C validates `version`, copies
/// the current `GIT_SUBMODULE_UPDATE_OPTIONS_INIT` template over the storage
/// it was handed, and retains no pointer into it. This is the reinitializing
/// counterpart of
/// [`GitSubmoduleUpdateOptions::new`](crate::api::submodule::GitSubmoduleUpdateOptions::new),
/// which writes the same defaults into fresh storage.
///
/// `version` is a compatibility gate, not a selector. C accepts only
/// `1..=GIT_SUBMODULE_UPDATE_OPTIONS_VERSION`, and every accepted value copies
/// the same single template. Any other value is refused with
/// `GIT_ERROR_INVALID` before the record is touched, so a failed call leaves
/// the caller's values exactly as they were.
///
/// A successful call overwrites the whole record. This type stores no `'data`
/// borrow directly, but both nested subrecords do: the checkout options'
/// labels and target directory, and the fetch options' custom headers,
/// callback table and proxy URL all stop being reachable through it. The
/// record owns none of them, so clearing them frees nothing and leaks nothing.
pub fn git_submodule_update_options_init(
    options: &mut GitSubmoduleUpdateOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies non-null writable
    // layout-compatible storage, and the initializer retains no pointer to
    // the options header or to any field it clears.
    let status = unsafe { ffi::git_submodule_update_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, safe_buf_bytes};

    fn file_url(path: &std::path::Path) -> std::ffi::CString {
        std::ffi::CString::new(format!("file://{}", path.to_str().unwrap())).unwrap()
    }

    #[derive(Debug, Eq, PartialEq)]
    struct SubmoduleObservation {
        name: Vec<u8>,
        path: Vec<u8>,
        url: Vec<u8>,
        head: Option<Vec<u8>>,
        index: Option<Vec<u8>>,
        workdir: Option<Vec<u8>>,
    }

    unsafe fn raw_add(repository: *mut ffi::git_repository, url: &CStr) -> SubmoduleObservation {
        let mut submodule = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_submodule_add_setup(
                    &mut submodule,
                    repository,
                    url.as_ptr(),
                    c"deps/child".as_ptr(),
                    1,
                )
            },
            0
        );
        let mut cloned = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_submodule_clone(&mut cloned, submodule, core::ptr::null()) },
            0
        );
        unsafe { ffi::git_repository_free(cloned) };
        assert_eq!(unsafe { ffi::git_submodule_add_finalize(submodule) }, 0);
        assert_eq!(unsafe { ffi::git_submodule_reload(submodule, 1) }, 0);
        let oid = |value: *const ffi::git_oid| {
            (!value.is_null()).then(|| unsafe { (*value).id }.to_vec())
        };
        let observation = SubmoduleObservation {
            name: unsafe { CStr::from_ptr(ffi::git_submodule_name(submodule)) }
                .to_bytes()
                .to_vec(),
            path: unsafe { CStr::from_ptr(ffi::git_submodule_path(submodule)) }
                .to_bytes()
                .to_vec(),
            url: unsafe { CStr::from_ptr(ffi::git_submodule_url(submodule)) }
                .to_bytes()
                .to_vec(),
            head: oid(unsafe { ffi::git_submodule_head_id(submodule) }),
            index: oid(unsafe { ffi::git_submodule_index_id(submodule) }),
            workdir: oid(unsafe { ffi::git_submodule_wd_id(submodule) }),
        };
        unsafe { ffi::git_submodule_free(submodule) };
        observation
    }

    fn safe_add(
        repository: crate::repository::GitRepositoryMut<'_>,
        url: &CStr,
    ) -> SubmoduleObservation {
        let mut submodule = git_submodule_add_setup(repository, url, c"deps/child", true).unwrap();
        drop(git_submodule_clone(&mut submodule.as_mut(), None).unwrap());
        git_submodule_add_finalize(&mut submodule.as_mut()).unwrap();
        git_submodule_reload(&mut submodule.as_mut(), true).unwrap();
        let oid =
            |value: Option<OidRef<'_>>| value.map(|id| id.raw_bytes().elems().collect::<Vec<_>>());
        let name = git_submodule_name(submodule.as_ref()).to_bytes().to_vec();
        let path = git_submodule_path(submodule.as_ref()).to_bytes().to_vec();
        let url = git_submodule_url(submodule.as_ref())
            .unwrap()
            .to_bytes()
            .to_vec();
        let head = oid(git_submodule_head_id(submodule.as_ref()));
        let index = oid(git_submodule_index_id(submodule.as_ref()));
        let workdir = oid(git_submodule_wd_id(&mut submodule.as_mut()));
        SubmoduleObservation {
            name,
            path,
            url,
            head,
            index,
            workdir,
        }
    }

    #[test]
    fn io_equiv_submodule_add_clone_and_finalize() {
        let _libgit2 = Libgit2Init::acquire();
        let child = HistoryFixture::new("submodule-child");
        let raw_parent = HistoryFixture::new("submodule-parent-raw");
        let safe_parent = HistoryFixture::new("submodule-parent-safe");
        let url = file_url(child.directory.path());
        let raw = unsafe { raw_add(raw_parent.repository.as_ptr(), &url) };
        let safe_repository = unsafe {
            crate::repository::GitRepositoryMut::from_ptr(safe_parent.repository.as_ptr())
        }
        .unwrap();
        assert_eq!(raw, safe_add(safe_repository, &url));
        assert_eq!(raw.name, b"deps/child");
        assert!(raw.index.is_some());
        assert_eq!(raw.index, raw.workdir);
    }

    unsafe fn raw_repo_init_and_null_output_clone(
        init_parent: &HistoryFixture,
        clone_parent: &HistoryFixture,
        url: &CStr,
    ) -> (bool, bool, Vec<u8>) {
        let mut submodule = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_submodule_add_setup(
                    &mut submodule,
                    init_parent.repository.as_ptr(),
                    url.as_ptr(),
                    c"deps/initialized".as_ptr(),
                    1,
                )
            },
            0
        );
        std::fs::remove_dir_all(init_parent.directory.path().join("deps/initialized")).unwrap();
        let _ = std::fs::remove_dir_all(
            init_parent
                .directory
                .path()
                .join(".git/modules/deps/initialized"),
        );
        let mut initialized = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_submodule_repo_init(&mut initialized, submodule, 1) },
            0
        );
        let empty = unsafe { ffi::git_repository_is_empty(initialized) } != 0;
        let workdir_present = !unsafe { ffi::git_repository_workdir(initialized) }.is_null();
        unsafe {
            ffi::git_repository_free(initialized);
            ffi::git_submodule_free(submodule);
        }

        let mut cloned = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_submodule_add_setup(
                    &mut cloned,
                    clone_parent.repository.as_ptr(),
                    url.as_ptr(),
                    c"deps/null-output".as_ptr(),
                    1,
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_submodule_clone(core::ptr::null_mut(), cloned, core::ptr::null()) },
            0
        );
        assert_eq!(unsafe { ffi::git_submodule_add_finalize(cloned) }, 0);
        unsafe { ffi::git_submodule_free(cloned) };
        let contents = std::fs::read(
            clone_parent
                .directory
                .path()
                .join("deps/null-output/README.md"),
        )
        .unwrap();
        (empty, workdir_present, contents)
    }

    fn safe_repo_init_and_null_output_clone(
        init_parent: &HistoryFixture,
        clone_parent: &HistoryFixture,
        url: &CStr,
    ) -> (bool, bool, Vec<u8>) {
        let init_repository = unsafe {
            crate::repository::GitRepositoryMut::from_ptr(init_parent.repository.as_ptr())
        }
        .unwrap();
        let submodule =
            git_submodule_add_setup(init_repository, url, c"deps/initialized", true).unwrap();
        std::fs::remove_dir_all(init_parent.directory.path().join("deps/initialized")).unwrap();
        let _ = std::fs::remove_dir_all(
            init_parent
                .directory
                .path()
                .join(".git/modules/deps/initialized"),
        );
        let mut initialized = git_submodule_repo_init(submodule.as_ref(), true).unwrap();
        let empty = crate::repository::git_repository_is_empty(&mut initialized.as_mut()).unwrap();
        let workdir_present =
            crate::repository::git_repository_workdir(initialized.as_ref()).is_some();
        drop(initialized);
        drop(submodule);

        let clone_repository = unsafe {
            crate::repository::GitRepositoryMut::from_ptr(clone_parent.repository.as_ptr())
        }
        .unwrap();
        let mut cloned =
            git_submodule_add_setup(clone_repository, url, c"deps/null-output", true).unwrap();
        git_submodule_clone_without_repository(&mut cloned.as_mut(), None).unwrap();
        git_submodule_add_finalize(&mut cloned.as_mut()).unwrap();
        let contents = std::fs::read(
            clone_parent
                .directory
                .path()
                .join("deps/null-output/README.md"),
        )
        .unwrap();
        (empty, workdir_present, contents)
    }

    #[test]
    fn io_equiv_submodule_repo_init_and_clone_without_repository_output() {
        let _libgit2 = Libgit2Init::acquire();
        let child = HistoryFixture::new("submodule-init-child");
        let raw_init = HistoryFixture::new("submodule-init-parent-raw");
        let raw_clone = HistoryFixture::new("submodule-null-clone-parent-raw");
        let safe_init = HistoryFixture::new("submodule-init-parent-safe");
        let safe_clone = HistoryFixture::new("submodule-null-clone-parent-safe");
        let url = file_url(child.directory.path());
        let raw = unsafe { raw_repo_init_and_null_output_clone(&raw_init, &raw_clone, &url) };
        let safe = safe_repo_init_and_null_output_clone(&safe_init, &safe_clone, &url);
        assert_eq!(raw, safe);
        assert!(raw.0);
        assert!(raw.1);
        assert_eq!(raw.2, b"fixture\nwith a third revision\n");
    }

    #[derive(Debug, Eq, PartialEq)]
    struct LifecycleObservation {
        status: ffi::git_submodule_status_t,
        location: ffi::git_submodule_status_t,
        update: ffi::git_submodule_update_t,
        ignore: ffi::git_submodule_ignore_t,
        recurse: ffi::git_submodule_recurse_t,
        branch: Option<Vec<u8>>,
        resolved_url: Vec<u8>,
        owner_is_bare: bool,
        child_is_bare: bool,
        exists: bool,
        foreach_names: Vec<Vec<u8>>,
        foreach_stop: i32,
    }

    unsafe fn raw_lifecycle(
        repository: *mut ffi::git_repository,
        url: &CStr,
    ) -> LifecycleObservation {
        assert_eq!(
            unsafe {
                ffi::git_submodule_set_branch(repository, c"deps/child".as_ptr(), c"topic".as_ptr())
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_submodule_set_ignore(
                    repository,
                    c"deps/child".as_ptr(),
                    ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_UNTRACKED,
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_submodule_set_update(
                    repository,
                    c"deps/child".as_ptr(),
                    ffi::git_submodule_update_t_GIT_SUBMODULE_UPDATE_CHECKOUT,
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_submodule_set_fetch_recurse_submodules(
                    repository,
                    c"deps/child".as_ptr(),
                    ffi::git_submodule_recurse_t_GIT_SUBMODULE_RECURSE_ONDEMAND,
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_submodule_set_url(repository, c"deps/child".as_ptr(), url.as_ptr()) },
            0
        );

        unsafe extern "C" fn collect(
            _submodule: *mut ffi::git_submodule,
            name: *const core::ffi::c_char,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            unsafe { &mut *payload.cast::<Vec<Vec<u8>>>() }
                .push(unsafe { CStr::from_ptr(name) }.to_bytes().to_vec());
            0
        }
        unsafe extern "C" fn stop(
            _submodule: *mut ffi::git_submodule,
            _name: *const core::ffi::c_char,
            _payload: *mut core::ffi::c_void,
        ) -> i32 {
            17
        }
        let mut foreach_names = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_submodule_foreach(
                    repository,
                    Some(collect),
                    core::ptr::from_mut(&mut foreach_names).cast(),
                )
            },
            0
        );
        let foreach_stop =
            unsafe { ffi::git_submodule_foreach(repository, Some(stop), core::ptr::null_mut()) };

        let exists = unsafe {
            ffi::git_submodule_lookup(core::ptr::null_mut(), repository, c"deps/child".as_ptr())
                == 0
        };
        let mut submodule = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_submodule_lookup(&mut submodule, repository, c"deps/child".as_ptr())
            },
            0
        );
        assert_eq!(unsafe { ffi::git_submodule_init(submodule, 1) }, 0);
        assert_eq!(unsafe { ffi::git_submodule_sync(submodule) }, 0);
        assert_eq!(unsafe { ffi::git_submodule_reload(submodule, 1) }, 0);
        assert_eq!(unsafe { ffi::git_submodule_add_to_index(submodule, 1) }, 0);
        let mut status = 0;
        assert_eq!(
            unsafe {
                ffi::git_submodule_status(
                    &mut status,
                    repository,
                    c"deps/child".as_ptr(),
                    ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_UNSPECIFIED,
                )
            },
            0
        );
        let mut location = 0;
        assert_eq!(
            unsafe { ffi::git_submodule_location(&mut location, submodule) },
            0
        );
        let mut resolved = unsafe { core::mem::zeroed::<ffi::git_buf>() };
        assert_eq!(
            unsafe { ffi::git_submodule_resolve_url(&mut resolved, repository, url.as_ptr()) },
            0
        );
        let resolved_url = unsafe {
            core::slice::from_raw_parts(resolved.ptr.cast::<u8>(), resolved.size).to_vec()
        };
        unsafe { ffi::git_buf_dispose(&mut resolved) };
        let mut child = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_submodule_open(&mut child, submodule) }, 0);
        let owner_is_bare =
            unsafe { ffi::git_repository_is_bare(ffi::git_submodule_owner(submodule)) != 0 };
        let child_is_bare = unsafe { ffi::git_repository_is_bare(child) != 0 };
        let branch = {
            let value = unsafe { ffi::git_submodule_branch(submodule) };
            (!value.is_null()).then(|| unsafe { CStr::from_ptr(value) }.to_bytes().to_vec())
        };
        let observation = LifecycleObservation {
            status,
            location,
            update: unsafe { ffi::git_submodule_update_strategy(submodule) },
            ignore: unsafe { ffi::git_submodule_ignore(submodule) },
            recurse: unsafe { ffi::git_submodule_fetch_recurse_submodules(submodule) },
            branch,
            resolved_url,
            owner_is_bare,
            child_is_bare,
            exists,
            foreach_names,
            foreach_stop,
        };
        unsafe {
            ffi::git_repository_free(child);
            ffi::git_submodule_free(submodule);
        }
        observation
    }

    fn safe_lifecycle(repository: *mut ffi::git_repository, url: &CStr) -> LifecycleObservation {
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        git_submodule_set_branch(&mut repository_view, c"deps/child", Some(c"topic")).unwrap();
        git_submodule_set_ignore(
            &mut repository_view,
            c"deps/child",
            GitSubmoduleIgnore::Untracked,
        )
        .unwrap();
        git_submodule_set_update(
            &mut repository_view,
            c"deps/child",
            GitSubmoduleUpdate::Checkout,
        )
        .unwrap();
        git_submodule_set_fetch_recurse_submodules(
            &mut repository_view,
            c"deps/child",
            GitSubmoduleRecurse::OnDemand,
        )
        .unwrap();
        git_submodule_set_url(&mut repository_view, c"deps/child", url).unwrap();
        let mut foreach_names = Vec::new();
        git_submodule_foreach(
            &mut repository_view,
            &mut |_submodule: crate::submodule::GitSubmoduleMut<'_>, name: &CStr| {
                foreach_names.push(name.to_bytes().to_vec());
                0
            },
        )
        .unwrap();
        let foreach_stop = git_submodule_foreach(
            &mut repository_view,
            &mut |_submodule: crate::submodule::GitSubmoduleMut<'_>, _name: &CStr| 17,
        )
        .unwrap_err();
        let exists = git_submodule_exists(&mut repository_view, c"deps/child").is_ok();
        let mut submodule = git_submodule_lookup(repository_view, c"deps/child").unwrap();
        git_submodule_init(&mut submodule.as_mut(), true).unwrap();
        git_submodule_sync(&mut submodule.as_mut()).unwrap();
        git_submodule_reload(&mut submodule.as_mut(), true).unwrap();
        git_submodule_add_to_index(&mut submodule.as_mut(), true).unwrap();
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let status = git_submodule_status(
            &mut repository_view,
            c"deps/child",
            GitSubmoduleIgnore::Unspecified,
        )
        .unwrap()
        .bits();
        let location = git_submodule_location(&mut submodule.as_mut())
            .unwrap()
            .bits();
        let resolved_url = safe_buf_bytes(
            git_submodule_resolve_url(&mut repository_view, url)
                .unwrap()
                .as_ref(),
        );
        let child = git_submodule_open(&mut submodule.as_mut()).unwrap();
        let owner_is_bare =
            crate::repository::git_repository_is_bare(git_submodule_owner(submodule.as_ref()));
        let child_is_bare = crate::repository::git_repository_is_bare(child.as_ref());
        LifecycleObservation {
            status,
            location,
            update: git_submodule_update_strategy(submodule.as_ref())
                .unwrap()
                .into(),
            ignore: git_submodule_ignore(submodule.as_ref()).unwrap().into(),
            recurse: git_submodule_fetch_recurse_submodules(submodule.as_ref())
                .unwrap()
                .into(),
            branch: git_submodule_branch(submodule.as_ref()).map(|value| value.to_bytes().to_vec()),
            resolved_url,
            owner_is_bare,
            child_is_bare,
            exists,
            foreach_names,
            foreach_stop,
        }
    }

    #[test]
    fn io_equiv_submodule_configuration_status_and_repository_lifecycle() {
        let _libgit2 = Libgit2Init::acquire();
        let child = HistoryFixture::new("submodule-lifecycle-child");
        let raw_parent = HistoryFixture::new("submodule-lifecycle-raw");
        let safe_parent = HistoryFixture::new("submodule-lifecycle-safe");
        let url = file_url(child.directory.path());
        unsafe { raw_add(raw_parent.repository.as_ptr(), &url) };
        let safe_repository = unsafe {
            crate::repository::GitRepositoryMut::from_ptr(safe_parent.repository.as_ptr())
        }
        .unwrap();
        safe_add(safe_repository, &url);
        let raw = unsafe { raw_lifecycle(raw_parent.repository.as_ptr(), &url) };
        assert_eq!(raw, safe_lifecycle(safe_parent.repository.as_ptr(), &url));
        assert!(raw.exists);
        assert_eq!(raw.branch.as_deref(), Some(b"topic".as_slice()));
    }

    fn advance_child(child: &HistoryFixture) -> ffi::git_oid {
        std::fs::write(
            child.directory.path().join("fetched-by-update.txt"),
            b"the update engine must fetch this object\n",
        )
        .unwrap();
        let add = std::process::Command::new("git")
            .current_dir(child.directory.path())
            .args(["add", "fetched-by-update.txt"])
            .status()
            .unwrap();
        assert!(add.success());
        let commit = std::process::Command::new("git")
            .current_dir(child.directory.path())
            .args([
                "-c",
                "user.name=Update Test",
                "-c",
                "user.email=update@example.com",
                "commit",
                "-m",
                "new submodule target",
            ])
            .env("GIT_AUTHOR_DATE", "1700090000 +0000")
            .env("GIT_COMMITTER_DATE", "1700090000 +0000")
            .status()
            .unwrap();
        assert!(commit.success());
        let output = std::process::Command::new("git")
            .current_dir(child.directory.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let oid = std::ffi::CString::new(String::from_utf8(output.stdout).unwrap().trim()).unwrap();
        let mut result = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_oid_fromstr(&mut result, oid.as_ptr()) },
            0
        );
        result
    }

    unsafe fn set_submodule_index_target(
        repository: *mut ffi::git_repository,
        target: &ffi::git_oid,
    ) {
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, repository) },
            0
        );
        let existing = unsafe { ffi::git_index_get_bypath(index, c"deps/child".as_ptr(), 0) };
        assert!(!existing.is_null());
        let mut entry = unsafe { existing.read() };
        entry.id = *target;
        assert_eq!(unsafe { ffi::git_index_add(index, &entry) }, 0);
        assert_eq!(unsafe { ffi::git_index_write(index) }, 0);
        unsafe { ffi::git_index_free(index) };
    }

    #[derive(Debug, Eq, PartialEq)]
    struct UpdateObservation {
        update_status: i32,
        workdir: Vec<u8>,
        head: Vec<u8>,
        file: Vec<u8>,
        status: ffi::git_submodule_status_t,
    }

    unsafe fn raw_update(repository: *mut ffi::git_repository) -> UpdateObservation {
        let mut submodule = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_submodule_lookup(&mut submodule, repository, c"deps/child".as_ptr())
            },
            0
        );
        let mut options = unsafe { core::mem::zeroed::<ffi::git_submodule_update_options>() };
        assert_eq!(
            unsafe {
                ffi::git_submodule_update_options_init(
                    &mut options,
                    ffi::GIT_SUBMODULE_UPDATE_OPTIONS_VERSION,
                )
            },
            0
        );
        options.checkout_opts.checkout_strategy = ffi::git_checkout_strategy_t_GIT_CHECKOUT_FORCE;
        let update_status = unsafe { ffi::git_submodule_update(submodule, 1, &mut options) };
        assert_eq!(update_status, 0);
        assert_eq!(unsafe { ffi::git_submodule_reload(submodule, 1) }, 0);
        let workdir = unsafe { (*ffi::git_submodule_wd_id(submodule)).id }.to_vec();
        let mut child = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_submodule_open(&mut child, submodule) }, 0);
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, child, c"HEAD".as_ptr()) },
            0
        );
        let path = unsafe { CStr::from_ptr(ffi::git_repository_workdir(child)) }
            .to_str()
            .unwrap();
        let file = std::fs::read(std::path::Path::new(path).join("fetched-by-update.txt")).unwrap();
        let mut status = 0;
        assert_eq!(
            unsafe {
                ffi::git_submodule_status(
                    &mut status,
                    repository,
                    c"deps/child".as_ptr(),
                    ffi::git_submodule_ignore_t_GIT_SUBMODULE_IGNORE_NONE,
                )
            },
            0
        );
        unsafe {
            ffi::git_repository_free(child);
            ffi::git_submodule_free(submodule);
        }
        UpdateObservation {
            update_status,
            workdir,
            head: head.id.to_vec(),
            file,
            status,
        }
    }

    fn safe_update(repository: *mut ffi::git_repository) -> UpdateObservation {
        let repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut submodule = git_submodule_lookup(repository_view, c"deps/child").unwrap();
        let mut options = crate::api::submodule::GitSubmoduleUpdateOptions::new();
        options
            .as_mut()
            .checkout_options_mut()
            .set_checkout_strategy(crate::api::checkout::GitCheckoutStrategy::FORCE);
        let update_status =
            git_submodule_update(&mut submodule.as_mut(), true, Some(options.as_ref()))
                .map_or_else(|error| error, |_| 0);
        assert_eq!(update_status, 0);
        git_submodule_reload(&mut submodule.as_mut(), true).unwrap();
        let workdir = git_submodule_wd_id(&mut submodule.as_mut())
            .unwrap()
            .raw_bytes()
            .elems()
            .collect();
        let child = git_submodule_open(&mut submodule.as_mut()).unwrap();
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, child.as_ptr(), c"HEAD".as_ptr()) },
            0
        );
        let path = crate::repository::git_repository_workdir(child.as_ref())
            .unwrap()
            .to_str()
            .unwrap();
        let file = std::fs::read(std::path::Path::new(path).join("fetched-by-update.txt")).unwrap();
        drop(child);
        drop(submodule);
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let status = git_submodule_status(
            &mut repository_view,
            c"deps/child",
            GitSubmoduleIgnore::None,
        )
        .unwrap()
        .bits();
        UpdateObservation {
            update_status,
            workdir,
            head: head.id.to_vec(),
            file,
            status,
        }
    }

    #[test]
    fn io_equiv_submodule_update_fetches_missing_index_commit_and_checks_it_out() {
        let _libgit2 = Libgit2Init::acquire();
        let child = HistoryFixture::new("submodule-update-child");
        let raw_parent = HistoryFixture::new("submodule-update-raw");
        let safe_parent = HistoryFixture::new("submodule-update-safe");
        let url = file_url(child.directory.path());
        unsafe {
            raw_add(raw_parent.repository.as_ptr(), &url);
        }
        let safe_repository = unsafe {
            crate::repository::GitRepositoryMut::from_ptr(safe_parent.repository.as_ptr())
        }
        .unwrap();
        safe_add(safe_repository, &url);
        let target = advance_child(&child);
        unsafe {
            set_submodule_index_target(raw_parent.repository.as_ptr(), &target);
            set_submodule_index_target(safe_parent.repository.as_ptr(), &target);
        }
        let raw = unsafe { raw_update(raw_parent.repository.as_ptr()) };
        assert_eq!(raw, safe_update(safe_parent.repository.as_ptr()));
        assert_eq!(raw.head, target.id.to_vec());
        assert_eq!(raw.workdir, target.id.to_vec());
    }
}

#[cfg(test)]
mod submodule_update_options_init_tests {
    use std::ffi::CString;

    use super::*;
    use crate::api::remote::GitRemoteUpdateFlags;
    use crate::api::submodule::GitSubmoduleUpdateOptions;
    use crate::remote::{GitFetchPrune, GitRemoteAutotagOption};

    /// Writes a non-default value into every reachable slot, including a
    /// borrowed string one nested subrecord must be able to forget.
    fn garble<'data>(
        options: &mut GitSubmoduleUpdateOptionsMut<'_, 'data>,
        directory: &'data CStr,
    ) {
        options.set_version(0);
        options.set_allow_fetch(false);
        {
            let mut checkout = options.checkout_options_mut();
            checkout.set_version(0);
            checkout.set_target_directory(Some(directory));
        }
        let mut fetch = options.fetch_options_mut();
        fetch.set_version(0);
        fetch.set_prune(GitFetchPrune::Prune);
        fetch.set_download_tags(GitRemoteAutotagOption::All);
        fetch.set_update_flags(GitRemoteUpdateFlags::REPORT_UNCHANGED);
        fetch.callbacks_mut().set_version(0);
        fetch.proxy_options_mut().set_version(0);
    }

    /// Asserts the whole record equals `GIT_SUBMODULE_UPDATE_OPTIONS_INIT`,
    /// including its nested checkout and fetch templates.
    fn assert_published_defaults(options: GitSubmoduleUpdateOptionsRef<'_, '_>) {
        assert_eq!(options.version(), ffi::GIT_SUBMODULE_UPDATE_OPTIONS_VERSION);
        assert!(options.allow_fetch());

        let checkout = options.checkout_options();
        assert_eq!(checkout.version(), ffi::GIT_CHECKOUT_OPTIONS_VERSION);
        assert_eq!(checkout.target_directory(), None);

        let fetch = options.fetch_options();
        assert_eq!(
            fetch.version(),
            ffi::GIT_FETCH_OPTIONS_VERSION as core::ffi::c_int
        );
        assert_eq!(fetch.prune(), Ok(GitFetchPrune::Unspecified));
        assert_eq!(
            fetch.download_tags(),
            Ok(GitRemoteAutotagOption::Unspecified)
        );
        assert_eq!(fetch.update_flags(), Ok(GitRemoteUpdateFlags::FETCH_HEAD));
        assert_eq!(
            fetch.callbacks().version(),
            ffi::GIT_REMOTE_CALLBACKS_VERSION
        );
        assert_eq!(
            fetch.proxy_options().version(),
            ffi::GIT_PROXY_OPTIONS_VERSION
        );
    }

    #[test]
    fn the_initializer_reinitializes_the_callers_own_record() {
        let _initialization = crate::libgit2::git_libgit2_init().unwrap();
        let directory = CString::new("/tmp/crustify-submodule").expect("a path without NUL");
        let mut options = GitSubmoduleUpdateOptions::new();
        garble(&mut options.as_mut(), &directory);

        git_submodule_update_options_init(
            &mut options.as_mut(),
            ffi::GIT_SUBMODULE_UPDATE_OPTIONS_VERSION,
        )
        .expect("the published version reinitializes the record in place");

        assert_published_defaults(options.as_ref());
        // The nested borrowed directory was cleared, never freed: the caller's
        // own string still owns its buffer and reads back unchanged.
        assert_eq!(directory.as_c_str(), c"/tmp/crustify-submodule");
    }

    #[test]
    fn the_rust_constructor_agrees_with_the_c_template() {
        let _initialization = crate::libgit2::git_libgit2_init().unwrap();
        let mut from_c = GitSubmoduleUpdateOptions::new();
        git_submodule_update_options_init(
            &mut from_c.as_mut(),
            ffi::GIT_SUBMODULE_UPDATE_OPTIONS_VERSION,
        )
        .expect("the published version initializes");

        // `GitSubmoduleUpdateOptions::new` hand-writes what the C template
        // contains, so the two must agree through every accessor.
        assert_published_defaults(from_c.as_ref());
        let from_rust = GitSubmoduleUpdateOptions::new();
        assert_published_defaults(from_rust.as_ref());
    }

    #[test]
    fn an_unsupported_version_is_refused_without_writing() {
        let _initialization = crate::libgit2::git_libgit2_init().unwrap();
        let directory = CString::new("/tmp/crustify-submodule").expect("a path without NUL");
        let mut options = GitSubmoduleUpdateOptions::new();
        garble(&mut options.as_mut(), &directory);

        for version in [0, ffi::GIT_SUBMODULE_UPDATE_OPTIONS_VERSION + 1] {
            assert_eq!(
                git_submodule_update_options_init(&mut options.as_mut(), version),
                Err(ffi::git_error_code_GIT_ERROR),
                "version {version} is outside the accepted compatibility range"
            );
        }

        // Refusal happens before the template is copied, so the caller keeps
        // every value it installed, nested subrecords included.
        let view = options.as_ref();
        assert_eq!(view.version(), 0);
        assert!(!view.allow_fetch());
        assert_eq!(view.checkout_options().version(), 0);
        assert_eq!(
            view.checkout_options().target_directory(),
            Some(directory.as_c_str())
        );
        assert_eq!(view.fetch_options().version(), 0);
    }

    #[test]
    fn the_initializer_takes_the_callers_storage_and_never_allocates_it() {
        // C's contract is "initialize the record I hand you". A wrapper that
        // returned fresh storage instead would make reinitialization of an
        // existing record unreachable from safe Rust.
        let wrapper: fn(
            &mut GitSubmoduleUpdateOptionsMut<'_, '_>,
            core::ffi::c_uint,
        ) -> Result<(), i32> = git_submodule_update_options_init;
        let _ = wrapper;
    }
}
