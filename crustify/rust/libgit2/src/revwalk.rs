//! Safe wrappers for libgit2 revwalk APIs.

use core::ffi::{CStr, c_void};
use core::marker::PhantomData;

use ffibox::CBox;

use crate::api::revwalk::GitRevwalkHideCallback;
pub use crate::api::revwalk::GitSortFlags;
use crate::ffi;
use crate::oid::{Oid, OidRef};
use crate::pathspec::GitPathspecRef;
use crate::repository::{GitRepositoryMut, GitRepositoryRef};

ffibox::define_ctype!(
    /// Wraps: git_revwalk
    /// An opaque revision walker managed by libgit2.
    ///
    /// A walker stores the repository supplied at construction without
    /// retaining a count on it, so that repository must outlive the walker;
    /// it does own one count on the repository's object database, released
    /// with the walker. Owned walker handles release their allocation with
    /// `git_revwalk_free`.
    ///
    GitRevwalk,
    GitRevwalkRef,
    GitRevwalkMut,
    ffi::git_revwalk
);

/// An owning walker whose stored repository pointer remains valid.
pub struct GitRevwalkOwned<'repo> {
    inner: CBox<GitRevwalk>,
    hide: Option<Box<Box<dyn GitRevwalkHideCallback>>>,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl GitRevwalkOwned<'_> {
    /// Borrows the walker.
    #[must_use]
    pub fn as_ref(&self) -> GitRevwalkRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the walker exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitRevwalkMut<'_> {
        self.inner.as_mut()
    }
}

// SAFETY: `git_revwalk_free` is the public destructor for a fully initialized
// walker. It disposes every walker-owned resource and frees the allocation;
// although the C function accepts null, `CBox` supplies a live non-null object
// exactly once.
ffibox::impl_dropped!(GitRevwalk, ffi::git_revwalk, ffi::git_revwalk_free);

/// Wraps: git_revwalk_hide
/// Hides a commit and its ancestors from the walk.
pub fn git_revwalk_hide(walk: &mut GitRevwalkMut<'_>, oid: OidRef<'_>) -> Result<(), i32> {
    // SAFETY: both typed handles are live for the call and the walker is exclusive.
    let status = unsafe { ffi::git_revwalk_hide(walk.as_mut_ptr(), oid.as_ptr()) };
    status_result(status)
}

/// Wraps: git_revwalk_hide_glob
/// Hides commits reachable from references matching `glob`.
pub fn git_revwalk_hide_glob(walk: &mut GitRevwalkMut<'_>, glob: &CStr) -> Result<(), i32> {
    // SAFETY: the exclusive walker and C string are live and neither is retained.
    let status = unsafe { ffi::git_revwalk_hide_glob(walk.as_mut_ptr(), glob.as_ptr()) };
    status_result(status)
}

/// Wraps: git_revwalk_hide_head
/// Hides the commit at the repository's `HEAD`.
pub fn git_revwalk_hide_head(walk: &mut GitRevwalkMut<'_>) -> Result<(), i32> {
    // SAFETY: the walker is live and exclusively borrowed for mutation.
    status_result(unsafe { ffi::git_revwalk_hide_head(walk.as_mut_ptr()) })
}

/// Wraps: git_revwalk_hide_ref
/// Hides commits reachable from `refname`.
pub fn git_revwalk_hide_ref(walk: &mut GitRevwalkMut<'_>, refname: &CStr) -> Result<(), i32> {
    // SAFETY: both arguments are live for the synchronous call and the walker is exclusive.
    status_result(unsafe { ffi::git_revwalk_hide_ref(walk.as_mut_ptr(), refname.as_ptr()) })
}

/// Wraps: git_revwalk_new
/// Creates a walker tied to the repository pointer it stores.
pub fn git_revwalk_new<'repo>(
    mut repository: GitRepositoryMut<'repo>,
) -> Result<GitRevwalkOwned<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and the repository is live and
    // exclusive. The result's lifetime keeps the stored repository pointer valid.
    let status = unsafe { ffi::git_revwalk_new(&mut output, repository.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete non-null walker allocation.
    let inner = unsafe { CBox::from_raw(output) }
        .expect("git_revwalk_new succeeded without returning a walker");
    Ok(GitRevwalkOwned {
        inner,
        hide: None,
        _repository: PhantomData,
    })
}

/// Wraps: git_revwalk_next
/// Returns the next object ID, or the libgit2 iteration/error code.
pub fn git_revwalk_next(walk: &mut GitRevwalkMut<'_>) -> Result<Oid, i32> {
    let mut oid = Oid::zeroed();
    // SAFETY: the raw place addresses live, correctly aligned OID storage and
    // the walker is exclusively borrowed for its state transition.
    let status = unsafe {
        ffi::git_revwalk_next(
            core::ptr::addr_of_mut!(oid).cast::<ffi::git_oid>(),
            walk.as_mut_ptr(),
        )
    };
    if status == 0 { Ok(oid) } else { Err(status) }
}

/// Wraps: git_revwalk_push
/// Adds a commit as a traversal root.
pub fn git_revwalk_push(walk: &mut GitRevwalkMut<'_>, oid: OidRef<'_>) -> Result<(), i32> {
    // SAFETY: both handles are live and the exclusive walker may be mutated.
    status_result(unsafe { ffi::git_revwalk_push(walk.as_mut_ptr(), oid.as_ptr()) })
}

/// Wraps: git_revwalk_push_glob
/// Pushes commits named by references matching `glob`.
pub fn git_revwalk_push_glob(walk: &mut GitRevwalkMut<'_>, glob: &CStr) -> Result<(), i32> {
    // SAFETY: the exclusive walker and C string are live for the call.
    status_result(unsafe { ffi::git_revwalk_push_glob(walk.as_mut_ptr(), glob.as_ptr()) })
}

/// Wraps: git_revwalk_push_head
/// Pushes the repository's `HEAD` commit.
pub fn git_revwalk_push_head(walk: &mut GitRevwalkMut<'_>) -> Result<(), i32> {
    // SAFETY: the walker is live and exclusively borrowed.
    status_result(unsafe { ffi::git_revwalk_push_head(walk.as_mut_ptr()) })
}

/// Wraps: git_revwalk_push_range
/// Hides and pushes the left and right endpoints of `range`, respectively.
pub fn git_revwalk_push_range(walk: &mut GitRevwalkMut<'_>, range: &CStr) -> Result<(), i32> {
    // SAFETY: the exclusive walker and C string are live for the call.
    status_result(unsafe { ffi::git_revwalk_push_range(walk.as_mut_ptr(), range.as_ptr()) })
}

/// Wraps: git_revwalk_push_ref
/// Pushes the commit named by `refname`.
pub fn git_revwalk_push_ref(walk: &mut GitRevwalkMut<'_>, refname: &CStr) -> Result<(), i32> {
    // SAFETY: the exclusive walker and C string are live for the call.
    status_result(unsafe { ffi::git_revwalk_push_ref(walk.as_mut_ptr(), refname.as_ptr()) })
}

/// Wraps: git_revwalk_reset
/// Clears all pushed and hidden commits for walker reuse.
pub fn git_revwalk_reset(walk: &mut GitRevwalkMut<'_>) -> Result<(), i32> {
    // SAFETY: the walker is live and exclusively borrowed for reset.
    status_result(unsafe { ffi::git_revwalk_reset(walk.as_mut_ptr()) })
}

/// Wraps: git_revwalk_simplify_first_parent
/// Restricts traversal to each commit's first parent.
pub fn git_revwalk_simplify_first_parent(walk: &mut GitRevwalkMut<'_>) -> Result<(), i32> {
    // SAFETY: the walker is live and exclusively borrowed for mutation.
    status_result(unsafe { ffi::git_revwalk_simplify_first_parent(walk.as_mut_ptr()) })
}

/// Wraps: git_revwalk_sorting
/// Replaces the walk's `GIT_SORT_*` bit set.
pub fn git_revwalk_sorting(
    walk: &mut GitRevwalkMut<'_>,
    sort_mode: GitSortFlags,
) -> Result<(), i32> {
    // SAFETY: the walker is live and exclusively borrowed; the C API accepts
    // the bit set as an unsigned scalar and retains no pointer.
    status_result(unsafe { ffi::git_revwalk_sorting(walk.as_mut_ptr(), sort_mode.bits()) })
}

fn status_result(status: i32) -> Result<(), i32> {
    if status == 0 { Ok(()) } else { Err(status) }
}

unsafe extern "C" fn hide_trampoline(oid: *const ffi::git_oid, payload: *mut c_void) -> i32 {
    if oid.is_null() || payload.is_null() {
        return 0;
    }
    // SAFETY: the payload points to the address-stable boxed trait object held
    // by the owning walker for as long as this callback remains registered.
    let callback = unsafe { &mut *payload.cast::<Box<dyn GitRevwalkHideCallback>>() };
    // SAFETY: libgit2 supplies a live transient commit ID for this invocation.
    let oid = unsafe { OidRef::from_ptr(oid.cast_mut()) }.expect("checked non-null");
    callback.call(oid)
}

/// Wraps: git_revwalk_add_hide_cb
/// Installs, replaces, or clears the callback retained by a revision walker.
pub fn git_revwalk_add_hide_cb(
    walk: &mut GitRevwalkOwned<'_>,
    callback: Option<Box<dyn GitRevwalkHideCallback>>,
) -> Result<(), i32> {
    let replacement = callback.map(Box::new);
    let function: ffi::git_revwalk_hide_cb = if replacement.is_some() {
        Some(hide_trampoline)
    } else {
        None
    };
    let payload = replacement.as_ref().map_or(core::ptr::null_mut(), |boxed| {
        core::ptr::from_ref::<Box<dyn GitRevwalkHideCallback>>(boxed.as_ref())
            .cast_mut()
            .cast()
    });
    let mut handle = walk.inner.as_mut();
    // SAFETY: the walker is exclusive and `payload` is null or points into the
    // address-stable replacement box. Libgit2 stores it only on success.
    let status = unsafe { ffi::git_revwalk_add_hide_cb(handle.as_mut_ptr(), function, payload) };
    if status == 0 {
        walk.hide = replacement;
        Ok(())
    } else {
        Err(status)
    }
}

/// Wraps: git_revwalk_pathspec
/// Installs a compiled pathspec used by subsequent traversal.
///
/// # Safety
///
/// `pathspec` must remain live and unmodified until `walk` is reset, freed, or
/// assigned another pathspec. Libgit2 stores the pointer without acquiring an
/// ownership count, and a borrowed walker handle cannot express that retained
/// lifetime in its type.
pub unsafe fn git_revwalk_pathspec(
    walk: &mut GitRevwalkMut<'_>,
    pathspec: GitPathspecRef<'_>,
) -> Result<(), i32> {
    // SAFETY: the caller provides the retained lifetime contract; both typed
    // handles are live for the call and the walker is exclusive.
    let status =
        unsafe { ffi::git_revwalk_pathspec(walk.as_mut_ptr(), pathspec.as_ptr().cast_mut()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_revwalk_repository
/// Borrows the repository retained by a revision walker.
#[must_use]
pub fn git_revwalk_repository<'a>(walk: GitRevwalkRef<'a>) -> GitRepositoryRef<'a> {
    // SAFETY: `walk` is live and C returns its required non-null repository.
    let repository = unsafe { ffi::git_revwalk_repository(walk.as_ptr().cast_mut()) };
    // SAFETY: the repository is required to remain live for the walker.
    unsafe { GitRepositoryRef::from_ptr(repository) }
        .expect("a valid revision walker has a repository")
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_revwalk_preserves_the_c_seam_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitRevwalk>();
        assert_dropped::<GitRevwalk>();
        assert_eq!(size_of::<GitRevwalk>(), size_of::<ffi::git_revwalk>());
        assert_eq!(align_of::<GitRevwalk>(), align_of::<ffi::git_revwalk>());
        assert_eq!(
            size_of::<GitRevwalkRef<'_>>(),
            size_of::<*const ffi::git_revwalk>()
        );
        assert_eq!(
            size_of::<GitRevwalkMut<'_>>(),
            size_of::<*mut ffi::git_revwalk>()
        );
        assert_eq!(
            size_of::<GitRevwalkOwned<'_>>(),
            size_of::<*mut ffi::git_revwalk>() * 2
        );
    }

    #[test]
    fn null_seams_create_no_revwalk_handles() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitRevwalkRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRevwalkMut::from_ptr(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn constructor_tethers_a_reusable_walker_to_its_repository() {
        // SAFETY: process-global initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut repository = crate::repository::git_repository_open(c"../../..").unwrap();
        let mut walker = git_revwalk_new(repository.as_mut()).unwrap();
        assert_eq!(
            git_revwalk_sorting(&mut walker.as_mut(), GitSortFlags::NONE),
            Ok(())
        );
        assert_eq!(
            git_revwalk_simplify_first_parent(&mut walker.as_mut()),
            Ok(())
        );
        assert_eq!(git_revwalk_reset(&mut walker.as_mut()), Ok(()));
        drop(walker);
        drop(repository);
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
