//! Safe wrappers for libgit2 deprecated APIs.

use core::ffi::CStr;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

use crate::api::types::GitSignatureRef;
use crate::ffi;
use crate::oid::OidRef;

ffibox::define_ctype!(
    /// Wraps: git_diff_format_email_options
    /// Layout-compatible options for the deprecated diff-email formatter.
    DiffFormatEmailOptions,
    DiffFormatEmailOptionsRef,
    DiffFormatEmailOptionsMut,
    ffi::git_diff_format_email_options
);

// SAFETY: the options value borrows every pointee and owns no resource, so
// disposing its inline storage requires no action.
unsafe impl CValued for DiffFormatEmailOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl DiffFormatEmailOptions {
    /// Constructs the documented default options value.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        let mut options_mut = options.as_mut();
        options_mut.set_version(ffi::GIT_DIFF_FORMAT_EMAIL_OPTIONS_VERSION);
        options_mut.set_patch_no(1);
        options_mut.set_total_patches(1);
        options
    }
}

impl<'a> DiffFormatEmailOptionsRef<'a> {
    /// Field: git_diff_format_email_options.flags
    /// Returns the raw deprecated formatting flags.
    #[must_use]
    pub fn flags(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Field: git_diff_format_email_options.version
    /// Returns the ABI version stored in this options value.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_diff_format_email_options.id
    /// Borrows the optional commit ID.
    #[must_use]
    pub fn id(&self) -> Option<OidRef<'a>> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to C-visible memory.
        let id = unsafe { addr_of!((*self.as_ptr()).id).read() };
        // SAFETY: a non-null field points to a live `git_oid` for the options
        // borrow, as required by the valid C options contract.
        unsafe { OidRef::from_ptr(id.cast_mut()) }
    }

    /// Field: git_diff_format_email_options.author
    /// Borrows the optional author signature.
    #[must_use]
    pub fn author(&self) -> Option<GitSignatureRef<'a>> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to C-visible memory.
        let author = unsafe { addr_of!((*self.as_ptr()).author).read() };
        // SAFETY: a non-null field points to a live signature for the options
        // borrow, as required by the valid C options contract.
        unsafe { GitSignatureRef::from_ptr(author.cast_mut()) }
    }

    /// Field: git_diff_format_email_options.summary
    /// Borrows the optional NUL-terminated change summary.
    #[must_use]
    pub fn summary(&self) -> Option<&'a CStr> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to C-visible memory.
        let summary = unsafe { addr_of!((*self.as_ptr()).summary).read() };
        if summary.is_null() {
            None
        } else {
            // SAFETY: a valid non-null summary is NUL-terminated and remains
            // live for the options handle's full shared borrow.
            Some(unsafe { CStr::from_ptr(summary) })
        }
    }

    /// Field: git_diff_format_email_options.body
    /// Borrows the optional NUL-terminated commit-message body.
    #[must_use]
    pub fn body(&self) -> Option<&'a CStr> {
        // SAFETY: this live shared handle permits reading the initialized
        // pointer field without forming a reference to C-visible memory.
        let body = unsafe { addr_of!((*self.as_ptr()).body).read() };
        if body.is_null() {
            None
        } else {
            // SAFETY: a valid non-null body is NUL-terminated and remains live
            // for the options handle's full shared borrow.
            Some(unsafe { CStr::from_ptr(body) })
        }
    }

    /// Field: git_diff_format_email_options.patch_no
    /// Returns this patch's one-based number.
    #[must_use]
    pub fn patch_no(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).patch_no).read() }
    }

    /// Field: git_diff_format_email_options.total_patches
    /// Returns the total number of patches in the series.
    #[must_use]
    pub fn total_patches(&self) -> usize {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).total_patches).read() }
    }
}

impl DiffFormatEmailOptionsMut<'_> {
    /// Sets the raw deprecated formatting flags.
    pub fn set_flags(&mut self, flags: u32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Sets the ABI version expected by libgit2.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Stores a borrowed commit ID.
    ///
    /// # Safety
    ///
    /// A non-null `id` must remain live for every later use of this options
    /// value, including uses after this mutable handle is released.
    pub unsafe fn set_borrowed_id(&mut self, id: Option<OidRef<'_>>) {
        let id = id.map_or(core::ptr::null(), |id| id.as_ptr());
        // SAFETY: this exclusive handle permits the write, and the caller
        // upholds the stored borrow's lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).id).write(id) }
    }

    /// Stores a borrowed author signature.
    ///
    /// # Safety
    ///
    /// A non-null `author` must remain live for every later use of this
    /// options value, including uses after this mutable handle is released.
    pub unsafe fn set_borrowed_author(&mut self, author: Option<GitSignatureRef<'_>>) {
        let author = author.map_or(core::ptr::null(), |author| author.as_ptr());
        // SAFETY: this exclusive handle permits the write, and the caller
        // upholds the stored borrow's lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).author).write(author) }
    }

    /// Stores a borrowed NUL-terminated change summary.
    ///
    /// # Safety
    ///
    /// A non-null `summary` must remain live for every later use of this
    /// options value, including uses after this mutable handle is released.
    pub unsafe fn set_borrowed_summary(&mut self, summary: Option<&CStr>) {
        let summary = summary.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the write, and the caller
        // upholds the stored borrow's lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).summary).write(summary) }
    }

    /// Stores a borrowed NUL-terminated commit-message body.
    ///
    /// # Safety
    ///
    /// A non-null `body` must remain live for every later use of this options
    /// value, including uses after this mutable handle is released.
    pub unsafe fn set_borrowed_body(&mut self, body: Option<&CStr>) {
        let body = body.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the write, and the caller
        // upholds the stored borrow's lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).body).write(body) }
    }

    /// Sets this patch's one-based number.
    pub fn set_patch_no(&mut self, patch_no: usize) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).patch_no).write(patch_no) }
    }

    /// Sets the total number of patches in the series.
    pub fn set_total_patches(&mut self, total_patches: usize) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).total_patches).write(total_patches) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn email_options_preserve_layout_and_inline_ownership() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<DiffFormatEmailOptions>();
        assert_valued::<DiffFormatEmailOptions>();
        assert_eq!(
            size_of::<DiffFormatEmailOptions>(),
            size_of::<ffi::git_diff_format_email_options>()
        );
        assert_eq!(
            align_of::<DiffFormatEmailOptions>(),
            align_of::<ffi::git_diff_format_email_options>()
        );
        assert_eq!(
            size_of::<DiffFormatEmailOptionsRef<'_>>(),
            size_of::<*const ffi::git_diff_format_email_options>()
        );
        assert_eq!(
            size_of::<DiffFormatEmailOptionsMut<'_>>(),
            size_of::<*mut ffi::git_diff_format_email_options>()
        );
    }

    #[test]
    fn defaults_and_borrowed_strings_round_trip() {
        let summary = c"summary";
        let body = c"body";
        let mut options = DiffFormatEmailOptions::new();

        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_DIFF_FORMAT_EMAIL_OPTIONS_VERSION
        );
        assert_eq!(options.as_ref().flags(), 0);
        assert_eq!(options.as_ref().patch_no(), 1);
        assert_eq!(options.as_ref().total_patches(), 1);
        assert!(options.as_ref().id().is_none());
        assert!(options.as_ref().author().is_none());
        assert!(options.as_ref().summary().is_none());
        assert!(options.as_ref().body().is_none());

        let mut options_mut = options.as_mut();
        options_mut.set_flags(1);
        options_mut.set_patch_no(2);
        options_mut.set_total_patches(3);
        // SAFETY: both static C string literals outlive `options`.
        unsafe {
            options_mut.set_borrowed_summary(Some(summary));
            options_mut.set_borrowed_body(Some(body));
        }
        assert_eq!(options_mut.as_ref().flags(), 1);
        assert_eq!(options_mut.as_ref().patch_no(), 2);
        assert_eq!(options_mut.as_ref().total_patches(), 3);
        assert_eq!(options_mut.as_ref().summary(), Some(summary));
        assert_eq!(options_mut.as_ref().body(), Some(body));
    }
}
