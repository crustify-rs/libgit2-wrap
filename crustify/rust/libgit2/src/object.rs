//! Safe wrappers for libgit2 object APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of_mut};

use ffibox::{CBox, CCloned, CVal};

use crate::api::buffer::GitBuf;
use crate::api::types::GitObjectType;
use crate::ffi;
use crate::oid::OidRef;
use crate::repository::GitRepositoryRef;

ffibox::define_ctype!(
    /// Wraps: git_object
    /// Opaque base object managed by libgit2.
    ///
    /// An object carries one reference to cached object storage. Owned handles
    /// release that reference with `git_object_free`, while cloning increments
    /// its reference count with `git_object_dup`. Repository-backed objects
    /// also borrow their repository, which must remain open while the object is
    /// used.
    GitObject,
    GitObjectRef,
    GitObjectMut,
    ffi::git_object
);

/// An owned reference to a libgit2 object.
pub type GitObjectOwned = CBox<GitObject>;

/// An owned object whose repository is guaranteed to remain live.
pub struct RepositoryObject<'repo> {
    inner: GitObjectOwned,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl RepositoryObject<'_> {
    /// Borrows the object.
    pub fn as_ref(&self) -> GitObjectRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the object exclusively.
    pub fn as_mut(&mut self) -> GitObjectMut<'_> {
        self.inner.as_mut()
    }
}

// SAFETY: `git_object_free` consumes one reference to a fully initialized
// `git_object`; its matching reference-count decrement releases the allocation
// only when the final reference is gone. `GitObject` is transparent over the
// corresponding bindgen type.
ffibox::impl_dropped!(GitObject, ffi::git_object, ffi::git_object_free);

// SAFETY: `git_object_dup` increments the live source object's reference count,
// writes the same pointer to its non-null output slot, and cannot fail. The new
// reference is independently released by the `CDropped` implementation above.
unsafe impl CCloned for GitObject {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live object. `duplicate` is a
        // valid output slot, and `GitObject` is layout-compatible with
        // `ffi::git_object`.
        let result = unsafe {
            ffi::git_object_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_object>(),
            )
        };
        debug_assert_eq!(result, 0);
        NonNull::new(duplicate.cast::<Self>())
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::{CCloned, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<GitObject>(), size_of::<ffi::git_object>());
        assert_eq!(align_of::<GitObject>(), align_of::<ffi::git_object>());
        assert_eq!(
            size_of::<GitObjectRef<'_>>(),
            size_of::<*const ffi::git_object>()
        );
        assert_eq!(
            size_of::<GitObjectMut<'_>>(),
            size_of::<*mut ffi::git_object>()
        );
        assert_eq!(
            size_of::<Option<GitObjectOwned>>(),
            size_of::<*mut ffi::git_object>()
        );
    }

    #[test]
    fn object_registers_refcount_lifecycle() {
        fn assert_refcounted<T: CDropped + CCloned>() {}
        assert_refcounted::<GitObject>();
    }

    #[test]
    fn borrowed_handles_preserve_the_object_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_object>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_object>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage,
            // and the shared handle remains within this scope.
            let shared = unsafe { GitObjectRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitObjectMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_object>>()) });
    }

    #[test]
    fn object_kind_names_round_trip_through_safe_functions() {
        for kind in [
            GitObjectType::COMMIT,
            GitObjectType::TREE,
            GitObjectType::BLOB,
            GitObjectType::TAG,
        ] {
            let name = git_object_type2string(kind);
            assert_eq!(git_object_string2type(Some(name)), kind);
            assert!(git_object_typeisloose(kind));
        }
        assert_eq!(git_object_string2type(None), GitObjectType::INVALID);
    }

    #[test]
    fn object_kind_lookup_matches_libgit2_prefix_semantics() {
        // The C loop accepts any input the published name is a prefix of, and
        // rejects an input that merely starts one.
        assert_eq!(
            git_object_string2type(Some(c"commitment")),
            GitObjectType::COMMIT
        );
        assert_eq!(git_object_string2type(Some(c"comm")), GitObjectType::INVALID);
        assert_eq!(git_object_string2type(Some(c"")), GitObjectType::INVALID);
        assert_eq!(
            git_object_string2type(Some(c"nonsense")),
            GitObjectType::INVALID
        );
    }

    #[test]
    fn unnamed_object_kinds_have_an_empty_static_name() {
        // `git_object_type2string` answers every checked value, returning the
        // empty static string outside the object table rather than null.
        assert_eq!(git_object_type2string(GitObjectType::ANY), c"");
        assert_eq!(git_object_type2string(GitObjectType::INVALID), c"");
        assert!(!git_object_typeisloose(GitObjectType::ANY));
        assert!(!git_object_typeisloose(GitObjectType::INVALID));
    }
}

/// Wraps: git_object_dup
/// Acquires another owned reference to the same repository-backed object.
pub fn git_object_dup<'repo>(source: &RepositoryObject<'repo>) -> RepositoryObject<'repo> {
    RepositoryObject {
        inner: source.inner.clone(),
        _repository: PhantomData,
    }
}

/// Wraps: git_object_free
/// Consumes one independently owned object reference.
pub fn git_object_free(object: GitObjectOwned) {
    drop(object);
}

/// Wraps: git_object_id
/// Borrows an object's inline ID.
pub fn git_object_id<'a>(object: GitObjectRef<'a>) -> OidRef<'a> {
    // SAFETY: libgit2 returns the live object's non-null inline ID.
    let raw = unsafe { ffi::git_object_id(object.as_ptr()) };
    // SAFETY: the field is live for the object borrow.
    unsafe { OidRef::from_ptr(raw.cast_mut()) }.expect("a live object has an ID")
}

/// Wraps: git_object_lookup
/// Looks up a full object ID and ties the returned object to its repository.
pub fn git_object_lookup<'repo>(
    repo: GitRepositoryRef<'repo>,
    id: OidRef<'_>,
    kind: GitObjectType,
) -> Result<RepositoryObject<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: `raw` is writable, both borrowed objects are live for the call,
    // and success returns one owned reference tied to `repo` below.
    let status = unsafe {
        ffi::git_object_lookup(
            addr_of_mut!(raw),
            repo.as_ptr().cast_mut(),
            id.as_ptr(),
            kind.as_raw(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes one non-null fully initialized owned reference.
    let inner = unsafe { GitObjectOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryObject {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_object_lookup_prefix
/// Looks up an object by a hexadecimal prefix length.
pub fn git_object_lookup_prefix<'repo>(
    repo: GitRepositoryRef<'repo>,
    id: OidRef<'_>,
    prefix_len: usize,
    kind: GitObjectType,
) -> Result<RepositoryObject<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: `raw` is writable, both borrowed objects are live for the call,
    // and success returns one owned reference tied to `repo` below.
    let status = unsafe {
        ffi::git_object_lookup_prefix(
            addr_of_mut!(raw),
            repo.as_ptr().cast_mut(),
            id.as_ptr(),
            prefix_len,
            kind.as_raw(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes one non-null fully initialized owned reference.
    let inner = unsafe { GitObjectOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryObject {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_object_peel
/// Peels an object while preserving its repository lifetime.
pub fn git_object_peel<'repo>(
    object: &RepositoryObject<'repo>,
    target: GitObjectType,
) -> Result<RepositoryObject<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    let object = object.as_ref();
    // SAFETY: the output is writable and the source is live; the returned
    // object uses the same repository as the source.
    let status =
        unsafe { ffi::git_object_peel(addr_of_mut!(raw), object.as_ptr(), target.as_raw()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one complete owned object reference.
    let inner = unsafe { GitObjectOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryObject {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_object_short_id
/// Computes an unambiguous abbreviated object ID.
pub fn git_object_short_id(object: GitObjectRef<'_>) -> Result<CVal<GitBuf>, i32> {
    let mut out = GitBuf::new();
    let status = {
        let mut output = out.as_mut();
        // SAFETY: `output` is writable and `object` remains live for the call.
        unsafe { ffi::git_object_short_id(output.as_mut_ptr(), object.as_ptr()) }
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_object_string2type
/// Converts an optional object-kind name, returning `INVALID` if unknown.
///
/// libgit2 compares with `git__prefixncmp`, so a name matches when a published
/// kind is a *prefix* of it: `"commit"` and `"commitment"` both resolve to
/// [`GitObjectType::COMMIT`], while a truncation such as `"comm"` does not.
pub fn git_object_string2type(name: Option<&CStr>) -> GitObjectType {
    // SAFETY: the pointer is null or a live C string and is not retained.
    let raw = unsafe { ffi::git_object_string2type(name.map_or(core::ptr::null(), CStr::as_ptr)) };
    GitObjectType::from_raw(raw).expect("libgit2 returns a published object kind")
}

/// Wraps: git_object_type
/// Returns the object's checked kind.
pub fn git_object_type(object: GitObjectRef<'_>) -> GitObjectType {
    // SAFETY: the object is live for this scalar query.
    let raw = unsafe { ffi::git_object_type(object.as_ptr()) };
    GitObjectType::from_raw(raw).expect("a live object has a published kind")
}

/// Wraps: git_object_type2string
/// Returns the static name for an object kind.
pub fn git_object_type2string(kind: GitObjectType) -> &'static CStr {
    // SAFETY: libgit2 returns a non-null pointer to immutable static storage.
    unsafe { CStr::from_ptr(ffi::git_object_type2string(kind.as_raw())) }
}

/// Wraps: git_object_typeisloose
/// Reports whether the object kind is a concrete loose-object type.
pub fn git_object_typeisloose(kind: GitObjectType) -> bool {
    // SAFETY: this pure query accepts every checked object-kind value.
    unsafe { ffi::git_object_typeisloose(kind.as_raw()) != 0 }
}
