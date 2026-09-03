//! Safe wrappers for libgit2 refdb APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CDropped, define_ctype};

use crate::ffi;
use crate::repository::{GitRepositoryMut, GitRepositoryRefdbOwned};

define_ctype!(
    /// Wraps: git_refdb
    /// An opaque, refcounted reference database managed by libgit2.
    ///
    /// [`GitRefdbOwned`] represents one owned reference count. Libgit2 does
    /// not publish a standalone up-reference operation, so owners are not
    /// cloneable. A refdb retains a borrowed pointer to its repository;
    /// wrappers that construct one must keep that repository alive.
    GitRefdb,
    GitRefdbRef,
    GitRefdbMut,
    ffi::git_refdb
);

/// One owned reference count to a [`GitRefdb`].
pub type GitRefdbOwned = CBox<GitRefdb>;

/// Wraps: git_refdb_free
// SAFETY: `git_refdb_free` releases exactly one reference count from a fully
// constructed `git_refdb` and destroys its backend and allocation only when
// that was the final count. It accepts null, although `CBox` supplies a live,
// non-null allocation. `GitRefdb` is transparent over the matching C type.
unsafe impl CDropped for GitRefdb {
    unsafe fn c_drop(object: NonNull<Self>) {
        // SAFETY: the trait contract supplies one live owned refdb count and
        // the wrapper is transparent over `ffi::git_refdb`.
        unsafe { ffi::git_refdb_free(object.as_ptr().cast()) }
    }
}

/// Wraps: git_refdb_t
/// The storage backend selected for a reference database.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum GitRefdbType {
    /// Store loose and packed references in files.
    Files = ffi::git_refdb_t_GIT_REFDB_FILES,
    /// Store references in reftable files.
    Reftable = ffi::git_refdb_t_GIT_REFDB_REFTABLE,
}

/// A C value that is not a published [`GitRefdbType`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitRefdbType(ffi::git_refdb_t);

impl InvalidGitRefdbType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_refdb_t {
        self.0
    }
}

impl From<GitRefdbType> for ffi::git_refdb_t {
    fn from(value: GitRefdbType) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_refdb_t> for GitRefdbType {
    type Error = InvalidGitRefdbType;

    fn try_from(value: ffi::git_refdb_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_refdb_t_GIT_REFDB_FILES => Ok(Self::Files),
            ffi::git_refdb_t_GIT_REFDB_REFTABLE => Ok(Self::Reftable),
            other => Err(InvalidGitRefdbType(other)),
        }
    }
}

/// Wraps: git_refdb_compress
/// Asks the selected reference backend to compact its storage.
pub fn git_refdb_compress(refdb: &mut GitRefdbMut<'_>) -> Result<(), i32> {
    // SAFETY: the exclusive handle provides live backend state for the call;
    // the backend retains no new pointer to the handle.
    let status = unsafe { ffi::git_refdb_compress(refdb.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_refdb_new
/// Creates an empty reference database tied to its repository.
pub fn git_refdb_new<'repo>(
    repository: &'repo mut GitRepositoryMut<'_>,
) -> Result<GitRepositoryRefdbOwned<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: `output` is writable and the exclusive repository is live. On
    // success C transfers one refdb count that borrows this repository.
    let status = unsafe { ffi::git_refdb_new(&mut output, repository.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one non-null fully initialized owned refdb.
    let inner = unsafe { GitRefdbOwned::from_raw(output) }
        .expect("git_refdb_new succeeded without a refdb");
    Ok(GitRepositoryRefdbOwned::from_inner(inner))
}

/// Wraps: git_refdb_open
/// Opens the repository's configured reference database and default backend.
pub fn git_refdb_open<'repo>(
    repository: &'repo mut GitRepositoryMut<'_>,
) -> Result<GitRepositoryRefdbOwned<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: `output` is writable and the exclusive repository is live. On
    // success C transfers one refdb count that borrows this repository.
    let status = unsafe { ffi::git_refdb_open(&mut output, repository.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one non-null fully initialized owned refdb.
    let inner = unsafe { GitRefdbOwned::from_raw(output) }
        .expect("git_refdb_open succeeded without a refdb");
    Ok(GitRepositoryRefdbOwned::from_inner(inner))
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn refdb_has_the_opaque_c_layout_and_lifecycle_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitRefdb>();
        assert_dropped::<GitRefdb>();
        // `struct git_refdb` is defined in the private `src/libgit2/refdb.h`,
        // so the binding is an opaque marker: the reference count and the
        // backend pointer stay unreachable from Rust.
        assert_eq!(size_of::<ffi::git_refdb>(), 0);
        assert_eq!(size_of::<GitRefdb>(), size_of::<ffi::git_refdb>());
        assert_eq!(align_of::<GitRefdb>(), align_of::<ffi::git_refdb>());
        assert_eq!(
            size_of::<GitRefdbRef<'_>>(),
            size_of::<*const ffi::git_refdb>()
        );
        assert_eq!(
            size_of::<GitRefdbMut<'_>>(),
            size_of::<*mut ffi::git_refdb>()
        );
    }

    #[test]
    fn null_refdb_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitRefdbRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRefdbMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRefdbOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn refdb_types_round_trip_and_reject_unknown_values() {
        for value in [GitRefdbType::Files, GitRefdbType::Reftable] {
            let raw = ffi::git_refdb_t::from(value);
            assert_eq!(GitRefdbType::try_from(raw), Ok(value));
        }

        let unknown = ffi::git_refdb_t_GIT_REFDB_REFTABLE + 1;
        let error = GitRefdbType::try_from(unknown).unwrap_err();
        assert_eq!(error.value(), unknown);
    }

    #[test]
    fn refdb_type_matches_the_c_abi_scalar() {
        assert_eq!(size_of::<GitRefdbType>(), size_of::<ffi::git_refdb_t>());
        assert_eq!(align_of::<GitRefdbType>(), align_of::<ffi::git_refdb_t>());
    }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use std::ffi::{CStr, CString};

    use super::*;
    use crate::api::types::GitObjectType;
    use crate::io_equiv_support::{Libgit2Init, TempDir};
    use crate::oid::RAW_DIGEST_LEN;

    #[derive(Debug, Eq, PartialEq)]
    struct ReftableObservation {
        names: Vec<Vec<u8>>,
        main_target: Vec<u8>,
        alias_target: Vec<u8>,
    }

    unsafe fn raw_observation(path: &CStr) -> ReftableObservation {
        let mut options: ffi::git_repository_init_options = unsafe { core::mem::zeroed() };
        assert_eq!(
            unsafe { ffi::git_repository_init_options_init(&mut options, 1) },
            0
        );
        options.refdb_type = ffi::git_refdb_t_GIT_REFDB_REFTABLE;

        let mut repository = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_init_ext(&mut repository, path.as_ptr(), &mut options) },
            0
        );
        assert!(!repository.is_null());

        let mut odb = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_repository_odb(&mut odb, repository) }, 0);
        let mut first: ffi::git_oid = unsafe { core::mem::zeroed() };
        let mut second: ffi::git_oid = unsafe { core::mem::zeroed() };
        assert_eq!(
            unsafe {
                ffi::git_odb_write(
                    &mut first,
                    odb,
                    c"first object".as_ptr().cast(),
                    c"first object".to_bytes().len(),
                    ffi::git_object_t_GIT_OBJECT_BLOB,
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_odb_write(
                    &mut second,
                    odb,
                    c"second object".as_ptr().cast(),
                    c"second object".to_bytes().len(),
                    ffi::git_object_t_GIT_OBJECT_BLOB,
                )
            },
            0
        );
        unsafe { ffi::git_odb_free(odb) };

        let mut reference = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reference_create(
                    &mut reference,
                    repository,
                    c"refs/heads/one".as_ptr(),
                    &first,
                    0,
                    core::ptr::null(),
                )
            },
            0
        );
        let mut renamed = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reference_rename(
                    &mut renamed,
                    reference,
                    c"refs/heads/main".as_ptr(),
                    0,
                    core::ptr::null(),
                )
            },
            0
        );
        unsafe { ffi::git_reference_free(reference) };
        reference = core::ptr::null_mut();
        let mut updated = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reference_set_target(&mut updated, renamed, &second, core::ptr::null())
            },
            0
        );
        unsafe { ffi::git_reference_free(renamed) };
        unsafe { ffi::git_reference_free(updated) };

        assert_eq!(
            unsafe {
                ffi::git_reference_symbolic_create(
                    &mut reference,
                    repository,
                    c"refs/heads/alias".as_ptr(),
                    c"refs/heads/one".as_ptr(),
                    0,
                    core::ptr::null(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_reference_symbolic_set_target(
                    &mut updated,
                    reference,
                    c"refs/heads/main".as_ptr(),
                    core::ptr::null(),
                )
            },
            0
        );
        unsafe { ffi::git_reference_free(reference) };
        unsafe { ffi::git_reference_free(updated) };

        assert_eq!(
            unsafe {
                ffi::git_reference_create(
                    &mut reference,
                    repository,
                    c"refs/tags/transient".as_ptr(),
                    &first,
                    0,
                    core::ptr::null(),
                )
            },
            0
        );
        assert_eq!(unsafe { ffi::git_reference_delete(reference) }, 0);
        unsafe { ffi::git_reference_free(reference) };
        assert_eq!(
            unsafe {
                ffi::git_reference_create(
                    &mut reference,
                    repository,
                    c"refs/heads/remove".as_ptr(),
                    &first,
                    0,
                    core::ptr::null(),
                )
            },
            0
        );
        unsafe { ffi::git_reference_free(reference) };
        assert_eq!(
            unsafe { ffi::git_reference_remove(repository, c"refs/heads/remove".as_ptr()) },
            0
        );

        let mut refdb = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_refdb(&mut refdb, repository) },
            0
        );
        assert_eq!(unsafe { ffi::git_refdb_compress(refdb) }, 0);
        unsafe { ffi::git_refdb_free(refdb) };

        let mut names = ffi::git_strarray {
            strings: core::ptr::null_mut(),
            count: 0,
        };
        assert_eq!(
            unsafe { ffi::git_reference_list(&mut names, repository) },
            0
        );
        let mut observed_names = Vec::with_capacity(names.count);
        for index in 0..names.count {
            let name = unsafe { *names.strings.add(index) };
            observed_names.push(unsafe { CStr::from_ptr(name) }.to_bytes().to_vec());
        }
        unsafe { ffi::git_strarray_dispose(&mut names) };
        observed_names.sort();

        assert_eq!(
            unsafe {
                ffi::git_reference_lookup(&mut reference, repository, c"refs/heads/main".as_ptr())
            },
            0
        );
        let target = unsafe { ffi::git_reference_target(reference) };
        let main_target =
            unsafe { core::slice::from_raw_parts((*target).id.as_ptr(), RAW_DIGEST_LEN).to_vec() };
        unsafe { ffi::git_reference_free(reference) };

        assert_eq!(
            unsafe {
                ffi::git_reference_lookup(&mut reference, repository, c"refs/heads/alias".as_ptr())
            },
            0
        );
        let alias = unsafe { ffi::git_reference_symbolic_target(reference) };
        let alias_target = unsafe { CStr::from_ptr(alias) }.to_bytes().to_vec();
        unsafe { ffi::git_reference_free(reference) };
        unsafe { ffi::git_repository_free(repository) };

        ReftableObservation {
            names: observed_names,
            main_target,
            alias_target,
        }
    }

    fn safe_observation(path: &CStr) -> ReftableObservation {
        let mut options = crate::repository::git_repository_init_options_init(1).unwrap();
        options
            .as_mut()
            .set_refdb_type(Some(GitRefdbType::Reftable));
        let mut repository =
            crate::repository::git_repository_init_ext(path, &mut options.as_mut()).unwrap();

        let (mut first, mut second) = {
            let odb = crate::repository::git_repository_odb(&mut repository.as_mut()).unwrap();
            (
                crate::odb::git_odb_write(odb.as_ref(), b"first object", GitObjectType::BLOB)
                    .unwrap(),
                crate::odb::git_odb_write(odb.as_ref(), b"second object", GitObjectType::BLOB)
                    .unwrap(),
            )
        };

        {
            let mut repository_view = repository.as_mut();
            let mut one = crate::refs::git_reference_create(
                &mut repository_view,
                c"refs/heads/one",
                unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(first).cast()) }
                    .unwrap(),
                false,
                None,
            )
            .unwrap();
            let mut one_view = one.as_mut();
            let mut main =
                crate::refs::git_reference_rename(&mut one_view, c"refs/heads/main", false, None)
                    .unwrap();
            let mut main_view = main.as_mut();
            let _updated = crate::refs::git_reference_set_target(
                &mut main_view,
                unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(second).cast()) }
                    .unwrap(),
                None,
            )
            .unwrap();
        }

        {
            let mut repository_view = repository.as_mut();
            let mut alias = crate::refs::git_reference_symbolic_create(
                &mut repository_view,
                c"refs/heads/alias",
                c"refs/heads/one",
                false,
                None,
            )
            .unwrap();
            let mut alias_view = alias.as_mut();
            let _updated = crate::refs::git_reference_symbolic_set_target(
                &mut alias_view,
                c"refs/heads/main",
                None,
            )
            .unwrap();
        }

        {
            let mut repository_view = repository.as_mut();
            let mut transient = crate::refs::git_reference_create(
                &mut repository_view,
                c"refs/tags/transient",
                unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(first).cast()) }
                    .unwrap(),
                false,
                None,
            )
            .unwrap();
            let mut transient_view = transient.as_mut();
            crate::refs::git_reference_delete(&mut transient_view).unwrap();
        }
        {
            let mut repository_view = repository.as_mut();
            let _remove = crate::refs::git_reference_create(
                &mut repository_view,
                c"refs/heads/remove",
                unsafe { crate::oid::OidRef::from_ptr(core::ptr::addr_of_mut!(first).cast()) }
                    .unwrap(),
                false,
                None,
            )
            .unwrap();
        }
        crate::refs::git_reference_remove(&mut repository.as_mut(), c"refs/heads/remove").unwrap();

        {
            let mut repository_view = repository.as_mut();
            let mut refdb = crate::repository::git_repository_refdb(&mut repository_view).unwrap();
            git_refdb_compress(&mut refdb.as_mut()).unwrap();
        }

        let names = crate::refs::git_reference_list(&mut repository.as_mut()).unwrap();
        let strings = names.as_ref().strings().unwrap();
        let mut observed_names = (0..strings.len())
            .map(|index| strings.get(index).unwrap().to_bytes().to_vec())
            .collect::<Vec<_>>();
        observed_names.sort();

        let main_target = {
            let mut repository_view = repository.as_mut();
            let main = crate::refs::git_reference_lookup(&mut repository_view, c"refs/heads/main")
                .unwrap();
            crate::refs::git_reference_target(main.as_ref())
                .unwrap()
                .raw_bytes()
                .elems()
                .collect()
        };
        let alias_target = {
            let mut repository_view = repository.as_mut();
            let alias =
                crate::refs::git_reference_lookup(&mut repository_view, c"refs/heads/alias")
                    .unwrap();
            crate::refs::git_reference_symbolic_target(alias.as_ref())
                .unwrap()
                .to_bytes()
                .to_vec()
        };

        ReftableObservation {
            names: observed_names,
            main_target,
            alias_target,
        }
    }

    unsafe fn raw_reftable_stack(path: &CStr) -> (usize, Vec<u8>, Vec<u8>) {
        let mut options = unsafe { core::mem::zeroed::<ffi::git_repository_init_options>() };
        assert_eq!(
            unsafe { ffi::git_repository_init_options_init(&mut options, 1) },
            0
        );
        options.refdb_type = ffi::git_refdb_t_GIT_REFDB_REFTABLE;
        let mut repository = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_init_ext(&mut repository, path.as_ptr(), &mut options) },
            0
        );
        let mut odb = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_repository_odb(&mut odb, repository) }, 0);
        let mut ids = [unsafe { core::mem::zeroed::<ffi::git_oid>() }; 2];
        for (id, contents) in ids
            .iter_mut()
            .zip([b"transaction first".as_slice(), b"transaction second"])
        {
            assert_eq!(
                unsafe {
                    ffi::git_odb_write(
                        id,
                        odb,
                        contents.as_ptr().cast(),
                        contents.len(),
                        ffi::git_object_t_GIT_OBJECT_BLOB,
                    )
                },
                0
            );
        }
        unsafe { ffi::git_odb_free(odb) };

        let names = (0..48)
            .map(|index| CString::new(format!("refs/heads/generated-{index:02}")).unwrap())
            .collect::<Vec<_>>();
        for name in &names {
            let mut reference = core::ptr::null_mut();
            assert_eq!(
                unsafe {
                    ffi::git_reference_create(
                        &mut reference,
                        repository,
                        name.as_ptr(),
                        &ids[0],
                        0,
                        core::ptr::null(),
                    )
                },
                0
            );
            unsafe { ffi::git_reference_free(reference) };
        }
        for round in 0..12 {
            for name in &names {
                let mut reference = core::ptr::null_mut();
                assert_eq!(
                    unsafe {
                        ffi::git_reference_create(
                            &mut reference,
                            repository,
                            name.as_ptr(),
                            &ids[round % 2],
                            1,
                            c"generated transaction update".as_ptr(),
                        )
                    },
                    0
                );
                unsafe { ffi::git_reference_free(reference) };
            }
        }

        let mut reference = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reference_symbolic_create(
                    &mut reference,
                    repository,
                    c"refs/heads/alias".as_ptr(),
                    names[0].as_ptr(),
                    0,
                    c"generated symbolic ref".as_ptr(),
                )
            },
            0
        );
        unsafe { ffi::git_reference_free(reference) };
        assert_eq!(
            unsafe { ffi::git_reference_remove(repository, names[47].as_ptr()) },
            0
        );

        let mut refdb = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_refdb(&mut refdb, repository) },
            0
        );
        assert_eq!(unsafe { ffi::git_refdb_compress(refdb) }, 0);
        unsafe { ffi::git_refdb_free(refdb) };
        let mut listed = unsafe { core::mem::zeroed::<ffi::git_strarray>() };
        assert_eq!(
            unsafe { ffi::git_reference_list(&mut listed, repository) },
            0
        );
        let count = listed.count;
        unsafe { ffi::git_strarray_dispose(&mut listed) };
        let mut reference = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_reference_lookup(&mut reference, repository, names[0].as_ptr()) },
            0
        );
        let target = unsafe { (*ffi::git_reference_target(reference)).id }.to_vec();
        unsafe { ffi::git_reference_free(reference) };
        reference = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_reference_lookup(&mut reference, repository, c"refs/heads/alias".as_ptr())
            },
            0
        );
        let alias = unsafe { CStr::from_ptr(ffi::git_reference_symbolic_target(reference)) }
            .to_bytes()
            .to_vec();
        unsafe {
            ffi::git_reference_free(reference);
            ffi::git_repository_free(repository);
        }
        (count, target, alias)
    }

    fn safe_reftable_stack(path: &CStr) -> (usize, Vec<u8>, Vec<u8>) {
        let mut options = crate::repository::git_repository_init_options_init(1).unwrap();
        options
            .as_mut()
            .set_refdb_type(Some(GitRefdbType::Reftable));
        let mut repository =
            crate::repository::git_repository_init_ext(path, &mut options.as_mut()).unwrap();
        let mut ids = {
            let odb = crate::repository::git_repository_odb(&mut repository.as_mut()).unwrap();
            [
                crate::odb::git_odb_write(odb.as_ref(), b"transaction first", GitObjectType::BLOB)
                    .unwrap(),
                crate::odb::git_odb_write(odb.as_ref(), b"transaction second", GitObjectType::BLOB)
                    .unwrap(),
            ]
        };
        let names = (0..48)
            .map(|index| CString::new(format!("refs/heads/generated-{index:02}")).unwrap())
            .collect::<Vec<_>>();
        let first =
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::from_mut(&mut ids[0]).cast()) }
                .unwrap();
        for name in &names {
            let _reference = crate::refs::git_reference_create(
                &mut repository.as_mut(),
                name,
                first,
                false,
                None,
            )
            .unwrap();
        }
        for round in 0..12 {
            let id = unsafe {
                crate::oid::OidRef::from_ptr(core::ptr::from_mut(&mut ids[round % 2]).cast())
            }
            .unwrap();
            for name in &names {
                let _reference = crate::refs::git_reference_create(
                    &mut repository.as_mut(),
                    name,
                    id,
                    true,
                    Some(c"generated transaction update"),
                )
                .unwrap();
            }
        }
        {
            let _alias = crate::refs::git_reference_symbolic_create(
                &mut repository.as_mut(),
                c"refs/heads/alias",
                &names[0],
                false,
                Some(c"generated symbolic ref"),
            )
            .unwrap();
        }
        crate::refs::git_reference_remove(&mut repository.as_mut(), &names[47]).unwrap();
        {
            let mut repository_view = repository.as_mut();
            let mut refdb = crate::repository::git_repository_refdb(&mut repository_view).unwrap();
            git_refdb_compress(&mut refdb.as_mut()).unwrap();
        }
        let count = crate::refs::git_reference_list(&mut repository.as_mut())
            .unwrap()
            .as_ref()
            .count();
        let target = {
            let mut repository_view = repository.as_mut();
            let reference =
                crate::refs::git_reference_lookup(&mut repository_view, &names[0]).unwrap();
            crate::refs::git_reference_target(reference.as_ref())
                .unwrap()
                .raw_bytes()
                .elems()
                .collect()
        };
        let alias = {
            let mut repository_view = repository.as_mut();
            let reference =
                crate::refs::git_reference_lookup(&mut repository_view, c"refs/heads/alias")
                    .unwrap();
            crate::refs::git_reference_symbolic_target(reference.as_ref())
                .unwrap()
                .to_bytes()
                .to_vec()
        };
        (count, target, alias)
    }

    #[test]
    fn io_equiv_reftable_reference_mutations() {
        let _libgit2 = Libgit2Init::acquire();
        let raw_directory = TempDir::new("reftable-raw");
        let safe_directory = TempDir::new("reftable-safe");
        let raw_path = CString::new(raw_directory.path().to_str().unwrap()).unwrap();
        let safe_path = CString::new(safe_directory.path().to_str().unwrap()).unwrap();

        let raw = unsafe { raw_observation(&raw_path) };
        assert_eq!(raw, safe_observation(&safe_path));
        assert_eq!(
            raw.names,
            [b"refs/heads/alias".to_vec(), b"refs/heads/main".to_vec()]
        );
        assert_eq!(raw.alias_target, b"refs/heads/main");
    }

    #[test]
    fn io_equiv_reftable_stack_updates_deletion_and_compaction() {
        let _libgit2 = Libgit2Init::acquire();
        let raw_directory = TempDir::new("reftable-transactions-raw");
        let safe_directory = TempDir::new("reftable-transactions-safe");
        let raw_path = CString::new(raw_directory.path().to_str().unwrap()).unwrap();
        let safe_path = CString::new(safe_directory.path().to_str().unwrap()).unwrap();
        let raw = unsafe { raw_reftable_stack(&raw_path) };
        let safe = safe_reftable_stack(&safe_path);
        assert_eq!(raw, safe);
        assert_eq!(raw.0, 48);
        assert_eq!(raw.2, b"refs/heads/generated-00");
    }
}
