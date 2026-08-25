//! Safe wrappers for libgit2 repository APIs.

use core::ffi::{CStr, c_char, c_uint};
use core::marker::PhantomData;
use core::ops::{BitOr, BitOrAssign};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CBox, CVal, CValued};

use crate::annotated_commit::AnnotatedCommitRef;
use crate::api::buffer::GitBufMut;
use crate::api::repository::GitRepositoryInitFlags;
use crate::config::{GitConfigMut, GitConfigOwned, GitConfigRef};
use crate::ffi;
use crate::index::{GitIndex, GitIndexOwned, GitIndexRef};
use crate::odb::{GitOdb, GitOdbOwned, GitOdbRef};
use crate::oid::{InvalidOidType, OidRef, OidType};
use crate::refdb::{GitRefdbMut, GitRefdbOwned, GitRefdbRef, GitRefdbType, InvalidGitRefdbType};
use crate::refs::{GitReferenceTetheredOwned, adopt_reference};
use crate::worktree::GitWorktreeRef;

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

/// A by-value initialized repository-options record.
pub type GitRepositoryInitOptionsOwned = CVal<GitRepositoryInitOptions>;

// SAFETY: this options header only borrows all pointer fields. Disposing it
// releases no resource and retaining its inline storage is always valid.
unsafe impl CValued for GitRepositoryInitOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

/// Wraps: git_repository_free
/// An owned libgit2 repository allocation, released on drop.
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
    /// Field: git_repository_init_options.mode
    /// Returns the requested repository directory mode.
    #[must_use]
    pub fn mode(&self) -> u32 {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).mode).read() }
    }

    /// Field: git_repository_init_options.flags
    /// Returns the checked repository-initialization flags.
    pub fn flags(&self) -> Result<GitRepositoryInitFlags, u32> {
        // SAFETY: as `mode`, for this initialized scalar field.
        let flags = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitRepositoryInitFlags::from_bits(flags).ok_or(flags)
    }

    /// Field: git_repository_init_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> c_uint {
        // SAFETY: as `mode`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_repository_init_options.oid_type
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

    /// Field: git_repository_init_options.refdb_type
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

    /// Field: git_repository_init_options.description
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

    /// Field: git_repository_init_options.initial_head
    /// Returns the optional caller-owned initial HEAD name.
    #[must_use]
    pub fn initial_head(&self) -> Option<&'a CStr> {
        // SAFETY: as `description`, for this initialized pointer field.
        let value = unsafe { addr_of!((*self.as_ptr()).initial_head).read() };
        // SAFETY: as `description`, for this borrowed string field.
        unsafe { optional_borrowed_string(value) }
    }

    /// Field: git_repository_init_options.template_path
    /// Returns the optional caller-owned template directory path.
    #[must_use]
    pub fn template_path(&self) -> Option<&'a CStr> {
        // SAFETY: as `description`, for this initialized pointer field.
        let value = unsafe { addr_of!((*self.as_ptr()).template_path).read() };
        // SAFETY: as `description`, for this borrowed string field.
        unsafe { optional_borrowed_string(value) }
    }

    /// Field: git_repository_init_options.origin_url
    /// Returns the optional caller-owned origin URL.
    #[must_use]
    pub fn origin_url(&self) -> Option<&'a CStr> {
        // SAFETY: as `description`, for this initialized pointer field.
        let value = unsafe { addr_of!((*self.as_ptr()).origin_url).read() };
        // SAFETY: as `description`, for this borrowed string field.
        unsafe { optional_borrowed_string(value) }
    }

    /// Field: git_repository_init_options.workdir_path
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

    /// Replaces the repository-initialization flags.
    pub fn set_flags(&mut self, flags: GitRepositoryInitFlags) {
        // SAFETY: as `set_mode`, for this initialized scalar field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
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
        let flags = GitRepositoryInitFlags::EXTERNAL_TEMPLATE | GitRepositoryInitFlags::MKDIR;
        options.set_flags(flags);
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
        assert_eq!(shared.flags(), Ok(flags));
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

/// An owned configuration whose repository-dependent backends remain tied to
/// the repository borrow that produced it.
pub struct GitRepositoryConfigOwned<'repo> {
    inner: GitConfigOwned,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl GitRepositoryConfigOwned<'_> {
    /// Borrows the configuration.
    #[must_use]
    pub fn as_ref(&self) -> GitConfigRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the configuration exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitConfigMut<'_> {
        self.inner.as_mut()
    }
}

/// An owned reference database tied to the repository retained by its backend.
pub struct GitRepositoryRefdbOwned<'repo> {
    inner: GitRefdbOwned,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl GitRepositoryRefdbOwned<'_> {
    /// Borrows the reference database.
    #[must_use]
    pub fn as_ref(&self) -> GitRefdbRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the reference database exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitRefdbMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_repository_config
/// Returns one owned count of the repository configuration.
pub fn git_repository_config<'repo>(
    repository: &'repo mut GitRepositoryMut<'_>,
) -> Result<GitRepositoryConfigOwned<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and the exclusive repository handle
    // is live. Success transfers one independently releasable config count.
    let status = unsafe { ffi::git_repository_config(&mut output, repository.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: a successful call returns one live owned config count. Its
    // repository-dependent backends are kept valid by the result lifetime.
    let inner = unsafe { GitConfigOwned::from_raw(output) }
        .expect("git_repository_config succeeded without a config");
    Ok(GitRepositoryConfigOwned {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_repository_discover
/// Finds a repository and writes its Git-directory path into `output`.
pub fn git_repository_discover(
    output: &mut GitBufMut<'_>,
    start_path: &CStr,
    across_filesystems: bool,
    ceiling_directories: Option<&CStr>,
) -> Result<(), i32> {
    let ceilings = ceiling_directories.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the buffer is live and exclusive, both string pointers are live
    // or null as permitted, and libgit2 retains none of the arguments.
    let status = unsafe {
        ffi::git_repository_discover(
            output.as_mut_ptr(),
            start_path.as_ptr(),
            i32::from(across_filesystems),
            ceilings,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_fetchhead_foreach_cb
/// Safe callable surface for entries read from `FETCH_HEAD`.
///
/// Both string arguments are optional. `fetchhead_ref_parse` clears
/// `remote_url` before parsing and only fills it from the description field
/// that newer Git clients write; a compatibility line shaped like a loose
/// reference carries no description, so libgit2 invokes the callback with a
/// null URL. `reference_name` is likewise null whenever the parsed name is
/// empty.
pub trait GitRepositoryFetchheadForeachCallback {
    /// Receives one transient entry. A nonzero result stops iteration.
    fn call(
        &mut self,
        reference_name: Option<&CStr>,
        remote_url: Option<&CStr>,
        oid: OidRef<'_>,
        is_merge: bool,
    ) -> i32;
}

impl<F> GitRepositoryFetchheadForeachCallback for F
where
    F: FnMut(Option<&CStr>, Option<&CStr>, OidRef<'_>, bool) -> i32,
{
    fn call(
        &mut self,
        reference_name: Option<&CStr>,
        remote_url: Option<&CStr>,
        oid: OidRef<'_>,
        is_merge: bool,
    ) -> i32 {
        self(reference_name, remote_url, oid, is_merge)
    }
}

/// Wraps: git_repository_get_namespace
/// Borrows the active namespace, if one is configured.
#[must_use]
pub fn git_repository_get_namespace<'repo>(
    repository: GitRepositoryRef<'repo>,
) -> Option<&'repo CStr> {
    // SAFETY: the repository is live for `'repo`; the returned pointer is null
    // or a repository-owned NUL string with the same lifetime.
    let namespace = unsafe { ffi::git_repository_get_namespace(repository.as_ptr().cast_mut()) };
    if namespace.is_null() {
        None
    } else {
        // SAFETY: justified by the libgit2 return contract above.
        Some(unsafe { CStr::from_ptr(namespace) })
    }
}

/// Wraps: git_repository_head
/// Resolves `HEAD` and ties the returned reference to its repository.
pub fn git_repository_head<'repo>(
    repository: &'repo mut GitRepositoryMut<'_>,
) -> Result<GitReferenceTetheredOwned<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and the repository is live and
    // exclusive for cache initialization during reference resolution.
    let status = unsafe { ffi::git_repository_head(&mut output, repository.as_mut_ptr()) };
    // SAFETY: `output` is null or a complete caller-owned reference produced
    // by the lookup, including on a later error path.
    let inner = unsafe { CBox::from_raw(output) };
    adopt_reference(status, inner)
}

/// Wraps: git_repository_head_detached
/// Reports whether `HEAD` directly names an existing object.
pub fn git_repository_head_detached(repository: &mut GitRepositoryMut<'_>) -> Result<bool, i32> {
    // SAFETY: the repository is live and exclusive for any lazy database
    // initialization performed while resolving HEAD.
    let status = unsafe { ffi::git_repository_head_detached(repository.as_mut_ptr()) };
    if status < 0 {
        Err(status)
    } else {
        Ok(status != 0)
    }
}

/// Wraps: git_repository_is_bare
/// Reports whether the repository has no working directory.
#[must_use]
pub fn git_repository_is_bare(repository: GitRepositoryRef<'_>) -> bool {
    // SAFETY: the required shared repository is live and retained only for the call.
    unsafe { ffi::git_repository_is_bare(repository.as_ptr()) != 0 }
}

/// Wraps: git_repository_is_empty
/// Reports whether the repository has no references and an unborn initial branch.
pub fn git_repository_is_empty(repository: &mut GitRepositoryMut<'_>) -> Result<bool, i32> {
    // SAFETY: the repository is live and exclusive for lazy reference/config
    // initialization performed by the check.
    let status = unsafe { ffi::git_repository_is_empty(repository.as_mut_ptr()) };
    if status < 0 {
        Err(status)
    } else {
        Ok(status != 0)
    }
}

/// Wraps: git_repository_is_shallow
/// Reports whether the repository has a nonempty shallow-boundary file.
pub fn git_repository_is_shallow(repository: GitRepositoryRef<'_>) -> Result<bool, i32> {
    // SAFETY: the shared repository is live. Despite the non-const C parameter
    // the body only reads `repo->commondir` and stats the `shallow` file, so a
    // shared handle is sufficient and no pointer is retained.
    let status = unsafe { ffi::git_repository_is_shallow(repository.as_ptr().cast_mut()) };
    if status < 0 {
        Err(status)
    } else {
        Ok(status != 0)
    }
}

/// Wraps: git_repository_is_worktree
/// Reports whether this repository represents a linked worktree.
#[must_use]
pub fn git_repository_is_worktree(repository: GitRepositoryRef<'_>) -> bool {
    // SAFETY: the required shared repository is live and retained only for the call.
    unsafe { ffi::git_repository_is_worktree(repository.as_ptr()) != 0 }
}

/// Wraps: git_repository_mergehead_foreach_cb
/// Safe callable surface for object IDs read from `MERGE_HEAD`.
pub trait GitRepositoryMergeheadForeachCallback {
    /// Receives one transient merge-head object ID. Nonzero stops iteration.
    fn call(&mut self, oid: OidRef<'_>) -> i32;
}

impl<F> GitRepositoryMergeheadForeachCallback for F
where
    F: FnMut(OidRef<'_>) -> i32,
{
    fn call(&mut self, oid: OidRef<'_>) -> i32 {
        self(oid)
    }
}

/// Wraps: git_repository_message
/// Reads the prepared commit message into `output`.
pub fn git_repository_message(
    output: &mut GitBufMut<'_>,
    repository: GitRepositoryRef<'_>,
) -> Result<(), i32> {
    // SAFETY: the output buffer is live and exclusive and the repository is a
    // live shared input; neither pointer is retained.
    let status =
        unsafe { ffi::git_repository_message(output.as_mut_ptr(), repository.as_ptr().cast_mut()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_message_remove
/// Removes the repository's prepared commit message.
pub fn git_repository_message_remove(repository: GitRepositoryRef<'_>) -> Result<(), i32> {
    // SAFETY: the shared repository and its path remain live for the call; the
    // function mutates filesystem state but not repository memory.
    let status = unsafe { ffi::git_repository_message_remove(repository.as_ptr().cast_mut()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_oid_type
/// Returns the repository hash algorithm, or `None` for a null repository.
pub fn git_repository_oid_type(
    repository: Option<GitRepositoryRef<'_>>,
) -> Result<Option<OidType>, InvalidOidType> {
    let repository = repository.map_or(core::ptr::null_mut(), |repo| repo.as_ptr().cast_mut());
    // SAFETY: the pointer is null or a live shared repository. This accessor
    // only reads its scalar algorithm field.
    let raw = unsafe { ffi::git_repository_oid_type(repository) };
    if raw == 0 {
        Ok(None)
    } else {
        OidType::try_from(raw).map(Some)
    }
}

/// Flags controlling repository discovery and opening.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitRepositoryOpenFlags(u32);

impl GitRepositoryOpenFlags {
    /// No optional discovery behavior.
    pub const EMPTY: Self = Self(0);
    /// Do not search parent directories.
    pub const NO_SEARCH: Self = Self(ffi::git_repository_open_flag_t_GIT_REPOSITORY_OPEN_NO_SEARCH);
    /// Permit discovery across filesystem boundaries.
    pub const CROSS_FS: Self = Self(ffi::git_repository_open_flag_t_GIT_REPOSITORY_OPEN_CROSS_FS);
    /// Open as bare and defer configuration loading.
    pub const BARE: Self = Self(ffi::git_repository_open_flag_t_GIT_REPOSITORY_OPEN_BARE);
    /// Do not append `.git` while searching.
    pub const NO_DOTGIT: Self = Self(ffi::git_repository_open_flag_t_GIT_REPOSITORY_OPEN_NO_DOTGIT);
    /// Respect Git environment variables.
    pub const FROM_ENV: Self = Self(ffi::git_repository_open_flag_t_GIT_REPOSITORY_OPEN_FROM_ENV);
    /// Every flag currently published by libgit2.
    pub const ALL: Self = Self(
        Self::NO_SEARCH.0 | Self::CROSS_FS.0 | Self::BARE.0 | Self::NO_DOTGIT.0 | Self::FROM_ENV.0,
    );

    /// Creates flags when every bit is published.
    #[must_use]
    pub const fn from_bits(bits: u32) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the raw C bit set.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Returns whether every bit in `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for GitRepositoryOpenFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitRepositoryOpenFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

fn adopt_repository(
    status: i32,
    repository: Option<GitRepositoryOwned>,
) -> Result<GitRepositoryOwned, i32> {
    if status == 0 {
        Ok(repository.expect("repository open succeeded without returning a repository"))
    } else {
        Err(status)
    }
}

/// Wraps: git_repository_open
/// Opens the repository located exactly at `path`.
pub fn git_repository_open(path: &CStr) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and `path` is a live C string that
    // libgit2 does not retain.
    let status = unsafe { ffi::git_repository_open(&mut output, path.as_ptr()) };
    // SAFETY: `output` is null or a complete caller-owned repository produced
    // by the open operation, including on a later error path.
    let repository = unsafe { GitRepositoryOwned::from_raw(output) };
    adopt_repository(status, repository)
}

/// Wraps: git_repository_open_bare
/// Opens a bare repository directly and without discovery.
pub fn git_repository_open_bare(path: &CStr) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and `path` is a live C string that
    // libgit2 does not retain.
    let status = unsafe { ffi::git_repository_open_bare(&mut output, path.as_ptr()) };
    // SAFETY: `output` is null or a complete caller-owned repository produced
    // by the open operation, including on a later error path.
    let repository = unsafe { GitRepositoryOwned::from_raw(output) };
    adopt_repository(status, repository)
}

/// Wraps: git_repository_open_ext
/// Discovers and opens a repository with explicit search controls.
pub fn git_repository_open_ext(
    start_path: Option<&CStr>,
    flags: GitRepositoryOpenFlags,
    ceiling_directories: Option<&CStr>,
) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    let start_path = start_path.map_or(core::ptr::null(), CStr::as_ptr);
    let ceilings = ceiling_directories.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the output slot is writable; both input pointers are null or
    // live C strings and are not retained. Null start paths are supported by
    // the `FROM_ENV` mode and rejected by libgit2 otherwise.
    let status =
        unsafe { ffi::git_repository_open_ext(&mut output, start_path, flags.bits(), ceilings) };
    // SAFETY: `output` is null or a complete caller-owned repository produced
    // by the open operation, including on a later error path.
    let repository = unsafe { GitRepositoryOwned::from_raw(output) };
    adopt_repository(status, repository)
}

/// Wraps: git_repository_open_from_worktree
/// Opens the repository associated with a linked worktree.
pub fn git_repository_open_from_worktree(
    worktree: GitWorktreeRef<'_>,
) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and the shared worktree is live;
    // libgit2 reads its path and retains no worktree pointer.
    let status = unsafe {
        ffi::git_repository_open_from_worktree(&mut output, worktree.as_ptr().cast_mut())
    };
    // SAFETY: `output` is null or a complete caller-owned repository produced
    // by the open operation, including on a later error path.
    let repository = unsafe { GitRepositoryOwned::from_raw(output) };
    adopt_repository(status, repository)
}

/// Wraps: git_repository_path
/// Borrows the repository's Git-directory path, when it has one.
#[must_use]
pub fn git_repository_path<'repo>(repository: GitRepositoryRef<'repo>) -> Option<&'repo CStr> {
    // SAFETY: the repository is live for `'repo`; the returned pointer is null
    // for a pathless fake repository or a repository-owned NUL string.
    let path = unsafe { ffi::git_repository_path(repository.as_ptr()) };
    if path.is_null() {
        None
    } else {
        // SAFETY: justified by the libgit2 return contract above.
        Some(unsafe { CStr::from_ptr(path) })
    }
}

/// Wraps: git_repository_refdb
/// Returns one owned refdb count tied to its backing repository.
pub fn git_repository_refdb<'repo>(
    repository: &'repo mut GitRepositoryMut<'_>,
) -> Result<GitRepositoryRefdbOwned<'repo>, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable and the exclusive repository handle
    // is live. Success transfers one independently releasable refdb count.
    let status = unsafe { ffi::git_repository_refdb(&mut output, repository.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one live refdb count; the result lifetime keeps
    // every repository pointer retained by the refdb and backend valid.
    let inner = unsafe { GitRefdbOwned::from_raw(output) }
        .expect("git_repository_refdb succeeded without a refdb");
    Ok(GitRepositoryRefdbOwned {
        inner,
        _repository: PhantomData,
    })
}

/// Wraps: git_repository_set_config
/// Replaces the repository configuration, consuming a standalone config count.
///
/// Libgit2 retains a new internal count before this wrapper releases the
/// supplied one. Consuming the owner prevents safe code from installing a
/// repository-tethered configuration whose backend could outlive its original
/// repository, and prevents simultaneous mutable access through a surviving
/// external config owner.
pub fn git_repository_set_config(
    repository: &mut GitRepositoryMut<'_>,
    config: GitConfigOwned,
) -> Result<(), i32> {
    // SAFETY: both handles are live, the repository is exclusive, and
    // libgit2 increments the config count before retaining its pointer.
    let status = unsafe {
        ffi::git_repository_set_config(repository.as_mut_ptr(), config.as_ref().as_ptr().cast_mut())
    };
    // The caller's count is released here; on success the repository's count
    // keeps the config alive, and on failure consuming the input is harmless.
    drop(config);
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod symbol_tests {
    use super::*;
    use crate::oid::Oid;

    #[test]
    fn open_flags_validate_and_combine_bits() {
        let flags = GitRepositoryOpenFlags::NO_SEARCH | GitRepositoryOpenFlags::CROSS_FS;
        assert!(flags.contains(GitRepositoryOpenFlags::NO_SEARCH));
        assert!(flags.contains(GitRepositoryOpenFlags::CROSS_FS));
        assert_eq!(GitRepositoryOpenFlags::from_bits(flags.bits()), Some(flags));
        assert_eq!(
            GitRepositoryOpenFlags::from_bits(GitRepositoryOpenFlags::ALL.bits() << 1),
            None
        );
    }

    #[test]
    fn failed_open_result_accepts_an_empty_typed_owner() {
        assert!(matches!(adopt_repository(-123, None), Err(-123)));
    }

    #[test]
    fn an_in_memory_repository_reports_absent_paths() {
        // SAFETY: libgit2 initialization is process-global and refcounted;
        // the shutdown below balances this successful acquisition.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let mut raw = core::ptr::null_mut();
        // SAFETY: the output slot is writable and `git_repository_new` needs
        // no other input to build a complete in-memory repository.
        let status = unsafe { ffi::git_repository_new(&mut raw) };
        assert_eq!(status, 0);
        // SAFETY: `raw` is the complete caller-owned repository transferred
        // through the output slot above.
        let repository = unsafe { GitRepositoryOwned::from_raw(raw) }
            .expect("git_repository_new succeeded without a repository");

        // A repository with no on-disk location keeps both path fields null.
        assert_eq!(git_repository_commondir(repository.as_ref()), None);
        assert_eq!(git_repository_path(repository.as_ref()), None);
        assert!(git_repository_is_bare(repository.as_ref()));

        drop(repository);
        // SAFETY: balances this test's successful initialization, after the
        // repository owner has been released.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn callback_surfaces_preserve_safe_arguments() {
        let oid = Oid::zeroed();
        let raw = core::ptr::addr_of!(oid).cast::<ffi::git_oid>().cast_mut();
        // SAFETY: `raw` addresses the live layout-compatible local OID for the
        // duration of both callback invocations.
        let oid = unsafe { OidRef::from_ptr(raw) }.expect("local address is non-null");

        let mut fetch = |name: Option<&CStr>, url: Option<&CStr>, _: OidRef<'_>, merge: bool| {
            i32::from(name == Some(c"refs/heads/main") && url == Some(c"origin") && merge)
        };
        assert_eq!(
            GitRepositoryFetchheadForeachCallback::call(
                &mut fetch,
                Some(c"refs/heads/main"),
                Some(c"origin"),
                oid,
                true,
            ),
            1
        );
        // A compatibility `FETCH_HEAD` line carries neither a description nor
        // a reference name, so both strings reach the callback as null.
        assert_eq!(
            GitRepositoryFetchheadForeachCallback::call(&mut fetch, None, None, oid, true),
            0
        );

        let mut merge = |value: OidRef<'_>| i32::from(value.as_ptr() == oid.as_ptr());
        assert_eq!(
            GitRepositoryMergeheadForeachCallback::call(&mut merge, oid),
            1
        );
    }
}

/// Wraps: git_repository_commondir
/// Borrows the repository's shared common-directory path, when it has one.
///
/// The path is absent for an in-memory repository created by
/// `git_repository_new`, which never receives one, exactly as
/// [`git_repository_path`] is absent for the same repository.
#[must_use]
pub fn git_repository_commondir<'a>(repo: GitRepositoryRef<'a>) -> Option<&'a CStr> {
    // SAFETY: `repo` is live and shared; the getter returns the repository's
    // own optional common-directory field without retaining the argument.
    let path = unsafe { ffi::git_repository_commondir(repo.as_ptr()) };
    if path.is_null() {
        None
    } else {
        // SAFETY: a non-null result is the repository-owned NUL-terminated
        // path, live for the repository borrow `'a`.
        Some(unsafe { CStr::from_ptr(path) })
    }
}

/// Wraps: git_repository_set_head
/// Changes `HEAD` to the named reference or commit.
pub fn git_repository_set_head(
    repository: &mut GitRepositoryMut<'_>,
    refname: &CStr,
) -> Result<(), i32> {
    // SAFETY: both arguments are live for the synchronous call; libgit2
    // retains neither pointer and the exclusive handle permits repository mutation.
    let status = unsafe { ffi::git_repository_set_head(repository.as_mut_ptr(), refname.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_set_head_detached
/// Detaches `HEAD` at the commit identified by `committish`.
pub fn git_repository_set_head_detached(
    repository: &mut GitRepositoryMut<'_>,
    committish: OidRef<'_>,
) -> Result<(), i32> {
    // SAFETY: the repository is exclusively borrowed and the OID is live and
    // shared for the call; neither pointer is retained.
    let status = unsafe {
        ffi::git_repository_set_head_detached(repository.as_mut_ptr(), committish.as_ptr())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_set_head_detached_from_annotated
/// Detaches `HEAD` using an annotated commit's exact reflog description.
pub fn git_repository_set_head_detached_from_annotated(
    repository: &mut GitRepositoryMut<'_>,
    committish: AnnotatedCommitRef<'_>,
) -> Result<(), i32> {
    // SAFETY: both typed handles are live for the synchronous call and the
    // repository is exclusively borrowed for the mutation.
    let status = unsafe {
        ffi::git_repository_set_head_detached_from_annotated(
            repository.as_mut_ptr(),
            committish.as_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_set_namespace
/// Replaces the active reference namespace, or clears it with `None`.
pub fn git_repository_set_namespace(
    repository: &mut GitRepositoryMut<'_>,
    namespace: Option<&CStr>,
) -> Result<(), i32> {
    let namespace = namespace.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the repository is exclusively borrowed and `namespace` is null
    // or a live C string. Libgit2 duplicates a non-null string before returning.
    let status = unsafe { ffi::git_repository_set_namespace(repository.as_mut_ptr(), namespace) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_set_refdb
/// Installs a reference database while retaining the caller's owned count.
pub fn git_repository_set_refdb(
    repository: &mut GitRepositoryMut<'_>,
    refdb: GitRefdbRef<'_>,
) -> Result<(), i32> {
    // SAFETY: both handles are live. Libgit2 acquires its own refdb count and
    // makes it repository-owned, so it does not retain the borrowed handle.
    let status = unsafe {
        ffi::git_repository_set_refdb(repository.as_mut_ptr(), refdb.as_ptr().cast_mut())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_set_workdir
/// Changes the repository working directory.
pub fn git_repository_set_workdir(
    repository: &mut GitRepositoryMut<'_>,
    workdir: &CStr,
    update_gitlink: bool,
) -> Result<(), i32> {
    // SAFETY: the exclusive repository and C string are live for the call;
    // libgit2 copies the normalized path and retains no caller pointer.
    let status = unsafe {
        ffi::git_repository_set_workdir(
            repository.as_mut_ptr(),
            workdir.as_ptr(),
            i32::from(update_gitlink),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_state
/// Reports the current libgit2 repository-state code.
pub fn git_repository_state(repository: &mut GitRepositoryMut<'_>) -> Result<i32, i32> {
    // SAFETY: the repository is live and exclusive for any lazy reference
    // database initialization performed while inspecting state.
    let state = unsafe { ffi::git_repository_state(repository.as_mut_ptr()) };
    if state < 0 { Err(state) } else { Ok(state) }
}

/// Wraps: git_repository_state_cleanup
/// Removes metadata for an in-progress merge, rebase, or similar operation.
pub fn git_repository_state_cleanup(repository: &mut GitRepositoryMut<'_>) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusively borrowed while libgit2
    // removes its state files and references.
    let status = unsafe { ffi::git_repository_state_cleanup(repository.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_workdir
/// Borrows the working-directory path, or returns `None` for a bare repository.
#[must_use]
pub fn git_repository_workdir<'repo>(repository: GitRepositoryRef<'repo>) -> Option<&'repo CStr> {
    // SAFETY: the repository is live for `'repo`; the returned pointer is null
    // or a repository-owned C string with that same lifetime.
    let workdir = unsafe { ffi::git_repository_workdir(repository.as_ptr()) };
    if workdir.is_null() {
        None
    } else {
        // SAFETY: justified by the libgit2 getter contract above.
        Some(unsafe { CStr::from_ptr(workdir) })
    }
}

/// Wraps: git_repository_index
/// Acquires an independently owned count on the repository index.
pub fn git_repository_index(repository: &mut GitRepositoryMut<'_>) -> Result<GitIndexOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and the exclusive repository handle
    // permits lazy index initialization. Success increments the index count.
    let status = unsafe {
        ffi::git_repository_index(core::ptr::addr_of_mut!(output), repository.as_mut_ptr())
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete owned index count.
    unsafe { CBox::<GitIndex>::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_repository_init_ext
/// Initializes a repository using a borrowed options record.
pub fn git_repository_init_ext(
    path: &CStr,
    options: GitRepositoryInitOptionsRef<'_>,
) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output slot is writable, and `path` plus every borrow held
    // by `options` remain live for this synchronous initialization.
    let status = unsafe {
        ffi::git_repository_init_ext(
            core::ptr::addr_of_mut!(output),
            path.as_ptr(),
            options.as_ptr().cast_mut(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete repository allocation.
    unsafe { GitRepositoryOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_repository_init_init_options
/// Creates the deprecated-version-compatible initialized options record.
pub fn git_repository_init_init_options(
    version: u32,
) -> Result<GitRepositoryInitOptionsOwned, i32> {
    if version != 1 {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    let mut options = CVal::new(GitRepositoryInitOptions::zeroed());
    let status = {
        let mut output = options.as_mut();
        // SAFETY: `output` is exclusive writable storage for the complete
        // options record; the initializer retains no pointer.
        unsafe { ffi::git_repository_init_init_options(output.as_mut_ptr(), version) }
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_repository_odb
/// Acquires an independently owned count on the repository object database.
pub fn git_repository_odb(repository: &mut GitRepositoryMut<'_>) -> Result<GitOdbOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and the exclusive repository handle
    // permits lazy ODB initialization. Success increments the ODB count.
    let status = unsafe {
        ffi::git_repository_odb(core::ptr::addr_of_mut!(output), repository.as_mut_ptr())
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete owned ODB count.
    unsafe { CBox::<GitOdb>::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_repository_set_index
/// Replaces or clears the repository index while retaining its own count.
pub fn git_repository_set_index(
    repository: &mut GitRepositoryMut<'_>,
    index: Option<GitIndexRef<'_>>,
) -> Result<(), i32> {
    // SAFETY: the repository is exclusive and `index` is null or live. The C
    // function acquires its own count before returning and retains no borrow.
    let status = unsafe {
        ffi::git_repository_set_index(
            repository.as_mut_ptr(),
            index.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_set_odb
/// Replaces the repository object database while retaining its own count.
pub fn git_repository_set_odb(
    repository: &mut GitRepositoryMut<'_>,
    odb: GitOdbRef<'_>,
) -> Result<(), i32> {
    // SAFETY: the repository is exclusive and the ODB is live. Libgit2 takes
    // its own count before returning, so it retains no Rust borrow.
    let status =
        unsafe { ffi::git_repository_set_odb(repository.as_mut_ptr(), odb.as_ptr().cast_mut()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_repository_wrap_odb
/// Creates an in-memory repository retaining its own count on `odb`.
pub fn git_repository_wrap_odb(odb: GitOdbRef<'_>) -> Result<GitRepositoryOwned, i32> {
    let mut output = core::ptr::null_mut();
    // SAFETY: the output is writable and the live ODB remains available while
    // libgit2 creates the repository and acquires a count on it.
    let status = unsafe {
        ffi::git_repository_wrap_odb(core::ptr::addr_of_mut!(output), odb.as_ptr().cast_mut())
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete repository allocation.
    unsafe { GitRepositoryOwned::from_raw(output) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod scheduled_symbol_tests {
    use super::*;

    #[test]
    fn deprecated_options_initializer_returns_a_safe_by_value_owner() {
        let options = git_repository_init_init_options(1).expect("the current options version");
        assert_eq!(options.as_ref().version(), 1);
        assert!(options.as_ref().flags().is_ok());
    }

    #[test]
    fn options_initializer_rejects_an_unknown_version() {
        assert!(git_repository_init_init_options(u32::MAX).is_err());
    }
}
