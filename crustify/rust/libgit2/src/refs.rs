//! Safe wrappers for libgit2 refs APIs.

use core::cmp::Ordering;
use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::NonNull;

use ffibox::{CBox, CCloned, CDropped};

use crate::api::types::{GitObjectType, GitReferenceType, InvalidGitReferenceType};
use crate::ffi;
use crate::object::{GitObjectOwned, GitObjectRef};
use crate::oid::{Oid, OidRef};
use crate::repository::{GitRepositoryMut, GitRepositoryRef};
use crate::sys::refdb_backend::{GitReferenceIteratorMut, GitReferenceIteratorOwned};

ffibox::define_ctype!(
    /// Wraps: git_reference
    /// An opaque Git reference managed by libgit2.
    ///
    /// Owned handles release references with `git_reference_free`. Cloning
    /// performs the public `git_reference_dup` deep copy, including an
    /// independent reference to the reference database.
    GitReference,
    GitReferenceRef,
    GitReferenceMut,
    ffi::git_reference
);

/// A raw-adopted owned libgit2 reference. Creating this owner from a pointer is
/// unsafe because the caller must separately keep its repository alive: a
/// reference counts its reference database, and that database only borrows the
/// repository it was opened from.
///
/// A safe wrapper that hands a newly constructed reference to its caller
/// therefore returns [`GitReferenceTetheredOwned`], which carries the
/// repository borrow in its type, rather than this bare owner.
pub type GitReferenceOwned = CBox<GitReference>;

/// An owned libgit2 reference tied to the repository or reference that keeps
/// its reference database's borrowed repository pointer alive.
pub struct GitReferenceTetheredOwned<'a> {
    inner: CBox<GitReference>,
    _keepalive: PhantomData<GitRepositoryRef<'a>>,
}

impl GitReferenceTetheredOwned<'_> {
    /// Borrows the reference.
    #[must_use]
    pub fn as_ref(&self) -> GitReferenceRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the reference exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitReferenceMut<'_> {
        self.inner.as_mut()
    }
}

impl Clone for GitReferenceTetheredOwned<'_> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _keepalive: PhantomData,
        }
    }
}

/// An object peeled through a reference, tied to the repository behind that
/// reference.
pub struct GitReferencePeeledObject<'a> {
    inner: GitObjectOwned,
    _keepalive: PhantomData<GitObjectRef<'a>>,
}

impl GitReferencePeeledObject<'_> {
    /// Borrows the peeled object.
    #[must_use]
    pub fn as_ref(&self) -> GitObjectRef<'_> {
        self.inner.as_ref()
    }
}

/// Wraps: git_reference_free
// SAFETY: `git_reference_free` is the public destructor for a complete
// `git_reference` allocation and accepts null, although `CDropped` supplies a
// live non-null allocation. `GitReference` is transparent over the matching
// bindgen type.
unsafe impl CDropped for GitReference {
    unsafe fn c_drop(object: NonNull<Self>) {
        // SAFETY: the trait contract supplies one complete live reference and
        // the wrapper is transparent over `ffi::git_reference`.
        unsafe { ffi::git_reference_free(object.as_ptr().cast()) }
    }
}

// SAFETY: `git_reference_dup` leaves its live source unchanged and, on
// success, writes a fresh fully initialized allocation that is independently
// releasable by `git_reference_free`. It reads `source->db` unconditionally,
// so an owner reaching this strategy must hold a reference attached to a
// reference database, the only shape libgit2's public constructors produce.
// The copy takes its own count on that same database and therefore inherits
// the source's repository dependency; that count is the only state the call
// mutates, and no Rust reference covers it.
unsafe impl CCloned for GitReference {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live source. `duplicate` is
        // a valid output slot, and `GitReference` is layout-compatible with
        // `ffi::git_reference`.
        let result = unsafe {
            ffi::git_reference_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_reference>(),
            )
        };

        if result == 0 {
            NonNull::new(duplicate.cast::<Self>())
        } else {
            debug_assert!(duplicate.is_null());
            None
        }
    }
}

/// Wraps: git_reference_name_is_valid
/// Checks a required NUL-terminated reference name.
pub fn git_reference_name_is_valid(refname: &core::ffi::CStr) -> Result<bool, i32> {
    let mut valid = 0;
    // SAFETY: `valid` is writable and `refname` is a live C string.
    let status = unsafe { ffi::git_reference_name_is_valid(&mut valid, refname.as_ptr()) };
    if status == 0 {
        Ok(valid != 0)
    } else {
        Err(status)
    }
}

/// Wraps: git_reference_normalize_name
/// Normalizes `name` into `buffer` and returns the resulting C string.
///
/// The numeric `flags` are libgit2's reference-format bit set.
///
/// An empty output buffer is rejected as [`GIT_EBUFS`] without calling C,
/// whose capacity check computes `buffer_size - 1` and so wraps to `SIZE_MAX`
/// for a zero-length buffer; the call then reaches `git_str_copy_cstr`, which
/// rejects a zero capacity through an argument assertion that aborts in a
/// hard-assert build.
///
/// [`GIT_EBUFS`]: ffi::git_error_code_GIT_EBUFS
pub fn git_reference_normalize_name<'a>(
    buffer: &'a mut [u8],
    name: &core::ffi::CStr,
    flags: u32,
) -> Result<&'a core::ffi::CStr, i32> {
    if buffer.is_empty() {
        return Err(ffi::git_error_code_GIT_EBUFS);
    }
    // SAFETY: `buffer` is a writable run of `buffer.len()` bytes and `name` is
    // a live C string. Libgit2 retains neither pointer and writes a terminating
    // NUL on success.
    let status = unsafe {
        ffi::git_reference_normalize_name(
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            name.as_ptr(),
            flags,
        )
    };
    if status != 0 {
        return Err(status);
    }
    core::ffi::CStr::from_bytes_until_nul(buffer).map_err(|_| ffi::git_error_code_GIT_EBUFS)
}

/// Adopts a successful constructor result as a repository-tethered owner.
///
/// `'a` is constrained by no argument: every caller must bind it to the borrow
/// of the repository, or of the reference whose database it shares, that the
/// new reference depends on, so the tether the returned owner claims is real.
pub(crate) fn adopt_optional_reference<'a>(
    inner: Option<CBox<GitReference>>,
) -> Option<GitReferenceTetheredOwned<'a>> {
    inner.map(|inner| GitReferenceTetheredOwned {
        inner,
        _keepalive: PhantomData,
    })
}

pub(crate) fn adopt_reference<'a>(
    status: i32,
    inner: Option<CBox<GitReference>>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    if status != 0 {
        return Err(status);
    }
    let inner = inner.expect("a successful reference constructor returns a non-null owner");
    Ok(GitReferenceTetheredOwned {
        inner,
        _keepalive: PhantomData,
    })
}

/// Borrows an optional C string returned by a libgit2 reference getter.
///
/// # Safety
///
/// A non-null `raw` must address a NUL-terminated string that stays live and
/// unmodified for the whole of `'a`, which the caller must bind to the handle
/// that owns it.
unsafe fn optional_cstr<'a>(raw: *const core::ffi::c_char) -> Option<&'a CStr> {
    if raw.is_null() {
        None
    } else {
        // SAFETY: the caller guarantees a live NUL-terminated string valid for
        // `'a`, which the null check above narrows to the non-null case.
        Some(unsafe { CStr::from_ptr(raw) })
    }
}

/// Wraps: git_reference_cmp
/// Compares two references using libgit2's stable ordering.
#[must_use]
pub fn git_reference_cmp(left: GitReferenceRef<'_>, right: GitReferenceRef<'_>) -> Ordering {
    // SAFETY: both references are live shared inputs retained only for the call.
    let result = unsafe { ffi::git_reference_cmp(left.as_ptr(), right.as_ptr()) };
    result.cmp(&0)
}

/// Wraps: git_reference_create
/// Creates or replaces a direct reference.
pub fn git_reference_create<'a>(
    repository: &'a mut GitRepositoryMut<'_>,
    name: &CStr,
    id: OidRef<'_>,
    force: bool,
    log_message: Option<&CStr>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and every borrowed input is live and
    // NUL-terminated where required. The resulting reference retains a refdb
    // whose repository borrow is bounded by `'a`.
    let status = unsafe {
        ffi::git_reference_create(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            name.as_ptr(),
            id.as_ptr(),
            i32::from(force),
            log_message.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    // SAFETY: `output` is null or a complete caller-owned reference produced
    // by this constructor, including on a later error path.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_create_matching
/// Conditionally creates a direct reference when its current target matches.
pub fn git_reference_create_matching<'a>(
    repository: &'a mut GitRepositoryMut<'_>,
    name: &CStr,
    id: OidRef<'_>,
    force: bool,
    current_id: Option<OidRef<'_>>,
    log_message: Option<&CStr>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    let current_id = current_id.map_or(core::ptr::null(), |id| id.as_ptr());
    // SAFETY: as `git_reference_create`; `current_id` is null or a live shared
    // OID used only for the conditional comparison.
    let status = unsafe {
        ffi::git_reference_create_matching(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            name.as_ptr(),
            id.as_ptr(),
            i32::from(force),
            current_id,
            log_message.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_delete
/// Deletes the on-disk reference without consuming its in-memory handle.
pub fn git_reference_delete(reference: &mut GitReferenceMut<'_>) -> Result<(), i32> {
    // SAFETY: the reference is live and, matching the non-const C parameter,
    // exclusively borrowed for the duration of the reference-database update.
    // Libgit2 reads its name and target and frees nothing: the allocation
    // remains owned by the caller and outlives the call.
    let status = unsafe { ffi::git_reference_delete(reference.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_reference_dup
/// Deep-copies a reference while preserving its repository keepalive lifetime.
pub fn git_reference_dup<'a>(
    source: GitReferenceRef<'a>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: `source` is live, is left unchanged, and `output` is a writable
    // slot for the independently owned deep copy.
    let status = unsafe {
        ffi::git_reference_dup(core::ptr::addr_of_mut!(output), source.as_ptr().cast_mut())
    };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_dwim
/// Resolves a shorthand reference name using Git's precedence rules.
pub fn git_reference_dwim<'a>(
    repository: &'a mut GitRepositoryMut<'_>,
    shorthand: &CStr,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot and both borrowed inputs are live for the call.
    let status = unsafe {
        ffi::git_reference_dwim(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            shorthand.as_ptr(),
        )
    };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_ensure_log
/// Ensures future updates to `name` append to a reflog.
pub fn git_reference_ensure_log(
    repository: &mut GitRepositoryMut<'_>,
    name: &CStr,
) -> Result<(), i32> {
    // SAFETY: both inputs are live for the call and no pointer is retained.
    let status = unsafe { ffi::git_reference_ensure_log(repository.as_mut_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_reference_has_log
/// Reports whether `name` has a reflog.
pub fn git_reference_has_log(
    repository: &mut GitRepositoryMut<'_>,
    name: &CStr,
) -> Result<bool, i32> {
    // SAFETY: both inputs are live for the call and no pointer is retained.
    let status = unsafe { ffi::git_reference_has_log(repository.as_mut_ptr(), name.as_ptr()) };
    match status {
        0 => Ok(false),
        1 => Ok(true),
        error => Err(error),
    }
}

/// Wraps: git_reference_is_branch
#[must_use]
pub fn git_reference_is_branch(reference: GitReferenceRef<'_>) -> bool {
    // SAFETY: the shared reference is live and retained only for the call.
    unsafe { ffi::git_reference_is_branch(reference.as_ptr()) != 0 }
}

/// Wraps: git_reference_is_note
#[must_use]
pub fn git_reference_is_note(reference: GitReferenceRef<'_>) -> bool {
    // SAFETY: the shared reference is live and retained only for the call.
    unsafe { ffi::git_reference_is_note(reference.as_ptr()) != 0 }
}

/// Wraps: git_reference_is_remote
#[must_use]
pub fn git_reference_is_remote(reference: GitReferenceRef<'_>) -> bool {
    // SAFETY: the shared reference is live and retained only for the call.
    unsafe { ffi::git_reference_is_remote(reference.as_ptr()) != 0 }
}

/// Wraps: git_reference_is_tag
#[must_use]
pub fn git_reference_is_tag(reference: GitReferenceRef<'_>) -> bool {
    // SAFETY: the shared reference is live and retained only for the call.
    unsafe { ffi::git_reference_is_tag(reference.as_ptr()) != 0 }
}

/// Wraps: git_reference_lookup
/// Looks up a full reference name.
pub fn git_reference_lookup<'a>(
    repository: &'a mut GitRepositoryMut<'_>,
    name: &CStr,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and both borrowed inputs are live.
    let status = unsafe {
        ffi::git_reference_lookup(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            name.as_ptr(),
        )
    };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_name
/// Borrows the full reference name.
#[must_use]
pub fn git_reference_name<'a>(reference: GitReferenceRef<'a>) -> &'a CStr {
    // SAFETY: the shared reference stays live for `'a`; its inline name is a
    // valid NUL-terminated string.
    let name = unsafe { ffi::git_reference_name(reference.as_ptr()) };
    // SAFETY: the reference's inline name lives as long as the handle's `'a`.
    unsafe { optional_cstr(name) }.expect("a live reference has a name")
}

/// Wraps: git_reference_name_to_id
/// Resolves a reference name directly into an owned object-ID value.
pub fn git_reference_name_to_id(
    repository: &mut GitRepositoryMut<'_>,
    name: &CStr,
) -> Result<Oid, i32> {
    let mut output = Oid::zeroed();
    // SAFETY: `output` is writable layout-compatible OID storage and both
    // borrowed inputs are live for the call.
    let status = unsafe {
        ffi::git_reference_name_to_id(
            core::ptr::addr_of_mut!(output).cast(),
            repository.as_mut_ptr(),
            name.as_ptr(),
        )
    };
    if status == 0 { Ok(output) } else { Err(status) }
}

/// Wraps: git_reference_peel
/// Peels a reference to an owned object tied to the same repository lifetime.
pub fn git_reference_peel<'a>(
    reference: GitReferenceRef<'a>,
    object_type: GitObjectType,
) -> Result<GitReferencePeeledObject<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: `output` is writable, `reference` is live, and `object_type` is
    // one of the validated public C values.
    let status = unsafe {
        ffi::git_reference_peel(
            core::ptr::addr_of_mut!(output),
            reference.as_ptr(),
            object_type.into(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one owned object reference, whose repository
    // dependency is preserved by the returned lifetime.
    let inner = unsafe { GitObjectOwned::from_raw(output) }
        .expect("a successful peel returns a non-null object");
    Ok(GitReferencePeeledObject {
        inner,
        _keepalive: PhantomData,
    })
}

/// Wraps: git_reference_rename
/// Renames a reference and returns the resulting owned snapshot.
pub fn git_reference_rename<'a>(
    reference: &'a mut GitReferenceMut<'_>,
    new_name: &CStr,
    force: bool,
    log_message: Option<&CStr>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the exclusive reference and borrowed strings are live, and the
    // writable output receives a new independently owned snapshot.
    let status = unsafe {
        ffi::git_reference_rename(
            core::ptr::addr_of_mut!(output),
            reference.as_mut_ptr(),
            new_name.as_ptr(),
            i32::from(force),
            log_message.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_resolve
/// Resolves a symbolic reference to an owned direct reference.
pub fn git_reference_resolve<'a>(
    reference: GitReferenceRef<'a>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: `reference` is live and shared and `output` is writable.
    let status =
        unsafe { ffi::git_reference_resolve(core::ptr::addr_of_mut!(output), reference.as_ptr()) };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_set_target
/// Replaces a direct reference target and returns the resulting snapshot.
pub fn git_reference_set_target<'a>(
    reference: &'a mut GitReferenceMut<'_>,
    id: OidRef<'_>,
    log_message: Option<&CStr>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: all borrowed inputs are live and `output` is writable.
    let status = unsafe {
        ffi::git_reference_set_target(
            core::ptr::addr_of_mut!(output),
            reference.as_mut_ptr(),
            id.as_ptr(),
            log_message.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_shorthand
/// Borrows the human-readable shorthand for a reference.
#[must_use]
pub fn git_reference_shorthand<'a>(reference: GitReferenceRef<'a>) -> &'a CStr {
    // SAFETY: the returned pointer aliases the live reference's inline name.
    let shorthand = unsafe { ffi::git_reference_shorthand(reference.as_ptr()) };
    // SAFETY: the shorthand points into the reference's inline name, which
    // lives as long as the handle's `'a`.
    unsafe { optional_cstr(shorthand) }.expect("a live reference has a shorthand")
}

/// Wraps: git_reference_symbolic_create
/// Creates or replaces a symbolic reference.
pub fn git_reference_symbolic_create<'a>(
    repository: &'a mut GitRepositoryMut<'_>,
    name: &CStr,
    target: &CStr,
    force: bool,
    log_message: Option<&CStr>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and all string inputs are live C strings.
    let status = unsafe {
        ffi::git_reference_symbolic_create(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            name.as_ptr(),
            target.as_ptr(),
            i32::from(force),
            log_message.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_symbolic_create_matching
/// Conditionally creates a symbolic reference when its target matches.
pub fn git_reference_symbolic_create_matching<'a>(
    repository: &'a mut GitRepositoryMut<'_>,
    name: &CStr,
    target: &CStr,
    force: bool,
    current_target: Option<&CStr>,
    log_message: Option<&CStr>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: as `git_reference_symbolic_create`; `current_target` is null or
    // a live C string used only for the conditional comparison.
    let status = unsafe {
        ffi::git_reference_symbolic_create_matching(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            name.as_ptr(),
            target.as_ptr(),
            i32::from(force),
            current_target.map_or(core::ptr::null(), CStr::as_ptr),
            log_message.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_symbolic_set_target
/// Replaces a symbolic reference target and returns the resulting snapshot.
pub fn git_reference_symbolic_set_target<'a>(
    reference: &'a mut GitReferenceMut<'_>,
    target: &CStr,
    log_message: Option<&CStr>,
) -> Result<GitReferenceTetheredOwned<'a>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the exclusive reference and both optional/required C strings are
    // live, and `output` is writable.
    let status = unsafe {
        ffi::git_reference_symbolic_set_target(
            core::ptr::addr_of_mut!(output),
            reference.as_mut_ptr(),
            target.as_ptr(),
            log_message.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    // SAFETY: `output` has the transferred-output contract described above.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_reference_symbolic_target
/// Borrows a symbolic target name, or returns `None` for a direct reference.
#[must_use]
pub fn git_reference_symbolic_target<'a>(reference: GitReferenceRef<'a>) -> Option<&'a CStr> {
    // SAFETY: a non-null result aliases the symbolic target string owned by
    // the live reference, which lives as long as the handle's `'a`.
    unsafe { optional_cstr(ffi::git_reference_symbolic_target(reference.as_ptr())) }
}

/// Wraps: git_reference_target
/// Borrows a direct target OID, or returns `None` for a symbolic reference.
#[must_use]
pub fn git_reference_target<'a>(reference: GitReferenceRef<'a>) -> Option<OidRef<'a>> {
    // SAFETY: a non-null result aliases the initialized inline target OID of
    // the live reference.
    let target = unsafe { ffi::git_reference_target(reference.as_ptr()) };
    // SAFETY: the optional handle is tied to the source reference's `'a`.
    unsafe { OidRef::from_ptr(target.cast_mut()) }
}

/// Wraps: git_reference_target_peel
/// Borrows the cached peeled object ID of a direct reference, when present.
#[must_use]
pub fn git_reference_target_peel<'a>(reference: GitReferenceRef<'a>) -> Option<OidRef<'a>> {
    // SAFETY: `reference` is live and libgit2 returns either null or the
    // address of its inline peeled OID, which remains valid for the same
    // borrow and is exposed read-only.
    let oid = unsafe { ffi::git_reference_target_peel(reference.as_ptr()) }.cast_mut();
    // SAFETY: a non-null result is the live inline OID described above.
    unsafe { OidRef::from_ptr(oid) }
}

/// Wraps: git_reference_type
/// Returns the published kind of a live reference.
pub fn git_reference_type(
    reference: GitReferenceRef<'_>,
) -> Result<GitReferenceType, InvalidGitReferenceType> {
    // SAFETY: `reference` is a live shared handle and the getter retains no
    // pointer.
    GitReferenceType::try_from(unsafe { ffi::git_reference_type(reference.as_ptr()) })
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::{CCloned, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        // `struct git_reference` is defined in the private `src/libgit2/refs.h`
        // and ends in a flexible array member, so the binding is an opaque
        // marker: no field is reachable and the object is heap-only.
        assert_eq!(size_of::<ffi::git_reference>(), 0);
        assert_eq!(size_of::<GitReference>(), size_of::<ffi::git_reference>());
        assert_eq!(align_of::<GitReference>(), align_of::<ffi::git_reference>());
        assert_eq!(
            size_of::<GitReferenceRef<'_>>(),
            size_of::<*const ffi::git_reference>()
        );
        assert_eq!(
            size_of::<GitReferenceMut<'_>>(),
            size_of::<*mut ffi::git_reference>()
        );
        assert_eq!(
            size_of::<Option<GitReferenceOwned>>(),
            size_of::<*mut ffi::git_reference>()
        );
    }

    #[test]
    fn reference_names_validate_and_normalize_into_rust_storage() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(git_reference_name_is_valid(c"refs/heads/main"), Ok(true));
        let mut buffer = [0; 64];
        assert_eq!(
            git_reference_normalize_name(&mut buffer, c"refs//heads/main", 0),
            Ok(c"refs/heads/main")
        );
        assert_eq!(
            git_reference_normalize_name(&mut [], c"refs/heads/main", 0),
            Err(ffi::git_error_code_GIT_EBUFS)
        );
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn reference_registers_deep_copy_lifecycle() {
        fn assert_lifecycle<T: CDropped + CCloned>() {}
        assert_lifecycle::<GitReference>();
    }

    #[test]
    fn borrowed_handles_preserve_the_reference_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_reference>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_reference>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage,
            // and the shared handle remains within this scope.
            let shared = unsafe { GitReferenceRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitReferenceMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from `Box::into_raw`, no handle remains, and this
        // cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_reference>>()) });
    }

    #[test]
    fn tethered_owner_is_covariant_in_its_repository_borrow() {
        fn shrink<'short, 'long: 'short>(
            owner: GitReferenceTetheredOwned<'long>,
        ) -> GitReferenceTetheredOwned<'short> {
            owner
        }

        // A keepalive marker may only narrow: `shrink` compiling proves the
        // tether cannot be widened past the repository borrow it records.
        let _ = shrink::<'_, 'static>;
    }

    #[test]
    fn failed_result_accepts_an_empty_typed_owner() {
        assert!(matches!(adopt_reference(-123, None), Err(-123)));
    }
}

/// Wraps: git_reference_iterator_glob_new
/// Creates an owned reference iterator filtered by `glob`.
pub fn git_reference_iterator_glob_new(
    repository: &mut GitRepositoryMut<'_>,
    glob: &CStr,
) -> Result<GitReferenceIteratorOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable, the repository is exclusively
    // available for lazy refdb initialization, and `glob` is copied by the
    // backend before returning. The iterator acquires its own refdb count.
    let status = unsafe {
        ffi::git_reference_iterator_glob_new(
            core::ptr::addr_of_mut!(output),
            repository.as_mut_ptr(),
            glob.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete iterator allocation.
    unsafe { GitReferenceIteratorOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_reference_iterator_new
/// Creates an owned iterator over all repository references.
pub fn git_reference_iterator_new(
    repository: &mut GitRepositoryMut<'_>,
) -> Result<GitReferenceIteratorOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and the exclusive repository handle
    // permits lazy refdb initialization. The result owns a refdb count.
    let status = unsafe {
        ffi::git_reference_iterator_new(core::ptr::addr_of_mut!(output), repository.as_mut_ptr())
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete iterator allocation.
    unsafe { GitReferenceIteratorOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_reference_next
/// Advances an iterator and returns an independently owned reference.
pub fn git_reference_next(
    iterator: &mut GitReferenceIteratorMut<'_>,
) -> Result<GitReferenceOwned, i32> {
    iterator.next_reference()
}

/// Wraps: git_reference_next_name
/// Advances an iterator and borrows its current backend-owned name.
pub fn git_reference_next_name<'a>(
    iterator: &'a mut GitReferenceIteratorMut<'_>,
) -> Result<&'a CStr, i32> {
    iterator.next_name()
}
