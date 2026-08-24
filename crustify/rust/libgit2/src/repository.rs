//! Safe wrappers for libgit2 repository APIs.

use core::ffi::{CStr, c_char, c_uint};
use core::ptr::{addr_of, addr_of_mut};

use ffibox::CBox;

use crate::ffi;
use crate::oid::{InvalidOidType, OidType};
use crate::refdb::{GitRefdbType, InvalidGitRefdbType};

ffibox::define_ctype!(
    /// Wraps: git_repository
    /// An opaque repository handle managed by libgit2.
    ///
    /// The public C API keeps the layout private. Owned handles represent
    /// complete repository allocations and release them with
    /// `git_repository_free`; borrowed access uses [`GitRepositoryRef`] and
    /// [`GitRepositoryMut`] without forming Rust references to C-owned memory.
    GitRepository,
    GitRepositoryRef,
    GitRepositoryMut,
    ffi::git_repository
);

/// An owned libgit2 repository allocation.
pub type GitRepositoryOwned = CBox<GitRepository>;

// SAFETY: `git_repository_free` is the public destructor for a complete
// libgit2-allocated repository. It cleans up all repository-owned internals,
// clears the header, and frees its allocation exactly once. It accepts null,
// although `CBox` supplies a live non-null allocation.
ffibox::impl_dropped!(GitRepository, ffi::git_repository, ffi::git_repository_free);

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

        assert_cell::<GitRepository>();
        assert_dropped::<GitRepository>();
        assert_eq!(size_of::<GitRepository>(), size_of::<ffi::git_repository>());
        assert_eq!(
            align_of::<GitRepository>(),
            align_of::<ffi::git_repository>()
        );
        assert_eq!(
            size_of::<GitRepositoryRef<'_>>(),
            size_of::<*const ffi::git_repository>()
        );
        assert_eq!(
            size_of::<GitRepositoryMut<'_>>(),
            size_of::<*mut ffi::git_repository>()
        );
        assert_eq!(
            size_of::<GitRepositoryOwned>(),
            size_of::<*mut ffi::git_repository>()
        );
    }

    #[test]
    fn null_seams_create_no_repository_handles() {
        // SAFETY: these conversion seams explicitly accept null and return
        // `None` without borrowing or adopting an object.
        unsafe {
            assert!(GitRepositoryRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRepositoryMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitRepositoryOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_repository_init_options
    /// Borrowing options that control repository initialization.
    GitRepositoryInitOptions,
    GitRepositoryInitOptionsRef,
    GitRepositoryInitOptionsMut,
    ffi::git_repository_init_options
);

impl<'a> GitRepositoryInitOptionsRef<'a> {
    /// Wraps: git_repository_init_options.mode
    /// Returns the requested repository directory mode.
    #[must_use]
    pub fn mode(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).mode).read() }
    }

    /// Wraps: git_repository_init_options.flags
    /// Returns the raw `git_repository_init_flag_t` bit set.
    #[must_use]
    pub fn flags(&self) -> u32 {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Wraps: git_repository_init_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> c_uint {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Wraps: git_repository_init_options.oid_type
    /// Returns the selected object-ID algorithm, or `None` for libgit2's default.
    pub fn oid_type(&self) -> Result<Option<OidType>, InvalidOidType> {
        // SAFETY: as `mode`, for this initialized scalar field.
        let raw = unsafe { addr_of!((*self.as_ptr()).oid_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            OidType::try_from(raw).map(Some)
        }
    }

    /// Wraps: git_repository_init_options.refdb_type
    /// Returns the selected reference database, or `None` for libgit2's default.
    pub fn refdb_type(&self) -> Result<Option<GitRefdbType>, InvalidGitRefdbType> {
        // SAFETY: as `mode`, for this initialized scalar field.
        let raw = unsafe { addr_of!((*self.as_ptr()).refdb_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            GitRefdbType::try_from(raw).map(Some)
        }
    }

    /// Wraps: git_repository_init_options.description
    /// Returns the optional caller-owned repository description.
    #[must_use]
    pub fn description(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to C-visible memory.
        let value = unsafe { addr_of!((*self.as_ptr()).description).read() };
        // SAFETY: a valid options value requires every non-null string field
        // to remain live and NUL-terminated for the handle lifetime.
        unsafe { optional_borrowed_string(value) }
    }

    /// Wraps: git_repository_init_options.initial_head
    /// Returns the optional caller-owned initial HEAD name.
    #[must_use]
    pub fn initial_head(&self) -> Option<&'a CStr> {
        // SAFETY: as `description`, for this initialized pointer field.
        let value = unsafe { addr_of!((*self.as_ptr()).initial_head).read() };
        // SAFETY: as `description`, for this borrowed string field.
        unsafe { optional_borrowed_string(value) }
    }

    /// Wraps: git_repository_init_options.template_path
    /// Returns the optional caller-owned template directory path.
    #[must_use]
    pub fn template_path(&self) -> Option<&'a CStr> {
        // SAFETY: as `description`, for this initialized pointer field.
        let value = unsafe { addr_of!((*self.as_ptr()).template_path).read() };
        // SAFETY: as `description`, for this borrowed string field.
        unsafe { optional_borrowed_string(value) }
    }

    /// Wraps: git_repository_init_options.origin_url
    /// Returns the optional caller-owned origin URL.
    #[must_use]
    pub fn origin_url(&self) -> Option<&'a CStr> {
        // SAFETY: as `description`, for this initialized pointer field.
        let value = unsafe { addr_of!((*self.as_ptr()).origin_url).read() };
        // SAFETY: as `description`, for this borrowed string field.
        unsafe { optional_borrowed_string(value) }
    }

    /// Wraps: git_repository_init_options.workdir_path
    /// Returns the optional caller-owned working-directory path.
    #[must_use]
    pub fn workdir_path(&self) -> Option<&'a CStr> {
        // SAFETY: as `description`, for this initialized pointer field.
        let value = unsafe { addr_of!((*self.as_ptr()).workdir_path).read() };
        // SAFETY: as `description`, for this borrowed string field.
        unsafe { optional_borrowed_string(value) }
    }
}

unsafe fn optional_borrowed_string<'a>(value: *const c_char) -> Option<&'a CStr> {
    if value.is_null() {
        None
    } else {
        // SAFETY: callers obtain this helper from getters on valid options
        // handles, whose non-null string fields remain live and NUL-terminated
        // for the handle lifetime.
        Some(unsafe { CStr::from_ptr(value) })
    }
}

impl GitRepositoryInitOptionsMut<'_> {
    /// Sets the requested repository directory mode.
    pub fn set_mode(&mut self, mode: u32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).mode).write(mode) }
    }

    /// Replaces the raw `git_repository_init_flag_t` bit set.
    pub fn set_flags(&mut self, flags: u32) {
        // SAFETY: as `set_mode`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Sets the ABI version of this options value.
    pub fn set_version(&mut self, version: c_uint) {
        // SAFETY: as `set_mode`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects an object-ID algorithm, or libgit2's default with `None`.
    pub fn set_oid_type(&mut self, oid_type: Option<OidType>) {
        let raw = oid_type.map_or(0, ffi::git_oid_t::from);
        // SAFETY: as `set_mode`, and `raw` is zero or a valid C enum value.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).oid_type).write(raw) }
    }

    /// Selects a reference database, or libgit2's default with `None`.
    pub fn set_refdb_type(&mut self, refdb_type: Option<GitRefdbType>) {
        let raw = refdb_type.map_or(0, ffi::git_refdb_t::from);
        // SAFETY: as `set_mode`, and `raw` is zero or a valid C enum value.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).refdb_type).write(raw) }
    }

    /// Stores an optional caller-owned repository description.
    ///
    /// # Safety
    ///
    /// A non-null `description` must remain alive and NUL-terminated for every
    /// later use of the options value, including after this handle is released.
    pub unsafe fn set_borrowed_description(&mut self, description: Option<&CStr>) {
        let value = description.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the raw-place write, and the
        // caller upholds the stored string's lifetime contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).description).write(value) }
    }

    /// Stores an optional caller-owned initial HEAD name.
    ///
    /// # Safety
    ///
    /// A non-null `initial_head` must remain alive and NUL-terminated for every
    /// later use of the options value, including after this handle is released.
    pub unsafe fn set_borrowed_initial_head(&mut self, initial_head: Option<&CStr>) {
        let value = initial_head.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: as `set_borrowed_description`, for this pointer field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).initial_head).write(value) }
    }

    /// Stores an optional caller-owned template directory path.
    ///
    /// # Safety
    ///
    /// A non-null `template_path` must remain alive and NUL-terminated for every
    /// later use of the options value, including after this handle is released.
    pub unsafe fn set_borrowed_template_path(&mut self, template_path: Option<&CStr>) {
        let value = template_path.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: as `set_borrowed_description`, for this pointer field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).template_path).write(value) }
    }

    /// Stores an optional caller-owned origin URL.
    ///
    /// # Safety
    ///
    /// A non-null `origin_url` must remain alive and NUL-terminated for every
    /// later use of the options value, including after this handle is released.
    pub unsafe fn set_borrowed_origin_url(&mut self, origin_url: Option<&CStr>) {
        let value = origin_url.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: as `set_borrowed_description`, for this pointer field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).origin_url).write(value) }
    }

    /// Stores an optional caller-owned working-directory path.
    ///
    /// # Safety
    ///
    /// A non-null `workdir_path` must remain alive and NUL-terminated for every
    /// later use of the options value, including after this handle is released.
    pub unsafe fn set_borrowed_workdir_path(&mut self, workdir_path: Option<&CStr>) {
        let value = workdir_path.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: as `set_borrowed_description`, for this pointer field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).workdir_path).write(value) }
    }

    /// Clears the optional repository description.
    pub fn clear_description(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime obligation.
        unsafe { self.set_borrowed_description(None) }
    }

    /// Clears the optional initial HEAD name.
    pub fn clear_initial_head(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime obligation.
        unsafe { self.set_borrowed_initial_head(None) }
    }

    /// Clears the optional template directory path.
    pub fn clear_template_path(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime obligation.
        unsafe { self.set_borrowed_template_path(None) }
    }

    /// Clears the optional origin URL.
    pub fn clear_origin_url(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime obligation.
        unsafe { self.set_borrowed_origin_url(None) }
    }

    /// Clears the optional working-directory path.
    pub fn clear_workdir_path(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime obligation.
        unsafe { self.set_borrowed_workdir_path(None) }
    }

    /// Clears every borrowed string field.
    pub fn clear_borrowed_strings(&mut self) {
        self.clear_description();
        self.clear_initial_head();
        self.clear_template_path();
        self.clear_origin_url();
        self.clear_workdir_path();
    }
}

#[cfg(test)]
mod init_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn init_options_preserve_the_c_layout() {
        assert_eq!(
            size_of::<GitRepositoryInitOptions>(),
            size_of::<ffi::git_repository_init_options>()
        );
        assert_eq!(
            align_of::<GitRepositoryInitOptions>(),
            align_of::<ffi::git_repository_init_options>()
        );
        assert_eq!(
            size_of::<GitRepositoryInitOptionsRef<'_>>(),
            size_of::<*const ffi::git_repository_init_options>()
        );
        assert_eq!(
            size_of::<GitRepositoryInitOptionsMut<'_>>(),
            size_of::<*mut ffi::git_repository_init_options>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_every_init_option() {
        let mut storage = GitRepositoryInitOptions::zeroed();
        let raw = addr_of_mut!(storage).cast::<ffi::git_repository_init_options>();

        // SAFETY: `raw` is fully initialized, remains live for the handle's
        // use, and this is the only access path during the exclusive borrow.
        let mut options = unsafe { GitRepositoryInitOptionsMut::from_ptr(raw) }
            .expect("the address of a stack value is non-null");
        options.set_version(1);
        options.set_flags(0x24);
        options.set_mode(0o2775);
        options.set_oid_type(Some(OidType::Sha256));
        options.set_refdb_type(Some(GitRefdbType::Reftable));
        // SAFETY: these literals have static storage and outlive every use of
        // the options value.
        unsafe {
            options.set_borrowed_description(Some(c"repository"));
            options.set_borrowed_initial_head(Some(c"main"));
            options.set_borrowed_template_path(Some(c"templates"));
            options.set_borrowed_origin_url(Some(c"https://example.test/repo"));
            options.set_borrowed_workdir_path(Some(c"worktree"));
        }

        let shared = options.as_ref();
        assert_eq!(shared.version(), 1);
        assert_eq!(shared.flags(), 0x24);
        assert_eq!(shared.mode(), 0o2775);
        assert_eq!(shared.oid_type(), Ok(Some(OidType::Sha256)));
        assert_eq!(shared.refdb_type(), Ok(Some(GitRefdbType::Reftable)));
        assert_eq!(shared.description(), Some(c"repository"));
        assert_eq!(shared.initial_head(), Some(c"main"));
        assert_eq!(shared.template_path(), Some(c"templates"));
        assert_eq!(shared.origin_url(), Some(c"https://example.test/repo"));
        assert_eq!(shared.workdir_path(), Some(c"worktree"));

        options.set_oid_type(None);
        options.set_refdb_type(None);
        options.clear_borrowed_strings();
        assert_eq!(options.as_ref().oid_type(), Ok(None));
        assert_eq!(options.as_ref().refdb_type(), Ok(None));
        assert_eq!(options.as_ref().description(), None);
        assert_eq!(options.as_ref().initial_head(), None);
        assert_eq!(options.as_ref().template_path(), None);
        assert_eq!(options.as_ref().origin_url(), None);
        assert_eq!(options.as_ref().workdir_path(), None);

        let invalid_oid = ffi::git_oid_t_GIT_OID_SHA256 + 1;
        let invalid_refdb = ffi::git_refdb_t_GIT_REFDB_REFTABLE + 1;
        // SAFETY: the exclusive handle permits these raw-place scalar writes;
        // both values are valid C integers even though the Rust wrappers must
        // reject them as unknown enum values.
        unsafe {
            addr_of_mut!((*options.as_mut_ptr()).oid_type).write(invalid_oid);
            addr_of_mut!((*options.as_mut_ptr()).refdb_type).write(invalid_refdb);
        }
        assert_eq!(
            options.as_ref().oid_type().unwrap_err().value(),
            invalid_oid
        );
        assert_eq!(
            options.as_ref().refdb_type().unwrap_err().value(),
            invalid_refdb
        );
    }
}
