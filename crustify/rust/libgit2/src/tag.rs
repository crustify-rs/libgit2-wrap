//! Safe wrappers for libgit2 tag APIs.

use core::ffi::{CStr, c_void};
use core::marker::PhantomData;
use core::ptr::NonNull;

use ffibox::{CBox, CCloned, CVal, define_ctype, impl_dropped};

use crate::api::tag::GitTagForeachCallback;
use crate::api::types::{GitObjectType, GitSignatureRef};
use crate::ffi;
use crate::object::{GitObjectOwned, GitObjectRef, RepositoryObject, adopt_repository_object};
use crate::oid::{Oid, OidRef};
use crate::repository::{GitRepositoryMut, GitRepositoryRef};
use crate::strarray::GitStrArray;

define_ctype!(
    /// Wraps: git_tag
    /// An opaque, reference-counted annotated Git tag.
    ///
    /// Owned handles release one cache reference with `git_tag_free`, while
    /// cloning acquires another reference with `git_tag_dup`. Repository-backed
    /// tags borrow their repository, which must remain alive while they are used.
    GitTag,
    GitTagRef,
    GitTagMut,
    ffi::git_tag
);

/// An owned reference to an annotated Git tag.
pub type GitTagOwned = CBox<GitTag>;

/// An owned tag whose repository is kept live by its type-level borrow.
pub struct RepositoryTag<'repo> {
    inner: GitTagOwned,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

pub(crate) fn adopt_repository_tag<'repo>(
    status: i32,
    inner: Option<GitTagOwned>,
) -> Result<RepositoryTag<'repo>, i32> {
    if status != 0 {
        return Err(status);
    }
    Ok(RepositoryTag {
        inner: inner.expect("a successful tag lookup returns an owner"),
        _repository: PhantomData,
    })
}

impl RepositoryTag<'_> {
    /// Borrows the tag.
    pub fn as_ref(&self) -> GitTagRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the tag exclusively.
    pub fn as_mut(&mut self) -> GitTagMut<'_> {
        self.inner.as_mut()
    }
}

impl Clone for RepositoryTag<'_> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _repository: PhantomData,
        }
    }
}

// SAFETY: `git_tag_free` consumes one reference to a complete tag and releases
// the allocation only when its underlying object cache refcount reaches zero.
// `GitTag` is transparent over the corresponding bindgen type.
impl_dropped!(GitTag, ffi::git_tag, ffi::git_tag_free);

// SAFETY: `git_tag_dup` increments the live tag's underlying object refcount
// and writes the same pointer to its non-null output slot. The new reference is
// independently released by the `CDropped` implementation above.
unsafe impl CCloned for GitTag {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live tag; `duplicate` is a
        // valid output slot, and the wrapper is layout-compatible with
        // `ffi::git_tag`.
        let result = unsafe {
            ffi::git_tag_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_tag>(),
            )
        };
        debug_assert_eq!(result, 0);
        NonNull::new(duplicate.cast::<Self>())
    }
}

/// Wraps: git_tag_name_is_valid
/// Checks whether `name` is a valid tag shorthand.
///
/// `None` mirrors libgit2's accepted null input and is reported as invalid.
pub fn git_tag_name_is_valid(name: Option<&core::ffi::CStr>) -> Result<bool, i32> {
    let mut valid = 0;
    let name = name.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `valid` is writable and `name` is null or a live C string.
    let status = unsafe { ffi::git_tag_name_is_valid(&mut valid, name) };
    if status == 0 {
        Ok(valid != 0)
    } else {
        Err(status)
    }
}

/// Wraps: git_tag_create_lightweight
/// Creates or replaces a lightweight tag and returns its object ID.
pub fn git_tag_create_lightweight(
    repo: &mut GitRepositoryMut<'_>,
    tag_name: &CStr,
    target: GitObjectRef<'_>,
    force: bool,
) -> Result<Oid, i32> {
    let mut oid = Oid::zeroed();
    // SAFETY: the output is writable layout-compatible storage; the repository
    // is live and exclusive; the string and target are live for this call.
    let status = unsafe {
        ffi::git_tag_create_lightweight(
            core::ptr::addr_of_mut!(oid).cast(),
            repo.as_mut_ptr(),
            tag_name.as_ptr(),
            target.as_ptr(),
            i32::from(force),
        )
    };
    if status == 0 { Ok(oid) } else { Err(status) }
}

/// Wraps: git_tag_delete
/// Deletes the tag reference named by `tag_name`.
pub fn git_tag_delete(repo: &mut GitRepositoryMut<'_>, tag_name: &CStr) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusive and the name is a live C
    // string retained only for this synchronous call.
    let status = unsafe { ffi::git_tag_delete(repo.as_mut_ptr(), tag_name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_tag_list
/// Returns all tag names as an owned string array.
pub fn git_tag_list(repo: &mut GitRepositoryMut<'_>) -> Result<CVal<GitStrArray>, i32> {
    let mut names = GitStrArray::new();
    let status = {
        let mut output = names.as_mut();
        // SAFETY: `output` is an empty exclusive output header and the
        // repository is live and exclusive for traversal and lazy lookup.
        unsafe { ffi::git_tag_list(output.as_mut_ptr(), repo.as_mut_ptr()) }
    };
    if status == 0 { Ok(names) } else { Err(status) }
}

/// Wraps: git_tag_list_match
/// Returns tag names matching the standard `fnmatch` pattern.
pub fn git_tag_list_match(
    repo: &mut GitRepositoryMut<'_>,
    pattern: &CStr,
) -> Result<CVal<GitStrArray>, i32> {
    let mut names = GitStrArray::new();
    let status = {
        let mut output = names.as_mut();
        // SAFETY: `output` is an empty exclusive output header; `pattern` is a
        // live C string and the repository is live for the traversal.
        unsafe { ffi::git_tag_list_match(output.as_mut_ptr(), pattern.as_ptr(), repo.as_mut_ptr()) }
    };
    if status == 0 { Ok(names) } else { Err(status) }
}

/// Wraps: git_tag_annotation_create
/// Writes an annotated tag object without creating a reference.
pub fn git_tag_annotation_create(
    repository: &mut GitRepositoryMut<'_>,
    tag_name: &CStr,
    target: GitObjectRef<'_>,
    tagger: GitSignatureRef<'_>,
    message: &CStr,
) -> Result<Oid, i32> {
    let mut oid = Oid::zeroed();
    // SAFETY: the output is writable and every typed input is live for this
    // synchronous call; libgit2 copies all strings into the new object.
    let status = unsafe {
        ffi::git_tag_annotation_create(
            core::ptr::addr_of_mut!(oid).cast(),
            repository.as_mut_ptr(),
            tag_name.as_ptr(),
            target.as_ptr(),
            tagger.as_ptr(),
            message.as_ptr(),
        )
    };
    if status == 0 { Ok(oid) } else { Err(status) }
}

/// Wraps: git_tag_create
/// Creates an annotated tag and its reference.
pub fn git_tag_create(
    repository: &mut GitRepositoryMut<'_>,
    tag_name: &CStr,
    target: GitObjectRef<'_>,
    tagger: GitSignatureRef<'_>,
    message: &CStr,
    force: bool,
) -> Result<Oid, i32> {
    let mut oid = Oid::zeroed();
    // SAFETY: as `git_tag_annotation_create`; `force` is passed as the C
    // boolean convention and no borrowed input is retained.
    let status = unsafe {
        ffi::git_tag_create(
            core::ptr::addr_of_mut!(oid).cast(),
            repository.as_mut_ptr(),
            tag_name.as_ptr(),
            target.as_ptr(),
            tagger.as_ptr(),
            message.as_ptr(),
            i32::from(force),
        )
    };
    if status == 0 { Ok(oid) } else { Err(status) }
}

/// Wraps: git_tag_foreach
/// Visits each tag reference name and transient object ID.
pub fn git_tag_foreach<C>(
    repository: &mut GitRepositoryMut<'_>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitTagForeachCallback,
{
    unsafe extern "C" fn trampoline<C: GitTagForeachCallback>(
        name: *const core::ffi::c_char,
        oid: *mut ffi::git_oid,
        payload: *mut c_void,
    ) -> i32 {
        if name.is_null() || oid.is_null() || payload.is_null() {
            return -1;
        }
        // SAFETY: the wrapper supplies this live callback for the complete
        // synchronous traversal.
        let callback = unsafe { &mut *payload.cast::<C>() };
        // SAFETY: both values are transient live inputs for this invocation.
        let name = unsafe { CStr::from_ptr(name) };
        // SAFETY: the checked non-null OID remains live for this callback.
        let oid = unsafe { OidRef::from_ptr(oid) }.expect("checked non-null");
        callback.call(name, oid)
    }

    // SAFETY: repository and callback stay live and exclusive for the
    // synchronous traversal, and no callback pointer is retained.
    let status = unsafe {
        ffi::git_tag_foreach(
            repository.as_mut_ptr(),
            Some(trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_tag_message
/// Borrows an annotated tag's optional message.
#[must_use]
pub fn git_tag_message<'a>(tag: GitTagRef<'a>) -> Option<&'a CStr> {
    // SAFETY: the tag is live and shared; a non-null result is tag-owned.
    let message = unsafe { ffi::git_tag_message(tag.as_ptr()) };
    if message.is_null() {
        None
    } else {
        // SAFETY: the non-null tag-owned message is NUL-terminated for `'a`.
        Some(unsafe { CStr::from_ptr(message) })
    }
}

/// Wraps: git_tag_name
/// Borrows an annotated tag's name.
#[must_use]
pub fn git_tag_name<'a>(tag: GitTagRef<'a>) -> &'a CStr {
    // SAFETY: a complete live tag has an initialized NUL-terminated name.
    let name = unsafe { ffi::git_tag_name(tag.as_ptr()) };
    assert!(!name.is_null(), "a complete tag has a name");
    // SAFETY: checked non-null and owned by the tag for `'a`.
    unsafe { CStr::from_ptr(name) }
}

/// Wraps: git_tag_peel
/// Peels an annotated tag to its final repository-backed object.
pub fn git_tag_peel<'a>(tag: GitTagRef<'a>) -> Result<RepositoryObject<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and the shared tag stays live while
    // libgit2 resolves and returns one owned object count.
    let status = unsafe { ffi::git_tag_peel(core::ptr::addr_of_mut!(output), tag.as_ptr()) };
    // SAFETY: the output is null on failure or one complete owned count.
    let output = unsafe { GitObjectOwned::from_raw(output) };
    adopt_repository_object(status, output)
}

/// Wraps: git_tag_tagger
/// Borrows an annotated tag's optional tagger signature.
#[must_use]
pub fn git_tag_tagger<'a>(tag: GitTagRef<'a>) -> Option<GitSignatureRef<'a>> {
    // SAFETY: the tag is live and shared; the result is null or tag-owned.
    let tagger = unsafe { ffi::git_tag_tagger(tag.as_ptr()) };
    // SAFETY: a non-null result remains live with the tag for `'a`.
    unsafe { GitSignatureRef::from_ptr(tagger.cast_mut()) }
}

/// Wraps: git_tag_target
/// Resolves the tag's immediate repository-backed target object.
pub fn git_tag_target<'a>(tag: GitTagRef<'a>) -> Result<RepositoryObject<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and the shared tag remains live while
    // libgit2 returns one owned target count.
    let status = unsafe { ffi::git_tag_target(core::ptr::addr_of_mut!(output), tag.as_ptr()) };
    // SAFETY: the output is null on failure or a complete owned object count.
    let output = unsafe { GitObjectOwned::from_raw(output) };
    adopt_repository_object(status, output)
}

/// Wraps: git_tag_target_id
/// Borrows the tag's inline target object ID.
#[must_use]
pub fn git_tag_target_id<'a>(tag: GitTagRef<'a>) -> OidRef<'a> {
    // SAFETY: a live tag returns the non-null address of its inline target ID.
    let oid = unsafe { ffi::git_tag_target_id(tag.as_ptr()) };
    // SAFETY: the inline ID remains live for the tag borrow.
    unsafe { OidRef::from_ptr(oid.cast_mut()) }.expect("an inline OID is non-null")
}

/// Wraps: git_tag_target_type
/// Returns the checked type of the tag's immediate target.
pub fn git_tag_target_type(tag: GitTagRef<'_>) -> Result<GitObjectType, ffi::git_object_t> {
    // SAFETY: the shared tag is live and the getter retains no pointer.
    let raw = unsafe { ffi::git_tag_target_type(tag.as_ptr()) };
    GitObjectType::from_raw(raw).ok_or(raw)
}

/// Wraps: git_tag_create_from_buffer
/// Validates and writes an annotated tag from its complete serialized form.
pub fn git_tag_create_from_buffer(
    repository: &mut GitRepositoryMut<'_>,
    buffer: &CStr,
    force: bool,
) -> Result<Oid, i32> {
    let mut oid = Oid::zeroed();
    // SAFETY: `oid` is writable, the repository is exclusive for object and
    // reference updates, and `buffer` is a live NUL-terminated serialization.
    let status = unsafe {
        ffi::git_tag_create_from_buffer(
            core::ptr::addr_of_mut!(oid).cast(),
            repository.as_mut_ptr(),
            buffer.as_ptr(),
            i32::from(force),
        )
    };
    if status == 0 { Ok(oid) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct TagObservation {
        name: Vec<u8>,
        message: Vec<u8>,
        target_id: Vec<u8>,
        target_type: ffi::git_object_t,
        tag_count: usize,
        matched_count: usize,
        foreach_names: Vec<Vec<u8>>,
        valid_name: bool,
    }

    unsafe extern "C" fn collect_tag(
        name: *const core::ffi::c_char,
        _id: *mut ffi::git_oid,
        payload: *mut core::ffi::c_void,
    ) -> i32 {
        unsafe { &mut *payload.cast::<Vec<Vec<u8>>>() }
            .push(unsafe { CStr::from_ptr(name) }.to_bytes().to_vec());
        0
    }

    unsafe fn raw_tags(repository: *mut ffi::git_repository) -> TagObservation {
        let mut tag_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(&mut tag_id, repository, c"refs/tags/v1.0".as_ptr())
            },
            0
        );
        let mut tag = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_tag_lookup(&mut tag, repository, &tag_id) },
            0
        );
        let name = unsafe { CStr::from_ptr(ffi::git_tag_name(tag)) }
            .to_bytes()
            .to_vec();
        let message = unsafe { CStr::from_ptr(ffi::git_tag_message(tag)) }
            .to_bytes()
            .to_vec();
        let target_id = unsafe { (*ffi::git_tag_target_id(tag)).id.to_vec() };
        let target_type = unsafe { ffi::git_tag_target_type(tag) };
        let mut target = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_tag_peel(&mut target, tag) }, 0);
        let mut light = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_tag_create_lightweight(
                    &mut light,
                    repository,
                    c"light".as_ptr(),
                    target,
                    0,
                )
            },
            0
        );
        let mut tags = unsafe { core::mem::zeroed::<ffi::git_strarray>() };
        let mut matched = unsafe { core::mem::zeroed::<ffi::git_strarray>() };
        assert_eq!(unsafe { ffi::git_tag_list(&mut tags, repository) }, 0);
        assert_eq!(
            unsafe { ffi::git_tag_list_match(&mut matched, c"v*".as_ptr(), repository) },
            0
        );
        let mut foreach_names = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_tag_foreach(
                    repository,
                    Some(collect_tag),
                    core::ptr::from_mut(&mut foreach_names).cast(),
                )
            },
            0
        );
        foreach_names.sort();
        let mut valid = 0;
        assert_eq!(
            unsafe { ffi::git_tag_name_is_valid(&mut valid, c"release/candidate".as_ptr()) },
            0
        );
        let observation = TagObservation {
            name,
            message,
            target_id,
            target_type,
            tag_count: tags.count,
            matched_count: matched.count,
            foreach_names,
            valid_name: valid != 0,
        };
        assert_eq!(
            unsafe { ffi::git_tag_delete(repository, c"light".as_ptr()) },
            0
        );
        unsafe {
            ffi::git_strarray_dispose(&mut matched);
            ffi::git_strarray_dispose(&mut tags);
            ffi::git_object_free(target);
            ffi::git_tag_free(tag);
        }
        observation
    }

    fn safe_tags(repository: *mut ffi::git_repository) -> TagObservation {
        let mut tag_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(&mut tag_id, repository, c"refs/tags/v1.0".as_ptr())
            },
            0
        );
        let tag_id_ref = unsafe { OidRef::from_ptr(core::ptr::from_mut(&mut tag_id)) }.unwrap();
        let repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let tag = crate::object_api::git_tag_lookup(repository_view, tag_id_ref).unwrap();
        let name = git_tag_name(tag.as_ref()).to_bytes().to_vec();
        let message = git_tag_message(tag.as_ref()).unwrap().to_bytes().to_vec();
        let target_id = git_tag_target_id(tag.as_ref())
            .raw_bytes()
            .elems()
            .collect();
        let target_type = git_tag_target_type(tag.as_ref()).unwrap().as_raw();
        let target = git_tag_peel(tag.as_ref()).unwrap();
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        git_tag_create_lightweight(&mut repository_view, c"light", target.as_ref(), false).unwrap();
        let tag_count = git_tag_list(&mut repository_view).unwrap().as_ref().count();
        let matched_count = git_tag_list_match(&mut repository_view, c"v*")
            .unwrap()
            .as_ref()
            .count();
        let mut foreach_names = Vec::new();
        git_tag_foreach(&mut repository_view, &mut |name: &CStr, _id: OidRef<'_>| {
            foreach_names.push(name.to_bytes().to_vec());
            0
        })
        .unwrap();
        foreach_names.sort();
        let valid_name = git_tag_name_is_valid(Some(c"release/candidate")).unwrap();
        git_tag_delete(&mut repository_view, c"light").unwrap();
        TagObservation {
            name,
            message,
            target_id,
            target_type,
            tag_count,
            matched_count,
            foreach_names,
            valid_name,
        }
    }

    #[test]
    fn io_equiv_tag_metadata_listing_foreach_and_lightweight_lifecycle() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("tag-surface-raw");
        let safe = HistoryFixture::new("tag-surface-safe");
        let raw_observation = unsafe { raw_tags(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_tags(safe.repository.as_ptr()));
        assert_eq!(raw_observation.tag_count, 2);
        assert!(raw_observation.valid_name);
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_tag_preserves_layout_and_refcount_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_refcounted<T: CDropped + CCloned>() {}

        assert_cell::<GitTag>();
        assert_refcounted::<GitTag>();
        assert_eq!(size_of::<GitTag>(), size_of::<ffi::git_tag>());
        assert_eq!(align_of::<GitTag>(), align_of::<ffi::git_tag>());
        assert_eq!(size_of::<GitTagRef<'_>>(), size_of::<*const ffi::git_tag>());
        assert_eq!(size_of::<GitTagMut<'_>>(), size_of::<*mut ffi::git_tag>());
        assert_eq!(size_of::<GitTagOwned>(), size_of::<*mut ffi::git_tag>());
    }

    #[test]
    fn null_tag_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitTagRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitTagMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitTagOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn tag_names_are_checked_through_the_safe_surface() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(git_tag_name_is_valid(Some(c"v1.0")), Ok(true));
        assert_eq!(git_tag_name_is_valid(None), Ok(false));
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
