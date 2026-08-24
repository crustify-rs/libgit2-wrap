//! Safe wrappers for libgit2 revparse APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

pub use crate::api::revparse::{GitRevspecFlags, InvalidGitRevspecFlags};
use crate::ffi;
use crate::object::{GitObjectOwned, GitObjectRef, RepositoryObject, adopt_repository_object};
use crate::refs::{GitReferenceTetheredOwned, adopt_optional_reference};
use crate::repository::{GitRepositoryMut, GitRepositoryRef};

ffibox::define_ctype!(
    /// Wraps: git_revspec
    /// A caller-allocated revision parse result that owns its object results.
    ///
    /// The result is **single-use**: `git_revparse` begins by `memset`ting the
    /// whole struct, so it overwrites both object fields without releasing
    /// what they held. A result that already carries objects must therefore be
    /// emptied before it is parsed into again — with
    /// [`GitRevspecMut::take_from`] / [`GitRevspecMut::take_to`] to keep the
    /// objects, or `set_from(None)` / `set_to(None)` to release them —
    /// otherwise those references leak.
    GitRevspec,
    GitRevspecRef,
    GitRevspecMut,
    ffi::git_revspec
);

/// A by-value revision parse result whose object fields are released on drop.
pub type GitRevspecOwned = CVal<GitRevspec>;

impl GitRevspec {
    /// Constructs an empty result ready for `git_revparse` to fill once.
    ///
    /// See the type documentation for why a filled result must be emptied
    /// before it is reused.
    #[must_use]
    pub fn new() -> GitRevspecOwned {
        // The C empty representation consists of null pointers and zero flags.
        CVal::new(Self::zeroed())
    }
}

impl<'a> GitRevspecRef<'a> {
    /// Field: git_revspec.flags
    /// Returns the validated parse-intent flags.
    pub fn flags(&self) -> Result<GitRevspecFlags, InvalidGitRevspecFlags> {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        let bits = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitRevspecFlags::try_from(bits)
    }

    /// Field: git_revspec.from
    /// Borrows the owned left-hand object, when one is present.
    #[must_use]
    pub fn from(&self) -> Option<GitObjectRef<'a>> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the result header.
        let object = unsafe { addr_of!((*self.as_ptr()).from).read() };
        // SAFETY: a non-null object is live and owned by this result, so this
        // shared view remains valid for the handle's lifetime.
        unsafe { GitObjectRef::from_ptr(object) }
    }

    /// Field: git_revspec.to
    /// Borrows the owned right-hand object, when the expression is a range.
    #[must_use]
    pub fn to(&self) -> Option<GitObjectRef<'a>> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the result header.
        let object = unsafe { addr_of!((*self.as_ptr()).to).read() };
        // SAFETY: as `from`, for the independently owned right-hand object.
        unsafe { GitObjectRef::from_ptr(object) }
    }
}

impl GitRevspecMut<'_> {
    /// Sets the parse-intent flags.
    pub fn set_flags(&mut self, flags: GitRevspecFlags) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Moves the left-hand object out, leaving that field empty.
    #[must_use]
    pub fn take_from(&mut self) -> Option<GitObjectOwned> {
        let result = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits reading and clearing the
        // owned field as one transfer, leaving a valid null representation.
        let object = unsafe {
            let object = addr_of!((*result).from).read();
            addr_of_mut!((*result).from).write(core::ptr::null_mut());
            object
        };
        // SAFETY: a non-null pointer moved out of a valid revspec carries one
        // independently owned libgit2 object reference.
        unsafe { GitObjectOwned::from_raw(object) }
    }

    /// Replaces the left-hand object and releases the previous one.
    pub fn set_from(&mut self, object: Option<GitObjectOwned>) {
        let object = object.map_or(core::ptr::null_mut(), GitObjectOwned::into_raw);
        let old = self.take_from();
        // SAFETY: the exclusive handle permits installing the transferred
        // compatible object reference after the old field was cleared.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).from).write(object) }
        drop(old);
    }

    /// Moves the right-hand object out, leaving that field empty.
    #[must_use]
    pub fn take_to(&mut self) -> Option<GitObjectOwned> {
        let result = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits reading and clearing the
        // independently owned field as one transfer.
        let object = unsafe {
            let object = addr_of!((*result).to).read();
            addr_of_mut!((*result).to).write(core::ptr::null_mut());
            object
        };
        // SAFETY: a non-null pointer moved out of a valid revspec carries one
        // independently owned libgit2 object reference.
        unsafe { GitObjectOwned::from_raw(object) }
    }

    /// Replaces the right-hand object and releases the previous one.
    pub fn set_to(&mut self, object: Option<GitObjectOwned>) {
        let object = object.map_or(core::ptr::null_mut(), GitObjectOwned::into_raw);
        let old = self.take_to();
        // SAFETY: the exclusive handle permits installing the transferred
        // compatible object reference after the old field was cleared.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).to).write(object) }
        drop(old);
    }
}

// SAFETY: a valid `GitRevspec` owns each non-null object field independently.
// Disposing the by-value header releases exactly those references and clears
// both fields while retaining the header storage.
unsafe impl CValued for GitRevspec {
    unsafe fn c_dispose(this: NonNull<Self>) {
        let result = this.as_ptr().cast::<ffi::git_revspec>();
        // SAFETY: the `CValued` contract grants exclusive teardown access to
        // the live header. Reading and clearing both fields transfers their
        // unique references into the temporary owners below.
        let (from, to) = unsafe {
            let from = addr_of!((*result).from).read();
            let to = addr_of!((*result).to).read();
            addr_of_mut!((*result).from).write(core::ptr::null_mut());
            addr_of_mut!((*result).to).write(core::ptr::null_mut());
            (from, to)
        };
        // SAFETY: every non-null pointer came from one owned field of this
        // valid revspec and is released exactly once after being cleared.
        let from = unsafe { GitObjectOwned::from_raw(from) };
        // SAFETY: as above, for the independent right-hand object field.
        let to = unsafe { GitObjectOwned::from_raw(to) };
        drop((from, to));
    }
}

/// The object and optional intermediate reference resolved by [`git_revparse_ext`].
pub struct GitRevparseExt<'repo> {
    object: RepositoryObject<'repo>,
    reference: Option<GitReferenceTetheredOwned<'repo>>,
}

impl GitRevparseExt<'_> {
    /// Borrows the resolved object.
    #[must_use]
    pub fn object(&self) -> GitObjectRef<'_> {
        self.object.as_ref()
    }

    /// Borrows the intermediate reference, when the expression traversed one.
    #[must_use]
    pub fn reference(&self) -> Option<crate::refs::GitReferenceRef<'_>> {
        self.reference
            .as_ref()
            .map(GitReferenceTetheredOwned::as_ref)
    }
}

/// Wraps: git_revparse_ext
/// Resolves an object and, when applicable, its intermediate reference.
pub fn git_revparse_ext<'repo>(
    mut repository: GitRepositoryMut<'repo>,
    spec: &CStr,
) -> Result<GitRevparseExt<'repo>, i32> {
    let mut object = core::ptr::null_mut();
    let mut reference = core::ptr::null_mut();
    // SAFETY: both output slots are writable, the repository is live and
    // exclusive for cache access, and `spec` is a live C string. No input
    // pointer is retained; successful outputs transfer one owned count each.
    let status = unsafe {
        ffi::git_revparse_ext(
            core::ptr::addr_of_mut!(object),
            core::ptr::addr_of_mut!(reference),
            repository.as_mut_ptr(),
            spec.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers a complete non-null object owner.
    let object = unsafe { GitObjectOwned::from_raw(object) };
    let object = adopt_repository_object(status, object)?;
    // SAFETY: the optional non-null reference output transfers one complete
    // owner whose reference database borrows the same repository.
    let reference = unsafe { ffibox::CBox::from_raw(reference) };
    Ok(GitRevparseExt {
        object,
        reference: adopt_optional_reference(reference),
    })
}

/// Wraps: git_revparse_single
/// Resolves one revision expression into a repository-tethered object.
pub fn git_revparse_single<'repo>(
    mut repository: GitRepositoryMut<'repo>,
    spec: &CStr,
) -> Result<RepositoryObject<'repo>, i32> {
    let mut object = core::ptr::null_mut();
    // SAFETY: the output slot is writable, the repository is live and
    // exclusive, and `spec` is a live C string retained only for this call.
    let status = unsafe {
        ffi::git_revparse_single(
            core::ptr::addr_of_mut!(object),
            repository.as_mut_ptr(),
            spec.as_ptr(),
        )
    };
    // SAFETY: the output is null on failure or one complete owned object count
    // on success, tied to the repository borrow by the returned type.
    let object = unsafe { GitObjectOwned::from_raw(object) };
    adopt_repository_object(status, object)
}

/// An owned revspec whose resolved objects cannot outlive their repository.
pub struct RepositoryRevspec<'repo> {
    inner: GitRevspecOwned,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl RepositoryRevspec<'_> {
    /// Borrows the parsed result.
    pub fn as_ref(&self) -> GitRevspecRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the parsed result exclusively.
    pub fn as_mut(&mut self) -> GitRevspecMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_revparse
/// Parses a single revision or range into repository-tethered owned objects.
pub fn git_revparse<'repo>(
    mut repository: GitRepositoryMut<'repo>,
    spec: &CStr,
) -> Result<RepositoryRevspec<'repo>, i32> {
    let mut result = GitRevspec::new();
    let status = {
        let mut output = result.as_mut();
        // SAFETY: `output` is exclusive initialized empty storage, the
        // repository is live and exclusive for cache access, and `spec` is a
        // live C string. Successful object outputs carry owned counts.
        unsafe { ffi::git_revparse(output.as_mut_ptr(), repository.as_mut_ptr(), spec.as_ptr()) }
    };
    if status != 0 {
        return Err(status);
    }
    Ok(RepositoryRevspec {
        inner: result,
        _repository: PhantomData,
    })
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use super::*;

    #[test]
    fn revspec_wrapper_preserves_layout_and_value_ownership() {
        fn assert_valued<T: CValued>() {}
        assert_valued::<GitRevspec>();
        assert_eq!(size_of::<GitRevspec>(), size_of::<ffi::git_revspec>());
        assert_eq!(align_of::<GitRevspec>(), align_of::<ffi::git_revspec>());
        assert_eq!(
            size_of::<GitRevspecRef<'_>>(),
            size_of::<*const ffi::git_revspec>()
        );
        assert_eq!(
            size_of::<GitRevspecMut<'_>>(),
            size_of::<*mut ffi::git_revspec>()
        );
        assert_eq!(size_of::<GitRevspecOwned>(), size_of::<ffi::git_revspec>());

        let empty = GitRevspec::new();
        assert!(empty.as_ref().from().is_none());
        assert!(empty.as_ref().to().is_none());
        assert_eq!(empty.as_ref().flags(), Ok(GitRevspecFlags::EMPTY));
    }

    #[test]
    fn borrowed_revspec_handles_project_objects_and_validate_flags() {
        let object = Box::new(MaybeUninit::<ffi::git_object>::zeroed());
        let object = Box::into_raw(object).cast::<ffi::git_object>();
        let mut raw = ffi::git_revspec {
            from: object,
            to: core::ptr::null_mut(),
            flags: GitRevspecFlags::SINGLE.bits(),
        };

        {
            // SAFETY: `raw` and the opaque object storage remain live, and
            // this scope has exclusive access to the revspec header.
            let mut revspec = unsafe { GitRevspecMut::from_ptr(&raw mut raw) }.unwrap();
            assert_eq!(revspec.as_ref().from().unwrap().as_ptr(), object);
            assert!(revspec.as_ref().to().is_none());
            assert_eq!(revspec.as_ref().flags(), Ok(GitRevspecFlags::SINGLE));

            revspec.set_flags(GitRevspecFlags::RANGE | GitRevspecFlags::MERGE_BASE);
            assert_eq!(
                revspec.as_ref().flags(),
                Ok(GitRevspecFlags::RANGE | GitRevspecFlags::MERGE_BASE)
            );
        }
        raw.flags = GitRevspecFlags::ALL.bits() << 1;
        {
            // SAFETY: the exclusive handle was released; `raw` remains live
            // and is now only shared for this handle's lifetime.
            let revspec = unsafe { GitRevspecRef::from_ptr(&raw mut raw) }.unwrap();
            assert_eq!(
                revspec.flags().unwrap_err().value(),
                GitRevspecFlags::ALL.bits() << 1
            );
        }
        // SAFETY: no handle retains the object pointer, and this recovers the
        // exact allocation originally produced by `Box::into_raw`.
        drop(unsafe { Box::from_raw(object.cast::<MaybeUninit<ffi::git_object>>()) });
    }

    #[test]
    fn exclusive_revspec_handles_move_object_ownership_between_fields() {
        // The two fields hold independent owned references, so taking one out
        // must clear it and installing it elsewhere must not duplicate it.
        // The reference is never dropped here: this opaque storage carries no
        // libgit2 refcount for `git_object_free` to release.
        let object = Box::new(MaybeUninit::<ffi::git_object>::zeroed());
        let object = Box::into_raw(object).cast::<ffi::git_object>();
        let mut raw = ffi::git_revspec {
            from: object,
            to: core::ptr::null_mut(),
            flags: GitRevspecFlags::SINGLE.bits(),
        };

        {
            // SAFETY: `raw` and the opaque object storage remain live, and
            // this scope has exclusive access to the revspec header.
            let mut revspec = unsafe { GitRevspecMut::from_ptr(&raw mut raw) }.unwrap();

            let taken = revspec.take_from().expect("the left-hand field was set");
            assert!(revspec.as_ref().from().is_none());
            assert!(revspec.take_from().is_none());

            revspec.set_to(Some(taken));
            assert!(revspec.as_ref().from().is_none());
            assert_eq!(revspec.as_ref().to().unwrap().as_ptr(), object);

            let taken = revspec.take_to().expect("the right-hand field was set");
            assert!(revspec.as_ref().to().is_none());
            // Surrender the owner instead of dropping it.
            assert_eq!(taken.into_raw(), object);
        }

        assert!(raw.from.is_null());
        assert!(raw.to.is_null());
        // SAFETY: no handle retains the object pointer, and this recovers the
        // exact allocation originally produced by `Box::into_raw`.
        drop(unsafe { Box::from_raw(object.cast::<MaybeUninit<ffi::git_object>>()) });
    }

    #[test]
    fn disposing_an_emptied_revspec_releases_nothing() {
        // `c_dispose` clears both fields, so the emptied header below has no
        // reference left to release when the `CVal` is dropped.
        let object = Box::new(MaybeUninit::<ffi::git_object>::zeroed());
        let object = Box::into_raw(object).cast::<ffi::git_object>();

        let mut revspec = GitRevspec::new();
        revspec.as_mut().set_flags(GitRevspecFlags::RANGE);
        // SAFETY: the opaque storage is live and this is its only owner.
        let owned = unsafe { GitObjectOwned::from_raw(object) }.unwrap();
        revspec.as_mut().set_from(Some(owned));
        assert_eq!(revspec.as_ref().from().unwrap().as_ptr(), object);

        // Emptying is what makes the result safe to parse into again.
        assert_eq!(
            revspec
                .as_mut()
                .take_from()
                .expect("the left-hand field was set")
                .into_raw(),
            object
        );
        assert!(revspec.as_ref().from().is_none());
        assert!(revspec.as_ref().to().is_none());
        drop(revspec);

        // SAFETY: the disposed revspec released nothing, so this recovers the
        // exact allocation originally produced by `Box::into_raw`.
        drop(unsafe { Box::from_raw(object.cast::<MaybeUninit<ffi::git_object>>()) });
    }
}
