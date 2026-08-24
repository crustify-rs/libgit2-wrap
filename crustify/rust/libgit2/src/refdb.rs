//! Safe wrappers for libgit2 refdb APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CDropped, define_ctype};

use crate::ffi;

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
