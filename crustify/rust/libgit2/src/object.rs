//! Safe wrappers for libgit2 object APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of_mut};

use ffibox::{CBox, CCloned, CVal};

use crate::api::buffer::GitBuf;
use crate::api::object::{GitObjectIdOptions, GitObjectIdOptionsRef};
use crate::api::types::GitObjectType;
use crate::ffi;
use crate::oid::{Oid, OidRef};
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

pub(crate) fn adopt_repository_object<'repo>(
    status: i32,
    inner: Option<GitObjectOwned>,
) -> Result<RepositoryObject<'repo>, i32> {
    if status != 0 {
        return Err(status);
    }
    let inner = inner.expect("a successful object constructor returns a non-null owner");
    Ok(RepositoryObject {
        inner,
        _repository: PhantomData,
    })
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

impl<'repo> RepositoryObject<'repo> {
    /// Attaches an already-owned object to the repository that keeps it valid.
    pub(crate) fn from_owned(inner: GitObjectOwned, repository: GitRepositoryRef<'repo>) -> Self {
        let _ = repository;
        Self {
            inner,
            _repository: PhantomData,
        }
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

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod unit_tests {
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
        assert_eq!(
            git_object_string2type(Some(c"comm")),
            GitObjectType::INVALID
        );
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

/// Wraps: git_object_lookup_bypath
/// Looks up an object below `treeish`, tying the owner to its repository.
pub fn git_object_lookup_bypath<'repo>(
    treeish: GitObjectRef<'repo>,
    path: &CStr,
    kind: GitObjectType,
) -> Result<RepositoryObject<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: inputs are live, `out` is writable, and the resulting object's
    // repository lifetime is bounded by the source object borrow.
    let status = unsafe {
        ffi::git_object_lookup_bypath(&mut out, treeish.as_ptr(), path.as_ptr(), kind.into())
    };
    // SAFETY: `out` is null or transfers one initialized object reference.
    let inner = unsafe { GitObjectOwned::from_raw(out) };
    adopt_repository_object(status, inner)
}

/// Wraps: git_object_owner
/// Borrows the repository retained by an object.
#[must_use]
pub fn git_object_owner<'a>(object: GitObjectRef<'a>) -> GitRepositoryRef<'a> {
    // SAFETY: a live repository-backed object carries a non-null repository
    // pointer which remains valid for the object's borrow.
    let repo = unsafe { ffi::git_object_owner(object.as_ptr()) };
    // SAFETY: the object borrow keeps the returned repository live.
    unsafe { GitRepositoryRef::from_ptr(repo) }.expect("a live object has an owner")
}

/// Wraps: git_object_rawcontent_is_valid
/// Validates length-delimited raw object content.
pub fn git_object_rawcontent_is_valid(
    bytes: &[u8],
    object_type: GitObjectType,
    oid_type: crate::oid::OidType,
) -> Result<bool, i32> {
    let mut valid = 0;
    // SAFETY: `valid` is writable and `bytes` supplies exactly the readable
    // run passed to C; neither pointer is retained.
    let status = unsafe {
        ffi::git_object_rawcontent_is_valid(
            &mut valid,
            bytes.as_ptr().cast(),
            bytes.len(),
            object_type.into(),
            oid_type.into(),
        )
    };
    if status == 0 {
        Ok(valid != 0)
    } else {
        Err(status)
    }
}

/// Wraps: git_object_type_is_valid
/// Reports whether a kind denotes a concrete parseable object type.
#[must_use]
pub fn git_object_type_is_valid(object_type: GitObjectType) -> bool {
    // SAFETY: the checked scalar is passed by value.
    unsafe { ffi::git_object_type_is_valid(object_type.into()) != 0 }
}

#[cfg(test)]
mod scheduled_object_tests {
    use super::*;
    #[test]
    fn concrete_object_types_are_valid() {
        assert!(git_object_type_is_valid(GitObjectType::COMMIT));
        assert!(git_object_type_is_valid(GitObjectType::BLOB));
        assert!(!git_object_type_is_valid(GitObjectType::ANY));
        assert!(
            git_object_rawcontent_is_valid(
                b"anything",
                GitObjectType::BLOB,
                crate::oid::OidType::Sha1
            )
            .unwrap()
        );
    }
}

/// Wraps: git_object_id_from_buffer
/// Computes an object ID for raw object content.
pub fn git_object_id_from_buffer(
    data: &[u8],
    options: Option<GitObjectIdOptionsRef<'_, '_>>,
) -> Result<Oid, i32> {
    let mut id = Oid::zeroed();
    // SAFETY: `id` is writable, `data` is a readable run of exactly its
    // reported length, and `options` remains live for the call.
    let status = unsafe {
        ffi::git_object_id_from_buffer(
            addr_of_mut!(id).cast(),
            data.as_ptr().cast(),
            data.len(),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    };
    if status == 0 { Ok(id) } else { Err(status) }
}

/// Wraps: git_object_id_from_file
/// Computes an object ID for the raw contents of a file.
pub fn git_object_id_from_file(
    path: &CStr,
    options: Option<GitObjectIdOptionsRef<'_, '_>>,
) -> Result<Oid, i32> {
    let mut id = Oid::zeroed();
    // SAFETY: `id` is writable and both borrowed inputs remain live for the
    // synchronous operation; C retains neither pointer.
    let status = unsafe {
        ffi::git_object_id_from_file(
            addr_of_mut!(id).cast(),
            path.as_ptr(),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    };
    if status == 0 { Ok(id) } else { Err(status) }
}

/// Wraps: git_object_id_options_init
/// Initializes object-ID options for the requested ABI version.
///
/// `'data` is the borrow a later
/// [`GitObjectIdOptionsMut::set_filters`](crate::api::object::GitObjectIdOptionsMut::set_filters)
/// parks in the value. It is invariant in the options type, so it is chosen
/// by the caller here rather than fixed to `'static`, which no filter list
/// borrowed from a repository could ever satisfy.
pub fn git_object_id_options_init<'data>(
    version: core::ffi::c_uint,
) -> Result<CVal<GitObjectIdOptions<'data>>, i32> {
    let mut options = GitObjectIdOptions::new();
    let status = {
        let mut output = options.as_mut();
        // SAFETY: `output` is writable layout-compatible storage and C
        // retains no pointer to it.
        unsafe { ffi::git_object_id_options_init(output.as_mut_ptr(), version) }
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod scheduled_object_id_options_tests {
    use super::*;

    /// The options type is invariant in `'data`, so pinning the constructor to
    /// `'static` would have made the value unusable with any filter list a
    /// repository lends out. Compiling this installation is the assertion.
    #[test]
    fn initialized_options_accept_a_filter_list_borrowed_for_less_than_static() {
        fn install(list: &mut crate::filter::RepositoryFilterList<'_>) {
            let mut options = git_object_id_options_init(ffi::GIT_OBJECT_ID_OPTIONS_VERSION)
                .expect("the current options version initializes");
            options.as_mut().set_filters(Some(list.as_mut()));
            assert!(options.as_mut().filters_mut().is_some());
        }

        let _: fn(&mut crate::filter::RepositoryFilterList<'_>) = install;
    }

    #[test]
    fn initialization_installs_the_requested_version() {
        let options = git_object_id_options_init(ffi::GIT_OBJECT_ID_OPTIONS_VERSION)
            .expect("the current options version initializes");
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_OBJECT_ID_OPTIONS_VERSION
        );
        assert!(options.as_ref().filters().is_none());
    }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, RawBuf, safe_buf_bytes};

    #[derive(Debug, Eq, PartialEq)]
    struct ObjectObservation {
        head: Vec<u8>,
        duplicate: Vec<u8>,
        prefix: Vec<u8>,
        tree: Vec<u8>,
        readme: Vec<u8>,
        short_id: Vec<u8>,
        owner_matches: bool,
        kinds: Vec<ffi::git_object_t>,
        names: Vec<Vec<u8>>,
        loose: Vec<bool>,
        raw_validity: Vec<(i32, bool)>,
        buffer_id: (i32, Vec<u8>),
    }

    unsafe fn raw_observe(repository: *mut ffi::git_repository) -> ObjectObservation {
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut object = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_object_lookup(
                    &mut object,
                    repository,
                    &head,
                    ffi::git_object_t_GIT_OBJECT_ANY,
                )
            },
            0
        );
        let mut duplicate = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_object_dup(&mut duplicate, object) }, 0);
        let duplicate_id = unsafe { (*ffi::git_object_id(duplicate)).id }.to_vec();

        let mut prefix_object = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_object_lookup_prefix(
                    &mut prefix_object,
                    repository,
                    &head,
                    8,
                    ffi::git_object_t_GIT_OBJECT_COMMIT,
                )
            },
            0
        );
        let prefix = unsafe { (*ffi::git_object_id(prefix_object)).id }.to_vec();

        let mut tree = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_object_peel(&mut tree, object, ffi::git_object_t_GIT_OBJECT_TREE) },
            0
        );
        let tree_id = unsafe { (*ffi::git_object_id(tree)).id }.to_vec();
        let mut readme = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_object_lookup_bypath(
                    &mut readme,
                    tree,
                    c"README.md".as_ptr(),
                    ffi::git_object_t_GIT_OBJECT_BLOB,
                )
            },
            0
        );
        let readme_id = unsafe { (*ffi::git_object_id(readme)).id }.to_vec();
        let mut short = RawBuf::new();
        assert_eq!(unsafe { ffi::git_object_short_id(&mut short.0, object) }, 0);
        let owner_matches = unsafe { ffi::git_object_owner(object) == repository };

        let kind_values = [
            ffi::git_object_t_GIT_OBJECT_COMMIT,
            ffi::git_object_t_GIT_OBJECT_TREE,
            ffi::git_object_t_GIT_OBJECT_BLOB,
            ffi::git_object_t_GIT_OBJECT_TAG,
            ffi::git_object_t_GIT_OBJECT_ANY,
            ffi::git_object_t_GIT_OBJECT_INVALID,
        ];
        let names = kind_values
            .iter()
            .map(|kind| {
                unsafe { core::ffi::CStr::from_ptr(ffi::git_object_type2string(*kind)) }
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        let loose = kind_values
            .iter()
            .map(|kind| unsafe { ffi::git_object_typeisloose(*kind) != 0 })
            .collect();
        let kinds = [
            c"commit",
            c"commitment",
            c"comm",
            c"tree",
            c"blob",
            c"tag",
            c"nonsense",
        ]
        .iter()
        .map(|name| unsafe { ffi::git_object_string2type(name.as_ptr()) })
        .collect();

        let samples: &[(&[u8], ffi::git_object_t)] = &[
            (b"arbitrary blob bytes", ffi::git_object_t_GIT_OBJECT_BLOB),
            (b"tree 0000000000000000000000000000000000000000\n", ffi::git_object_t_GIT_OBJECT_COMMIT),
            (b"not a tree", ffi::git_object_t_GIT_OBJECT_TREE),
            (b"object 0000000000000000000000000000000000000000\ntype commit\ntag v1\ntagger A <a@example.com> 1 +0000\n\ntag\n", ffi::git_object_t_GIT_OBJECT_TAG),
        ];
        let raw_validity = samples
            .iter()
            .map(|(bytes, kind)| {
                let mut valid = 0;
                let status = unsafe {
                    ffi::git_object_rawcontent_is_valid(
                        &mut valid,
                        bytes.as_ptr().cast(),
                        bytes.len(),
                        *kind,
                        ffi::git_oid_t_GIT_OID_SHA1,
                    )
                };
                (status, valid != 0)
            })
            .collect();

        let buffer = b"object-id buffer contents\n";
        let mut buffer_oid = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        let buffer_status = unsafe {
            ffi::git_object_id_from_buffer(
                &mut buffer_oid,
                buffer.as_ptr().cast(),
                buffer.len(),
                core::ptr::null(),
            )
        };

        unsafe {
            ffi::git_object_free(readme);
            ffi::git_object_free(tree);
            ffi::git_object_free(prefix_object);
            ffi::git_object_free(duplicate);
            ffi::git_object_free(object);
        }
        ObjectObservation {
            head: head.id.to_vec(),
            duplicate: duplicate_id,
            prefix,
            tree: tree_id,
            readme: readme_id,
            short_id: short.bytes(),
            owner_matches,
            kinds,
            names,
            loose,
            raw_validity,
            buffer_id: (buffer_status, buffer_oid.id.to_vec()),
        }
    }

    fn safe_observe(repository: *mut ffi::git_repository) -> ObjectObservation {
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let repository =
            unsafe { crate::repository::GitRepositoryRef::from_ptr(repository) }.unwrap();
        let head_ref = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(head).cast()) }.unwrap();
        let object = git_object_lookup(repository, head_ref, GitObjectType::ANY).unwrap();
        let duplicate = git_object_dup(&object);
        let prefix_object =
            git_object_lookup_prefix(repository, head_ref, 8, GitObjectType::COMMIT).unwrap();
        let tree = git_object_peel(&object, GitObjectType::TREE).unwrap();
        let readme =
            git_object_lookup_bypath(tree.as_ref(), c"README.md", GitObjectType::BLOB).unwrap();
        let short = git_object_short_id(object.as_ref()).unwrap();

        let kind_values = [
            GitObjectType::COMMIT,
            GitObjectType::TREE,
            GitObjectType::BLOB,
            GitObjectType::TAG,
            GitObjectType::ANY,
            GitObjectType::INVALID,
        ];
        let names = kind_values
            .iter()
            .map(|kind| git_object_type2string(*kind).to_bytes().to_vec())
            .collect();
        let loose = kind_values
            .iter()
            .map(|kind| git_object_typeisloose(*kind))
            .collect();
        let kinds = [
            c"commit",
            c"commitment",
            c"comm",
            c"tree",
            c"blob",
            c"tag",
            c"nonsense",
        ]
        .iter()
        .map(|name| git_object_string2type(Some(*name)).as_raw())
        .collect();
        let samples: &[(&[u8], GitObjectType)] = &[
            (b"arbitrary blob bytes", GitObjectType::BLOB),
            (b"tree 0000000000000000000000000000000000000000\n", GitObjectType::COMMIT),
            (b"not a tree", GitObjectType::TREE),
            (b"object 0000000000000000000000000000000000000000\ntype commit\ntag v1\ntagger A <a@example.com> 1 +0000\n\ntag\n", GitObjectType::TAG),
        ];
        let raw_validity = samples
            .iter()
            .map(|(bytes, kind)| {
                match git_object_rawcontent_is_valid(bytes, *kind, crate::oid::OidType::Sha1) {
                    Ok(valid) => (0, valid),
                    Err(status) => (status, false),
                }
            })
            .collect();
        let buffer = b"object-id buffer contents\n";
        let buffer_id = match git_object_id_from_buffer(buffer, None) {
            Ok(mut id) => {
                let id = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(id).cast()) }.unwrap();
                (0, id.raw_bytes().elems().collect())
            }
            Err(status) => (status, vec![0; core::mem::size_of_val(&head.id)]),
        };
        ObjectObservation {
            head: head.id.to_vec(),
            duplicate: git_object_id(duplicate.as_ref())
                .raw_bytes()
                .elems()
                .collect(),
            prefix: git_object_id(prefix_object.as_ref())
                .raw_bytes()
                .elems()
                .collect(),
            tree: git_object_id(tree.as_ref()).raw_bytes().elems().collect(),
            readme: git_object_id(readme.as_ref()).raw_bytes().elems().collect(),
            short_id: safe_buf_bytes(short.as_ref()),
            owner_matches: git_object_owner(object.as_ref()).as_ptr() == repository.as_ptr(),
            kinds,
            names,
            loose,
            raw_validity,
            buffer_id,
        }
    }

    #[test]
    fn io_equiv_object_lookup_peeling_validation_and_identity() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("object-raw");
        let safe = HistoryFixture::new("object-safe");
        let raw_observation = unsafe { raw_observe(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_observe(safe.repository.as_ptr()));
        assert!(raw_observation.owner_matches);
    }
}
