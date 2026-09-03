//! Safe wrappers for libgit2 reset APIs.

use crate::annotated_commit::AnnotatedCommitRef;
use crate::api::checkout::GitCheckoutOptionsRef;
use crate::ffi;
use crate::object::GitObjectRef;
use crate::repository::GitRepositoryMut;
use crate::strarray::GitStrArrayRef;

/// Wraps: git_reset_t
/// Selects which repository state a reset updates.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResetType {
    /// Move `HEAD` to the target commit without changing the index or worktree.
    Soft = ffi::git_reset_t_GIT_RESET_SOFT,
    /// Perform a soft reset and replace the index with the target tree.
    Mixed = ffi::git_reset_t_GIT_RESET_MIXED,
    /// Perform a mixed reset and replace tracked worktree files from the index.
    Hard = ffi::git_reset_t_GIT_RESET_HARD,
}

/// An integer that is not a published [`ResetType`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidResetType(ffi::git_reset_t);

impl InvalidResetType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_reset_t {
        self.0
    }
}

impl From<ResetType> for ffi::git_reset_t {
    fn from(reset_type: ResetType) -> Self {
        reset_type as Self
    }
}

impl TryFrom<ffi::git_reset_t> for ResetType {
    type Error = InvalidResetType;

    fn try_from(reset_type: ffi::git_reset_t) -> Result<Self, Self::Error> {
        match reset_type {
            ffi::git_reset_t_GIT_RESET_SOFT => Ok(Self::Soft),
            ffi::git_reset_t_GIT_RESET_MIXED => Ok(Self::Mixed),
            ffi::git_reset_t_GIT_RESET_HARD => Ok(Self::Hard),
            value => Err(InvalidResetType(value)),
        }
    }
}

/// Wraps: git_reset_default
/// Resets the selected index paths to `target`, or removes them with `None`.
pub fn git_reset_default(
    repository: &mut GitRepositoryMut<'_>,
    target: Option<GitObjectRef<'_>>,
    pathspecs: GitStrArrayRef<'_>,
) -> Result<(), i32> {
    if pathspecs.count() == 0 {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    let target = target.map_or(core::ptr::null(), |object| object.as_ptr());
    // SAFETY: the repository is exclusively borrowed, `target` is null or a
    // live object, and the nonempty pathspec array is live for the call. The
    // function retains none of these pointers.
    let status =
        unsafe { ffi::git_reset_default(repository.as_mut_ptr(), target, pathspecs.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_reset
/// Moves `HEAD` to `target` and updates repository state selected by
/// `reset_type`.
pub fn git_reset(
    repository: &mut GitRepositoryMut<'_>,
    target: GitObjectRef<'_>,
    reset_type: ResetType,
    checkout_options: Option<GitCheckoutOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let checkout_options = checkout_options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository is exclusively borrowed; `target` and the
    // optional checkout options remain live for the synchronous operation.
    // Libgit2 retains none of these pointers.
    let status = unsafe {
        ffi::git_reset(
            repository.as_mut_ptr(),
            target.as_ptr(),
            reset_type.into(),
            checkout_options,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn reset_types_round_trip_through_the_c_type() {
        for reset_type in [ResetType::Soft, ResetType::Mixed, ResetType::Hard] {
            let raw = ffi::git_reset_t::from(reset_type);
            assert_eq!(ResetType::try_from(raw), Ok(reset_type));
        }
    }

    #[test]
    fn invalid_reset_type_is_rejected_without_constructing_an_enum() {
        let invalid = ffi::git_reset_t_GIT_RESET_HARD + 1;
        let error = ResetType::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn reset_type_matches_the_c_abi_scalar() {
        assert_eq!(size_of::<ResetType>(), size_of::<ffi::git_reset_t>());
        assert_eq!(align_of::<ResetType>(), align_of::<ffi::git_reset_t>());
    }

    #[test]
    fn default_reset_rejects_an_empty_pathspec_before_ffi() {
        let repository = Box::new(core::mem::MaybeUninit::<ffi::git_repository>::zeroed());
        let raw = Box::into_raw(repository).cast::<ffi::git_repository>();
        {
            // SAFETY: `raw` addresses live opaque storage and remains exclusively
            // owned for this handle. The wrapper rejects the empty pathspec before
            // the C implementation can inspect repository state.
            let mut repository = unsafe { GitRepositoryMut::from_ptr(raw) }.unwrap();
            let pathspecs = crate::strarray::GitStrArray::new();
            assert_eq!(
                git_reset_default(&mut repository, None, pathspecs.as_ref()),
                Err(ffi::git_error_code_GIT_EINVALID)
            );
        }
        // SAFETY: no handle remains and this recovers the exact allocation.
        drop(unsafe { Box::from_raw(raw.cast::<core::mem::MaybeUninit<ffi::git_repository>>()) });
    }

    #[test]
    fn annotated_reset_exposes_only_typed_borrows() {
        let _: fn(
            &mut GitRepositoryMut<'_>,
            AnnotatedCommitRef<'_>,
            ResetType,
            Option<GitCheckoutOptionsRef<'_, '_>>,
        ) -> Result<(), i32> = git_reset_from_annotated;
    }
}

/// Wraps: git_reset_from_annotated
/// Moves `HEAD` to an annotated commit and updates the selected repository
/// state.
pub fn git_reset_from_annotated(
    repository: &mut GitRepositoryMut<'_>,
    commit: AnnotatedCommitRef<'_>,
    reset_type: ResetType,
    checkout_options: Option<GitCheckoutOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let checkout_options = checkout_options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository is exclusively borrowed; the annotated commit
    // and optional checkout options remain live for the synchronous reset,
    // and libgit2 retains none of the pointers.
    let status = unsafe {
        ffi::git_reset_from_annotated(
            repository.as_mut_ptr(),
            commit.as_ptr(),
            reset_type.into(),
            checkout_options,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod annotated_reset_tests {
    use core::ptr::{addr_of, addr_of_mut};

    use super::*;
    use crate::api::commit::{GitCommitCreateOptions, GitCommitCreateOptionsRef};
    use crate::oid::{Oid, OidMut, OidRef, OidType};
    use crate::repository::{GitRepositoryInitFlags, GitRepositoryOwned};

    /// Holds one libgit2 initialization count for the duration of a test.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and reference
            // counted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the initialization this guard represents,
            // after every libgit2 owner in the test has been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// Commits the (empty) stage and returns the object ID that was written.
    fn commit(
        repository: &mut GitRepositoryOwned,
        options: GitCommitCreateOptionsRef<'_>,
        message: &core::ffi::CStr,
    ) -> Oid {
        let mut created = Oid::zeroed();
        {
            // SAFETY: `created` is live, initialized, exclusively borrowed
            // local storage for the whole life of this handle.
            let mut out = unsafe { OidMut::from_ptr(addr_of_mut!(created).cast()) }
                .expect("the address of a local value is non-null");
            crate::commit::git_commit_create_from_stage(
                &mut out,
                &mut repository.as_mut(),
                message,
                Some(options),
            )
            .expect("the commit is created");
        }
        created
    }

    /// Resolves `HEAD` to the object ID it currently names.
    fn head_id(repository: &mut GitRepositoryOwned) -> Oid {
        crate::refs::git_reference_name_to_id(&mut repository.as_mut(), c"HEAD")
            .expect("HEAD resolves to an object")
    }

    /// Borrows a local object-ID value.
    fn borrow(id: &Oid) -> OidRef<'_> {
        // SAFETY: `id` is live, initialized storage the caller keeps alive for
        // the returned handle, and this borrow is the only path to it.
        unsafe { OidRef::from_ptr(addr_of!(*id).cast_mut().cast()) }
            .expect("the address of a local value is non-null")
    }

    /// Walks `HEAD` back one commit through the annotated-commit entry point.
    ///
    /// The composition is the point of the test as much as the reset is: an
    /// annotated commit tethers itself to the repository it was looked up in,
    /// and the reset takes that same repository exclusively, so the two only
    /// compose because the constructor borrows a transient
    /// `&mut GitRepositoryMut<'repo>` rather than consuming the handle.
    #[test]
    fn a_soft_reset_moves_head_to_the_annotated_commit() {
        let _libgit2 = Libgit2Init::acquire();

        let directory = std::env::temp_dir().join(format!(
            "crustify-reset-from-annotated-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let path = std::ffi::CString::new(
            directory
                .to_str()
                .expect("a temporary directory path is UTF-8"),
        )
        .expect("a temporary directory path holds no interior NUL");

        // A SHA-256 repository sidesteps the bundled SHA1DC collision
        // detector, whose unaligned 32-bit loads trip the C build's UBSan on
        // every object it hashes.
        let mut init_options = crate::repository::git_repository_init_options_init(1)
            .expect("the current init-options version");
        init_options.as_mut().set_oid_type(Some(OidType::Sha256));
        init_options
            .as_mut()
            .set_flags(GitRepositoryInitFlags::MKPATH | GitRepositoryInitFlags::BARE);
        let mut repository =
            crate::repository::git_repository_init_ext(&path, &mut init_options.as_mut())
                .expect("a fresh directory initializes as a bare repository");

        let signature = crate::signature::git_signature_now(c"Crustify", c"crustify@example.com")
            .expect("a signature stamped with the current time");
        let mut options = GitCommitCreateOptions::new();
        // Both commits carry the same empty tree, so the creating form needs
        // the empty-commit gate opened.
        options.as_mut().set_allow_empty_commit(true);
        // SAFETY: the signature owner outlives the options and every call
        // that reads them below.
        unsafe {
            options
                .as_mut()
                .set_borrowed_author(Some(signature.as_ref()));
            options
                .as_mut()
                .set_borrowed_committer(Some(signature.as_ref()));
        }

        let first = commit(&mut repository, options.as_ref(), c"first");
        let second = commit(&mut repository, options.as_ref(), c"second");
        assert!(!crate::oid::git_oid_equal(borrow(&first), borrow(&second)));
        assert!(crate::oid::git_oid_equal(
            borrow(&head_id(&mut repository)),
            borrow(&second)
        ));

        {
            let mut handle = repository.as_mut();
            let annotated =
                crate::annotated_commit::git_annotated_commit_lookup(&mut handle, borrow(&first))
                    .expect("the first commit is readable by ID");
            assert!(crate::oid::git_oid_equal(
                crate::annotated_commit::git_annotated_commit_id(annotated.as_ref())
                    .expect("a real annotated commit carries an ID"),
                borrow(&first)
            ));
            git_reset_from_annotated(&mut handle, annotated.as_ref(), ResetType::Soft, None)
                .expect("a soft reset to an ancestor succeeds");
        }

        assert!(crate::oid::git_oid_equal(
            borrow(&head_id(&mut repository)),
            borrow(&first)
        ));

        drop(repository);
        let _ = std::fs::remove_dir_all(&directory);
    }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use core::ffi::CStr;

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct ResetObservation {
        head: Vec<u8>,
        readme: Vec<u8>,
        status: Vec<u8>,
    }

    fn stage_readme(fixture: &HistoryFixture) {
        std::fs::write(
            fixture.directory.path().join("README.md"),
            b"staged reset-default contents\n",
        )
        .unwrap();
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args(["add", "README.md"])
            .status()
            .unwrap();
        assert!(status.success());
    }

    unsafe fn resolve(repository: *mut ffi::git_repository, spec: &CStr) -> *mut ffi::git_object {
        let mut object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut object, repository, spec.as_ptr()) },
            0
        );
        object
    }

    fn observe(fixture: &HistoryFixture) -> ResetObservation {
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut head,
                    fixture.repository.as_ptr(),
                    c"HEAD".as_ptr(),
                )
            },
            0
        );
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args(["status", "--porcelain=v1"])
            .output()
            .unwrap();
        assert!(status.status.success());
        ResetObservation {
            head: head.id.to_vec(),
            readme: std::fs::read(fixture.directory.path().join("README.md")).unwrap(),
            status: status.stdout,
        }
    }

    unsafe fn raw_reset(fixture: &HistoryFixture) -> ResetObservation {
        let repository = fixture.repository.as_ptr();
        let original = unsafe { resolve(repository, c"HEAD") };
        let prior = unsafe { resolve(repository, c"HEAD~1") };
        let older = unsafe { resolve(repository, c"HEAD~2") };
        assert_eq!(
            unsafe {
                ffi::git_reset(
                    repository,
                    prior,
                    ffi::git_reset_t_GIT_RESET_SOFT,
                    core::ptr::null(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_reset(
                    repository,
                    older,
                    ffi::git_reset_t_GIT_RESET_MIXED,
                    core::ptr::null(),
                )
            },
            0
        );
        std::fs::write(
            fixture.directory.path().join("README.md"),
            b"discard this hard-reset dirt\n",
        )
        .unwrap();
        let mut checkout = unsafe { core::mem::zeroed::<ffi::git_checkout_options>() };
        assert_eq!(
            unsafe {
                ffi::git_checkout_options_init(&mut checkout, ffi::GIT_CHECKOUT_OPTIONS_VERSION)
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_reset(
                    repository,
                    original,
                    ffi::git_reset_t_GIT_RESET_HARD,
                    &checkout,
                )
            },
            0
        );

        stage_readme(fixture);
        let mut path = c"README.md".as_ptr().cast_mut();
        let paths = ffi::git_strarray {
            strings: core::ptr::from_mut(&mut path),
            count: 1,
        };
        assert_eq!(
            unsafe { ffi::git_reset_default(repository, prior, &paths) },
            0
        );

        let mut annotated = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_annotated_commit_lookup(
                    &mut annotated,
                    repository,
                    ffi::git_object_id(older),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_reset_from_annotated(
                    repository,
                    annotated,
                    ffi::git_reset_t_GIT_RESET_SOFT,
                    core::ptr::null(),
                )
            },
            0
        );
        unsafe {
            ffi::git_annotated_commit_free(annotated);
            ffi::git_object_free(older);
            ffi::git_object_free(prior);
            ffi::git_object_free(original);
        }
        observe(fixture)
    }

    fn safe_reset(fixture: &HistoryFixture) -> ResetObservation {
        let repository = fixture.repository.as_ptr();
        let original = unsafe { resolve(repository, c"HEAD") };
        let prior = unsafe { resolve(repository, c"HEAD~1") };
        let older = unsafe { resolve(repository, c"HEAD~2") };
        let original_ref = unsafe { GitObjectRef::from_ptr(original) }.unwrap();
        let prior_ref = unsafe { GitObjectRef::from_ptr(prior) }.unwrap();
        let older_ref = unsafe { GitObjectRef::from_ptr(older) }.unwrap();
        let mut repository_view = unsafe { GitRepositoryMut::from_ptr(repository) }.unwrap();
        git_reset(&mut repository_view, prior_ref, ResetType::Soft, None).unwrap();
        git_reset(&mut repository_view, older_ref, ResetType::Mixed, None).unwrap();
        std::fs::write(
            fixture.directory.path().join("README.md"),
            b"discard this hard-reset dirt\n",
        )
        .unwrap();
        let checkout = crate::api::checkout::GitCheckoutOptions::new();
        git_reset(
            &mut repository_view,
            original_ref,
            ResetType::Hard,
            Some(checkout.as_ref()),
        )
        .unwrap();

        stage_readme(fixture);
        let mut path = c"README.md".as_ptr().cast_mut();
        let mut raw_paths = ffi::git_strarray {
            strings: core::ptr::from_mut(&mut path),
            count: 1,
        };
        let paths = unsafe { GitStrArrayRef::from_ptr(&raw mut raw_paths) }.unwrap();
        git_reset_default(&mut repository_view, Some(prior_ref), paths).unwrap();

        let older_id =
            unsafe { crate::oid::OidRef::from_ptr(ffi::git_object_id(older).cast_mut()) }.unwrap();
        let annotated =
            crate::annotated_commit::git_annotated_commit_lookup(&mut repository_view, older_id)
                .unwrap();
        git_reset_from_annotated(
            &mut repository_view,
            annotated.as_ref(),
            ResetType::Soft,
            None,
        )
        .unwrap();
        drop(annotated);
        unsafe {
            ffi::git_object_free(older);
            ffi::git_object_free(prior);
            ffi::git_object_free(original);
        }
        observe(fixture)
    }

    #[test]
    fn io_equiv_soft_mixed_hard_default_and_annotated_reset() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("reset-raw");
        let safe = HistoryFixture::new("reset-safe");
        let raw = unsafe { raw_reset(&raw) };
        assert_eq!(raw, safe_reset(&safe));
        assert!(!raw.status.is_empty());
    }
}
