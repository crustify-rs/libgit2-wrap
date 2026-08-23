//! Safe wrappers for libgit2 blame APIs.

use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CDropped, define_ctype};

use crate::ffi;
use crate::oid::{OidMut, OidRef};

define_ctype!(
    /// Wraps: git_blame
    /// An opaque blame result owned by libgit2.
    ///
    /// Use [`ffibox::CBox<GitBlame>`] for an owning handle. Dropping that handle
    /// releases the result with `git_blame_free`. Shared and exclusive borrows
    /// are represented by [`GitBlameRef`] and [`GitBlameMut`], without forming
    /// Rust references to memory that libgit2 owns.
    ///
    /// Libgit2 retains a borrowed repository pointer inside each result.
    /// Construction wrappers must therefore keep that repository alive for as
    /// long as the owning blame handle can be used.
    GitBlame,
    GitBlameRef,
    GitBlameMut,
    ffi::git_blame
);

// SAFETY: `git_blame_free` is the public destructor for a fully constructed
// `git_blame`. `CBox::from_raw` requires callers to transfer one uniquely owned
// result allocated by libgit2, and `CBox` invokes this implementation once.
unsafe impl CDropped for GitBlame {
    unsafe fn c_drop(obj: NonNull<Self>) {
        // SAFETY: the `CDropped` contract guarantees that `obj` denotes one
        // live, uniquely owned `git_blame` allocation.
        unsafe { ffi::git_blame_free(obj.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CBox, CCell};

    use super::*;

    #[test]
    fn opaque_blame_has_c_layout_and_lifecycle_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitBlame>();
        assert_dropped::<GitBlame>();
        assert_eq!(size_of::<GitBlame>(), size_of::<ffi::git_blame>());
        assert_eq!(align_of::<GitBlame>(), align_of::<ffi::git_blame>());
        assert_eq!(
            size_of::<GitBlameRef<'_>>(),
            size_of::<*mut ffi::git_blame>()
        );
        assert_eq!(
            size_of::<GitBlameMut<'_>>(),
            size_of::<*mut ffi::git_blame>()
        );
    }

    #[test]
    fn null_blame_seams_create_no_handle() {
        // SAFETY: each conversion explicitly accepts a null pointer and
        // returns `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitBlameRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitBlameMut::from_ptr(ptr::null_mut()).is_none());
            assert!(CBox::<GitBlame>::from_raw(ptr::null_mut()).is_none());
        }
    }
}

define_ctype!(
    /// Wraps: git_blame_options
    /// Options controlling the commits, lines, and copy tracking considered by
    /// a blame operation.
    GitBlameOptions,
    GitBlameOptionsRef,
    GitBlameOptionsMut,
    ffi::git_blame_options
);

impl<'a> GitBlameOptionsRef<'a> {
    /// Wraps: git_blame_options.flags
    /// Returns the raw bit set of `git_blame_flag_t` options.
    #[must_use]
    pub fn flags(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Wraps: git_blame_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Wraps: git_blame_options.max_line
    /// Returns the inclusive last line to blame, or zero for the file's end.
    #[must_use]
    pub fn max_line(&self) -> usize {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).max_line).read() }
    }

    /// Wraps: git_blame_options.min_line
    /// Returns the inclusive first line to blame.
    #[must_use]
    pub fn min_line(&self) -> usize {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).min_line).read() }
    }

    /// Wraps: git_blame_options.oldest_commit
    /// Borrows the oldest commit boundary embedded in this options value.
    #[must_use]
    pub fn oldest_commit(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection points at the initialized embedded
        // `git_oid`, which remains live for this handle's `'a` borrow.
        unsafe { OidRef::from_ptr(addr_of!((*self.as_ptr()).oldest_commit).cast_mut()) }
            .expect("an embedded field has a non-null address")
    }

    /// Wraps: git_blame_options.newest_commit
    /// Borrows the newest commit boundary embedded in this options value.
    #[must_use]
    pub fn newest_commit(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection points at the initialized embedded
        // `git_oid`, which remains live for this handle's `'a` borrow.
        unsafe { OidRef::from_ptr(addr_of!((*self.as_ptr()).newest_commit).cast_mut()) }
            .expect("an embedded field has a non-null address")
    }

    /// Wraps: git_blame_options.min_match_characters
    /// Returns the copy-tracking match threshold.
    #[must_use]
    pub fn min_match_characters(&self) -> u16 {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).min_match_characters).read() }
    }
}

impl GitBlameOptionsMut<'_> {
    /// Replaces the raw bit set of `git_blame_flag_t` options.
    pub fn set_flags(&mut self, flags: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits a raw-place write of the
        // scalar without forming a reference to C-visible memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Sets the ABI version of this options value.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: as `set_flags`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Sets the inclusive last line to blame, using zero for the file's end.
    pub fn set_max_line(&mut self, line: usize) {
        // SAFETY: as `set_flags`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).max_line).write(line) }
    }

    /// Sets the inclusive first line to blame.
    pub fn set_min_line(&mut self, line: usize) {
        // SAFETY: as `set_flags`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).min_line).write(line) }
    }

    /// Borrows the embedded oldest-commit boundary exclusively.
    #[must_use]
    pub fn oldest_commit_mut(&mut self) -> OidMut<'_> {
        // SAFETY: raw-place projection points at the initialized embedded
        // `git_oid`, and the result is bounded by this exclusive reborrow.
        unsafe { OidMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).oldest_commit)) }
            .expect("an embedded field has a non-null address")
    }

    /// Borrows the embedded newest-commit boundary exclusively.
    #[must_use]
    pub fn newest_commit_mut(&mut self) -> OidMut<'_> {
        // SAFETY: raw-place projection points at the initialized embedded
        // `git_oid`, and the result is bounded by this exclusive reborrow.
        unsafe { OidMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).newest_commit)) }
            .expect("an embedded field has a non-null address")
    }

    /// Sets the copy-tracking match threshold.
    pub fn set_min_match_characters(&mut self, characters: u16) {
        // SAFETY: as `set_flags`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).min_match_characters).write(characters) }
    }
}

#[cfg(test)]
mod options_tests {
    use core::mem::{align_of, size_of};

    use crate::oid::OidType;

    use super::*;

    #[test]
    fn blame_options_preserve_the_c_layout() {
        assert_eq!(
            size_of::<GitBlameOptions>(),
            size_of::<ffi::git_blame_options>()
        );
        assert_eq!(
            align_of::<GitBlameOptions>(),
            align_of::<ffi::git_blame_options>()
        );
    }

    #[test]
    fn blame_options_handles_read_and_write_every_field() {
        let mut options = GitBlameOptions::zeroed();
        let raw = addr_of_mut!(options).cast::<ffi::git_blame_options>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above and this is its only active handle.
        let mut options = unsafe { GitBlameOptionsMut::from_ptr(raw) }
            .expect("the address of a stack value is non-null");

        options.set_version(1);
        options.set_flags(0b101);
        options.set_min_match_characters(42);
        options.set_min_line(7);
        options.set_max_line(19);
        options.newest_commit_mut().set_oid_type(OidType::Sha256);
        options.oldest_commit_mut().set_oid_type(OidType::Sha1);

        let shared = options.as_ref();
        assert_eq!(shared.version(), 1);
        assert_eq!(shared.flags(), 0b101);
        assert_eq!(shared.min_match_characters(), 42);
        assert_eq!(shared.min_line(), 7);
        assert_eq!(shared.max_line(), 19);
        assert_eq!(shared.newest_commit().oid_type(), Ok(OidType::Sha256));
        assert_eq!(shared.oldest_commit().oid_type(), Ok(OidType::Sha1));
    }
}
