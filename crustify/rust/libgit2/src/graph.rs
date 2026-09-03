//! Safe wrappers for libgit2 graph APIs.

use crate::ffi;
use crate::oid::OidRef;
use crate::repository::{GitRepositoryMut, GitRepositoryRef};

/// Wraps: git_graph_ahead_behind
/// Counts commits unique to the local and upstream histories.
pub fn git_graph_ahead_behind(
    repository: GitRepositoryRef<'_>,
    local: OidRef<'_>,
    upstream: OidRef<'_>,
) -> Result<(usize, usize), i32> {
    let (mut ahead, mut behind) = (0, 0);
    // SAFETY: both outputs are writable and all three borrowed handles remain
    // live for this non-retaining graph walk.
    let status = unsafe {
        ffi::git_graph_ahead_behind(
            &mut ahead,
            &mut behind,
            repository.as_ptr().cast_mut(),
            local.as_ptr(),
            upstream.as_ptr(),
        )
    };
    if status == 0 {
        Ok((ahead, behind))
    } else {
        Err(status)
    }
}

/// Wraps: git_graph_descendant_of
/// Tests strict ancestry; a commit is not considered its own descendant.
pub fn git_graph_descendant_of(
    repository: GitRepositoryRef<'_>,
    commit: OidRef<'_>,
    ancestor: OidRef<'_>,
) -> Result<bool, i32> {
    // SAFETY: all borrowed handles remain live and C retains none of them.
    let status = unsafe {
        ffi::git_graph_descendant_of(
            repository.as_ptr().cast_mut(),
            commit.as_ptr(),
            ancestor.as_ptr(),
        )
    };
    match status {
        0 => Ok(false),
        1 => Ok(true),
        error => Err(error),
    }
}

/// Wraps: git_graph_reachable_from_any
/// Reports whether `commit` is reachable from any descendant ID.
///
/// The repository is taken exclusively because the walk writes through it:
/// `git_revwalk_new` reaches `git_repository_odb__weakptr`, which builds an
/// object database on first use and installs it in `repo->_odb`.
pub fn git_graph_reachable_from_any(
    repository: &mut GitRepositoryMut<'_>,
    commit: OidRef<'_>,
    descendants: ffibox::CSlice<'_, crate::oid::Oid>,
) -> Result<bool, i32> {
    // SAFETY: `Oid` is layout-compatible with `git_oid`; the repository is
    // exclusive for the object-database install, the object IDs are readable
    // for their supplied counts, and the walk retains none of them.
    let status = unsafe {
        ffi::git_graph_reachable_from_any(
            repository.as_mut_ptr(),
            commit.as_ptr(),
            descendants.as_ptr().cast_const(),
            descendants.len(),
        )
    };
    match status {
        0 => Ok(false),
        1 => Ok(true),
        error => Err(error),
    }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct GraphObservation {
        ahead_behind: (usize, usize),
        descendant: bool,
        reachable: bool,
    }

    fn write_commit_graph(fixture: &HistoryFixture) {
        let status = std::process::Command::new("git")
            .args([
                "-C",
                fixture.directory.path().to_str().unwrap(),
                "commit-graph",
                "write",
                "--reachable",
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }

    unsafe fn raw_graph(path: &core::ffi::CStr) -> GraphObservation {
        let mut repository = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_open(&mut repository, path.as_ptr()) },
            0
        );
        let mut head: ffi::git_oid = unsafe { core::mem::zeroed() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut object, repository, c"HEAD~2".as_ptr()) },
            0
        );
        let ancestor = unsafe { *ffi::git_object_id(object) };
        let (mut ahead, mut behind) = (0, 0);
        assert_eq!(
            unsafe {
                ffi::git_graph_ahead_behind(&mut ahead, &mut behind, repository, &head, &ancestor)
            },
            0
        );
        let descendant = unsafe { ffi::git_graph_descendant_of(repository, &head, &ancestor) } != 0;
        let descendants = [head];
        let reachable = unsafe {
            ffi::git_graph_reachable_from_any(repository, &ancestor, descendants.as_ptr(), 1)
        } != 0;
        unsafe { ffi::git_object_free(object) };
        unsafe { ffi::git_repository_free(repository) };
        GraphObservation {
            ahead_behind: (ahead, behind),
            descendant,
            reachable,
        }
    }

    fn safe_graph(path: &core::ffi::CStr) -> GraphObservation {
        let mut repository = crate::repository::git_repository_open(path).unwrap();
        let mut head =
            crate::refs::git_reference_name_to_id(&mut repository.as_mut(), c"HEAD").unwrap();
        let mut object = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_revparse_single(&mut object, repository.as_ptr(), c"HEAD~2".as_ptr())
            },
            0
        );
        let mut ancestor = unsafe { *ffi::git_object_id(object) };
        let head_ref =
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(head).cast()) }.unwrap();
        let ancestor_ref =
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(ancestor).cast()) }
                .unwrap();
        let ahead_behind =
            git_graph_ahead_behind(repository.as_ref(), head_ref, ancestor_ref).unwrap();
        let descendant =
            git_graph_descendant_of(repository.as_ref(), head_ref, ancestor_ref).unwrap();
        let ids = [head];
        let descendants = unsafe {
            ffibox::CSlice::from_raw_parts(
                core::ptr::NonNull::new(ids.as_ptr().cast_mut()).unwrap(),
                ids.len(),
            )
        };
        let reachable =
            git_graph_reachable_from_any(&mut repository.as_mut(), ancestor_ref, descendants)
                .unwrap();
        unsafe { ffi::git_object_free(object) };
        GraphObservation {
            ahead_behind,
            descendant,
            reachable,
        }
    }

    #[test]
    fn io_equiv_commit_graph_accelerated_queries() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("commit-graph-raw");
        let safe = HistoryFixture::new("commit-graph-safe");
        write_commit_graph(&raw);
        write_commit_graph(&safe);
        let raw_path = raw.directory.c_path();
        let safe_path = safe.directory.c_path();
        let raw_observation = unsafe { raw_graph(&raw_path) };
        assert_eq!(raw_observation, safe_graph(&safe_path));
        assert_eq!(raw_observation.ahead_behind, (2, 0));
    }
}

#[cfg(test)]
mod scheduled_graph_tests {
    use core::ptr::{NonNull, addr_of};

    use super::*;
    use crate::oid::Oid;

    /// Borrows an owned array of object IDs as the counted run C reads.
    fn descendants(ids: &[Oid]) -> ffibox::CSlice<'_, Oid> {
        let ptr = addr_of!(*ids).cast::<Oid>().cast_mut();
        // SAFETY: `ids` is a live Rust-owned run of exactly `ids.len()`
        // initialized, layout-compatible object IDs, and the returned view
        // borrows it for the slice's own lifetime.
        unsafe { ffibox::CSlice::from_raw_parts(NonNull::new(ptr).unwrap(), ids.len()) }
    }

    #[test]
    fn reachability_walks_the_repository_it_borrows_exclusively() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after every repository-derived owner below has been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        // This crate lives inside the repository under test, so its own
        // history supplies a commit and one of its ancestors.
        let mut repository = crate::repository::git_repository_open(c"../../..")
            .expect("the crate is checked out inside a repository");
        let head = crate::refs::git_reference_name_to_id(&mut repository.as_mut(), c"HEAD")
            .expect("HEAD resolves to a commit");
        let parent = {
            let object = crate::revparse::git_revparse_single(repository.as_mut(), c"HEAD~1")
                .expect("HEAD has at least one ancestor");
            crate::oid::git_oid_cpy(crate::object::git_object_id(object.as_ref()))
        };

        // SAFETY: `head` is a live initialized layout-compatible stack value
        // borrowed only by this handle.
        let head_ref = unsafe { crate::oid::OidRef::from_ptr(addr_of!(head).cast_mut().cast()) }
            .expect("the address of a stack value is non-null");
        // SAFETY: as above, for the independent `parent` value.
        let parent_ref =
            unsafe { crate::oid::OidRef::from_ptr(addr_of!(parent).cast_mut().cast()) }
                .expect("the address of a stack value is non-null");

        // The ancestor is reachable from the descendant, but not the reverse.
        // Both walks open the repository's object database, which is why the
        // wrapper holds the repository exclusively.
        let heads = [head];
        let parents = [parent];
        assert_eq!(
            git_graph_reachable_from_any(&mut repository.as_mut(), parent_ref, descendants(&heads)),
            Ok(true)
        );
        assert_eq!(
            git_graph_reachable_from_any(&mut repository.as_mut(), head_ref, descendants(&parents)),
            Ok(false)
        );
        // An empty descendant set short-circuits before any walk.
        assert_eq!(
            git_graph_reachable_from_any(&mut repository.as_mut(), head_ref, descendants(&[])),
            Ok(false)
        );

        drop(repository);
        // SAFETY: balances the successful initialization above, after the one
        // repository owner this test created has been released.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
