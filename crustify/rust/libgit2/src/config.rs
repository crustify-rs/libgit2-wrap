//! Safe wrappers for libgit2 config APIs.

use core::ffi::CStr;
use core::ptr::addr_of;
use ffibox::CBox;

use crate::ffi;

/// Wraps: git_config_level_t
/// Priority level of a libgit2 configuration source.
///
/// Values at or above [`Self::APP`] are reserved for application-defined
/// configuration sources. [`Self::HIGHEST`] is the query sentinel that asks
/// libgit2 to select the highest level currently loaded.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GitConfigLevel(ffi::git_config_level_t);

impl GitConfigLevel {
    /// The system-wide configuration file.
    pub const SYSTEM: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_SYSTEM);
    /// The XDG configuration file.
    pub const XDG: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_XDG);
    /// The user-specific global configuration file.
    pub const GLOBAL: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_GLOBAL);
    /// The repository-local configuration file.
    pub const LOCAL: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_LOCAL);
    /// The worktree-specific configuration file.
    pub const WORKTREE: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_WORKTREE);
    /// The first level available to application-defined configuration sources.
    pub const APP: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_APP);
    /// Sentinel selecting the highest configuration level currently loaded.
    pub const HIGHEST: Self = Self(ffi::git_config_level_t_GIT_CONFIG_HIGHEST_LEVEL);

    /// Converts a C value when it denotes a published or application level.
    pub const fn from_raw(raw: ffi::git_config_level_t) -> Option<Self> {
        if raw == Self::HIGHEST.0
            || (raw >= Self::SYSTEM.0 && raw <= Self::WORKTREE.0)
            || raw >= Self::APP.0
        {
            Some(Self(raw))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 configuration level.
    pub const fn as_raw(self) -> ffi::git_config_level_t {
        self.0
    }

    /// Returns whether this is an application-defined level.
    pub const fn is_application(self) -> bool {
        self.0 >= Self::APP.0
    }
}

ffibox::define_ctype!(
    /// Wraps: git_config
    /// Opaque, refcounted configuration object managed by libgit2.
    GitConfig,
    GitConfigRef,
    GitConfigMut,
    ffi::git_config
);

/// An owned reference count to a [`GitConfig`].
///
/// Dropping it calls `git_config_free`, which releases one count. No exported
/// operation takes a `git_config *` and yields an additional count: the
/// refcount is incremented only where libgit2 hands out a config it already
/// holds, such as `git_repository_config`. This owner therefore intentionally
/// does not implement `Clone`.
pub type GitConfigOwned = CBox<GitConfig>;

// SAFETY: `git_config_free` consumes exactly one reference to a fully formed
// `git_config`; it decrements the embedded count and releases the allocation
// and its backend fields when that was the final reference. It accepts null,
// although `CBox` always supplies a non-null pointer.
ffibox::impl_dropped!(GitConfig, ffi::git_config, ffi::git_config_free);

/// Wraps: git_config_parse_bool
/// Parses a libgit2 boolean spelling.
///
/// `None` is not an error: `git__parse_bool` treats a missing value as the
/// bare `[section] key` form and yields `true`. An empty string yields
/// `false`, and anything else falls through to the integer parser.
pub fn git_config_parse_bool(value: Option<&core::ffi::CStr>) -> Result<bool, i32> {
    let mut out = 0;
    let value = value.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `out` is writable and `value` is null or a live C string.
    let status = unsafe { ffi::git_config_parse_bool(&mut out, value) };
    if status == 0 {
        Ok(out != 0)
    } else {
        Err(status)
    }
}

/// Wraps: git_config_parse_int32
/// Parses a 32-bit integer, including libgit2's `k`, `m`, and `g` suffixes.
pub fn git_config_parse_int32(value: Option<&core::ffi::CStr>) -> Result<i32, i32> {
    let mut out = 0;
    let value = value.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `out` is writable and `value` is null or a live C string.
    let status = unsafe { ffi::git_config_parse_int32(&mut out, value) };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_config_parse_int64
/// Parses a 64-bit integer, including libgit2's `k`, `m`, and `g` suffixes.
pub fn git_config_parse_int64(value: Option<&core::ffi::CStr>) -> Result<i64, i32> {
    let mut out = 0;
    let value = value.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `out` is writable and `value` is null or a live C string.
    let status = unsafe { ffi::git_config_parse_int64(&mut out, value) };
    if status == 0 { Ok(out) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use ffibox::CDropped;

    use super::*;

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
            // this guard. The config owner is dropped before the guard.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    #[test]
    fn published_levels_round_trip() {
        let levels = [
            GitConfigLevel::SYSTEM,
            GitConfigLevel::XDG,
            GitConfigLevel::GLOBAL,
            GitConfigLevel::LOCAL,
            GitConfigLevel::WORKTREE,
            GitConfigLevel::APP,
            GitConfigLevel::HIGHEST,
        ];

        for level in levels {
            assert_eq!(GitConfigLevel::from_raw(level.as_raw()), Some(level));
        }
    }

    #[test]
    fn application_levels_are_extensible_but_gaps_are_rejected() {
        let custom = GitConfigLevel::from_raw(GitConfigLevel::APP.as_raw() + 17)
            .expect("application levels begin at APP");
        assert!(custom.is_application());

        assert_eq!(GitConfigLevel::from_raw(0), None);
        assert_eq!(GitConfigLevel::from_raw(1), None);
        assert_eq!(GitConfigLevel::from_raw(-2), None);
    }

    #[test]
    fn config_representation_matches_the_c_seam_and_registers_drop() {
        fn assert_dropped<T: CDropped>() {}

        assert_eq!(size_of::<GitConfig>(), size_of::<ffi::git_config>());
        assert_eq!(align_of::<GitConfig>(), align_of::<ffi::git_config>());
        assert_eq!(
            size_of::<GitConfigRef<'_>>(),
            size_of::<*const ffi::git_config>()
        );
        assert_eq!(
            size_of::<GitConfigMut<'_>>(),
            size_of::<*mut ffi::git_config>()
        );
        assert_eq!(
            size_of::<GitConfigOwned>(),
            size_of::<*mut ffi::git_config>()
        );
        assert_dropped::<GitConfig>();
    }

    #[test]
    fn public_scalar_parsers_return_typed_values() {
        // Libgit2 requires initialization before any entry point; these
        // parsers reach `git_error_set` on their failure paths.
        let _init = Libgit2Init::acquire();
        assert_eq!(git_config_parse_bool(Some(c"yes")), Ok(true));
        assert_eq!(git_config_parse_bool(Some(c"off")), Ok(false));
        assert_eq!(git_config_parse_int32(Some(c"2k")), Ok(2048));
        assert_eq!(git_config_parse_int64(Some(c"3m")), Ok(3 * 1024 * 1024));
    }

    #[test]
    fn absent_values_follow_each_parser_rather_than_the_pointer() {
        // A valueless key is `true` for the boolean parser, ...
        assert_eq!(git_config_parse_bool(None), Ok(true));
        assert_eq!(git_config_parse_bool(Some(c"")), Ok(false));
        // ... but the integer parsers reject a missing string outright.
        assert!(git_config_parse_int32(None).is_err());
        assert!(git_config_parse_int64(None).is_err());
    }

    #[test]
    fn config_owner_provides_shared_and_exclusive_handles() {
        let _init = Libgit2Init::acquire();
        let mut config = git_config_new().expect("an empty config can be allocated");
        let raw = config.as_ptr();

        let shared = config.as_ref();
        assert_eq!(shared.as_ptr(), raw.cast_const());

        let mut exclusive = config.as_mut();
        assert_eq!(exclusive.as_mut_ptr(), raw);
        assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
    }

    #[test]
    fn config_error_result_releases_a_typed_owner() {
        let _init = Libgit2Init::acquire();
        let config = git_config_new().expect("an empty config can be allocated");

        assert!(matches!(config_result(-123, Some(config)), Err(-123)));
    }

    #[test]
    fn config_snapshot_returns_an_independent_owner() {
        let _init = Libgit2Init::acquire();
        let mut raw = core::ptr::null_mut();
        // SAFETY: libgit2 is initialized and `raw` is writable.
        assert_eq!(unsafe { ffi::git_config_new(&mut raw) }, 0);
        // SAFETY: success returned one fresh owned config count.
        let config = unsafe { GitConfigOwned::from_raw(raw) }.unwrap();
        let snapshot = git_config_snapshot(config.as_ref()).unwrap();
        assert_ne!(snapshot.as_ref().as_ptr(), config.as_ref().as_ptr());
    }
}

ffibox::define_ctype!(
    /// Wraps: git_config_entry
    /// A configuration entry whose storage is managed by its concrete backend.
    GitConfigEntry,
    GitConfigEntryRef,
    GitConfigEntryMut,
    ffi::git_config_entry
);

/// An owned configuration entry returned by libgit2.
pub type GitConfigEntryOwned = CBox<GitConfigEntry>;

// SAFETY: a fully formed entry is the first member of a
// `git_config_backend_entry`; `git_config_entry_free` invokes that concrete
// entry's finalizer exactly once and accepts null, although `CBox` is non-null.
ffibox::impl_dropped!(
    GitConfigEntry,
    ffi::git_config_entry,
    ffi::git_config_entry_free
);

impl<'a> GitConfigEntryRef<'a> {
    /// Field: git_config_entry.name
    /// Returns the normalized configuration name.
    #[must_use]
    pub fn name(&self) -> &'a CStr {
        // SAFETY: every live backend entry has a non-null NUL-terminated name
        // kept alive by its concrete entry owner. Raw-place projection forms
        // no reference to the C-visible entry itself.
        unsafe { CStr::from_ptr(addr_of!((*self.as_ptr()).name).read()) }
    }

    /// Field: git_config_entry.value
    /// Returns the literal value, or `None` for a valueless entry.
    #[must_use]
    pub fn value(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the C-visible entry.
        let value = unsafe { addr_of!((*self.as_ptr()).value).read() };
        if value.is_null() {
            None
        } else {
            // SAFETY: a non-null value is a NUL-terminated string kept alive
            // by the concrete entry owner and bounded by this handle's borrow.
            Some(unsafe { CStr::from_ptr(value) })
        }
    }

    /// Field: git_config_entry.level
    /// Returns the validated source level of this entry.
    #[must_use]
    pub fn level(&self) -> Option<GitConfigLevel> {
        // SAFETY: this live handle permits a raw-place scalar read without
        // forming a reference to the C-visible entry.
        let level = unsafe { addr_of!((*self.as_ptr()).level).read() };
        GitConfigLevel::from_raw(level)
    }

    /// Field: git_config_entry.include_depth
    /// Returns the include nesting depth at which this entry was read.
    #[must_use]
    pub fn include_depth(&self) -> u32 {
        // SAFETY: this live handle permits a raw-place scalar read without
        // forming a reference to the C-visible entry.
        unsafe { addr_of!((*self.as_ptr()).include_depth).read() }
    }

    /// Field: git_config_entry.origin_path
    /// Returns the optional path from which this entry was read.
    #[must_use]
    pub fn origin_path(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the C-visible entry.
        let path = unsafe { addr_of!((*self.as_ptr()).origin_path).read() };
        if path.is_null() {
            None
        } else {
            // SAFETY: a non-null origin path is a NUL-terminated string kept
            // alive by the concrete entry owner and bounded by this borrow.
            Some(unsafe { CStr::from_ptr(path) })
        }
    }

    /// Field: git_config_entry.backend_type
    /// Returns the backend kind that supplied this entry.
    #[must_use]
    pub fn backend_type(&self) -> &'a CStr {
        // SAFETY: every live backend entry has a non-null NUL-terminated
        // backend kind kept alive by its owner. Raw-place projection forms no
        // reference to the C-visible entry itself.
        unsafe { CStr::from_ptr(addr_of!((*self.as_ptr()).backend_type).read()) }
    }
}

#[cfg(test)]
mod entry_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn config_entry_wrapper_preserves_the_c_layout() {
        assert_eq!(
            size_of::<GitConfigEntry>(),
            size_of::<ffi::git_config_entry>()
        );
        assert_eq!(
            align_of::<GitConfigEntry>(),
            align_of::<ffi::git_config_entry>()
        );
        assert_eq!(
            size_of::<GitConfigEntryRef<'_>>(),
            size_of::<*const ffi::git_config_entry>()
        );
        assert_eq!(
            size_of::<GitConfigEntryMut<'_>>(),
            size_of::<*mut ffi::git_config_entry>()
        );
    }

    #[test]
    fn config_entry_handle_reads_required_and_optional_fields() {
        let mut raw = ffi::git_config_entry {
            name: c"core.bare".as_ptr(),
            value: c"true".as_ptr(),
            backend_type: c"file".as_ptr(),
            origin_path: c"/repo/.git/config".as_ptr(),
            include_depth: 2,
            level: GitConfigLevel::LOCAL.as_raw(),
        };

        // SAFETY: `raw` and all string literals remain live for the handle's
        // complete use, and this is the only handle accessing the local value.
        let entry = unsafe { GitConfigEntryMut::from_ptr(&raw mut raw) }
            .expect("the address of a local entry is non-null");
        let shared = entry.as_ref();
        assert_eq!(shared.name(), c"core.bare");
        assert_eq!(shared.value(), Some(c"true"));
        assert_eq!(shared.backend_type(), c"file");
        assert_eq!(shared.origin_path(), Some(c"/repo/.git/config"));
        assert_eq!(shared.include_depth(), 2);
        assert_eq!(shared.level(), Some(GitConfigLevel::LOCAL));

        raw.value = core::ptr::null();
        raw.origin_path = core::ptr::null();
        raw.level = 1;
        // SAFETY: the prior handle is no longer used, and `raw` remains live
        // with valid null optional fields and an initialized integer level.
        let entry = unsafe { GitConfigEntryRef::from_ptr(&raw mut raw) }
            .expect("the address of a local entry is non-null");
        assert_eq!(entry.value(), None);
        assert_eq!(entry.origin_path(), None);
        assert_eq!(entry.level(), None);
    }
}

fn config_result(status: i32, config: Option<GitConfigOwned>) -> Result<GitConfigOwned, i32> {
    if status == 0 {
        Ok(config.expect("libgit2 succeeded without returning a config"))
    } else {
        drop(config);
        Err(status)
    }
}

/// Wraps: git_config_add_file_ondisk
/// Adds a file backend without a repository-dependent include context.
pub fn git_config_add_file_ondisk(
    config: &mut GitConfigMut<'_>,
    path: &CStr,
    level: GitConfigLevel,
    force: bool,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `path` is a live C string.
    // Passing null guarantees the backend stores no repository borrow.
    let status = unsafe {
        ffi::git_config_add_file_ondisk(
            config.as_mut_ptr(),
            path.as_ptr(),
            level.as_raw(),
            core::ptr::null(),
            i32::from(force),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_add_file_ondisk
/// Adds a file backend that retains `repo` for conditional-include matching.
///
/// # Safety
///
/// `repo` must remain alive until `config` and every config sharing the added
/// backend have been freed. The existing config type cannot express a lifetime
/// introduced by mutating it in place.
pub unsafe fn git_config_add_file_ondisk_with_repository(
    config: &mut GitConfigMut<'_>,
    path: &CStr,
    level: GitConfigLevel,
    repo: crate::repository::GitRepositoryRef<'_>,
    force: bool,
) -> Result<(), i32> {
    // SAFETY: typed handles and `path` are live for the call; the caller
    // upholds the backend's longer-lived repository-pointer obligation.
    let status = unsafe {
        ffi::git_config_add_file_ondisk(
            config.as_mut_ptr(),
            path.as_ptr(),
            level.as_raw(),
            repo.as_ptr(),
            i32::from(force),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_delete_entry
/// Deletes the highest-priority writable value for `name`.
pub fn git_config_delete_entry(config: &mut GitConfigMut<'_>, name: &CStr) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `name` is a live C string.
    let status = unsafe { ffi::git_config_delete_entry(config.as_mut_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_delete_multivar
/// Deletes multivar values matching `regexp`.
pub fn git_config_delete_multivar(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    regexp: &CStr,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and both strings are live.
    let status = unsafe {
        ffi::git_config_delete_multivar(config.as_mut_ptr(), name.as_ptr(), regexp.as_ptr())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

fn find_config_path(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    call: unsafe extern "C" fn(*mut ffi::git_buf) -> core::ffi::c_int,
) -> Result<(), i32> {
    // SAFETY: `out` is live and exclusive and `call` is one of the scheduled
    // libgit2 path-finder functions with this exact output contract.
    let status = unsafe { call(out.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_find_global
/// Writes the discovered global configuration path.
pub fn git_config_find_global(out: &mut crate::api::buffer::GitBufMut<'_>) -> Result<(), i32> {
    find_config_path(out, ffi::git_config_find_global)
}

/// Wraps: git_config_find_system
/// Writes the discovered system configuration path.
pub fn git_config_find_system(out: &mut crate::api::buffer::GitBufMut<'_>) -> Result<(), i32> {
    find_config_path(out, ffi::git_config_find_system)
}

/// Wraps: git_config_find_xdg
/// Writes the discovered XDG configuration path.
pub fn git_config_find_xdg(out: &mut crate::api::buffer::GitBufMut<'_>) -> Result<(), i32> {
    find_config_path(out, ffi::git_config_find_xdg)
}

/// Wraps: git_config_get_bool
/// Reads and parses a boolean configuration value.
pub fn git_config_get_bool(config: GitConfigRef<'_>, name: &CStr) -> Result<bool, i32> {
    let mut out = 0;
    // SAFETY: the config and string are live and `out` is writable.
    let status = unsafe { ffi::git_config_get_bool(&mut out, config.as_ptr(), name.as_ptr()) };
    if status == 0 {
        Ok(out != 0)
    } else {
        Err(status)
    }
}

/// Wraps: git_config_get_int32
/// Reads and parses a 32-bit integer configuration value.
pub fn git_config_get_int32(config: GitConfigRef<'_>, name: &CStr) -> Result<i32, i32> {
    let mut out = 0;
    // SAFETY: the config and string are live and `out` is writable.
    let status = unsafe { ffi::git_config_get_int32(&mut out, config.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_config_get_int64
/// Reads and parses a 64-bit integer configuration value.
pub fn git_config_get_int64(config: GitConfigRef<'_>, name: &CStr) -> Result<i64, i32> {
    let mut out = 0;
    // SAFETY: the config and string are live and `out` is writable.
    let status = unsafe { ffi::git_config_get_int64(&mut out, config.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_config_get_path
/// Expands and writes a path-valued configuration entry.
pub fn git_config_get_path(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    config: GitConfigRef<'_>,
    name: &CStr,
) -> Result<(), i32> {
    // SAFETY: both handles and `name` are live, and `out` is exclusive.
    let status =
        unsafe { ffi::git_config_get_path(out.as_mut_ptr(), config.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_get_string
/// Borrows a string from a read-only snapshot config.
pub fn git_config_get_string<'a>(config: GitConfigRef<'a>, name: &CStr) -> Result<&'a CStr, i32> {
    let mut out = core::ptr::null();
    // SAFETY: the config and name are live and `out` is writable. Success
    // returns a string retained by the snapshot config.
    let status = unsafe { ffi::git_config_get_string(&mut out, config.as_ptr(), name.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    debug_assert!(!out.is_null());
    // SAFETY: success returns a non-null NUL-terminated string retained by `config`.
    Ok(unsafe { CStr::from_ptr(out) })
}

/// Wraps: git_config_get_string_buf
/// Copies a string-valued entry into `out`.
pub fn git_config_get_string_buf(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    config: GitConfigRef<'_>,
    name: &CStr,
) -> Result<(), i32> {
    // SAFETY: both handles and `name` are live, and `out` is exclusive.
    let status =
        unsafe { ffi::git_config_get_string_buf(out.as_mut_ptr(), config.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_new
/// Allocates an empty configuration object.
pub fn git_config_new() -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable owner output slot.
    let status = unsafe { ffi::git_config_new(&mut out) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_open_default
/// Opens the default prioritized configuration stack.
pub fn git_config_open_default() -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable owner output slot.
    let status = unsafe { ffi::git_config_open_default(&mut out) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_open_global
/// Opens the writable global or XDG backend from `config`.
pub fn git_config_open_global(config: GitConfigRef<'_>) -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `config` is live and source inspection shows it is only read;
    // `out` is writable. The returned config refcounts its backend instance.
    let status = unsafe { ffi::git_config_open_global(&mut out, config.as_ptr().cast_mut()) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_open_level
/// Opens a focused configuration object for one priority level.
pub fn git_config_open_level(
    parent: GitConfigRef<'_>,
    level: GitConfigLevel,
) -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `parent` is live, `out` is writable, and a checked level is passed.
    let status = unsafe { ffi::git_config_open_level(&mut out, parent.as_ptr(), level.as_raw()) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_open_ondisk
/// Opens one on-disk configuration file.
pub fn git_config_open_ondisk(path: &CStr) -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `path` is live and `out` is writable.
    let status = unsafe { ffi::git_config_open_ondisk(&mut out, path.as_ptr()) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_set_bool
/// Stores a boolean in the highest-priority writable backend.
pub fn git_config_set_bool(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    value: bool,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `name` is live.
    let status =
        unsafe { ffi::git_config_set_bool(config.as_mut_ptr(), name.as_ptr(), i32::from(value)) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_set_int32
/// Stores a 32-bit integer in the highest-priority writable backend.
pub fn git_config_set_int32(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    value: i32,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `name` is live.
    let status = unsafe { ffi::git_config_set_int32(config.as_mut_ptr(), name.as_ptr(), value) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_set_int64
/// Stores a 64-bit integer in the highest-priority writable backend.
pub fn git_config_set_int64(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    value: i64,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `name` is live.
    let status = unsafe { ffi::git_config_set_int64(config.as_mut_ptr(), name.as_ptr(), value) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_set_multivar
/// Replaces every value matching `regexp` in the highest-priority writable
/// backend. Libgit2 reads all strings only for the duration of the call.
pub fn git_config_set_multivar(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    regexp: &CStr,
    value: &CStr,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies a live config and all three
    // arguments are live NUL-terminated strings that libgit2 does not retain.
    let status = unsafe {
        ffi::git_config_set_multivar(
            config.as_mut_ptr(),
            name.as_ptr(),
            regexp.as_ptr(),
            value.as_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_set_string
/// Copies a string into the highest-priority writable backend.
pub fn git_config_set_string(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    value: &CStr,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies a live config and both strings
    // remain live and NUL-terminated throughout this non-retaining call.
    let status =
        unsafe { ffi::git_config_set_string(config.as_mut_ptr(), name.as_ptr(), value.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_snapshot
/// Creates an independently owned, read-only snapshot of `config`.
pub fn git_config_snapshot(config: GitConfigRef<'_>) -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and `config` is a live shared handle. The C
    // declaration predates const-correctness but snapshotting only reads the
    // source's reader vector and asks each backend for a snapshot. The
    // snapshot backend does record the source backend pointer, but consults it
    // solely in the `open` that `git_config_add_backend` performs inside this
    // call, and copies out every entry there; the result is independent of the
    // source once this call returns.
    let status = unsafe { ffi::git_config_snapshot(&mut out, config.as_ptr().cast_mut()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one fresh owned config count, released by the
    // `GitConfig` drop contract.
    unsafe { GitConfigOwned::from_raw(out) }.ok_or(-1)
}
