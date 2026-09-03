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
/// C also accepts a null pathspec, which installs nothing and leaves any
/// previously installed one in place rather than clearing it. The wrapper
/// requires the pathspec, so it never spells that no-op.
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
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    unsafe fn raw_walk(repository: *mut ffi::git_repository) -> Vec<Vec<u8>> {
        let mut walk = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_revwalk_new(&mut walk, repository) }, 0);
        assert_eq!(unsafe { ffi::git_revwalk_push_head(walk) }, 0);
        let mut ids = Vec::new();
        loop {
            let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
            match unsafe { ffi::git_revwalk_next(&mut id, walk) } {
                0 => ids.push(id.id.to_vec()),
                ffi::git_error_code_GIT_ITEROVER => break,
                error => panic!("raw revision walk failed: {error}"),
            }
        }
        unsafe { ffi::git_revwalk_free(walk) };
        ids
    }

    fn safe_walk(repository: crate::repository::GitRepositoryMut<'_>) -> Vec<Vec<u8>> {
        let mut walk = git_revwalk_new(repository).unwrap();
        git_revwalk_push_head(&mut walk.as_mut()).unwrap();
        let mut ids = Vec::new();
        loop {
            match git_revwalk_next(&mut walk.as_mut()) {
                Ok(mut id) => {
                    let id =
                        unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(id).cast()) }
                            .unwrap();
                    ids.push(id.raw_bytes().elems().collect());
                }
                Err(ffi::git_error_code_GIT_ITEROVER) => break,
                Err(error) => panic!("safe revision walk failed: {error}"),
            }
        }
        ids
    }

    unsafe fn raw_revisions(repository: *mut ffi::git_repository) -> Vec<Vec<u8>> {
        [c"HEAD", c"HEAD~1", c"HEAD~2", c"v1.0^{commit}"]
            .into_iter()
            .map(|spec| {
                let mut object = core::ptr::null_mut();
                assert_eq!(
                    unsafe { ffi::git_revparse_single(&mut object, repository, spec.as_ptr()) },
                    0
                );
                let id = unsafe { ffi::git_object_id(object) };
                let bytes = unsafe { (*id).id }.to_vec();
                unsafe { ffi::git_object_free(object) };
                bytes
            })
            .collect()
    }

    fn safe_revisions(repository: *mut ffi::git_repository) -> Vec<Vec<u8>> {
        [c"HEAD", c"HEAD~1", c"HEAD~2", c"v1.0^{commit}"]
            .into_iter()
            .map(|spec| {
                let view =
                    unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
                let object = crate::revparse::git_revparse_single(view, spec).unwrap();
                crate::object::git_object_id(object.as_ref())
                    .raw_bytes()
                    .elems()
                    .collect()
            })
            .collect()
    }

    #[test]
    fn io_equiv_revision_walk_and_parse() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("revwalk-raw");
        let safe = HistoryFixture::new("revwalk-safe");
        let raw_walk = unsafe { raw_walk(raw.repository.as_ptr()) };
        let raw_revisions = unsafe { raw_revisions(raw.repository.as_ptr()) };
        assert_eq!(raw_revisions, safe_revisions(safe.repository.as_ptr()));
        let safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(raw_walk, safe_walk(safe_repository));
        assert_eq!(raw_walk.len(), 3);
    }

    fn expand_graph(fixture: &HistoryFixture) {
        let path = fixture.directory.path();
        let run = |arguments: &[&str], tick: usize| {
            let status = std::process::Command::new("git")
                .current_dir(path)
                .args(arguments)
                .env("GIT_AUTHOR_DATE", format!("17002{tick:05} +0000"))
                .env("GIT_COMMITTER_DATE", format!("17002{tick:05} +0000"))
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {arguments:?}");
        };
        for revision in 0..14 {
            std::fs::write(
                path.join("series.txt"),
                format!("main revision {revision}\n"),
            )
            .unwrap();
            run(&["add", "series.txt"], revision);
            run(
                &[
                    "-c",
                    "user.name=Crustify",
                    "-c",
                    "user.email=crustify@example.com",
                    "commit",
                    "-q",
                    "-m",
                    &format!("main {revision}"),
                ],
                revision,
            );
        }
        run(&["checkout", "-q", "-b", "walk-side", "HEAD~5"], 30);
        for revision in 0..3 {
            std::fs::write(path.join("side.txt"), format!("side revision {revision}\n")).unwrap();
            run(&["add", "side.txt"], 31 + revision);
            run(
                &[
                    "-c",
                    "user.name=Crustify",
                    "-c",
                    "user.email=crustify@example.com",
                    "commit",
                    "-q",
                    "-m",
                    &format!("side {revision}"),
                ],
                31 + revision,
            );
        }
        run(&["checkout", "-q", "master"], 40);
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "merge",
                "-q",
                "--no-ff",
                "-m",
                "merge walk side",
                "walk-side",
            ],
            41,
        );
    }

    unsafe fn raw_drain(walk: *mut ffi::git_revwalk) -> Vec<Vec<u8>> {
        let mut output = Vec::new();
        loop {
            let mut oid = unsafe { core::mem::zeroed::<ffi::git_oid>() };
            match unsafe { ffi::git_revwalk_next(&mut oid, walk) } {
                0 => output.push(oid.id.to_vec()),
                ffi::git_error_code_GIT_ITEROVER => break,
                error => panic!("revision walk failed: {error}"),
            }
        }
        output
    }

    unsafe fn raw_modes(repository: *mut ffi::git_repository) -> (Vec<Vec<Vec<u8>>>, usize, bool) {
        unsafe extern "C" fn visible(
            _: *const ffi::git_oid,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            unsafe { *payload.cast::<usize>() += 1 };
            0
        }
        let mut walk = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_revwalk_new(&mut walk, repository) }, 0);
        let owner_matches = unsafe { ffi::git_revwalk_repository(walk) == repository };
        let mut variants = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_revwalk_sorting(
                    walk,
                    ffi::git_sort_t_GIT_SORT_TOPOLOGICAL | ffi::git_sort_t_GIT_SORT_TIME,
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_revwalk_push_glob(walk, c"refs/heads/*".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_revwalk_hide_ref(walk, c"refs/heads/walk-side".as_ptr()) },
            0
        );
        variants.push(unsafe { raw_drain(walk) });
        assert_eq!(unsafe { ffi::git_revwalk_reset(walk) }, 0);
        assert_eq!(
            unsafe {
                ffi::git_revwalk_sorting(
                    walk,
                    ffi::git_sort_t_GIT_SORT_REVERSE | ffi::git_sort_t_GIT_SORT_TIME,
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_revwalk_push_range(walk, c"HEAD~10..HEAD".as_ptr()) },
            0
        );
        variants.push(unsafe { raw_drain(walk) });
        assert_eq!(unsafe { ffi::git_revwalk_reset(walk) }, 0);
        assert_eq!(
            unsafe { ffi::git_revwalk_push_ref(walk, c"refs/heads/master".as_ptr()) },
            0
        );
        let mut hidden_object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut hidden_object, repository, c"HEAD~5".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_revwalk_hide(walk, ffi::git_object_id(hidden_object)) },
            0
        );
        unsafe { ffi::git_object_free(hidden_object) };
        variants.push(unsafe { raw_drain(walk) });
        assert_eq!(unsafe { ffi::git_revwalk_reset(walk) }, 0);
        assert_eq!(unsafe { ffi::git_revwalk_push_head(walk) }, 0);
        assert_eq!(unsafe { ffi::git_revwalk_simplify_first_parent(walk) }, 0);
        variants.push(unsafe { raw_drain(walk) });
        assert_eq!(unsafe { ffi::git_revwalk_reset(walk) }, 0);
        let mut callbacks = 0usize;
        assert_eq!(
            unsafe {
                ffi::git_revwalk_add_hide_cb(
                    walk,
                    Some(visible),
                    core::ptr::from_mut(&mut callbacks).cast(),
                )
            },
            0
        );
        assert_eq!(unsafe { ffi::git_revwalk_push_head(walk) }, 0);
        variants.push(unsafe { raw_drain(walk) });
        unsafe { ffi::git_revwalk_free(walk) };
        (variants, callbacks, owner_matches)
    }

    fn safe_drain(walk: &mut GitRevwalkOwned<'_>) -> Vec<Vec<u8>> {
        let mut output = Vec::new();
        loop {
            match git_revwalk_next(&mut walk.as_mut()) {
                Ok(mut oid) => output.push(
                    unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(oid).cast()) }
                        .unwrap()
                        .raw_bytes()
                        .elems()
                        .collect(),
                ),
                Err(ffi::git_error_code_GIT_ITEROVER) => break,
                Err(error) => panic!("revision walk failed: {error}"),
            }
        }
        output
    }

    fn safe_modes(repository: *mut ffi::git_repository) -> (Vec<Vec<Vec<u8>>>, usize, bool) {
        let repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut walk = git_revwalk_new(repository_view).unwrap();
        let owner_matches = git_revwalk_repository(walk.as_ref()).as_ptr() == repository;
        let mut variants = Vec::new();
        git_revwalk_sorting(
            &mut walk.as_mut(),
            GitSortFlags::TOPOLOGICAL | GitSortFlags::TIME,
        )
        .unwrap();
        git_revwalk_push_glob(&mut walk.as_mut(), c"refs/heads/*").unwrap();
        git_revwalk_hide_ref(&mut walk.as_mut(), c"refs/heads/walk-side").unwrap();
        variants.push(safe_drain(&mut walk));
        git_revwalk_reset(&mut walk.as_mut()).unwrap();
        git_revwalk_sorting(
            &mut walk.as_mut(),
            GitSortFlags::REVERSE | GitSortFlags::TIME,
        )
        .unwrap();
        git_revwalk_push_range(&mut walk.as_mut(), c"HEAD~10..HEAD").unwrap();
        variants.push(safe_drain(&mut walk));
        git_revwalk_reset(&mut walk.as_mut()).unwrap();
        git_revwalk_push_ref(&mut walk.as_mut(), c"refs/heads/master").unwrap();
        let mut hidden_object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut hidden_object, repository, c"HEAD~5".as_ptr()) },
            0
        );
        let hidden =
            unsafe { crate::oid::OidRef::from_ptr(ffi::git_object_id(hidden_object).cast_mut()) }
                .unwrap();
        git_revwalk_hide(&mut walk.as_mut(), hidden).unwrap();
        unsafe { ffi::git_object_free(hidden_object) };
        variants.push(safe_drain(&mut walk));
        git_revwalk_reset(&mut walk.as_mut()).unwrap();
        git_revwalk_push_head(&mut walk.as_mut()).unwrap();
        git_revwalk_simplify_first_parent(&mut walk.as_mut()).unwrap();
        variants.push(safe_drain(&mut walk));
        git_revwalk_reset(&mut walk.as_mut()).unwrap();
        let callback_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let callback_counter = callback_count.clone();
        git_revwalk_add_hide_cb(
            &mut walk,
            Some(Box::new(move |_: crate::oid::OidRef<'_>| {
                callback_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                0
            })),
        )
        .unwrap();
        git_revwalk_push_head(&mut walk.as_mut()).unwrap();
        variants.push(safe_drain(&mut walk));
        let callbacks = callback_count.load(std::sync::atomic::Ordering::Relaxed);
        (variants, callbacks, owner_matches)
    }

    #[test]
    fn io_equiv_revwalk_sort_push_hide_reset_callback_and_first_parent_modes() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("revwalk-modes-raw");
        let safe = HistoryFixture::new("revwalk-modes-safe");
        expand_graph(&raw);
        expand_graph(&safe);
        let raw_observation = unsafe { raw_modes(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_modes(safe.repository.as_ptr()));
        assert!(raw_observation.2);
        assert!(raw_observation.1 > 0);
    }
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
