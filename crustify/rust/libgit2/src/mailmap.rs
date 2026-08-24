//! Safe wrappers for libgit2 mailmap APIs.

use core::ffi::CStr;

use ffibox::CBox;

use crate::ffi;
use crate::repository::GitRepositoryRef;

ffibox::define_ctype!(
    /// Wraps: git_mailmap
    /// Opaque mailmap state managed by libgit2.
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
// `git_mailmap`. It releases the entries and the allocation, and accepts null,
// although `CBox` always supplies a live non-null pointer.
ffibox::impl_dropped!(GitMailmap, ffi::git_mailmap, ffi::git_mailmap_free);

#[cfg(test)]
mod tests {
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

    #[test]
    fn mailmap_constructors_and_mutation_are_owned() {
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
            git_mailmap_from_buffer(b"Real Name <real@example.com> <alias@example.com>\n").unwrap();
        drop(parsed);
    }
}
