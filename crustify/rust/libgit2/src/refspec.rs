//! Safe wrappers for libgit2 refspec APIs.

use core::ffi::CStr;

use ffibox::CBox;

use crate::api::buffer::GitBufMut;
use crate::ffi;
use crate::util::net::{Direction, InvalidDirection};

ffibox::define_ctype!(
    /// Wraps: git_refspec
    /// An opaque mapping between local and remote reference names.
    ///
    /// The public C API keeps the layout private. An owned handle represents
    /// a complete refspec allocated by libgit2 and releases it with
    /// `git_refspec_free`.
    GitRefspec,
    GitRefspecRef,
    GitRefspecMut,
    ffi::git_refspec
);

/// An owned libgit2 refspec allocation.
pub type GitRefspecOwned = CBox<GitRefspec>;

// SAFETY: `git_refspec_free` is the public destructor for a complete
// libgit2-allocated `git_refspec`. It disposes all owned strings and frees the
// header exactly once. It accepts null, although `CBox` supplies a live,
// non-null allocation.
ffibox::impl_dropped!(GitRefspec, ffi::git_refspec, ffi::git_refspec_free);

/// Wraps: git_refspec_direction
/// Returns whether this refspec is used for fetching or pushing.
pub fn git_refspec_direction(spec: GitRefspecRef<'_>) -> Result<Direction, InvalidDirection> {
    // SAFETY: `spec` is live and shared and the getter retains no pointer.
    Direction::try_from(unsafe { ffi::git_refspec_direction(spec.as_ptr()) })
}

/// Wraps: git_refspec_dst
/// Borrows the optional destination descriptor.
#[must_use]
pub fn git_refspec_dst<'a>(refspec: GitRefspecRef<'a>) -> Option<&'a CStr> {
    // SAFETY: the getter returns null or a refspec-owned NUL string that lives
    // for the handle borrow.
    let string = unsafe { ffi::git_refspec_dst(refspec.as_ptr()) };
    if string.is_null() {
        None
    } else {
        // SAFETY: non-null is the live NUL string described above.
        Some(unsafe { CStr::from_ptr(string) })
    }
}

/// Wraps: git_refspec_dst_matches
/// Checks whether a reference name matches the destination descriptor.
#[must_use]
pub fn git_refspec_dst_matches(refspec: GitRefspecRef<'_>, refname: &CStr) -> bool {
    // SAFETY: both inputs are live and shared and neither is retained.
    unsafe { ffi::git_refspec_dst_matches(refspec.as_ptr(), refname.as_ptr()) != 0 }
}

/// Wraps: git_refspec_force
/// Returns whether updates selected by this refspec are forced.
#[must_use]
pub fn git_refspec_force(refspec: GitRefspecRef<'_>) -> bool {
    // SAFETY: `refspec` is live and shared and the getter retains no pointer.
    unsafe { ffi::git_refspec_force(refspec.as_ptr()) != 0 }
}

/// Wraps: git_refspec_parse
/// Parses `input` into an independently owned refspec.
///
/// `is_fetch` selects the fetch grammar; `false` parses a push refspec.
pub fn git_refspec_parse(input: &CStr, is_fetch: bool) -> Result<GitRefspecOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: `raw` is a writable output slot and `input` is a live
    // NUL-terminated string that libgit2 only reads, copying whatever the
    // result keeps. libgit2 clears the slot on entry and frees its allocation
    // on every failure path, so the slot is null unless the call succeeds.
    let status = unsafe { ffi::git_refspec_parse(&mut raw, input.as_ptr(), i32::from(is_fetch)) };
    if status == 0 {
        // SAFETY: success transfers one complete refspec allocation, released
        // exactly once by the `CDropped` implementation above.
        Ok(unsafe { GitRefspecOwned::from_raw(raw) }
            .expect("libgit2 succeeded without returning a refspec"))
    } else {
        Err(status)
    }
}

/// Wraps: git_refspec_rtransform
/// Reverse-transforms a destination name into `out`.
pub fn git_refspec_rtransform(
    out: &mut GitBufMut<'_>,
    spec: GitRefspecRef<'_>,
    name: &CStr,
) -> Result<(), i32> {
    // SAFETY: `out` is live and exclusive, while `spec` and `name` are live
    // shared inputs; libgit2 retains none of the pointers.
    let status =
        unsafe { ffi::git_refspec_rtransform(out.as_mut_ptr(), spec.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_refspec_src
/// Borrows the optional source descriptor.
#[must_use]
pub fn git_refspec_src<'a>(refspec: GitRefspecRef<'a>) -> Option<&'a CStr> {
    // SAFETY: the getter returns null or a refspec-owned NUL string that lives
    // for the handle borrow.
    let string = unsafe { ffi::git_refspec_src(refspec.as_ptr()) };
    if string.is_null() {
        None
    } else {
        // SAFETY: non-null is the live NUL string described above.
        Some(unsafe { CStr::from_ptr(string) })
    }
}

/// Wraps: git_refspec_src_matches
/// Checks whether a reference name matches the source descriptor.
#[must_use]
pub fn git_refspec_src_matches(refspec: GitRefspecRef<'_>, refname: &CStr) -> bool {
    // SAFETY: both inputs are live and shared and neither is retained.
    unsafe { ffi::git_refspec_src_matches(refspec.as_ptr(), refname.as_ptr()) != 0 }
}

/// Wraps: git_refspec_string
/// Borrows the original textual refspec.
#[must_use]
pub fn git_refspec_string<'a>(refspec: GitRefspecRef<'a>) -> &'a CStr {
    // SAFETY: a fully formed refspec owns a non-null NUL-terminated original
    // string that remains live for the handle borrow.
    let string = unsafe { ffi::git_refspec_string(refspec.as_ptr()) };
    assert!(
        !string.is_null(),
        "a complete refspec has an original string"
    );
    // SAFETY: the assertion and refspec invariant establish a live C string.
    unsafe { CStr::from_ptr(string) }
}

/// Wraps: git_refspec_transform
/// Transforms a source name into `out`.
pub fn git_refspec_transform(
    out: &mut GitBufMut<'_>,
    spec: GitRefspecRef<'_>,
    name: &CStr,
) -> Result<(), i32> {
    // SAFETY: `out` is live and exclusive, while `spec` and `name` are live
    // shared inputs; libgit2 retains none of the pointers.
    let status =
        unsafe { ffi::git_refspec_transform(out.as_mut_ptr(), spec.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_refspec_src_matches_negative
/// Reports whether a negative refspec excludes `refname`.
#[must_use]
pub fn git_refspec_src_matches_negative(refspec: GitRefspecRef<'_>, refname: &CStr) -> bool {
    // SAFETY: both inputs are live and shared for the synchronous match.
    unsafe { ffi::git_refspec_src_matches_negative(refspec.as_ptr(), refname.as_ptr()) != 0 }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_matches_the_c_seam() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitRefspec>();
        assert_dropped::<GitRefspec>();
        assert_eq!(size_of::<GitRefspec>(), size_of::<ffi::git_refspec>());
        assert_eq!(align_of::<GitRefspec>(), align_of::<ffi::git_refspec>());
        assert_eq!(
            size_of::<GitRefspecRef<'_>>(),
            size_of::<*const ffi::git_refspec>()
        );
        assert_eq!(
            size_of::<GitRefspecMut<'_>>(),
            size_of::<*mut ffi::git_refspec>()
        );
        assert_eq!(
            size_of::<GitRefspecOwned>(),
            size_of::<*mut ffi::git_refspec>()
        );
    }

    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and refcounted;
            // `Drop` below balances this successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard. Refspec owners are dropped before the guard.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    #[test]
    fn parsed_refspec_owns_its_strings_until_drop() {
        let _init = Libgit2Init::acquire();
        let spec = git_refspec_parse(c"+refs/heads/*:refs/remotes/origin/*", true)
            .expect("a well-formed fetch refspec parses");

        let shared = spec.as_ref();
        assert_eq!(
            git_refspec_string(shared),
            c"+refs/heads/*:refs/remotes/origin/*"
        );
        assert_eq!(git_refspec_src(shared), Some(c"refs/heads/*"));
        assert_eq!(git_refspec_dst(shared), Some(c"refs/remotes/origin/*"));
        assert!(git_refspec_force(shared));
        assert_eq!(git_refspec_direction(shared), Ok(Direction::Fetch));
        assert!(git_refspec_src_matches(shared, c"refs/heads/main"));
        assert!(git_refspec_dst_matches(shared, c"refs/remotes/origin/main"));
        assert!(!git_refspec_src_matches(shared, c"refs/tags/v1"));

        // `git_refspec_free` releases the three strings and the header here.
        drop(spec);
    }

    #[test]
    fn a_push_refspec_reports_the_push_direction() {
        let _init = Libgit2Init::acquire();
        let spec = git_refspec_parse(c"refs/heads/main:refs/heads/main", false)
            .expect("a well-formed push refspec parses");

        assert_eq!(git_refspec_direction(spec.as_ref()), Ok(Direction::Push));
        assert!(!git_refspec_force(spec.as_ref()));
    }

    #[test]
    fn an_invalid_refspec_transfers_no_owner() {
        let _init = Libgit2Init::acquire();
        assert!(git_refspec_parse(c"refs/heads/*", true).is_err());
    }

    #[test]
    fn null_seams_create_no_refspec_handles() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitRefspecRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRefspecMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRefspecOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
