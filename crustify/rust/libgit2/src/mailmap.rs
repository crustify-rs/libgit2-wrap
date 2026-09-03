//! Safe wrappers for libgit2 mailmap APIs.
//!
//! Every constructor here allocates through libgit2's configured allocator,
//! which fails each request until `git_libgit2_init` has run.

use core::ffi::CStr;

use ffibox::CBox;

use crate::api::types::{GitSignatureOwned, GitSignatureRef};
use crate::ffi;
use crate::repository::GitRepositoryRef;

ffibox::define_ctype!(
    /// Wraps: git_mailmap
    /// Opaque mailmap state managed by libgit2.
    ///
    /// A mailmap owns every entry it holds and borrows nothing once built:
    /// `git_mailmap_add_entry` duplicates its four strings, and both
    /// `git_mailmap_from_buffer` and `git_mailmap_from_repository` parse
    /// borrowed input into copies. A completed mailmap therefore outlives the
    /// buffer or repository it was built from, which is why
    /// [`GitMailmapOwned`] carries no lifetime.
    ///
    /// Owned mailmaps are represented by [`GitMailmapOwned`] and released by
    /// `git_mailmap_free`.
    GitMailmap,
    GitMailmapRef,
    GitMailmapMut,
    ffi::git_mailmap
);

/// Wraps: git_mailmap_free
/// An exclusively owned libgit2 mailmap.
pub type GitMailmapOwned = CBox<GitMailmap>;

// SAFETY: `git_mailmap_free` is the public destructor for a fully initialized
// `git_mailmap`. It returns on null and otherwise frees each entry and its
// duplicated strings, disposes the `entries` vector and then frees the header,
// leaving nothing shared with another owner. `CBox` always supplies one live
// non-null pointer exactly once.
ffibox::impl_dropped!(GitMailmap, ffi::git_mailmap, ffi::git_mailmap_free);

#[cfg(test)]
mod unit_tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::CDropped;

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<GitMailmap>(), size_of::<ffi::git_mailmap>());
        assert_eq!(align_of::<GitMailmap>(), align_of::<ffi::git_mailmap>());
        assert_eq!(
            size_of::<GitMailmapRef<'_>>(),
            size_of::<*const ffi::git_mailmap>()
        );
        assert_eq!(
            size_of::<GitMailmapMut<'_>>(),
            size_of::<*mut ffi::git_mailmap>()
        );
        assert_eq!(
            size_of::<Option<GitMailmapOwned>>(),
            size_of::<*mut ffi::git_mailmap>()
        );
    }

    #[test]
    fn mailmap_registers_its_c_destructor() {
        fn assert_dropped<T: CDropped>() {}
        assert_dropped::<GitMailmap>();
    }

    #[test]
    fn borrowed_handles_preserve_the_mailmap_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_mailmap>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_mailmap>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage
            // and the shared handle remains within this scope.
            let shared = unsafe { GitMailmapRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live,
            // and this scope has exclusive access to it.
            let mut exclusive = unsafe { GitMailmapMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_mailmap>>()) });
    }
}

/// Wraps: git_mailmap_add_entry
/// Copies one identity replacement into `mailmap`.
pub fn git_mailmap_add_entry(
    mailmap: &mut GitMailmapMut<'_>,
    real_name: Option<&CStr>,
    real_email: Option<&CStr>,
    replace_name: Option<&CStr>,
    replace_email: &CStr,
) -> Result<(), i32> {
    let real_name = real_name.map_or(core::ptr::null(), CStr::as_ptr);
    let real_email = real_email.map_or(core::ptr::null(), CStr::as_ptr);
    let replace_name = replace_name.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the mailmap is exclusively borrowed and every non-null string
    // remains live and NUL-terminated while libgit2 copies it.
    let status = unsafe {
        ffi::git_mailmap_add_entry(
            mailmap.as_mut_ptr(),
            real_name,
            real_email,
            replace_name,
            replace_email.as_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_mailmap_from_buffer
/// Parses a counted, non-NUL-containing mailmap buffer.
pub fn git_mailmap_from_buffer(data: &[u8]) -> Result<GitMailmapOwned, i32> {
    let mut out = core::ptr::null_mut();
    let data_ptr = if data.is_empty() {
        b"".as_ptr()
    } else {
        data.as_ptr()
    };
    // SAFETY: `data` supplies exactly `data.len()` readable bytes and C copies
    // parsed entries before returning. `out` is a writable result slot.
    let status = unsafe { ffi::git_mailmap_from_buffer(&mut out, data_ptr.cast(), data.len()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one fresh complete mailmap allocation.
    unsafe { GitMailmapOwned::from_raw(out) }.ok_or(-1)
}

/// Wraps: git_mailmap_from_repository
/// Loads and copies all configured mailmap sources from `repository`.
pub fn git_mailmap_from_repository(
    repository: GitRepositoryRef<'_>,
) -> Result<GitMailmapOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and the repository remains live throughout
    // loading. The returned mailmap owns copied entries and retains no borrow.
    let status =
        unsafe { ffi::git_mailmap_from_repository(&mut out, repository.as_ptr().cast_mut()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one fresh complete mailmap allocation.
    unsafe { GitMailmapOwned::from_raw(out) }.ok_or(-1)
}

/// Wraps: git_mailmap_new
/// Allocates an empty mailmap.
pub fn git_mailmap_new() -> Result<GitMailmapOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable slot for the fresh allocation.
    let status = unsafe { ffi::git_mailmap_new(&mut out) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one fresh complete mailmap allocation.
    unsafe { GitMailmapOwned::from_raw(out) }.ok_or(-1)
}

#[cfg(test)]
mod wrapper_tests {
    use super::*;

    /// Runs `body` with libgit2 initialized.
    ///
    /// libgit2 installs `git_failalloc_*` as its allocator until
    /// `git_libgit2_init` runs, so every constructor in this module fails
    /// before that call — and its failure path reports through thread-local
    /// error state that initialization is what creates, so the failure is not
    /// even observable as an `Err`.
    fn with_libgit2<R>(body: impl FnOnce() -> R) -> R {
        // SAFETY: libgit2 initialization is refcounted, and this successful
        // call is balanced by the shutdown below.
        let initialized = unsafe { ffi::git_libgit2_init() };
        assert!(initialized > 0);
        let result = body();
        // SAFETY: balances this scope's successful initialization call.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
        result
    }

    #[test]
    fn mailmap_constructors_and_mutation_are_owned() {
        with_libgit2(|| {
            let mut map = git_mailmap_new().unwrap();
            git_mailmap_add_entry(
                &mut map.as_mut(),
                Some(c"Real Name"),
                Some(c"real@example.com"),
                None,
                c"alias@example.com",
            )
            .unwrap();

            let parsed =
                git_mailmap_from_buffer(b"Real Name <real@example.com> <alias@example.com>\n")
                    .unwrap();
            drop(parsed);
        });
    }

    #[test]
    fn every_optional_identity_field_may_be_absent() {
        with_libgit2(|| {
            let mut map = git_mailmap_new().unwrap();
            git_mailmap_add_entry(&mut map.as_mut(), None, None, None, c"alias@example.com")
                .unwrap();
        });
    }

    #[test]
    fn an_empty_buffer_parses_into_an_owned_mailmap() {
        with_libgit2(|| {
            drop(git_mailmap_from_buffer(b"").unwrap());
        });
    }

    #[test]
    fn a_buffer_containing_a_nul_byte_is_rejected() {
        with_libgit2(|| {
            // `mailmap_add_buffer` refuses a buffer with an interior NUL and
            // releases the partially built mailmap before returning.
            assert!(
                git_mailmap_from_buffer(b"Real Name <real@example.com> <alias@example.com>\n\0")
                    .is_err()
            );
        });
    }
}

/// Wraps: git_mailmap_resolve_signature
/// Resolves a signature into a new independently owned signature.
pub fn git_mailmap_resolve_signature(
    mailmap: GitMailmapRef<'_>,
    signature: GitSignatureRef<'_>,
) -> Result<GitSignatureOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and both shared handles remain live while C
    // reads them and allocates an independent signature copy.
    let status = unsafe {
        ffi::git_mailmap_resolve_signature(&mut out, mailmap.as_ptr(), signature.as_ptr())
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete signature allocation.
    unsafe { GitSignatureOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_mailmap_resolve
/// Resolves an identity, borrowing replacements from `mailmap` and otherwise
/// returning the input strings.
///
/// C also accepts a null mailmap, which resolves every identity to the name
/// and email it was given. That is a no-op a Rust caller expresses by not
/// calling this at all, so the wrapper requires the mailmap.
pub fn git_mailmap_resolve<'a>(
    mailmap: GitMailmapRef<'a>,
    name: &'a CStr,
    email: &'a CStr,
) -> Result<(&'a CStr, &'a CStr), i32> {
    let (mut real_name, mut real_email) = (core::ptr::null(), core::ptr::null());
    // SAFETY: outputs are writable; all inputs share `'a`, which bounds both
    // possible return sources (mailmap entries or the original strings).
    let status = unsafe {
        ffi::git_mailmap_resolve(
            &mut real_name,
            &mut real_email,
            mailmap.as_ptr(),
            name.as_ptr(),
            email.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    debug_assert!(!real_name.is_null() && !real_email.is_null());
    // SAFETY: success returns the live input strings or NUL-terminated strings
    // owned by the live mailmap, all bounded by `'a`.
    Ok(unsafe { (CStr::from_ptr(real_name), CStr::from_ptr(real_email)) })
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::Libgit2Init;

    const MAP: &[u8] = b"Real Person <real@example.com> Alias <alias@example.com>\n\
Second Person <second@example.com> <other@example.com>\n";

    unsafe fn raw_resolution() -> (Vec<u8>, Vec<u8>) {
        let mut map = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_mailmap_from_buffer(&mut map, MAP.as_ptr().cast(), MAP.len()) },
            0
        );
        let mut signature = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_signature_new(
                    &mut signature,
                    c"Alias".as_ptr(),
                    c"alias@example.com".as_ptr(),
                    1_700_000_000,
                    0,
                )
            },
            0
        );
        let mut resolved = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_mailmap_resolve_signature(&mut resolved, map, signature) },
            0
        );
        let observation = (
            unsafe { CStr::from_ptr((*resolved).name) }
                .to_bytes()
                .to_vec(),
            unsafe { CStr::from_ptr((*resolved).email) }
                .to_bytes()
                .to_vec(),
        );
        unsafe {
            ffi::git_signature_free(resolved);
            ffi::git_signature_free(signature);
            ffi::git_mailmap_free(map);
        }
        observation
    }

    fn safe_resolution() -> (Vec<u8>, Vec<u8>) {
        let map = git_mailmap_from_buffer(MAP).unwrap();
        let signature =
            crate::signature::git_signature_new(c"Alias", c"alias@example.com", 1_700_000_000, 0)
                .unwrap();
        let resolved = git_mailmap_resolve_signature(map.as_ref(), signature.as_ref()).unwrap();
        (
            resolved.as_ref().name().to_bytes().to_vec(),
            resolved.as_ref().email().to_bytes().to_vec(),
        )
    }

    #[test]
    fn io_equiv_mailmap_signature_resolution() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = unsafe { raw_resolution() };
        assert_eq!(raw, safe_resolution());
        assert_eq!(raw, (b"Real Person".to_vec(), b"real@example.com".to_vec()));
    }
}
